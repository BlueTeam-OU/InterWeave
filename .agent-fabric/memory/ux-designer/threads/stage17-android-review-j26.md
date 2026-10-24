---
role: "ux-designer"
class: threads
topic: "stage17-android-review-j26"
description: "j26: plan §20 clause (j) — my on-device look-and-feel review of the Android app is a condition of Stage 17's close; status and what it waits on"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c5c6fe06da4c4e08
  - ec3c800d09a69b37
---

## j26: plan §20 clause (j) — my on-device look-and-feel review of the Android app is a condition of Stage 17's close; status and what it waits on

2026-10-19: architect-cto-01 (REQUEST 01a1526e-81ce) added clause (j) to plan §20's exit gate on the owner's word: the Android app's look and feel are "far below standard"; bring it to the level of mainstream Android messengers (owner named Telegram and WhatsApp). Rank-1 findings must be built by rust-ui-dev-01 and re-reviewed before the close; lower ranks built or carried by name.
- File: design/2026-10-19-android-review.md on develop-qzapp/ux-designer-01/docs/android-review (frame at 2fae2e80: standard S1-S5, method, tasks 1-10, ranking). My own PR once findings exist.
- Told architect-cto (REPLY 01a15271-cae5): heuristic + spec review, not a test with people; reference = conventions, not their feature set (read receipts, reactions, typing meet product rules -> product questions, never rank-1).
- Blocked: Pixel unreachable (no route to a private-network address:43191, 2026-10-19 04:40Z); fabric-coordinator asked the owner (their j280, REPLY 01a15272-ab38). rust-ui-dev-01 installs main's build with their key first.
- Review against main's specs after #300 (my swipe-followups fold, head incl. ab0073d1) merges.
Related: [[device-review-pixel-lease]], [[android-look-proposal]]
- Candidate missing flow for the review: does the Android app offer any backup of contacts/trust/settings? human-client-ui.md §9 requires stating that recovery needs them; if no backup exists the advice cannot be acted on.

