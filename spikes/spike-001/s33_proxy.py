# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S33 proxy: the real claude-channel bridge, behind a pipe
that records every line in s32_reply_stub.py's record shape, so S32 (b)'s
readers (s32_reply.stub_record, calls_of, event_of) read a real bridge's
exchange as they read the stub's. EVIDENCE ONLY.

    python3 -I s33_proxy.py <log> <bridge> [bridge args...]

Every line passes through whole and in order, with two exceptions, each
set by the driver and each S32 (b)'s stub's own behaviour:

- ``SPIKE_EXTRA_INSTRUCTIONS``: appended, after a newline, to the
  ``instructions`` of the bridge's ``initialize`` result (S32's
  ``plugin`` cell). The bridge carries no such sentence
  (``architecture/plugin/INSTRUCTIONS.md``), so this is the one way to
  put it there without a second build of the bridge.
- ``SPIKE_TOOL_DELAY_MS``: the bridge's answer to a ``status`` call is
  held that long before it goes on (S32's ``tool`` arrival answered each
  ``status`` after 3 s). Every other line, a channel notification among
  them, is not held behind it.

What is recorded is when a line left the proxy, no earlier than its
writer wrote it.
"""

import json
import os
import signal
import subprocess
import sys
import threading
import time

LOG = open(sys.argv[1], "a", buffering=1)
EXTRA = os.environ.get("SPIKE_EXTRA_INSTRUCTIONS")
DELAY_S = int(os.environ.get("SPIKE_TOOL_DELAY_MS", "0")) / 1000
WRITE = threading.Lock()
LOG_LOCK = threading.Lock()
STATUS_IDS = set()


def record(direction, body):
    # Two threads log; one lock keeps each record a line of its own, as
    # the stub's LOCK does (PR #628's review).
    with LOG_LOCK:
        LOG.write(json.dumps({"wall": time.time(), "dir": direction, "body": body}) + "\n")


def parsed(line):
    try:
        return json.loads(line)
    except ValueError:
        return line.decode("utf-8", "replace")


def to_host(line, body):
    with WRITE:
        try:
            sys.stdout.buffer.write(line)
            sys.stdout.buffer.flush()
        except BrokenPipeError:
            # The host is gone; the line reached nobody, and says so.
            record("undelivered", body)
            return
        record("out", body)


def host_to_bridge(bridge):
    for line in iter(sys.stdin.buffer.readline, b""):
        body = parsed(line)
        if isinstance(body, dict) and body.get("method") == "tools/call" and \
                (body.get("params") or {}).get("name") == "status":
            STATUS_IDS.add(body.get("id"))
        record("in", body)
        bridge.stdin.write(line)
        bridge.stdin.flush()
    # The host closed our stdin: close the bridge's, as the host would.
    bridge.stdin.close()


def main():
    bridge = subprocess.Popen(sys.argv[2:], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    # The host's signal is the bridge's: forwarded, so the bridge ends as
    # it would were it the host's child, and the proxy outlives it to
    # pass its last lines on.
    for sig in (signal.SIGINT, signal.SIGTERM):
        signal.signal(sig, lambda s, _f: bridge.send_signal(s))
    threading.Thread(target=host_to_bridge, args=(bridge,), daemon=True).start()
    for line in iter(bridge.stdout.readline, b""):
        body = parsed(line)
        if isinstance(body, dict):
            result = body.get("result")
            if EXTRA and isinstance(result, dict) and isinstance(result.get("instructions"), str):
                result["instructions"] += "\n" + EXTRA
                line = (json.dumps(body) + "\n").encode()
            if DELAY_S and body.get("id") in STATUS_IDS and "method" not in body:
                STATUS_IDS.discard(body.get("id"))
                threading.Timer(DELAY_S, to_host, (line, body)).start()
                continue
        to_host(line, body)
    code = bridge.wait()
    # A held status answer still due goes out before the proxy exits.
    for t in threading.enumerate():
        if isinstance(t, threading.Timer):
            t.join()
    sys.exit(code)


if __name__ == "__main__":
    main()
