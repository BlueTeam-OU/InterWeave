---
role: "rust-ui-dev"
class: workflow
topic: "e2e-person-input-xtest"
description: "desktop-e2e real key/click without xdotool - XTEST via x11rb (no CI package); raise the window before a click or pixel read; ring read from pixels at AT-SPI extents"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8c75fb32eb97a60a
---

## desktop-e2e real key/click without xdotool - XTEST via x11rb (no CI package); raise the window before a click or pixel read; ring read from pixels at AT-SPI extents

A desktop-e2e case that needs a person's own key or click (one the winit filter must see, which an
AT-SPI action and the testing backend both skip) uses XTEST through x11rb, which is already a
dev-dependency with the `xtest` feature on (no lock change). Xvfb serves XTEST by default, so CI needs
no package. The j85 job assumed xdotool would be needed and that devex-tooling would have to add it;
neither was true (PR #551, 2026-11-04).

- `display::tab()` sends a key to the focus window, so call `display::focus(pid)` first.
- `display::click(pid, x, y)` takes window coordinates. Get them from `a11y::Window::extents`
  (AT-SPI Component, CoordType::Window). `focused()` reads the AT-SPI Focused state.
- With no window manager, every window opens at (0,0) and the last one mapped is on top. A click goes
  to whatever window is under the pointer, and GetImage returns the covering window's pixels. So
  `luma_of` and `click` raise the window first (review F1 on #551). A window mapped after the raise
  can still cover it. Only nextest's `end-to-end` group (max-threads 1) excludes that; plain threaded
  `cargo test` does not.
- The focus ring is read as the 1 px middle line of its stroke, 4 px outside the field, corners
  skipped. Compare it with a still frame taken before any key.
- A refusal screen killed with SIGTERM exits non-zero. End a refusal case with Close and check its
  exit class (EX_DATAERR 65 for a store that needs recovery).

Related: [[slint-press-on-textinput-invisible]] [[desktop-e2e-needs-with-display]] [[e2e-runs-prebuilt-binary]]

*References: desktop-e2e-needs-with-display, e2e-runs-prebuilt-binary, slint-press-on-textinput-invisible*

*Anchors: `tests/desktop-e2e/tests/human_app/display.rs::click`, `tests/desktop-e2e/tests/human_app/ring.rs::tab_then_click`*

*Observed 2026-11-04 (rust-ui-dev)*
