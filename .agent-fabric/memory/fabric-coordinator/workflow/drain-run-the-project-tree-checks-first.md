---
role: "fabric-coordinator"
class: workflow
topic: "drain-run-the-project-tree-checks-first"
description: "before committing an InterWeave memory drain, run the project's licence-header check on the result; lint.py does not see what the project's CI refuses"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: user
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - f651482ecb84bbc0
---

## before committing an InterWeave memory drain, run the project's licence-header check on the result; lint.py does not see what the project's CI refuses

The 2026-10-24 InterWeave drain (PR #410) went red on "tree checks": a rust-ui-dev
thread slice quoted the rights-reserved phrase that
`tools/checks/check_license_headers.sh` refuses in any tracked file, and
agent-fabric's `lint.py` and the assembler's hygiene pass both let it through.
The fix was a re-run from the base with `assemble.py --hold <role>/<class>:<topic>`
on that author's bundle only (a hold key naming no slice refuses the run), plus a
REQUEST to the author to reword and split it.

**Why:** a drain writes text no reviewer wrote into a public repository whose CI
scans every tracked file, so a quoted phrase in someone's memory turns the drain PR red.

**How to apply:** after assembling and before committing, run
`bash tools/checks/check_license_headers.sh` in the drain's working copy (or
`cargo xtask checks`; on this host `check_rustc_pin` and `check_unused_dependencies`
fail for host reasons). Re-run a drain from the base with `git checkout <base> -- .agent-fabric/memory`
rather than editing a slice. See [[drain-merge-target-is-an-exact-heading]].

*References: drain-merge-target-is-an-exact-heading*

*Anchors: `tools/checks/check_license_headers.sh::PROPRIETARY_RE`, `agent-fabric:tools/fabric/assemble.py::--hold`*

*Observed 2026-10-24 (fabric-coordinator)*
