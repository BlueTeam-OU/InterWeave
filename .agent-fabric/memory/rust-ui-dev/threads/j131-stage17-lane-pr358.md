---
role: "rust-ui-dev"
class: threads
topic: "j131-stage17-lane-pr358"
description: "InterWeave Stage 17 close, rust-ui-dev lane (j131): #358 merged fbca15e6 2026-10-22; what it carried, review lessons, what remains with others"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - cad7e671c560868b
---

## InterWeave Stage 17 close, rust-ui-dev lane (j131): #358 merged fbca15e6 2026-10-22; what it carried, review lessons, what remains with others

#358 merged 2026-10-22 06:42 UTC (fbca15e6): F17 (one `selection` element, test counts it by id),
F9 step 5 (heading placed, top padding goes with the app bar), tests/android-e2e/tests/app_lifecycle.rs
(notice while unfocused; process death -> FGS restart into the same store; device only, provisions),
device-runs 2026-10-21 Pixel + 2026-10-22 A40, STORE-CHECK target-SDK half cites SPIKE-008's verdict
(architect-cto supplies d481b2c1, 3055e07a). Three review rounds; architect-cto writes the closing record.

Lessons:
- A wait "for event X" must test a condition false before X: a crash runs no cancel, so a posted
  notice survives into the restarted process's wait (review F1). Withdraw it first.
- `ActivityManager` lengthens a recently crashed service's restart delay past PATIENCE: pause minutes
  between `am crash` reruns.
- The store refuses a copied db/wal unless dir 0700 and files 0600.
- `dumpsys notification --noredact` prints `android.title=String (<title>)`: assert the title is present
  before asserting the text is absent.
- Run `cargo fmt --all --check` before pushing, not only package clippy (CI caught it).
Related: [[a40-own-adb-server]] [[pixel-checks-owed]] [[interweave-stage17-close-j111]]

*References: a40-own-adb-server, interweave-stage17-close-j111, pixel-checks-owed*

*Observed 2026-10-22 (rust-ui-dev)*
