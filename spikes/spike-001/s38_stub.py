# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S38 stub: s23_stub.py's legacy channel server, with two
things the driver controls in place of S23's timer. EVIDENCE ONLY.

- ``SPIKE_GATE``: a path. The stub reads nothing from stdin until the
  path exists, so Claude Code's ``initialize`` waits and the connection
  completes when the driver says, not when the process happens to start.
  It is what lets S38 put the connection before or after a turn's start
  on purpose.
- ``SPIKE_TRIGGER``: a path. Each line appended to it is a nonce, and
  the stub sends one ``notifications/claude/channel`` event carrying it,
  in order, once the handshake is done. S23's own timed event is not
  sent.

Everything else (``SPIKE_LOG``, ``SPIKE_ERA``, the declared capabilities
and the tool) is s23_stub.py's, imported, so the server is the one S23
to S37 measured.
"""

import os
import pathlib
import sys
import threading
import time

# Run with -I, as every stub here is, so the script's own directory is
# not on the path: s23_stub.py beside it is added by name.
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import s23_stub  # noqa: E402

GATE = pathlib.Path(os.environ["SPIKE_GATE"])
TRIGGER = pathlib.Path(os.environ["SPIKE_TRIGGER"])
_started = threading.Event()


def watch_trigger():
    sent = 0
    while True:
        lines = TRIGGER.read_text().splitlines() if TRIGGER.exists() else []
        for nonce in lines[sent:]:
            s23_stub.send({"jsonrpc": "2.0", "method": "notifications/claude/channel",
                           "params": {"content": f"S38 {nonce}: a channel event on the {s23_stub.ERA} stub",
                                      "meta": {"chat_id": "s38", "era": s23_stub.ERA}}})
        sent = len(lines)
        time.sleep(0.2)


def notify_on_trigger():
    # Called by s23_stub at the end of the handshake, once per connection.
    if not _started.is_set():
        _started.set()
        threading.Thread(target=watch_trigger, daemon=True).start()


if __name__ == "__main__":
    s23_stub.record("event", f"gate: waiting for {GATE.name}")
    while not GATE.exists():
        time.sleep(0.05)
    s23_stub.record("event", "gate: open")
    s23_stub.notify_later = notify_on_trigger
    s23_stub.main()
