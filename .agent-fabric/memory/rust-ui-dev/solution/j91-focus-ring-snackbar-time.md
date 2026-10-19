---
role: "rust-ui-dev"
class: solution
topic: "j91-focus-ring-snackbar-time"
description: "j91 (2026-10-17): focus ring on every focusable (N2), snackbar time re-read on settings change (N3), notice button ink; Slint placed-child sizing trick"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8d3a1e0d190b06d5
---

## j91 (2026-10-17): focus ring on every focusable (N2), snackbar time re-read on settings change (N3), notice button ink; Slint placed-child sizing trick

j91 answers ux-designer-01's review of 761d72b3 (OBSERVATION 01a146a5). The branch develop-qzapp/rust-ui-dev-01/feat/j91-swipe-review-of-761d72b3-on-the is stacked on #292's head 0057924e and pushed. It has no PR yet: it opens after #292 lands. N1 is j92.
- 2cbbadf3 N2: a `FocusRing` component in app.slint, a 3 px ring 2 px outside, per #293's android-look spec section 7. The colour is `Palette.focus-ring`, and `focus-ring-on-notice` on a banner. Test: views.rs `every_tab_stop_draws_exactly_one_focus_ring`. It reads the rings' `computed_opacity`, which is why the ring uses opacity rather than `visible`.
- fa18d847 N3: MainActivity has a ContentObserver on the accessibility_(non_)interactive_ui_timeout_ms keys and on ANIMATOR_DURATION_SCALE, plus a re-read 500 ms later. Not run on a device. ux-designer-01 confirmed the cause (REPLY 01a14713): both runs set the 30000 by `adb shell settings put` with the app open and in front. The peer-id Tab stop stays until #293 moves the id into Details. Their device re-check of #292 + j91 covers both N3 cases.
- db12b760: an ActionButton's min-width is its label's width, and `on-notice` gives it the notice ink. In dark mode, "Check storage again" had been light text on #fff4ce and cut to "Check storage".
- Slint fact (i-slint-compiler 1.18.1 passes/default_geometry.rs gen_layout_info_prop): a child with `x` or `y` set is left out of its parent's layout info. So a placed overlay moves no layout, and the "layout must be the only child" rule from [[slint-rendered-window-lessons]] does not apply to it.
- Slint trap: wrapping a TextInput in a Rectangle inside a layout makes the Rectangle stretch. With `vertical-stretch: 0` it collapses to 0 height. `height: input.preferred-height` works.
- To force dark mode under Xvfb, build with `SLINT_STYLE=fluent-dark` into a separate CARGO_TARGET_DIR (3.3 GB, removed after).
- Already on the base, not fixed: app.slint gives 7 "binding loop ... may cause panic at runtime" warnings (root.narrow / width / keyboard-overlap). Queued as a job.
Related: [[j90-swipe-device-fixes]] [[slint-rendered-window-lessons]]

*References: j90-swipe-device-fixes, slint-rendered-window-lessons*

*Anchors: `crates/human/ui-slint/ui/app.slint::FocusRing`, `apps/human-android/android/app/src/main/kotlin/org/interweave/human/MainActivity.kt::settingsObserver`*

*Observed 2026-10-16 (rust-ui-dev)*
