# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S36 (2): S28's registration rows on a build after the
fleet's pin, 2.1.322 -- does a channel server that negotiates the
2026-07-28 revision register on 2.1.325?

    PATH=<the build's bin>:$PATH python3 s36_era.py a-modern|b-legacy

s28_era.py unchanged in what it measures (the claude on PATH, a private
``/etc`` with an empty ``claude-code``, s23_stub.py), its runs written
as ``runs/s36-<row>``. EVIDENCE ONLY.
"""

import s28_era

if __name__ == "__main__":
    raise SystemExit(s28_era.main("s36"))
