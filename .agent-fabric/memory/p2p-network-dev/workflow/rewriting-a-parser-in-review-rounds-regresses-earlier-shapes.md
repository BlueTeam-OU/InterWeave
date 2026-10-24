---
role: "p2p-network-dev"
class: workflow
topic: "rewriting-a-parser-in-review-rounds-regresses-earlier-shapes"
description: "When a review round rewrites a hand-rolled reader (a tokenizer, a scan), run every input an earlier round probed through old and new before committing; #381 lost a handled shape three times"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8ebcf9bd42f5aba7
---

## When a review round rewrites a hand-rolled reader (a tokenizer, a scan), run every input an earlier round probed through old and new before committing; #381 lost a handled shape three times

#381 (tools/ci/release_notices.py, 2026-10-22) rewrote its licence-word and SPDX readers in five review rounds. Three of the fail-open findings were inputs an earlier head already handled, each lost by the rewrite that fixed something else:
- "MIT/Frob License": refused at 028c7252, passing at d680b743, once "/" joined the word class.
- "MIT / EPL-2.0": refused at 9db7d6d9, dropped at 6e418f13, once the expression ended at any non-operator.
- "( MIT ) OR WTFPL": read whole at 6e418f13, dropped at adf61bfc, once a lone ")" counted as an end.

Each was reproduced by loading both revisions in memory and comparing the outputs. That comparison was never run before a commit, only by the reviewer afterwards.

**Why:** a fix round reasons about the new finding's shape. The shapes earlier rounds settled live only in the test file and the review history. A test pins the one case it names, so a shape a reviewer probed by hand and nobody wrote down has nothing guarding it.

**How to apply:**
- Before committing a rewrite of a reader, keep a probe table: every input any round's review or judge named, from the PR comments.
- Run the table through the old revision (`git show <sha>:path`, exec'd in memory) and the new one, and read each line whose output changed.
- A change that turns a refusal into an acceptance is a regression until shown otherwise.
- Every probe a reviewer reproduced becomes a self-test case in the same commit.
- If the reader is a tripwire under a stronger gate (here the header digests, ADR-0058 A 2026-10-22 (iii)), prefer the smallest edit to a rewrite.

Related: [[derived-notice-rules-must-follow-the-record-literally]], [[review-loop-same-invariant-escalate]].

*References: derived-notice-rules-must-follow-the-record-literally, review-loop-same-invariant-escalate*

*Anchors: `tools/ci/release_notices.py::spdx_expression`, `tools/ci/release_notices.py::unrecognised_names`*

*Observed 2026-10-22 (p2p-network-dev)*
