# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S36 (2): S33's, S34's and S35's reply cells on a build
after the fleet's pin, 2.1.322, with the real claude-channel bridge
across two daemons.

    PATH=<the build's bin>:$PATH python3 s36_reply.py <told>-<message>-<arrival>-<N>
    python3 s36_reply.py table

s33_reply.py unchanged in what it measures (the release bridge behind
s33_proxy.py, S16's two daemons, the far peer on daemon B, the claude on
PATH), its runs written as ``runs/s36-reply-<cell>-<N>``. ``prompt`` and
``plugin`` are S33's and S34's cells (``s33_reply.spec_of``); ``promptw``,
``prompts`` and ``prompta`` are S35's (``s35_reply.spec_of``). The table
is S33's with S35's leak count beside it. EVIDENCE ONLY.
"""

import sys

import s33_reply
import s35_reply

TOLD = ("prompt", "plugin", "promptw", "prompts", "prompta")
_s33_spec = s33_reply.spec_of


def spec_of(name):
    if name.split("-", 1)[0] in ("promptw", "prompts", "prompta"):
        return s35_reply.spec_of(name)
    return _s33_spec(name)


def table():
    cells = {}
    for r in s33_reply.committed_runs("s36"):
        if not r.get("passed"):
            continue
        c = cells.setdefault(r["cell"], {"runs": 0, "replies": 0, "after": [], "joined": 0, "leaks": 0,
                                         "tokens": 0})
        c["runs"] += 1
        c["replies"] += bool(r["outcome"]["replied"])
        c["joined"] += r["outcome"]["joined_running_turn"]
        c["leaks"] += any(s35_reply.LEAK.search(t or "") for t in r["peer_received"])
        if r["outcome"]["reply_after_s"] is not None:
            c["after"].append(r["outcome"]["reply_after_s"])
        for res in r["results"]:
            u = res.get("usage") or {}
            c["tokens"] += sum(u.get(k) or 0 for k in ("input_tokens", "cache_creation_input_tokens",
                                                        "cache_read_input_tokens", "output_tokens"))
    print("| Cell | Runs | Joined a running turn | Replies | Leaks | Median s to reply | Tokens |")
    print("|---|---|---|---|---|---|---|")
    for cell in sorted(cells):
        c = cells[cell]
        after = sorted(c["after"])
        n = len(after)
        median = None if not n else after[n // 2] if n % 2 else (after[n // 2 - 1] + after[n // 2]) / 2
        print(f"| `{cell}` | {c['runs']} | {c['joined']} | {c['replies']} | {c['leaks']} | "
              f"{'-' if median is None else f'{median:.1f}'} | {c['tokens']:,} |")
    print(f"\ntokens: {sum(c['tokens'] for c in cells.values()):,}")


if __name__ == "__main__":
    if sys.argv[1:] == ["table"]:
        table()
        raise SystemExit(0)
    s33_reply.spec_of = spec_of
    s33_reply.TOLD = TOLD
    raise SystemExit(s33_reply.main("s36"))
