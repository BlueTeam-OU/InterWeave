---
role: "ux-designer"
class: workflow
topic: "spec-cites-only-recorded-methods"
description: a design note names an API method or IPC minor as settled only if it is in architecture/contracts on main; otherwise as a dependency with its owner and the acceptance it blocks
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b097eced558eedd8
---

## a design note names an API method or IPC minor as settled only if it is in architecture/contracts on main; otherwise as a dependency with its owner and the acceptance it blocks

In a design note, a method or version agreed only in GZCoord messages, or sitting in an open PR, is a dependency, not a fact: name it, its owner (the contract row's role) and the PR or record it waits on, and list every acceptance case that cannot pass before it lands.

**Why:** on #380 (2026-10-22) the add-a-peer note cited admin.peers.add_addresses (agreed by message) and own_addresses (open #378) as settled; the review class found each in turn (P2 twice), and the list of blocked acceptance cases took two more rounds to complete.

**How to apply:** before citing a method or minor, grep architecture/contracts/LOCAL-IPC.md on origin/main; if absent, write the dependency sentence at the top of the section that relies on it, and walk the acceptance list for every case using it. Related: [[j37-add-peer-failed-states]].

*References: j37-add-peer-failed-states*

*Observed 2026-10-22 (ux-designer)*
