---
role: "rust-ui-dev"
class: solution
topic: "render-after-every-intent-breaks-desktop"
description: "app-core ModelSide::turn must not render again after every intent; that broke the desktop app's conversation opening (4 human_app e2e cases)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 17c094f75f8a364d
---

## app-core ModelSide::turn must not render again after every intent; that broke the desktop app's conversation opening (4 human_app e2e cases)

On 2026-10-26, #453 tried to fix a stale view snapshot by setting `unrendered = true` for every
`ViewEvent::Intent` in `ModelSide::turn`, instead of only for Hide/HideConversation. The unit
tests passed. Four `tests/desktop-e2e/tests/human_app` cases then failed: activating a
conversation's row over AT-SPI never showed its messages, and the tree stayed on "Select a
conversation". With that one change reverted and the binary rebuilt, the case passed in 4 s.
The cause is unproven. Most likely a render raises intents itself (a shown conversation's
MarkRead), so the extra render loops or reorders the selection's turn.

The fix stays in ui-slint instead. `Drawn::own_photo_queued` is set at `Shared::push`, and only
when the queue accepts the input. `take_own_photo` sets `own_photo_taken`. Both clear at the first
render outside `View::take_events`, which renders by itself when `selection_unrendered` is set,
before the model has applied what it took (`CardScreens::rendering_in_take`).

An interim rule cleared the mark when the drawn photo step changed. It was wrong: a second unusable
pick ends on the same step (#453 round 2, F5).

**Why:** render-trigger changes in `turn` reach every screen, and only the desktop e2e catches
them.

**How to apply:**
- Before widening `unrendered` in `turn`, run `tests/desktop-e2e --test human_app` under
  `tools/ci/with_display.sh`, with the binary rebuilt ([[e2e-runs-prebuilt-binary]]).
- Prefer a fix in the view that owns the stale snapshot.

*References: e2e-runs-prebuilt-binary*

*Anchors: `crates/human/app-core/src/model_side.rs::unrendered`*

*Observed 2026-10-25 (rust-ui-dev)*
