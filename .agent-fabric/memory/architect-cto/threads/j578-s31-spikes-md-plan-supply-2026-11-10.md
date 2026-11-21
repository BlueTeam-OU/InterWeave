---
role: "architect-cto"
class: threads
topic: "j578-s31-spikes-md-plan-supply-2026-11-10"
description: "SPIKE-001 S31 (facts 80-85) SPIKES.md entry + plan §19 lines supplied for #606; what is still owed after its merge"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - af695daca77ede34
---

## SPIKE-001 S31 (facts 80-85) SPIKES.md entry + plan §19 lines supplied for #606; what is still owed after its merge

DELIVERED 2026-11-10: commit f10554846 on
`develop-qzapp/architect-cto-01/for/rust-services-dev-01/s31-spikes-md-plan`
(based on rust-services-dev-01's head b935b4007 of InterWeave #606), INFO
01a1c2c5-9b30 sent. It adds the S31 entry to `architecture/roadmap/SPIKES.md`,
dates S29's and S30's "not established" tails, and makes three insertions in
plan §19 (carried list, fact 45/74 joining note → fact 84, fact 75 mode note →
fact 85).

**CLOSED 2026-11-10 (j579):** the propagation below landed as InterWeave #607
(1 work, 1 fix), merged ab8753f22 2026-11-10T05:34Z on the owner's word at
8296b277b. Lesson from its review (F1): a carried list copies every "Not
established" item of the SPIKES.md entry — a fact that measures nine kinds
narrows the open question, it does not retire it — and plan §19's list moves
in the same commit.

**Was owed after #606 merged (mine):** `CHANNEL-EVENT.md` §Content and
`CLAUDE-CODE-CHANNEL.md` §Delivery still state S30's picture — 16 MiB line
bound on 2.1.311 (fact 80), tokens a byte by content (81), compaction keeps the
frame not the body (82-83), joining decided by the turn's work not its kind
(84), unflagged mode follows a served feature flag (85). Same shape as
[[j547-s30-propagation-2026-11-06]]. `packaging/claude-plugin/INSTALL.md`
lines 171/187 are rust-services-dev's; told them.

The job closes when f10554846 is an ancestor of origin/main
([[supply-job-closes-on-the-supply-sha-reaching-origin-main]]).

**Review round (2026-11-10):** f10554846 folded into #606 at 881c8ec35; the
blind review's F1 (SPIKES.md S31 headline states the model-request cause of
joining as measured — it is inferred; only the turn's work and kind were
varied) and F5 (plan §19 "fact 84, below" → "above") are on my hunks. j580:
DELIVERED 08d64ea83 (parent 8c23a45e6, review-fix) on the same for/ branch,
REPLY 01a1c2e1-60f8 sent; j580 closes when 08d64ea83 is an ancestor of
origin/main. The j578 worktree's local for/ branch still stands at f10554846
(a session may not merge outside its job's copy; push HEAD:<ref> instead).
Lesson: a spike entry's bold headline states only what the runs varied; a
mechanism the README infers stays marked inferred in the roadmap too.

*References: j547-s30-propagation-2026-11-06, supply-job-closes-on-the-supply-sha-reaching-origin-main*

*Observed 2026-11-10 (architect-cto)*
