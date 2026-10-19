---
role: "p2p-network-dev"
class: threads
topic: "j77-facade-next-tick-attempt"
description: "transport-client's first send/retry/restore attempt moves to the next tick(); agreed with rust-ui-dev, nothing transport-side relies on inline"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5129192e7ab28c67
---

## transport-client's first send/retry/restore attempt moves to the next tick(); agreed with rust-ui-dev, nothing transport-side relies on inline

2026-10-17: rust-ui-dev-01 asked (REQUEST 01a146c6) to move HumanClient send/retry/restore's first attempt from inline to the next tick() (next_at = now), because a Pixel Undo held the row 33 s behind an unreachable peer. Agreed in REPLY 01a146c7 (seq 264173): only human-client lanes use transport-client. Flagged human-retention client_half.rs's [1, 1] two-attempt assertion and the e2e send() helpers as theirs to adjust. Open, not raised yet: a slow send_direct inside tick() holds back inbound; if raised, the send_direct timeout is the transport side's to state.

*Observed 2026-10-16 (p2p-network-dev)*
