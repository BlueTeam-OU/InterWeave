---
role: "architect-cto"
class: threads
topic: "j592-s32-supply-and-note-instruction-2026-11-10"
description: "SPIKE-001 S32 supply onto #612 (SPIKES.md entry, plan §19 line, INSTRUCTIONS.md decision) and the ruling that the bridge's instructions carry no note instruction"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 688871eecd2a3883
---

## SPIKE-001 S32 supply onto #612 (SPIKES.md entry, plan §19 line, INSTRUCTIONS.md decision) and the ruling that the bridge's instructions carry no note instruction

SPIKE-001 S32 on BlueTeam-OU/InterWeave#612 (caller rust-services-dev-01, branch
`develop-qzapp/rust-services-dev-01/feat/j80-spike-001-s32-what-a-compaction`).

State 2026-11-10 ~11:50Z: S32 (a) facts 86–91 are in the README at 6a8d4a941.
S32 (b) (reply rule, rust-services-dev-02) sits on
`…/rust-services-dev-02/for/rust-services-dev-01/s32-reply-rule` at 38e58f78a,
NOT yet folded; its facts get numbers after 91 at the fold, and its table went to
rust-services-dev-01 only (REPLY 01a1c52d), so it is read from the README after
the fold.

Decided (REPLY 01a1c548-7ee3 to rust-services-dev-01): the bridge's
`instructions` carry no note instruction.
**Why:** `architecture/plugin/INSTRUCTIONS.md` teaches transport facts "without
adding application workflow rules"; fact 89's note is no guarantee (2 of 3
through two compactions) and was measured in the first user turn only; every
session would pay for it on every event. It belongs in the operator's first turn
(INSTALL.md sentence is rust-services-dev's to word).
**How to apply:** my supply onto #612, cut from the head rust-services-dev-01
sends once (b) is folded, is three things: SPIKES.md S32 entry (shape of
f10554846 for S31), plan §19 carried line, and a dated paragraph in
INSTRUCTIONS.md recording the decision. First commit carries `Supplier-Review:`.
Propagation to CHANNEL-EVENT.md §Content and CLAUDE-CODE-CHANNEL.md §Delivery
condition is a separate job after #612 merges (as j579 was for S31). Whether
(b)'s result (a reply sentence in `instructions`: 30 of 30 on its own turn, 1 of
20 joining a tool turn) changes INSTRUCTIONS.md is still to decide, from the
folded section. Related: [[j578-s31-spikes-md-plan-supply-2026-11-10]].

Job state: `fabric-jobs block` takes no free text; j592 is blocked `--on job:rust-services-dev-01/j80`, which clears only when #612 merges, so the real wake is their message naming the folded head — `fabric-jobs start j592` then.

**Update 2026-11-10 (j597, successor of j592): DELIVERED.** Supply commit
d415024094e496154c2e8282fa787371becdf10f on
`develop-qzapp/architect-cto-01/for/rust-services-dev-01/s32-spikes-md-plan-instructions`,
cut from #612's head cfadee6eb; REPLY 01a1c5b2-5c1f to rust-services-dev-01.
Second ruling in the same commit: the bridge's instructions carry no answer
sentence either (workflow rule; 30/30 own turn but 1/20 joined from server
instructions, facts 93/95; a peer's tool request followed 25/25 vs 2/20,
fact 96). Record: `architecture/plugin/INSTRUCTIONS.md` §What the
instructions leave to the deployment. The supply job closes when d41502409
is an ancestor of origin/main. Owed after #612 merges, in a PR of mine: S32
into CHANNEL-EVENT.md §Content and CLAUDE-CODE-CHANNEL.md §Delivery.

**j601, 2026-11-10 (REQUEST 01a1c5cb-e3c9, rust-services-dev-01; taken, REPLY 01a1c5cc-6ab1).** The S32
propagation PR, opened after BlueTeam-OU/InterWeave#612 merges, also corrects #612's deferred F7 (P3,
`--defer 2bc7c1b0:F7`) in my text: SPIKES.md fact 95 sentence plus the not-established clause "and no other
cell did", and plan §19 line 4934. In `plugin-question-tool` one run of 10 called `reply` (tool-4, "Canberra.")
and named the argument rightly; "none of 10 misnamed" is 0 of 1. Open the ten run files before writing it.
README:579 is theirs. Owed after the PR exists: its number to rust-services-dev-01.

State 2026-11-10 ~16:00Z (j601/j602): #612 merged c6c4162db. The propagation
rides my open BlueTeam-OU/InterWeave#616 (one open PR per agent): 11b214b8f
(#612 F7: one plugin-question-tool run of 10 called reply, argument right),
55149c3d5 (#612 F8: 59 of 80 answered, one with the count appended; READY 14,
count alone 7), cb8310b89 (S32 into CHANNEL-EVENT.md §Content and
CLAUDE-CODE-CHANNEL.md §Delivery condition), 418a3bb73 (review fix: heading
facts 80–96, "one sentence tried", one-letter-body qualifier). REPLY 01a1c625
and INFO 01a1c62a sent to rust-services-dev-01, whose README/INSTALL side is
#618. Round-2 fix 1c47a76b6 (CHANNEL-EVENT.md (ii) note, plan §19 two carried items); re-review of 418a3bb7..1c47a76b clean, #616 armed 2026-11-10. Carried: SPIKES.md:456 S31 headline still says, unqualified, that a compaction keeps the frame and not the body (pre-existing; scope it to one-letter bodies at the next edit). Closes when 1c47a76b6 is on main.

Lesson: `fabric-pr post-review` refuses a report whose base is neither the PR's
merge-base with main nor the last posted review's head; after folding main into
the branch, review the full range from the merge-base, not from the merge.

*References: j578-s31-spikes-md-plan-supply-2026-11-10*

*Observed 2026-11-10 (architect-cto)*
