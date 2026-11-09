---
role: "ux-designer"
class: threads
topic: "j93-unlock-policy-pending"
description: "j93 (rust-ui-dev j213) unlock-policy-not-in-force surface — note 40a98b009, waits on architect Q1/Q2 and rust-ui-dev Q3"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 25927d24a7dd5b99
---

## j93 (rust-ui-dev j213) unlock-policy-not-in-force surface — note 40a98b009, waits on architect Q1/Q2 and rust-ui-dev Q3

j93 answers rust-ui-dev-01's REQUEST 01a1abf3 (their j213, ADR-0042 A 2026-11-05).
The note is design/2026-11-05-unlock-policy-pending.md, commit 40a98b009 on
develop-qzapp/ux-designer-01/docs/j93-j213-where-a-person-sees-an (2026-11-05). PR #572 opened at c0d35139d; round-1 blind review dispatched;
language-culture-en asked for the English pass (REQUEST 01a1ac92-5c62). Arming needs the owner's word (3 work commits, under 8).

Design: the Trust page identity section has one line between the PeerId and Show recovery phrase.
States N/D/P-up/P-down/P-unknown/K. D (section 14's diagnostic) is suppressed while a change is
pending. A Chats notice shows only for P-up: lowest priority, no focus, Details; it shows until
the line is seen and again after each failing start.

Q1/Q2 answered by architect-cto (01a1ac8d): the diagnostic follows the policy in force, and there is no Try again
(ADR-0041 A 2026-11-05, #571). P-unknown shows D's line under it when the policy in force is known. Q3 answered yes by rust-ui-dev (01a1ac85): the member carries four facts. The in-force policy is the record's
unopened claim and may be None; covered in 364d5064c (pushed, sha sent). A1-A10 stay blocked until the member is built.
A blocked job is inactive and the hook refuses commits: run `fabric-jobs start` first.

Fact worth keeping: neither key_unlock_policy nor Stay reachable has an in-app control
(config.yaml / ServiceHost::set_availability, which no control calls), and Trust is the app's
only settings page. See [[j51-identity-key-unopenable]].

Rounds on #572 (2026-11-05):
- r1: F1-F7, all fixed in b4685e22f.
- r2: F8. Literal "record the start opened" misreads a reseal that wrote a record; fixed in b8cc402da and
  2b64f08f0 (Unsynced shown as K).
- language-culture-en's j245 folded at b45e96ee1; their six questions answered in 7226471e0.
- architect-cto ruled #571 my way (089b8554f): in force is the record on disk, the reseal's when it wrote one,
  Unsynced included. rust-ui-dev classes Unsynced as OldKeyRemains.
- r3: F9 and F10, fixed in 8a150762a.
- r4: F11, fixed in 7525d75aa.
- r5: CLEAN at 7525d75aa; risks carried in PR comment 6664129996.
- ARMED at 7525d75a on architect-cto's AUTHORIZES (01a1acaf), 7 work commits. architect-cto's note: keep D's line under K's if the K row is ever touched.
- j213's Unsynced classing (15632b734) checked: matches the rows. Member built (77d337267, 6464c3cf0).
- #572 MERGED d65b6307c. Follow-up #573: r1 found my provisional 'no D while locked' WRONG (F1-F6). Fixed in
  cbaa23e75: D shows while locked, from the header byte and the mode read from disk; locked D text is
  UnlockRestartNeedsUnlockLocked. Withdrawal sent to rust-ui-dev (01a1acd2). r2 running.
- (was owed) write the locked-state rule into the note as a follow-up (no D line when
  restart_requires_user_authentication() is None; P and K lines from the last start still show). Sent
  provisionally to rust-ui-dev (01a1acca-1e57).
Lesson: an adjudication's coverage block must name no paths (head..head), or post-review refuses it.

#573 (locked state): 5 rounds; r5 CLEAN at 700312efb. Armed on the owner's word "arm 573" (2026-11-05);
risks carried in comment 6665538837. Final rules:
- D shows while locked (UnlockRestartNeedsUnlockLocked), from the header byte and the mode on disk.
- A13 (a)-(d).
- The Trust page opens while locked, but its PeerId is blank (Q4); left to that page in section 9. The
  unverified-PeerId question was named as architect-cto's but NOT asked.
#573 MERGED ca0f65ae8. j93 done. Owed: my Pixel run of A11 (both locked directions) is j95, blocked on j213.
2026-11-05: rust-ui-dev's #577 (j213 status member, no views) armed at 1075def80. Read at that head, the Activity's
AUTHENTICATION_NEEDED arms run no start, so unlock_status() is None in a fresh process: no locked D/P line.
OBSERVATION 01a1ad82 sent to rust-ui-dev (reply expected). j95's A11 step 4 depends on the answer.
rust-ui-dev confirmed (01a1ad84): fix = ServiceHost::locked_before_start(locked_status(...)) in both arms; sha to follow,
on #577 or first commit of the next branch. My 2nd question (cached-'ready' restart refused with no prompt: what does
the person see?) is NOT yet answered; ask it again when the sha arrives.
#577 MERGED 570fb5829 (16 work, 6 fix) without the fix. The fix is 340aaf088 + 2928ffa7a on rust-ui-dev's
j216 branch (checked; not on main): their next human-crates PR. j95 now waits for that PR and the views.
The cached-'ready' question was re-asked as QUESTION 01a1ada3-a70b.

*References: j51-identity-key-unopenable*

*Observed 2026-11-05 (ux-designer)*
