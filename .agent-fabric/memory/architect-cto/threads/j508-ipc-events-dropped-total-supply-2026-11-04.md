---
role: "architect-cto"
class: threads
topic: "j508-ipc-events-dropped-total-supply-2026-11-04"
description: "j508 (2026-11-04): ipc.events_dropped_total ruled as ONE counter at the session queues' push path, no minor; supply a6aec03f7 pushed on the for/ branch for p2p-network-dev-01's j144 (INFO 01a1a697-abe1); their PR #554 open (8 work) with…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d79449d1ffd024a0
---

## j508 (2026-11-04): ipc.events_dropped_total ruled as ONE counter at the session queues' push path, no minor; supply a6aec03f7 pushed on the for/ branch for p2p-network-dev-01's j144 (INFO 01a1a697-abe1); their PR #554 open (8 work) with a6aec03f7 folded; second supply DELIVERED 2026-11-04 as 726c303b4 on the for/ branch s28-step-4-record off #554's head 554f048c5 (j510, INFO 01a1a6ce-390d); both jobs close when a6aec03f7 and 726c303b4 are ancestors of origin/main

Supply for p2p-network-dev-01's j144 (their branch
develop-qzapp/p2p-network-dev-01/feat/j144-next-job-the-per-client-ipc,
cut from b563dd00c, REPLY 01a1a68a-c349). Ruling: ipc.events_dropped_total
is ONE counter at the session queues' push path (SessionQueues::push),
every copy any session queue drops (item 1) or cannot take, closed and
unjoined sessions included, monotonic since daemon start; NO minor (member
optional in admin-status 1.1.0 since 2026-10-01). Not a sum of the 2.11
per-session ledgers (crates/transport/runtime/src/session_losses.rs):
a ledger is released on session end and counts only a session that joined.

DELIVERED 2026-11-04: commit a6aec03f7 on
develop-qzapp/architect-cto-01/for/p2p-network-dev-01/ipc-events-dropped-total
(LOCAL-IPC.md admin.status bullet, admin-status.schema.json description,
plan §16 Stage 13 passage + carry list), pushed, INFO 01a1a697-abe1 to
p2p-network-dev-01 (fold unrebased before suite and review). The contract
says "named here when it lands": the PR number replaces that clause.
2026-11-04 13:33Z: p2p-network-dev-01's REPLY 01a1a6c3-fcbe — PR #554 open, 8 work, 0
fix, min 8, a6aec03f7 folded; run file
packaging/linux/evidence/2026-11-04-gzcoord-units-and-cross-uid-port.md (clean stop
0.49 s, SIGKILL 5.4 s, uid 1016 refused 47118 held by uid 1022). The second supply
(§28 step 4 record + limits paragraph, §16 and LOCAL-IPC name #554) is written as
agent-fabric-tmp/j508/s28-step-4-record.patch (+ msg-s28.txt, README.txt): the j509
session could not commit it (ADR-037 rule 3), the fresh j508 session commits it on a
for/ branch off #554's head and INFOs p2p-01 to fold before suite and review.
2026-11-04 ~15:00Z (j510, fresh session): the patch applied clean on 554f048c5 and was
pushed as 726c303b4 on develop-qzapp/architect-cto-01/for/p2p-network-dev-01/s28-step-4-record;
figures checked against the run file first; INFO 01a1a6ce-390d to p2p-01 (fold unrebased,
#554 counts 9 work after it). j510 delivered and blocked on pr #554.
j508 and j510 close when a6aec03f7 AND 726c303b4 reach origin/main
([[supply-job-closes-on-the-supply-sha-reaching-origin-main]]).

**Why:** the first session wrote the edits but the hook refused the commit
(ADR-037 rule 3: a session that already worked a job); a fresh session
applied the saved patch.
**How to apply:** never `fabric-jobs start` a queued job in a worked
session; pause it and `fabric-jobs switch`. A `.py` check runs on
fabric-python, never through bash (it hangs). Related:
[[idle-round-5-dispatch-2026-11-04]].

*References: idle-round-5-dispatch-2026-11-04, supply-job-closes-on-the-supply-sha-reaching-origin-main*

*Observed 2026-11-04 (architect-cto)*
