---
role: "rust-ui-dev"
class: solution
topic: "android-accesskit-bridge"
description: "Slint 1.18.1 Android has no AccessKit; what an app-side bridge can and cannot do, and the matching accesskit_android version"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 941e3545ebeaeefe
---

## Slint 1.18.1 Android has no AccessKit; what an app-side bridge can and cannot do, and the matching accesskit_android version

Measured from source 2026-10-17 (j92, REPLY 01a14717 to architect-cto-01; not built, not on device):
- accesskit_android matching Slint 1.18.1's accesskit 0.24.1 / consumer 0.38.0 is **0.7.5** (what accesskit_winit 0.33.2 names); 0.8/0.9 need accesskit 0.25.
- Unpatched app-side adapter is possible: i-slint-core 1.18.1 exposes (doc-hidden) WindowInner::from_pub, component(), accessibility::accessible_descendents, ItemRc accessible props/actions, PropertyTracker. Host view: MainActivity (our NativeActivity subclass) passes android.R.id.content's child over JNI.
- Unpatched gaps: explore-by-touch dead because i-slint-backend-android-activity returns InputStatus::Handled for HoverMove (androidwindowadapter.rs:423-428), so hover never reaches the view; handle_focus_change reaches only the backend's own adapter (wrapping it fails, Window weak-refs the inner) -> poll focus_item.
- Slint has no scroll accessibility action -> no SCROLL_FORWARD/BACKWARD, no scroll-into-view.
- The android feature's `slint/accessibility` was inert (slint -> selector -> `i-slint-backend-winit?/accessibility`; neither crate reads the flag): REMOVED with a comment saying why, j96 commit 156c450c on develop-qzapp/rust-ui-dev-01/feat/j96-... (joins the next ui-slint PR after #292). j97's bridge must not expect that feature to exist on Android.
Ruling context: architect-cto says not an ADR-0039 revisit; patched backend would be ADR-0051 shape; shipped gap is the owner's call. See [[android-shell-build]].

*References: android-shell-build*

*Anchors: `crates/human/ui-slint/Cargo.toml::slint/backend-android-activity-06`, `Cargo.toml::i-slint-core = { version = "=1.18.1"`*

*Observed 2026-10-16 (rust-ui-dev)*
