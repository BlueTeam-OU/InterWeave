# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S34 (1): fact 36's read, from the 2.1.322 binary. READ,
not measured: no session is started.

    python3 s34_gate.py [<binary>]    # default: the claude on PATH

The binary embeds its JavaScript as text beside its bytecode. This finds
the channel gate by its era refusal's reason, prints the ``kind`` of each
check in the order the function returns them, the session notice's era
text, and every connection filter that names the permission relay's
``claude/channel/permission`` capability with its era clause. The
minified names change between builds, so nothing is found by them.
Prints JSON; exits 1 if a piece is not found. EVIDENCE ONLY.
"""

import hashlib
import json
import pathlib
import re
import shutil
import sys

ERA_REASON = b'reason:"connection negotiated a modern protocol revision with no unsolicited notification path"'
NOTICE = re.compile(rb'Zn?\.kind==="era"\?`(Channel messages from "\$\{[^}]+\}" are unavailable: [^`]*)`'
                    rb'|kind==="era"\?`(Channel messages from "\$\{[^}]+\}" are unavailable: [^`]*)`')
RELAY = re.compile(rb'function \w+\(\w+,\w+,\w+\)\{return \w+\.filter\(\(\w+\)=>[^;]{0,400}?'
                   rb'"claude/channel/permission"[^;]{0,200}?protocolEra[^;]{0,40}?\)\}')


def main() -> int:
    binary = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else shutil.which("claude")).resolve()
    data = binary.read_bytes()
    out = {"binary": binary.name, "sha256": hashlib.sha256(data).hexdigest(), "read_not_measured": True}

    at = data.find(ERA_REASON)
    start = data.rfind(b"function ", 0, at)
    end = data.find(b'return{action:"register"}}', at)
    if at < 0 or start < 0 or end < 0:
        print(json.dumps({**out, "error": "gate not found"}, indent=2))
        return 1
    gate = data[start:end + len(b'return{action:"register"}}')]
    out["version"] = (re.findall(rb"// Version: ([0-9.]+)", data[max(0, start - 4000):start]) or [b"?"])[-1].decode()
    out["gate_head"] = gate[:gate.find(b"{")].decode()
    out["gate_checks_in_order"] = [k.decode() for k in re.findall(rb'kind:"(\w+)"', gate)]
    out["gate_era_clause"] = re.search(rb'if\(\w+==="modern"\)return\{action:"skip",kind:"era"', gate) is not None
    # The kinds whose skip also drops a handler registered before (an
    # earlier connection of the same server): the era is one of them.
    hard = re.search(rb'function \w+\(\w+\)\{return \w+==="provider"\|\|[^}]*\}', data)
    out["hard_skip_kinds"] = re.findall(r'"(\w+)"', hard.group(0).decode()) if hard else None

    notices = sorted({(m.group(1) or m.group(2)).decode() for m in NOTICE.finditer(data)})
    out["era_notice"] = notices
    out["era_reason_copies"] = data.count(ERA_REASON)
    relays = sorted({m.group(0).decode() for m in RELAY.finditer(data)})
    out["permission_relay_filters"] = relays
    out["relay_excludes_modern"] = bool(relays) and all('protocolEra!=="modern"' in r for r in relays)
    print(json.dumps(out, indent=2))
    return 0 if notices and relays and out["gate_era_clause"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
