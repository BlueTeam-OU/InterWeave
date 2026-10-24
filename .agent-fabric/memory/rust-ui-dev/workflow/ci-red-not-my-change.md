---
role: "rust-ui-dev"
class: workflow
topic: "ci-red-not-my-change"
description: "A required CI job red for a reason my PR did not cause (a RustSec advisory published overnight, a corrupt toolchain download, a merge-queue eviction): how to tell, whose it is, and what I do on my PR"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3bb280b1dda5e3d5
---

## A required CI job red for a reason my PR did not cause (a RustSec advisory published overnight, a corrupt toolchain download, a merge-queue eviction): how to tell, whose it is, and what I do on my PR

Seen in InterWeave 2026-10-18..21: cargo-deny's advisories check failed on every branch when
RustSec published an advisory (zbus family; rustls-webpki RUSTSEC-2026-0382) with no lockfile
change; the android check failed on a corrupt NDK zip and evicted an authorized PR from the
merge queue, dropping its auto-merge.

**How to tell:** same lockfile entry on main, main green before the advisory's date; or the
PR's own pull_request run of that job green on the same head while the merge_group run failed
in a download step.

**How to apply:**
- the lockfile, deny.toml and the CI toolchain are devex-tooling's: send an OBSERVATION (job,
  run id, error line, what I verified and did not), touch nothing in my PR;
- once their fix merges, fold main into my branch (an advisory) or re-run (a flake);
- an eviction needs a person or session watching (`fabric-pr wait-merged`); re-arm the same
  head under the same AUTHORIZES, which still covers it because the head did not move, and say
  so to the authorizer.

*Anchors: `deny.toml::[advisories]`, `tools/ci/android_ci_toolchain.sh::SDK_INSTALL_ATTEMPTS`*

*Observed 2026-10-23 (rust-ui-dev)*
