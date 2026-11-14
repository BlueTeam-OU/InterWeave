# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
r"""SPIKE-001's S34 (1): fact 36's read, from the 2.1.322 binary, and
S36's from 2.1.325. READ, not measured: no session is started.

    python3 s34_gate.py [<binary>]    # default: the claude on PATH

The binary embeds its JavaScript as text beside its bytecode. This finds
the channel gate by its era refusal's reason, prints the ``kind`` of each
check in the order the function returns them, the session notice's era
text, and every connection filter that names the permission relay's
``claude/channel/permission`` capability with its era clause. The
minified names change between builds, so nothing is found by them, and
a name is matched as ``[\w$]+`` (a minified name may hold ``$``; see
``hard_skip_head`` in the 2.1.325 read). It ASSUMES that a handler for
``notifications/claude/channel`` is registered with the notification's
schema object, called as ``<schema>()``: each such call is listed with
the gate call before it in the same function, the era argument that
call passes, and the ``Channel notifications`` and ``channel_enable``
log strings between its keyword and where a brace count closes it
(``function_end``, whose limits its docstring states). A handler registered otherwise, through a second schema
object or a reference to it, is not seen. The output carries the tree
it was read at (``tree_stamp``).
Prints JSON; exits 1 if a piece is not found. EVIDENCE ONLY.
"""

import hashlib
import json
import pathlib
import re
import shutil
import sys

from s28_era import tree_stamp

ERA_REASON = b'reason:"connection negotiated a modern protocol revision with no unsolicited notification path"'
NOTICE = re.compile(rb'kind==="era"\?`(Channel messages from "\$\{[^}]+\}" are unavailable: [^`]*)`')
N = rb"[\w$]+"  # a minified name: 2.1.325 has one starting with $
RELAY = re.compile(rb'function ' + N + rb'\(' + N + rb',' + N + rb',' + N + rb'\)\{return ' + N
                   + rb'\.filter\(\(' + N + rb'\)=>[^;]{0,400}?'
                   rb'"claude/channel/permission"[^;]{0,200}?protocolEra[^;]{0,40}?\)\}')


def function_end(data: bytes, fn: int) -> int:
    """The offset after the closing brace of the function whose
    ``function`` keyword is at ``fn``: braces counted from its opening
    one, skipping '...' and "..." literals and a template's text (its
    ``${...}`` counted as code). A regular-expression literal or a
    comment holding a brace or a quote is not recognised, so a count can
    end early or late. ``schema_call_inside`` flags only an early end
    (before the schema call); a late one is not detected, so each site
    also records ``end_context``, the bytes either side of the end, for
    a reader to check that a function ends there and another begins."""
    i, stack = data.find(b"{", fn), []  # each entry: "{" code brace, "$" a template's ${
    n = len(data)
    while i < n:
        c = data[i]
        if c in (0x27, 0x22):  # ' or "
            i += 1
            while i < n and data[i] != c:
                i += 2 if data[i] == 0x5C else 1
        elif c == 0x60 or (c == 0x7D and stack and stack[-1] == "$"):  # ` opens a template, } ends a ${
            if c == 0x7D:
                stack.pop()
            i += 1
            while i < n and data[i] != 0x60:
                if data[i] == 0x5C:
                    i += 2
                    continue
                if data[i] == 0x24 and data[i + 1] == 0x7B:  # ${
                    stack.append("$")
                    i += 1
                    break
                i += 1
        elif c == 0x7B:
            stack.append("{")
        elif c == 0x7D:
            stack.pop()
            if not stack:
                return i + 1
        i += 1
    return n


def registration_sites(data: bytes, gate: bytes) -> list:
    """Each call of the channel notification's schema object (assumed to
    be what every handler for it is registered with), the last call of
    the gate before it in the same function, the era argument it passes,
    and the function's log strings (``log_lines``)."""
    name = re.match(rb"function (" + N + rb")", gate).group(1)
    schema = re.search(rb"var (" + N + rb")=" + N + rb'\(\(\)=>' + N + rb'\(\{method:' + N
                       + rb'\("notifications/claude/channel"\)', data)
    if not schema:
        return []
    sites = []
    for use in re.finditer(rb"(?<![\w$])" + re.escape(schema.group(1)) + rb"\(\)", data):
        fn = data.rfind(b"function ", 0, use.start())
        call = None
        for call in re.finditer(rb"(?<![\w$])" + re.escape(name) + rb"\(([^()]*(?:\([^()]*\))?[^()]*)\)",
                                data[fn:use.start()]):
            pass
        if call is None or data[fn:].startswith(b"function " + name + b"("):
            sites.append({"enclosing": data[fn:fn + 40].decode(), "log_lines": None, "gate_call": None,
                          "gate_era_arg": None})
            continue
        args = call.group(1).decode()
        body_end = function_end(data, fn)
        logs = sorted({m.decode() for m in re.findall(rb"Channel notifications [a-z -]+|channel_enable[a-z ;]+",
                                                       data[fn:body_end])})
        sites.append({"enclosing": data[fn:data.find(b"{", fn)].decode(), "log_lines": logs,
                      "function_bytes": body_end - fn, "schema_call_inside": body_end > use.end(),
                      "end_context": data[body_end - 40:body_end + 40].decode("utf-8", "replace"),
                      "gate_call": f"{name.decode()}({args})",
                      "gate_era_arg": args.split(",")[-1].endswith(".protocolEra")})
    return sites


def main() -> int:
    binary = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else shutil.which("claude")).resolve()
    data = binary.read_bytes()
    out = {**tree_stamp(), "binary": binary.name, "sha256": hashlib.sha256(data).hexdigest(), "read_not_measured": True}

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
    out["gate_era_clause"] = re.search(rb'if\(' + N + rb'==="modern"\)return\{action:"skip",kind:"era"', gate) is not None
    # The kinds whose skip also drops a handler registered before (an
    # earlier connection of the same server): the era is one of them.
    hard = re.search(rb'function ' + N + rb'\(' + N + rb'\)\{return ' + N + rb'==="provider"\|\|[^}]*\}', data)
    out["hard_skip_head"] = hard.group(0)[:hard.group(0).find(b"{")].decode() if hard else None
    out["hard_skip_kinds"] = re.findall(r'"(\w+)"', hard.group(0).decode()) if hard else None

    out["registration_sites"] = registration_sites(data, gate)

    notices = sorted({m.group(1).decode() for m in NOTICE.finditer(data)})
    out["era_notice"] = notices
    out["era_reason_copies"] = data.count(ERA_REASON)
    relays = sorted({m.group(0).decode() for m in RELAY.finditer(data)})
    out["permission_relay_filters"] = relays
    out["relay_excludes_modern"] = bool(relays) and all('protocolEra!=="modern"' in r for r in relays)
    guarded = bool(out["registration_sites"]) and all(r["gate_era_arg"] and r["schema_call_inside"]
                                                       for r in out["registration_sites"])
    out["every_registration_after_gate_with_era"] = guarded
    print(json.dumps(out, indent=2))
    return 0 if notices and relays and out["gate_era_clause"] and guarded else 1


if __name__ == "__main__":
    raise SystemExit(main())
