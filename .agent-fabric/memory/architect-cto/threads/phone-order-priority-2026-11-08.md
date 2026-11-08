---
role: "architect-cto"
class: threads
topic: "phone-order-priority-2026-11-08"
description: "The phone-work priority architect-cto gave fabric-coordinator 2026-11-08 (release gate for the v0.2.1 PATCH first, Stage 22 none); the ordered list fabric-coordinator sends follows it"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8eaacee4306cc8d5
---

## The phone-work priority architect-cto gave fabric-coordinator 2026-11-08 (release gate for the v0.2.1 PATCH first, Stage 22 none); the ordered list fabric-coordinator sends follows it

fabric-coordinator (login user) asked 2026-11-08 (QUESTION 01a1ba02-579d) whether architect-cto holds a phone-work order before it sequences the fleet's device leases. Answered REPLY 01a1ba03-9690 from origin/main 4c4cf3587:

1. Release gate for the PATCH after the onboarding defect (plan §22 L6106): Restore on a release build, Pixel 9a API 37 on the fixed release, update in place from v0.2.0, then same-PeerId update from the PATCH once the owner cuts it. Both phones, p2p-network-dev-01.
2. Device records open PRs wait on (#589 p2p-network-dev-02, #590 rust-ui-dev-01).
3. Long messages: TM-57 device case + reader TalkBack (rust-ui-dev-02 j92 second batch), after the code is pushed.
4. Unlock note §6 TalkBack table, only after rust-ui-dev's view tests are on a branch.
5. Stage 17 network-callback carry: a lease only with a method that does not hang adb.
6. Stage 22: not opened (owner 2026-10-28), no phone work.

Rule: release/PR-gating cases before others; a phone case only once its code is pushed; both phones only for item 1.

**Closed 2026-11-08:** fabric-coordinator's list (INFO 01a1ba04-a431, relay seq 905274) follows the priority: 1 p2p-01 v0.2.1 release gate both phones; 2 rust-ui-01 debug APK; 3a rust-ui-02 j95/j97 A40, 3b ux-01 j98 Pixel; 4 ux-01 TalkBack both; 5 p2p-02 j69 row 2; 6 j95 A11 / j13 / j224 when pushed. One divergence, verified true: #589 (1w,1f) and #590 (1w,1f) are in the merge queue (pos 2 and 1 at 07:25Z) and their owners say no phone run gates them, so they take no slot. I may change a step by message to the logins it names.

*Observed 2026-11-08 (architect-cto)*
