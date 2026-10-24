---
role: "rust-ui-dev"
class: workflow
topic: "commit-and-shell-traps"
description: "commit trailers must be one block, one line each; pkill -f kills the session's own shell; release templates must not carry the rights-reserved boilerplate"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d7bfba8b71115ec8
---

## commit trailers must be one block, one line each; pkill -f kills the session's own shell; release templates must not carry the rights-reserved boilerplate

Three traps met on j111 (2026-10-19), each cost a retry:
- **Trailers.** The commit-msg hook reads `Kind:`, `Answers:` and `Supplier-Review:` only as one trailer block at the end, each trailer on one line. A `Supplier-Review:` wrapped over two lines breaks the block and the commit is refused as having no kind.
- **pkill -f.** `pkill -f <pattern>` matches the session's own shell when the pattern is in its command line, and kills it (exit 144). Find the PID with `pgrep -af`, then `kill <pid>`.
- **Licence check on templates.** `tools/checks/check_license_headers.sh` refuses the rights-reserved boilerplate a notice usually carries, in any tracked file, so a release-page or notice template must describe the notice, not hold its text. Do not quote that boilerplate in a memory either: a drained slice is a tracked file, and the drain of 2026-10-24 was held for it (REQUEST 01a16d79).

**Why:** each cost a retry, and the quoted boilerplate held a drain.

**How to apply:** write trailers as one unwrapped block; kill by PID; describe a licence notice instead of quoting it.

*Anchors: `interweave:tools/checks/check_license_headers.sh::PROPRIETARY_RE`*

*Observed 2026-10-24 (rust-ui-dev)*
