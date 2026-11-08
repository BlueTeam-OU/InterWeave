---
role: "rust-ui-dev"
class: threads
topic: "j69-cli-control-carrier"
description: "j69 (owner priority): interweave-human as the fabric's control-plane carrier - PR #477, what is built, what is owed, design facts"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c8e4c218873c679c
---

## j69 (owner priority): interweave-human as the fabric's control-plane carrier - PR #477, what is built, what is owed, design facts

REQUEST 01a17c02 (architect-cto, owner's priority 2026-10-27): the fabric control plane rides InterWeave through interweave-human
(--json send/listen), agents on remote hosts. PR #477, branch develop-qzapp/rust-ui-dev-02/feat/cli-control-carrier, worktree
../interweave-carrier. j68 (fix/j68-client-findings, local b156191f 752c4b12) waits behind it (one open PR).

Built on #477: G1 listen --json --input, G3 --consume, G4 send --no-pending, G6 local_endpoint, G2 --endpoint, and (architect-cto
REQUEST 01a17c78, j70, both before the arming word) G5 `problem` on every outbound event, and --consume draining the store's unread
rows at open and on UnreadInStore plus declining card proposals (at open and on arrival, said `declined` once CardResolved).
G7/G8 not built. Round-1 review (posted, review 5664084108 on d273fa14): F1-F6 all judged real; fixed in c7b5f684..31412466.

Design facts worth keeping:
- The store records NO local endpoint per inbound row, so a drained row prints local_endpoint null and may be another endpoint's.
- A pending row records no source endpoint: it goes out from whatever endpoint the sending run leases (--endpoint mixes them).
- Session batches: one per step (Script::take_events pops one), so DraftChanged + send_draft answer two steps later.
- will() only queues; a stop after it loses the batch -> listen `continue`s to step before any select/return.
- Test lesson: a test whose stop fires on an event hangs (not fails) under a mutation that removes the event; bound every stop.
- e2e: a send to a profile is accepted only while a client of it holds the lease (start a Listener on the receiver);
  `transportctl endpoints list --json` shows `lease` per endpoint row.
j68 (next PR): the start-window design is on main (#472, 2d961966, ux-designer-01 INFO 01a17cc5); it carries one check of mine:
whether a pending row that does not decode is still retried (constraint b); Q1 pending ceiling is architect-cto's.
MERGED 2026-10-27 (#477, 5 work 11 fix, armed on the owner's word at 44de4c90); architect-cto told (REPLY 01a17cd9). j69/j70 done.
P3s F11/F12 carried in [[j68-handoff]].

*References: j68-handoff*

*Observed 2026-10-27 (rust-ui-dev)*
