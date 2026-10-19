---
role: "rust-ui-dev"
class: solution
topic: "ui-slint-embedded-image-decoder"
description: "An @image-url in InterWeave's app.slint needs i-slint-core image-decoders, which only backends turn on; workspace CI hides a featureless ui-slint build failure"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d3da1953e02d2df6
---

## An @image-url in InterWeave's app.slint needs i-slint-core image-decoders, which only backends turn on; workspace CI hides a featureless ui-slint build failure

Found 2026-10-11 on gzapi-org/InterWeave#265 (window icon). slint-build embeds an
`@image-url` PNG as bytes and the generated code calls `sp::load_image_from_embedded_data`,
which exists only under i-slint-core's `image-decoders`. `slint` does not forward that
feature; backend-winit, android-activity and backend-testing enable it. So
`cargo check -p interweave-human-ui-slint` with no feature failed while CI's
`cargo clippy --workspace` / `cargo test --workspace` passed through feature unification,
and so did `nextest -p` (the dev-dep testing backend brings the decoder).

Fix in 1d1ef5e7: workspace dep `i-slint-core = { "=1.18.1", default-features = false,
features = ["image-decoders"] }`, named in ui-slint, listed in cargo-machete `ignored`
(the same pattern as i-slint-common for fontconfig-dlopen).

**How to apply:** after any `.slint` change that adds an image, run
`cargo check -p interweave-human-ui-slint` with no features; a wrong `@image-url`
path fails the build at compile time ("Cannot find image file").
Also: the window icon shows on X11 only; Wayland needs a .desktop entry
(packaging/linux has none yet).

*Observed 2026-10-11 (rust-ui-dev)*
