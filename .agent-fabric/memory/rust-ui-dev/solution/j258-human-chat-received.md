---
role: "rust-ui-dev"
class: solution
topic: "j258-human-chat-received"
description: "where InterWeave gate (c)'s desktop <-> Android HumanChatV2 matrix lives (android-e2e human_chat.rs::chat, keys::RECEIVED), and why it was not moved to tests/interoperability"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b4994ca22ba4f3b3
---

## where InterWeave gate (c)'s desktop <-> Android HumanChatV2 matrix lives (android-e2e human_chat.rs::chat, keys::RECEIVED), and why it was not moved to tests/interoperability

Gate (c)'s desktop <-> Android matrix (plan §20) is two comparisons in
tests/android-e2e/tests/human_chat.rs::chat, which all four rows call
(the stand-in row and the three #[ignore]d device/Wi-Fi rows): the
phone's SENT equals what D's facade decoded, and the phone's RECEIVED
(keys::RECEIVED, emitted by the android-e2e-cases human_chat case)
equals D's two answers and its close, the whole HumanChatV2, keyed by
app_message_id because the two answers can cross in either order.
Delivered as for/p2p-network-dev-02/human-chat-received, range
0aaea8cce..c8dae26f8 (2026-11-11), which p2p-network-dev-02 folds into
their j13 branch; tests/interoperability holds only a README pointer
(theirs).

The driver was NOT moved into a shared src module, on architect-cto's
ruling (option (c)). Moving it would have run the slowest stand-in
exchange twice per CI run, made android-platform/transport-client/
ui-model normal dependencies of android-e2e, and left the device rows
uncovered or duplicated.

A device run of these rows needs an androidTest APK built from that
range or later: an older APK reports no "received" and fails at
expect("received").

*Anchors: `tests/android-e2e/tests/human_chat.rs::chat`, `tests/android-e2e-cases/src/lib.rs::RECEIVED`*

*Observed 2026-11-11 (rust-ui-dev)*
