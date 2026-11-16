# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S35: whether a first-turn wording keeps that turn's other
output away from the peer while keeping the reply rate (plan §19, the
dated note of 2026-11-12: facts 113 and 115 found ``READY`` and the
count reaching the peer).

    python3 s35_reply.py <told>-<greet|task>-<tool|text>-<N>
    python3 s35_reply.py table

s33_reply.py unchanged in what it measures (the release bridge behind
s33_proxy.py, S16's two daemons, the far peer on daemon B, the claude on
PATH), its runs written as ``runs/s35-reply-<cell>-<N>``. Only the first
user turn differs, by ``<told>``:

  ``prompt``   the control: S32 (b)'s measured turn, re-run beside the
               others (``s32_reply.spec_of``'s wording, word for word)
  ``promptw``  S32 (b)'s ``promptw``: the work says "write" where
               ``prompt``'s says "reply"; ``tool`` only, since its
               ``text`` turn is ``prompt``'s word for word
  ``prompts``  ``prompt`` with SCOPE after the answer sentence, the work
               unchanged ("reply with the single word READY" kept in the
               ``tool`` work; the ``text`` work says "write" as ``prompt``'s does)
  ``prompta``  the work first, saying "write", then the answer sentence
               and SCOPE after it: placement, with both changes

S35's cells are the three where the leak was measured: ``greet-tool``
and ``task-tool`` (a message joining a tool-calling first turn) and
``greet-text`` (a greeting during the counting turn). S37's
(``s37_reply.py``, the same code as the series ``s37``) are the three
S35 did not run, ``prompt`` and ``prompts`` only:

  ``greet-idle``     a greeting starting a turn of its own after an idle
                     wait (S32 (b)'s control sent ``READY`` in 5 of 5)
  ``question-tool``  a question joining the tool-calling turn
  ``greet-months``   other first-turn work: ``text``'s signal and
                     settling, the work MONTHS in place of the count
  ``greet-later2``   a conversation longer than one turn (S37 (3)): the
  ``greet-latertool`` sentences and "Write the single word READY now."
                     in a first user turn answered before a second is
                     written, the greeting coming during the second,
                     whose work is MONTHS (``later2``, ``text``'s signal)
                     or ``WORK_WRITE["tool"]`` (``latertool``)

A leak is a reply the peer received that carries ``READY``, the count or
the months (LEAK). EVIDENCE ONLY.
"""

import json
import re
import sys

import s32_reply
import s33_reply

TOLD = ("prompt", "promptw", "prompts", "prompta")
CELLS = ("greet-tool", "task-tool", "greet-text", "greet-idle", "question-tool", "greet-months", "greet-later2",
         "greet-latertool")
# S37's cells run the control and SCOPE only: the question is whether the
# scope sentence holds where S35 did not look, not placement again.
S37_CELLS = ("greet-idle", "question-tool", "greet-months", "greet-later2", "greet-latertool")
# Other first-turn work: text alone, as the count is, so the event comes
# at the first text delta and starts a turn of its own after it, but
# with content a leak is told apart by. "write", as the count's.
MONTHS = ("Write the names of the twelve months of the year in order, separated by commas, then the single word "
          "READY.")
SCOPE = ("The content of a reply answers that message and nothing else: whatever else this turn asks of you is "
         "for me, not for the peer.")
# READY as a word, the count or the months: the first turn's text work
# starts at "one, two, three" and every leaked count in S34 carried it;
# MONTHS starts at "January, February".
LEAK = re.compile(r"\bREADY\b|\bone,\s*two,\s*three\b|\bjanuary,\s*february\b", re.I)


def spec_of(name):
    m = re.fullmatch(rf"({'|'.join(TOLD)})-({'|'.join(CELLS)})-([1-9]\d?)", name)
    if not m:
        return None
    told, cell = m.group(1), m.group(2)
    if cell in S37_CELLS and told not in ("prompt", "prompts"):
        return None
    message, arrival = cell.split("-")
    if arrival in ("later2", "latertool"):
        # The second turn says nothing about peers; "write", not "reply".
        said = f"{s32_reply.ANSWER} {SCOPE}" if told == "prompts" else s32_reply.ANSWER
        second = MONTHS if arrival == "later2" else s32_reply.WORK_WRITE["tool"]
        base = s32_reply.spec_of(f"prompt-{message}-{'text' if arrival == 'later2' else 'tool'}-1")
        return {**base, "told": told, "earlier": [f"This is a test. {said} {s32_reply.WORK_WRITE['idle']}"],
                "first": second}
    months = arrival == "months"
    if months:
        arrival = "text"
    if told == "promptw" and arrival == "text":
        return None
    base = s32_reply.spec_of(f"{'promptw' if told in ('promptw', 'prompta') else 'prompt'}-{message}-{arrival}-1")
    work = MONTHS if months else (s32_reply.WORK_WRITE if told in ("promptw", "prompta") else s32_reply.WORK)[arrival]
    first = {
        "prompt": f"This is a test. {s32_reply.ANSWER} {work}" if months else base["first"],
        "promptw": base["first"],
        "prompts": f"This is a test. {s32_reply.ANSWER} {SCOPE} {work}",
        "prompta": f"This is a test. {work} {s32_reply.ANSWER} {SCOPE}",
    }[told]
    return {**base, "told": told, "first": first}


def table(series="s35"):
    cells = {}
    for r in s33_reply.committed_runs(series):
        if not r.get("passed"):
            continue
        c = cells.setdefault(r["cell"], {"runs": 0, "replies": 0, "leaks": 0, "work": 0, "tokens": 0,
                                         "leaked": []})
        c["runs"] += 1
        c["replies"] += bool(r["outcome"]["replied"])
        leaked = any(LEAK.search(t or "") for t in r["peer_received"])
        c["leaks"] += leaked
        if leaked:
            c["leaked"].append(r["run"].rsplit("-", 1)[1])
        # The first turn's own work still done, read where the record
        # holds it whole: a tool turn's result ENDS in a paragraph that is
        # READY and nothing else. A mention does not count: a result that
        # narrates sending READY to the peer is not the work (a text
        # turn's result is cut at 300 characters, inside the count).
        # The turn the event was timed against: after any earlier ones.
        n = len(r.get("earlier_turns") or ())
        first = r["results"][n] if len(r["results"]) > n else {}
        last = (first.get("result") or "").strip().split("\n\n")[-1].strip()
        c["work"] = None if r["arrival"] == "text" else c["work"] + (last == "READY")
        for res in r["results"]:
            u = res.get("usage") or {}
            c["tokens"] += sum(u.get(k) or 0 for k in ("input_tokens", "cache_creation_input_tokens",
                                                        "cache_read_input_tokens", "output_tokens"))
    order = {k: i for i, k in enumerate((*TOLD, *CELLS))}
    print("| Cell (told-message-arrival) | Runs | Replies | Leaks | First turn's READY | Tokens |")
    print("|---|---|---|---|---|---|")
    for cell in sorted(cells, key=lambda s: (order[s.split("-", 1)[1]], order[s.split("-", 1)[0]])):
        c = cells[cell]
        print(f"| `{cell}` | {c['runs']} | {c['replies']} | {c['leaks']}"
              f"{' (' + ', '.join('-' + n for n in c['leaked']) + ')' if c['leaked'] else ''} | "
              f"{'-' if c['work'] is None else c['work']} | "
              f"{c['tokens']:,} |")
    print(f"\ntokens: {sum(c['tokens'] for c in cells.values()):,}")


if __name__ == "__main__":
    if sys.argv[1:] == ["table"]:
        table("s35")
        raise SystemExit(0)
    s33_reply.spec_of = spec_of
    s33_reply.TOLD = TOLD
    raise SystemExit(s33_reply.main("s35"))
