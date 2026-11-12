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
               unchanged ("reply with the single word READY" kept)
  ``prompta``  the work first, saying "write", then the answer sentence
               and SCOPE after it: placement, with both changes

The cells are the three where the leak was measured: ``greet-tool`` and
``task-tool`` (a message joining a tool-calling first turn) and
``greet-text`` (a greeting during the counting turn). A leak is a reply
the peer received that carries ``READY`` or the count (LEAK). EVIDENCE
ONLY.
"""

import json
import re
import sys

import s32_reply
import s33_reply

TOLD = ("prompt", "promptw", "prompts", "prompta")
CELLS = ("greet-tool", "task-tool", "greet-text")
SCOPE = ("The content of a reply answers that message and nothing else: whatever else this turn asks of you is "
         "for me, not for the peer.")
# READY as a word, or the count: the first turn's text work starts at
# "one, two, three" and every leaked count in S34 carried it.
LEAK = re.compile(r"\bREADY\b|\bone,\s*two,\s*three\b", re.I)


def spec_of(name):
    m = re.fullmatch(rf"({'|'.join(TOLD)})-({'|'.join(CELLS)})-([1-9]\d?)", name)
    if not m:
        return None
    told, cell = m.group(1), m.group(2)
    message, arrival = cell.split("-")
    if told == "promptw" and arrival == "text":
        return None
    base = s32_reply.spec_of(f"{'promptw' if told in ('promptw', 'prompta') else 'prompt'}-{cell}-1")
    work = (s32_reply.WORK_WRITE if told in ("promptw", "prompta") else s32_reply.WORK)[arrival]
    first = {
        "prompt": base["first"],
        "promptw": base["first"],
        "prompts": f"This is a test. {s32_reply.ANSWER} {SCOPE} {work}",
        "prompta": f"This is a test. {work} {s32_reply.ANSWER} {SCOPE}",
    }[told]
    return {**base, "told": told, "first": first}


def table():
    cells = {}
    for r in s33_reply.committed_runs("s35"):
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
        first = r["results"][0] if r["results"] else {}
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
        table()
        raise SystemExit(0)
    s33_reply.spec_of = spec_of
    s33_reply.TOLD = TOLD
    raise SystemExit(s33_reply.main("s35"))
