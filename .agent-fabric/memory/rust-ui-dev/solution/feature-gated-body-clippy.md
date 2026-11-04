---
role: "rust-ui-dev"
class: solution
topic: "feature-gated-body-clippy"
description: "a method whose whole body is cfg(feature) fails clippy unused_self in the featureless build; --all-targets hides it because dev-deps turn the feature on"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a586aaa394c1f2ac
---

## a method whose whole body is cfg(feature) fails clippy unused_self in the featureless build; --all-targets hides it because dev-deps turn the feature on

A `&self` method whose only use of `self` sits under `#[cfg(feature = "test-hooks")]` fails `clippy::unused_self` when the crate is built without the feature — e.g. `cargo clippy -p interweave-android-e2e-tests`, or `-p interweave-human-android-platform --lib --no-default-features`. `--all-targets` on the crate itself PASSES, because its dev-dependencies unify test-hooks on; the workspace CI run never sees it either.

Fix used (j178, commit 415888e4, `crates/human/android-platform/src/hub.rs` `before_command`): `#[cfg_attr(not(feature = "test-hooks"), expect(clippy::unused_self, reason = "..."))]` — `expect`, so it fails if the body changes.

**How to apply:** after adding a cfg-gated body or feature in a human crate, run clippy with `--lib --no-default-features` and each feature alone, not only `--all-targets`. Same class as [[ui-slint-embedded-image-decoder]].

*References: ui-slint-embedded-image-decoder*

*Anchors: `crates/human/android-platform/src/hub.rs::before_command`*

*Observed 2026-10-30 (rust-ui-dev)*
