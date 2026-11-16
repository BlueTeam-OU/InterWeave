# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S37 (1): S35's scope sentence in the three cells S35 did
not run, an idle wait, a question and other first-turn work, with the
control beside it (plan §19's S35 note; the README's S35 "Not
established").

    python3 s37_reply.py <prompt|prompts>-<greet-idle|question-tool|greet-months>-<N>
    python3 s37_reply.py table

s35_reply.py's cells and leak rule (``s35_reply.S37_CELLS``, LEAK), run
by s33_reply.py unchanged as the series ``s37``: the release bridge
behind s33_proxy.py, S16's two daemons, the far peer on daemon B, the
claude on PATH. Runs are written as ``runs/s37-reply-<cell>-<N>``.
EVIDENCE ONLY.
"""

import re
import sys

import s33_reply
import s35_reply

TOLD = ("prompt", "prompts")


def spec_of(name):
    m = re.fullmatch(rf"({'|'.join(TOLD)})-({'|'.join(s35_reply.S37_CELLS)})-([1-9]\d?)", name)
    return s35_reply.spec_of(name) if m else None


if __name__ == "__main__":
    if sys.argv[1:] == ["table"]:
        s35_reply.table("s37")
        raise SystemExit(0)
    s33_reply.spec_of = spec_of
    s33_reply.TOLD = TOLD
    raise SystemExit(s33_reply.main("s37"))
