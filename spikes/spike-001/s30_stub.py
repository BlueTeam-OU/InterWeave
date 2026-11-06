# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S30 stub: s29_stub.py, unchanged, except that with
``SPIKE_EACH`` set each event waits for a trigger of its own -- the
event numbered k is written once ``<SPIKE_TRIGGER>.<k>`` exists -- so the
driver can let every event's turn end before the next one arrives.
Without ``SPIKE_EACH`` it is s29_stub.py. EVIDENCE ONLY.
"""

import os
import sys
import threading
import time

# Run under `python3 -I`, which leaves the script's own directory off the path.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import s29_stub  # noqa: E402

EACH = bool(os.environ.get("SPIKE_EACH"))


def events_each_on_trigger():
    def run():
        for index, size in enumerate(s29_stub.SIZES, 1):
            path = f"{s29_stub.TRIGGER}.{index}"
            while not os.path.exists(path):
                time.sleep(0.1)
            s29_stub.record("event", f"trigger {index} seen; size={size}")
            line, chars = s29_stub.event_line(index, size)
            s29_stub.write_line(line, {"method": "notifications/claude/channel", "index": index,
                                       "content_chars": chars})
    threading.Thread(target=run, daemon=True).start()


if __name__ == "__main__":
    if EACH:
        s29_stub.events_on_trigger = events_each_on_trigger
    s29_stub.main()