2026-10-19 ~11:20: run 2 done on main ffce28df; §6 ranked (F1-F13, M1-M3) at dcdb2b40,
PR InterWeave#304 opened (mine, unarmed until §7 re-review). Sent: REQUEST 01a15305-0f1f
to rust-ui-dev-01 (findings, seeded content for tasks 9-10, N1, F10/F13 constraints);
REPLY 01a15305-111f to architect-cto-01 (status + M1-M3, F10 rule). Next: their replies,
then §7 re-review of rank 1 on the build they name (commit + APK sha).
- Replies in: architect-cto-01 01a15308-4fa0 (M1/M2 carried from Stage 15, M3 = steps 7-8 recovery flow, F10 rule: pending never given up); rust-ui-dev-01 01a15308-9534 (all findings on feat/j111-android-recovery, F1 built 4159658e; seeding option (a) chosen by me). #304 head 50a8e74d adds §8 (swipe spec §11 results = j1, delivered). Waiting: their re-review build (commit + APK sha) when rank 1 is complete, then write §7.
- 2026-10-19 13:05: §7 round 1 at b96e4bec on 80fe2281/c85eb889: F1 F2 F8 F13 pass; F3 (insertion handle), F4 (monospace), F9 (list not anchored at newest; spec step 5 added) owed; F14 new rank 2. Sent REPLY 01a15368-9a4e. Next: round 2 on their next build.
- 2026-10-19 ~12:25Z: round 2 on ebeccf2c (APK 8a3e871e) begun; F14 pass, F4 monospace still not on phone, new: pending time clipped in portrait; keyboard-up-on-Chats seen once. Partial REPLY 01a153b5-d20a to rust-ui-dev-01. Paused: a person using the phone; polling. Owed: F3/F5/F9, K8 keep, tasks 9-10, §7 round 2 write-up, cutout-inset + two-pane Back spec lines.
- 2026-10-19 ~16:20Z: answered rust-ui-dev-01 on F7/F11/F12 (65231264, 34023aa1 AppOptions name, 11b07651 F7 short texts name=visible WCAG 2.5.3, desktop copy snackbar). #309 identity: §2 Activity rule is human-client-ui §9 (architect's); acceptance 10 reading (2) with 3 conditions; conditions 1+3 fail (unknown + "will connect"); a user-presence profile is stranded offline at any launch outside 60 s until the launch-prompt piece. Asked architect-cto-01 (QUESTION 01a154f6-519c) whether carrying past #309 is OK for the gate; my view: only if user-presence unreachable in the closing build or the piece lands first.
- 16:21Z architect-cto-01 ruled (REPLY 01a154f7-be25): acceptance 10 carried past #309, NOT past the close; the launch-prompt piece (state, diagnostic+action, prompt before bind) lands before the close under plan §20 (b) "user-presence restart diagnostic". User-presence reachable only by editing config (default background-compatible). I verify it on the phone when built.
- 2026-10-20 ~09:30Z: Pixel reachable again (owner made it available; nothing destructive on it, data stays). Waiting for rust-ui-dev-01 to install #313's build (04605e70 + Chats-notice fix) before round 2, one run for F3/F5/F9/K8/9-10 + F7/F10/F11/F12 + unlock. Owner finding (relayed): PeerId beside the 24 words misleading -> specified as separate page on C2/S2 (2d102b4c, focus eada0aa2); LC request 01a1589c-6d75 for IdPeerIdTitle/IdPeerIdWritten/IdNext/IdWrittenDown.
- 2026-10-20 ~10:00Z: #304 over 16 work commits -> landing it. Reviews: r1 4cf3deed (4 P2, 5 P3, all fixed), r2 28223a78 (no P1/P2, 4 P3 carried to next branch, listed in PR body), r3 a6fd2c3f = architect's custody supply (ruling: return raises no prompt): P2 needs ADR-0042 amendment -> OBSERVATION 01a158b8-8cc1 to architect-cto-01. Don't arm until their fix + re-review. Round 2 + P3s go on my next branch after merge.
- 2026-10-20 ~10:35Z: #304 ARMED at 935caec0 after 5 review rounds (r3-r5 on architect's ADR-0042 2026-10-20 amendment supply). Carried to my next branch: round-2 P3s (Chats subtitle style, #313 keys, F4 copy reason, Chats overflow focus, long-press finger, chips, identity acc 3, look §9 strings, §6 "no error") + identity: C4/R4 cancelled start stays on screen at a continuation. Architect owns r5 P3s (log-line rule, history list). Next branch also gets §7 round 2.
- 2026-10-20 ~10:55Z: #304 MERGED 53f3454c. New branch develop-qzapp/ux-designer-01/docs/android-review-2 (pushed, no PR yet) head 5c6e3869: all carried P3s fixed (look §5/§9/acc 2, swipe §4 press-after-menu, identity Copy PeerId button §5 + acc 3, §6 C4/R4 continuation, app keys rows; F4 reason). INFO 01a158f0 to rust-ui-dev-01. Still waiting: #313 Pixel build with F3/F4/F9/F14 -> round 2 (write as §7 round 2 on this branch), then PR + review + arm.
- 11:31Z architect ruling (INFO 01a15914-65f3): F3 handle after Back = Slint 1.18.1 backend limit, no patch; record in round 2 as NOT met, toolkit-limited, carried by name to release gate (acceptance unchanged). F4 monospace = 62fad28a, F9 anchoring = 0a7affc4 on #313. Round 2 runs in the owner's phone window (01a1590c-1bde) on #313's finished head.
- 2026-10-20 ~15:15Z: ROUND 2 DONE on #313 35c71169 (APK 2faa357a) -> §7 round 2 at fb3e98ff on docs/android-review-2. Pass F3(ranked)/F7/F11/F12/F14/acc22; F4 monospace NOT met; F16 NEW rank 1 (…x6gY6Eri composer takes no focus, silent; likely untrusted peer); F9 landscape unchecked. Phone handed to p2p-network-dev-02. Sent OBSERVATION 01a159dc-e3f2 (rust-ui-dev), INFO 01a159dc-ed93 (architect). Next: F16 cause -> spec the composer's disabled state -> open PR for docs/android-review-2.
- 15:20Z: F9 landscape walked (partly: step 5 not met; cutout strip remains). F16 test (a) failed with no copy -> app's, touch reaches app; trust ruled out (architect: trust gates send only; composer disabled only when network denied -> spec'd in look §6, 85b17c42). Fonts on Pixel: CutiveMono, DroidSansMono (F4 fix by rust-ui-dev). PR #321 opened (docs/android-review-2, 8 work) ; blind review r1 running at 585d2dc6. Phone released (coordinator told).
- 15:45Z: #321 ARMED at 20a6fd86 after 4 review rounds (8 work, 6 fix). Carried: look §7 snackbar offset w/o composer + swipe §6 pointer; identity §6 "ends like S2's Done" wording. rust-ui-dev-01 fixed F16 (ff981b5e, removed b010afb9 clear-focus) and F4 (ea91c966 Droid Sans Mono) on #313, not yet on a build. Close still owes: device check of F4, F16, F9 step 5 (next phone window, architect asks owner).

- 2026-10-21 01:50Z: rust-ui-dev-01 (INFO 01a15c27) traced FLAG_SECURE left set on Chats to a stopping MainActivity taking SECURE_OFF; fix f382d5ba on #313, not yet on a device. Hold until they send the build to install.
- 2026-10-21 10:26-10:31 phone: ROUND 3 DONE on APK 338cbf0e (installed 09:20, a4b40c99 per rust-ui-dev-01): F4 met, F16 met, F9 step 5 NOT met (bubble rows 115-123, composer top 236, gap 113 px, landscape 2424x1080, 1 and 3 messages alike). Phone released, INFO 01a15d28-5070. S1 primary becomes Unlock (owner), own key pending from language-culture-en. Next: section 7 round 3 on docs/android-review-3, PR, review, arm, sha to architect-cto-01.

2026-10-21: rust-ui-dev-01 (INFO 01a15d44) says F9 step 5 is fixed on the host only, in d9b98c92 on #313. The refused-send line now has no height while empty, and the list strip went from 28 to 53 px at 923x411 with the keyboard up. When the phone is back, check on the device whether 53 px is enough; a bubble's first line can still be cut.

*References: android-look-proposal, device-review-pixel-lease*

*Observed 2026-10-21 (ux-designer)*
