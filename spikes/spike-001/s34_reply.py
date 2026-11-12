# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S34 (2): S32 (b)'s greet, question and task cells told
``prompt`` and ``plugin`` that S33 did not run, on the fleet's pin with
the real claude-channel bridge across two daemons.

    python3 s34_reply.py <prompt|plugin>-<greet|question|task>-<tool|text|idle>-<N>
    python3 s34_reply.py table

s33_reply.py unchanged in what it measures, its runs written as
``runs/s34-reply-<cell>-<N>`` and tabled apart from S33's. EVIDENCE ONLY.
"""

import s33_reply

if __name__ == "__main__":
    raise SystemExit(s33_reply.main("s34"))
