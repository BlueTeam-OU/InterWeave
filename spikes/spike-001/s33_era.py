# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S33 (1): S28's registration rows on the fleet's pin,
Claude Code 2.1.322 -- does a channel server that negotiates the
2026-07-28 revision register on a build after 2.1.317?

    python3 s33_era.py a-modern|b-legacy

s28_era.py unchanged in what it measures (the claude on PATH, a private
``/etc`` with an empty ``claude-code``, s23_stub.py), its runs written
as ``runs/s33-<row>``. EVIDENCE ONLY.
"""

import sys

import s28_era

if __name__ == "__main__":
    raise SystemExit(s28_era.main("s33"))
