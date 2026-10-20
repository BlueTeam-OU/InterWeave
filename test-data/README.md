# Mutable test data

Non-normative inputs used to exercise tests. These may evolve without protocol/version changes. Keep them separate from frozen `fixtures/`.

- `human-chat/render-parity.json` -- the render-parity golden (`human-client-ui.md` section 13): what `crates/human/ui-slint` draws for each `HumanChatV2` text, compared by its `tests/render_parity.rs` and, at Stage 17, by the Android instrumented test. `RENDER_PARITY_WRITE=1` rewrites it; review every change.
- `upgrade/` -- the previous build's XDG tree, one directory per build that made one, which Stage 19's upgrade and rollback cases (`tests/desktop-e2e/tests/human_app/release.rs`) read with every later build. Never edited or regenerated in place; its README says how a tree is made and why its key is test-only.
