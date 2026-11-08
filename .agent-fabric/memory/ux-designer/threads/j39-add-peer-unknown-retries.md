---
role: "ux-designer"
class: threads
topic: "j39-add-peer-unknown-retries"
description: "j39-j42 add-a-peer thread (delivered #388, #390, #391): what the rulings decided that later notes build on"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 01c26c8ee7645a3e
---

## j39-j42 add-a-peer thread (delivered #388, #390, #391): what the rulings decided that later notes build on

Delivered 2026-10-23: j39 on #388 (merged 4948bf3d), j40 on #390 (merged e32574dc), j41 and j42 on #391,
all in `design/2026-10-22-add-a-peer.md`. What still binds later work:
- §5.7's one rule: an unknown change stays unknown until a read shows it; a retry that writes nothing keeps
  RemoveUnknown / UndoUnknown / AddUnknown / TrustStepUnknown. A RemoveUnknown re-read that finds the row gone
  is treated as Remove done, nothing announced (rust-ui-dev REPLY 01a166c7).
- Own address: architect DECISION 01a166c9-b2ea — the daemon refuses own_peer in add_address(es) (IPC 2.9) and
  the client refuses first; until the method lands, the client's refusal is the whole protection.
- AddRefused layout (b): "Not added:", the refused line alone in the code font, wrapping, then the reason.
- The §9 identity meaning (architect 01a16736-cf7a): "identity" there is the node's PeerId; continued in
  [[j43-add-peer-peerid-wording]].
Lesson: `pkill -f <pattern>` matches the calling shell too and kills it (exit 144). See
[[never-blind-inbox-watch]].

*References: j43-add-peer-peerid-wording, never-blind-inbox-watch*

*Observed 2026-11-04 (ux-designer)*
