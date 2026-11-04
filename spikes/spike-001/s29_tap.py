# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S29 tap: sits on the bridge's stdout, passes every line
on to Claude Code as it arrives, and records each line's wall-clock time
and byte length, never its content. EVIDENCE ONLY.

    python3 s29_tap.py <log>

A channel notification is recorded with its ``payload_encoding`` and its
content's length in characters, so a run can say which representation
the bridge chose and how long the line the host took was. The tap adds a
pipe and a read loop between the bridge and the host; what it records
is when the line left the tap, which is no earlier than the bridge wrote
it.
"""

import json
import sys
import time


def main():
    log = open(sys.argv[1], "a", buffering=1)
    src, dst = sys.stdin.buffer, sys.stdout.buffer
    while True:
        line = src.readline()
        if not line:
            break
        dst.write(line)
        dst.flush()
        entry = {"wall": time.time(), "bytes": len(line.rstrip(b"\n"))}
        try:
            value = json.loads(line)
        except ValueError:
            entry["unparsed"] = True
        else:
            entry["method"] = value.get("method")
            if "id" in value:
                entry["id"] = value["id"]
            params = value.get("params") or {}
            if entry["method"] == "notifications/claude/channel":
                entry["content_chars"] = len(params.get("content", ""))
                entry["payload_encoding"] = (params.get("meta") or {}).get("payload_encoding")
        log.write(json.dumps(entry) + "\n")


if __name__ == "__main__":
    main()
