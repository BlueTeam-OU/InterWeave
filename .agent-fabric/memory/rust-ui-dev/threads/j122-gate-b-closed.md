---
role: "rust-ui-dev"
class: threads
topic: "j122-gate-b-closed"
description: "InterWeave Stage 17 gate (b) eight Platform-tests lines: final mapping sent 2026-10-22 (REPLY 01a16260), what is instrumented, what is record-only, what stays unmeasured"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - cfdd86219ee8e55e
---

## InterWeave Stage 17 gate (b) eight Platform-tests lines: final mapping sent 2026-10-22 (REPLY 01a16260), what is instrumented, what is record-only, what stays unmeasured

j122 closed 2026-10-22 with REPLY 01a16260 to architect-cto-01 (re REQUEST 01a1590c), read on main fbca15e6.
No branch was needed: clause (b) was amended 2026-10-21 on the owner's word (INFO 01a15fce, #344) so a dated
device-runs/ record meets a line; the two instrumented cases (notification while unfocused, process-death
restart) landed in #358 as tests/android-e2e/tests/app_lifecycle.rs, run on the A40 at API 30.

Still unmeasured, asked to be named in the closing record: device transfer under dataExtractionRules at
API 31+ (Pixel has Seedvault, no GMS D2dTransport; the A40's D2d run is API 30); notification case at API 33+
(needs a fresh-profile phone with POST_NOTIFICATIONS). Carried already: Keystore user-presence grant, network
callbacks. Named-but-unbuilt instrumented cases (secure_span, fgs_lifecycle, backup_exclusion,
presence_prompt_and_cancel) are not built unless architect-cto asks.

Lesson: a job queued for days can be overtaken by a decision in another thread; read the recent history
(fabric-history --since) before building what the original request asked.
Related: [[j131-stage17-lane-pr358]] [[pixel-checks-owed]] [[interweave-stage17-close-j111]]

*References: interweave-stage17-close-j111, j131-stage17-lane-pr358, pixel-checks-owed*

*Observed 2026-10-22 (rust-ui-dev)*
