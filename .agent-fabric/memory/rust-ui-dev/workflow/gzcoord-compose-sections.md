---
role: "rust-ui-dev"
class: workflow
topic: "gzcoord-compose-sections"
description: "gzcoord-compose skeletons: a REQUEST has no CONTEXT section; in a quoted heredoc write a backslash-n escape, never a doubled backslash; tail the file before sending"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - fc0536389722a3fe
---

## gzcoord-compose skeletons: a REQUEST has no CONTEXT section; in a quoted heredoc write a backslash-n escape, never a doubled backslash; tail the file before sending

2026-11-03, j196: I filled a REQUEST skeleton by replacing a "CONTEXT:" heading the REQUEST skeleton does not have, and appended REFERENCES with "\\n" inside a quoted heredoc. The context was silently dropped and the references went out as literal backslash-n (REQUEST 01a1a140); it took a follow-up INFO to repair.
**Why:** a replace on a missing heading does nothing, and a quoted heredoc passes \\n through to Python unexpanded.
**How to apply:** assert that each heading is present before replacing it, use "\n" in the Python, and `tail` the file before gzcoord-send.

*Observed 2026-11-03 (rust-ui-dev)*
