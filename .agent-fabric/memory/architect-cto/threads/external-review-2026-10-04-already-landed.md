---
role: "architect-cto"
class: threads
topic: "external-review-2026-10-04-already-landed"
description: "The owner's external stage review of InterWeave main 58463fb9 (2026-10-04) was fully landed by 2026-10-05; if it is pasted again, verify #192/#187 on origin/main and report, do not re-triage"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 04fd14ce856ffb2a
---

## The owner's external stage review of InterWeave main 58463fb9 (2026-10-04) was fully landed by 2026-10-05; if it is pasted again, verify #192/#187 on origin/main and report, do not re-triage

The external stage-by-stage review of main 58463fb9 (two P2, three P3) was re-pasted to architect-cto on 2026-10-20. Every finding was already judged by the review class and landed on 2026-10-05:

- P2-1 key path ancestor symlink: #192, 9256936d (judged on disk, links followed); plan §16 (13), §Stage 15 (3), config.schema.yaml, STATE.md all carry "A 2026-10-05".
- P2-2 desktop bridge: rust-ui-dev #187, facade→window `sync_channel(TO_WINDOW=256)`; window→facade unbounded with its bound argued in `apps/human-desktop/src/facade_thread.rs` and the app README.
- P3-1 events(0): NOT REAL, trait doc sentence c9bd3a82.
- P3-2 IPC minor check: e7dd2d34. P3-3 key dir owner on load: 9a0a9296 + 6f1fce2c.

**Why:** the review is dated and names the old gzapi-org origin; a session that triages it afresh duplicates p2p-network-dev's j15.
**How to apply:** answer from `git merge-base --is-ancestor <sha> origin/main` and `gh pr view 192 187`; nothing is owed. One unverified residue, rust-ui-dev's: `crates/human/android-platform/src/facade.rs:85` opens the same unbounded command channel without the desktop's bound argument (the view side is bounded, `hub.rs` TO_VIEW=256, a slow view detached). Related: [[threads-carried-2026-10-18]].

**Delivery (2026-10-20, owner's question via fabric-coordinator).** The plan for the re-pasted review was approved and delivered as a report, not a PR: nothing lands because every finding is already on main (#192 f2b771ea, #187). Its artifacts are this memory file and the session's report of 2026-10-20 to the owner. The only open residue is the Android facade command-channel prose gap, rust-ui-dev's, not yet sent as an OBSERVATION; the owner decides whether it is chased.

**Resolved 2026-10-20.** OBSERVATION 01a15a5b sent to rust-ui-dev-01 on the owner's ask; their REPLY 01a15a5d: the desktop's bound holds on Android (the only command sender is app-core's ModelSide through the Rust ViewSide, which drops a command while the same one is in flight; the facade drains every waiting command per turn), so the missing piece was the sentence, now 4312c2ba on #313. Nothing is open from the 2026-10-04 review.

*References: stage-17-closing-plan-2026-10-19*

*Observed 2026-10-20 (architect-cto)*
