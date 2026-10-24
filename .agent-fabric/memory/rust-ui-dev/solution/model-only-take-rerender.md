---
role: "rust-ui-dev"
class: solution
topic: "model-only-take-rerender"
description: "app-core ModelSide::turn renders once then takes: an action the model does alone (Hide, Undo, snackbar close, trust input) needs a render after the take or nothing redraws"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7522d8de3e969c37
---

## app-core ModelSide::turn renders once then takes: an action the model does alone (Hide, Undo, snackbar close, trust input) needs a render after the take or nothing redraws

ModelSide::turn (crates/human/app-core/src/model_side.rs) renders first, then takes the view's events. An input that changes only the model and asks the facade nothing brings no update, so no next turn: on the Pixel a dismissed row stayed slid out with no snackbar. Fixed in #289 (ffa402c4) by the `unrendered` flag, which trust inputs already used. Also: a view judges a whole take against the model as rendered, so the root must re-check each intent as it issues it (UiModel::offers, #289), and with the facade on its own thread (desktop and Android) answers can lag any number of turns: never size a bound by "one take".

*Observed 2026-10-16 (rust-ui-dev)*
