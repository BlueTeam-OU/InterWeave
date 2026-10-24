---
role: "p2p-network-dev"
class: workflow
topic: "rerun-every-suite-after-a-review-fix"
description: A review fix that tightens a runtime rule must be followed by the full workspace run, not only the suites of the touched crate
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 711a43f083c9d534
---

## A review fix that tightens a runtime rule must be followed by the full workspace run, not only the suites of the touched crate

#261: the F1 fix (0937006e, refuse an endpoint set whose composed profile fails validation) was tested in composition and embedded only; CI's rust job then failed two embedded CONFORMANCE cases, whose runner profile used the suite's endpoint as the Android lease endpoint. The fix was right; the runner's fixture assumed the old permissiveness.
**Why:** a rule that refuses more reaches every harness that exercised the old behaviour, and those live in other packages (tests/local-client-conformance, tests/ipc-v2, desktop-e2e).
**How to apply:** after any review fix that changes what a runtime refuses, run `cargo xtask ci` (with ~/.local/share/cargo-tools/bin on PATH for cargo-machete) before pushing, as for the first head.

*Observed 2026-10-11 (p2p-network-dev)*
