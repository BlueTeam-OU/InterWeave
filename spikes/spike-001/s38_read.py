# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S38 (1), the read (the README's numbering): where a print session registers a
channel handler for a server that connects after a turn has started,
read from the build's own JavaScript. No session is started.

    python3 s38_read.py <path to a claude binary>   # runs/s38-read/read-<version>.json

What it records, each as the matched text so a reader can find it again
in the binary:

- ``wire_turn``: the print runner's per-turn wiring. It registers a
  channel handler for every client connected AT THAT MOMENT, and it is
  what logs "Channel notifications re-registered after reconnect" in a
  print session (S36, the review of #670).
- ``wire_turn_site``: where the runner calls it, once per batch of
  commands, i.e. once per turn.
- ``registration_callers``: the two hooks named ``registerChannelHandler``
  (the per-turn wiring) and ``onReconnected`` (the reconnect watcher),
  and whether they call one function.
- ``registration_calls``: every call of that function in the binary,
  matched as ``<name>(<id>,<its second argument>)``, each with the
  110 bytes before it, so a reader sees which hook each one is. The
  first read (#696's review, F2) recorded the two hooks alone and the
  README then called the watcher "the only other caller"; there are
  four.
- ``reconnect_watcher_transports``: the watcher's own transport test,
  which leaves out ``stdio`` and ``sdk``; a plugin channel server is
  ``stdio``.
- ``prewait_deadline_expr`` and ``prewait_default_ms_candidates``: the
  expression giving the first command's deadline for pending MCP
  servers, and every number assigned anywhere in the binary to its
  default's name. Minified names are reused across scopes, so these are
  candidates, not the resolved default.

Names are matched as ``[\\w$]+``: minified names carry ``$`` (S36).
EVIDENCE ONLY.
"""

import hashlib
import json
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
N = rb"[\w$]+"


def first(pattern, blob):
    m = re.search(pattern, blob)
    return m.group(0).decode("latin1") if m else None


def main() -> int:
    if len(sys.argv) != 2:
        raise SystemExit(f"usage: {sys.argv[0]} <claude binary>")
    path = pathlib.Path(sys.argv[1]).resolve()
    blob = path.read_bytes()
    version = subprocess.run([str(path), "--version"], capture_output=True, text=True,
                             env={"DISABLE_AUTOUPDATER": "1", "PATH": "/usr/bin:/bin"}).stdout.split(" ")[0]
    read = {"version": version, "sha256": hashlib.sha256(blob).hexdigest()}
    read["wire_turn"] = first(rb"wireTurn\(\)\{let " + N + rb"=this\.store\.getState\(\)[^}]{0,240}\}", blob)
    read["wire_turn_site"] = first(rb"wireTurn:\(\)=>" + N + rb"\.wireTurn\(\)", blob)
    read["batch_calls_wire_turn"] = first(rb"s\.enter\(\"tool_pool\"\);let " + N + rb"=" + N + rb"\.wireTurn\(" + N
                                          + rb"\)", blob)
    callers = re.findall(rb"(registerChannelHandler|onReconnected):\((" + N + rb")\)=>(" + N + rb")\(\2,(" + N
                         + rb")\)", blob)
    read["registration_callers"] = sorted({f"{k.decode()}:{f.decode()}" for k, _, f, _ in callers})
    read["registration_callers_share_one_function"] = len({f for _, _, f, _ in callers}) == 1
    if len({(f, a) for _, _, f, a in callers}) == 1:
        f, a = callers[0][2], callers[0][3]
        read["registration_calls"] = [
            blob[max(0, m.start() - 110):m.end()].decode("latin1")
            for m in re.finditer(rb"(?<![\w$.])" + re.escape(f) + rb"\(" + N + rb"," + re.escape(a) + rb"\)", blob)]
    # The watcher's own test, matched where it logs its hold-off line.
    at = blob.find(b"transport closed again soon after it connected: reconnecting in ${")
    while at != -1 and not re.search(rb"\|\|!" + N + rb"\(i\.config\)\)continue", blob[at:at + 1500]):
        at = blob.find(b"transport closed again soon after it connected: reconnecting in ${", at + 1)
    seg = blob[at:at + 1500] if at != -1 else b""
    test = re.search(rb"\|\|!(" + N + rb")\(i\.config\)\)continue", seg)
    body = re.search(rb"function " + re.escape(test.group(1)) + rb"\(n\)\{return [^}]{0,120}\}", seg) if test else None
    read["reconnect_watcher_skip"] = test.group(0).decode() if test else None
    read["reconnect_watcher_transports"] = body.group(0).decode() if body else None
    dl = re.search(rb"deadlineMs:" + N + rb"\?\?(" + N + rb")\}\);this\.firstCommandTaken", blob)
    read["prewait_deadline_expr"] = dl.group(0).decode() if dl else None
    if dl:
        values = sorted({int(v) for v in re.findall(rb"(?<![\w$.])" + re.escape(dl.group(1)) + rb"=(\d{1,6})(?!\d|n)",
                                                    blob)})
        read["prewait_default_ms_candidates"] = values
    out = HERE / "runs" / "s38-read"
    out.mkdir(parents=True, exist_ok=True)
    (out / f"read-{version}.json").write_text(json.dumps(read, indent=2) + "\n")
    print(json.dumps(read, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
