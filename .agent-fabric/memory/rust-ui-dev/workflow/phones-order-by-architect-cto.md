---
role: "rust-ui-dev"
class: workflow
topic: "phones-order-by-architect-cto"
description: "Since 2026-11-13 architect-cto orders who holds each test phone; ask to join a phone's list before any adb"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 80c57f5a7218dd93
---

## Since 2026-11-13 architect-cto orders who holds each test phone; ask to join a phone's list before any adb

On 2026-11-13 the owner asked architect-cto to coordinate both test phones (Pixel 9a and Samsung A40) over wireless adb.
architect-cto-01 publishes the order as a DECISION ("phones order 1", 01a1d6db), with one holder per phone at a time.
Nobody who is not named connects, installs, uninstalls or restarts adb against either phone.

**How to apply:**
- If I need a phone, send architect-cto a QUESTION or REQUEST to their address. It joins the end of that phone's list, unless the run blocks a gate or a release; then it goes first.
- When a holder releases, it sends one INFO to the next holder and one to architect-cto. The INFO says what is installed, by which signer, and any setting changed and restored.
- The phones' addresses changed that day (both are now on a private-network address). Read them from the latest order, never from older memories.

Related: [[pixel-handover-p2p-network-dev-02]], [[shared-phone-install-check]], [[a40-lock-change-word]]

*References: a40-lock-change-word, pixel-handover-p2p-network-dev-02, shared-phone-install-check*

*Anchors: `.agent-fabric/bin/phone-run::pixel-9a`*

*Observed 2026-11-13 (rust-ui-dev)*
