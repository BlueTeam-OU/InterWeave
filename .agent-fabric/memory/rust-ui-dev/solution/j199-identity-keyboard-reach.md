---
role: "rust-ui-dev"
class: solution
topic: "j199-identity-keyboard-reach"
description: "j199 identity §8 focus stops pushed c3bbb1c5 on e87e1462; Q1-Q3 answered; blocked only on j200's supply (in p2p-network-dev-02's j17 branch, no PR on 2026-11-03) landing; segment focus model"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c461e413693bbbe3
---

## j199 identity §8 focus stops pushed c3bbb1c5 on e87e1462; Q1-Q3 answered; blocked only on j200's supply (in p2p-network-dev-02's j17 branch, no PR on 2026-11-03) landing; segment focus model

j199 (2026-11-03): branch develop-qzapp/rust-ui-dev-01/feat/j199-identity-screens-design-8-keyboard-reach, pushed at c3bbb1c5 (3 work commits). It MERGES e87e1462 (j200's for/p2p-network-dev-02/identity-trace supply, not yet on main), because both change IdentityScreen. Open no PR until that supply lands. Then fold main; 2 work commits so far, minimum 8.

What it builds (crates/human/ui-slint/ui/app.slint IdentityScreen, IdentityStop):
- The stops are segments in reading order: 0 banner, 1 grid, 2 chips, 3 keys, 4 delete, 5 clear, 6 primary, 7 secondary, 8 tertiary; 9/10 are the dialog's actions. Each stop reports (seg, idx) through took(); TAB is step().
- `changed page` applies §8's rules. The dialog focuses keep, then gives focus back to the opener. A new banner takes focus. A chip hands focus to the first enabled key (IdKey.first). Otherwise focus is put back on its stop.
- Focus keys name the kind only ("identity-key", "identity-chip"), so the trace never spells the phrase. Slots are "identity-slot:<i>".
- ui-model: Back on the open discard dialog = Continue entering (before: it dropped every word).
- ux-designer-01 REPLY 01a1a331 answered QUESTION 01a1a324:
  - Q1 yes: Back, Escape or a tap outside the dialog = Continue entering.
  - Q2 yes, plus Space. While typing (hardware letter, Backspace, or the rule's Enter), Enter = suggestion rule only and Space = nothing; this is in IdentityScreen's capture-key-pressed. After TAB, a tap or a press, both press the focused control. Both built in c3bbb1c5.
  - Q3: the spec wants the grid under the keypad. My REPLY 01a1a333 said the grid does not fit: about 800 dp needed against about 570 dp at 100 %. The grid stays above until ux revises s4/s8. ux REPLY 01a1a33f (2026-11-03) agreed: the grid stays above the picker, spec revised on #543.
- Tests: tests/trace.rs, the four identity tests; 13 mutants all caught. No device run yet.
- C3's Back dialog is not built: j201 (ux confirmed it is wanted). The 130 % 'Show all my words' switch is not built: j202, after Q3.

Related: [[slint-modal-page-tab-trap]] [[slint-focus-new-element]]

State 2026-11-03 (later session): e87e1462 is not on main. It is on origin/develop-qzapp/p2p-network-dev-02/feat/j17-stage-17-gates-and-a-split, which has no PR yet. j199 is paused on that alone; j202 is next.

State 2026-11-03 (j202 session): still waiting only on e87e1462 reaching main. Stacked on j199 now: j201 90cd9adb, then j202 2fbf0388 (pushed). The one PR, opened from the j202 head once the supply lands, carries all three. Next on the stack: j203 (acceptance 15) and j204 (app bar arrow).

State 2026-11-04: the stack is j199 → j201 → j202 (393bb85e, with j206's AddClose commits) → j203 11dc8a51. One PR from j203's head once e87e1462 lands.

State 2026-11-04 (session 2): stack head is j204 d0ff179d. e87e1462 still not on main (647e37e3); feat/j17 has no PR. Sent QUESTION 01a1a3c9 to p2p-network-dev-02: when does it land, or may my PR carry e87e1462 itself (only on their yes). Job blocked on that request. The sweep removed the clean j204 worktree; the branch stays on origin.
REPLY 01a1a3ca (p2p-network-dev-02): their j17 PR opens 2026-11-04 from cbc6024e, lands before the device run; they will say when armed. They declined my carrying e87e1462: origin/main..e87e1462 also holds their fe037349 and 603c9c8a, so my PR would land their unreviewed code. If my stack is ready and theirs is still unarmed, ask them again.
- 2026-11-04: on the owner's word, REQUEST 01a1a3db asked p2p-network-dev-02 to put the j17 PR first; job blocked on it.
- #544 opened at cbc6024e (REPLY 01a1a3dd), first in their queue; job blocked on pr:#544.
- #543 merged caca4154 (INFO 01a1a3ec): ux answers for j199/j202/j203 are final spec on main. Carried undecided: Back from R2 via K1; first focus R3/C4/R2-back-from-R3; focus after wrong word on C3; landscape scroll side. ux runs acceptance 13+15 on the Pixel once C2 PeerId page + landscape are built — tell them the build.
- 2026-11-04 (session 3): #544 merged 363f4798. j199 branch fast-forwarded to j204 d0ff179d, origin/main merged by hand (5d53f7a5: kept the stack's IdentityScreen, re-applied b0241f07 + ad6aaa28 ring-reach patches with git apply, dropped 22321132's two tests for the stack's own dialog test + a ported reverse-order test, mutant caught), j209 folded unrebased. Head 21be079a, 15 work commits, xtask ci green. PR #548 opened; round-1 blind review dispatched (brief a scratch path). Next: post review, answer findings, arm (15 >= 8), tell ux-designer-01 the build for acceptance 13/15 once C2 PeerId page + landscape exist.
- 2026-11-04 (session 3, later): #548 armed at a710aa32 (15 work, 5 fix, 4 merge). Rounds: r1 F1 (P2 leaf word -> delete fallback, keypad()), F2 (R2->K1 Back), F3/F4 tests; r2 clean; #547 merged in at 8857cf92 (KeyboardScope/navigates; kept stack's step(), Shift-held Tab); r3 F5 vacuous ring check -> own test with no prior Tab; r4 clean. Next after merge: tell ux-designer-01 the build for acceptance 13/15 once C2 PeerId page + landscape exist; device run owed.
- Lesson: a judgement report whose coverage omits range paths cannot be posted (post-review refuses); give the judge the whole range's path list, or let the re-review carry the closing judgements.

*References: slint-focus-new-element, slint-modal-page-tab-trap*

*Observed 2026-11-04 (rust-ui-dev)*
