# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S38 (1) rate and (2): S37's reply cells, with the release
bridge across two daemons, on the build on PATH -- how often a
``--channels`` print session starts with no channel, per build, and
S37's cells on 2.1.325.

    PATH=<the build's bin>:$PATH python3 s38_reply.py <prompt|prompts>-<cell>-<N>
    python3 s38_reply.py table

s37_reply.py's cells run by s33_reply.py unchanged, as the series
``s38-<build>``: runs are ``runs/s38-<build>-reply-<cell>-<N>``, one
build per run. Since S38 each run's ``inits`` carries init's
``mcp_servers`` (s29_print.init_record), so a run says whether the
bridge was still ``pending`` when the first turn started; the debug
log's registration line is in its evidence as before. The table adds,
per build, the sessions whose bridge never registered. EVIDENCE ONLY.
"""

import re
import sys

import s33_reply
import s35_reply
import s37_reply
from spike_common import claude_version

SERVER = "plugin:interweave:interweave"


def table():
    builds = sorted({p.name.split("-reply-")[0] for p in (s33_reply.HERE / "runs").glob("s38-2.*-reply-*")})
    print("| Build | Runs | First init: bridge pending | Never registered | Registered, pending at first init |")
    print("|---|---|---|---|---|")
    for series in builds:
        runs = pending = never = late = 0
        for path in sorted((s33_reply.HERE / "runs").glob(f"{series}-reply-*/evidence.md")):
            text = path.read_text()
            m = re.search(r"## Run\n\n```json\n(.*?)\n```", text, re.S)
            if not m or not s33_reply.json.loads(m.group(1)).get("passed"):
                continue
            r = s33_reply.json.loads(m.group(1))
            runs += 1
            first = (r.get("inits") or [{}])[0].get("mcp_servers") or []
            status = next((s["status"] for s in first if s["name"] == SERVER), None)
            registered = "Channel notifications re-registered after reconnect" in text or \
                "Channel notifications registered" in text
            pending += status == "pending"
            never += not registered
            late += registered and status == "pending"
        print(f"| {series[4:]} | {runs} | {pending} | {never} | {late} |")


if __name__ == "__main__":
    if sys.argv[1:] == ["table"]:
        table()
        for series in sorted({p.name.split("-reply-")[0] for p in (s33_reply.HERE / "runs").glob("s38-2.*-reply-*")}):
            print(f"\n{series}:")
            s35_reply.table(series)
        raise SystemExit(0)
    s33_reply.spec_of = s37_reply.spec_of
    s33_reply.TOLD = s37_reply.TOLD
    raise SystemExit(s33_reply.main("s38-" + claude_version()))
