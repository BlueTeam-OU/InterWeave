---
role: "ux-designer"
class: threads
topic: "j131-r2-check-running"
description: "j131 — R2 while its check runs: written on #683 f71f27d0c; waits LC pass (01a1e4e6-cbc5), then review, then owner arm word"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 239584ff41057b77
---

## j131 — R2 while its check runs: written on #683 f71f27d0c; waits LC pass (01a1e4e6-cbc5), then review, then owner arm word

2026-11-15 23:20Z. Build (#682, j279): during R2's check the grid/chips stay live; a chip that fills the last empty position sends focus to the keypad since IdCheckWords is disabled. §8 line ~649 is silent on a running check.
Provisional REPLY 01a1e181: (1) words fixed while the check runs (no position, chip, letter, full list; Back per §7) — a check reports on the words it read; (2) focus stays on IdCheckWords shown disabled; existing rules place focus at the result. Long window only after K1 (IdGoingOffline up to 10 s, system prompt, read-back).
Owed: write into design/2026-10-19-android-identity.md §8 (and the state in §4/§6), LC pass, review; tell rust-ui-dev-01 the PR. One open PR rule: #681 still open (j130).
Related: [[answer-in-spec-before-message]], [[j88-identity-keyboard-answers]].

2026-11-16: written on #683 as f71f27d0c (§6 Check running row, §7 Back waits, §8 held words + focus stays, acceptance 15 held-check case). Correction to the provisional answer: Back WAITS during the check, not "per §7". Visual hypothesis: only IdCheckWords disabled, keys keep their look.
REPLY 01a1e4e6-ca2d to rust-ui-dev-01 included code diagnosis: words_held() ignores `checking`, so press/delete/clear/edit act during a check; checked() uses entry.phrase() at result time.
Next: LC pass (REQUEST 01a1e4e6-cbc5 to language-culture-en), fold, review class on e0b33312..head, then the owner's arm word (#683 under 8 work).
Not covered: §6 profile-busy says IdGoingOffline shows, but the build shows no waiting line on R2 (only S1). Carried, not raised.
2026-11-16 15:13Z: LC pass delivered (REPLY 01a1e4eb): 57d0a24ee on develop-qzapp/language-culture-en/for/ux-designer-01/j330-683-identity-r2-english, directly on f71f27d0c; read: every condition whole. Next session: fold it (ff, unrebased), then answer LC's two questions IN THE SPEC: (1) fallback disables chips and grid positions too, not only keys; (2) "Enter or Space pressed it" includes a screen reader's activate (per [[sr-activate-is-press]]). That answer then needs its own LC pass ([[lc-pass-before-arm]]), then review e0b33312..head, then the owner's arm word (#683: 5 work, 2 fix before the fold).
Mistake this session: started a watch with `& disown >/dev/null` again; fix is `gzcoord-inbox --until-delivery --replace` as a background tool call, never pkill ([[never-blind-inbox-watch]]).
2026-11-16 ~17:00Z: owner said "arm 683" in session (j133). Folded 57d0a24ee (head). Review r5 dispatched on e0b33312..57d0a24ee. LC Q1/Q2 answered provisionally (REPLY to LC): Q1 all held controls (keys, delete, clear, chips, positions, show-all) show disabled in the fallback; Q2 SR activate is a press. OWED: write both into identity §8 in the next PR + LC pass.
2026-11-16 17:10Z: r5 found F8-F10 (P2), fixed in 1145b8eff, r6 adjudication: all fixed, posted. Carried risks: R3 has no title key (IdR3Question is its top text); host test owner unnamed. Waiting LC pass REQUEST 01a1e54f on 1145b8eff, then arm on owner's word (said "arm 683" in session).
2026-11-16 17:20Z: r7 full review on 5d139e912: no P1/P2; P3 F11 (long-message load figure, run 1 not read), F12 (Tab/arrows during check undecided), F13 (R3 has no title: name IdR3Question) deferred at arm, carried to my follow-up PR with LC Q1/Q2. F7 deferred (rust-ui-dev-02's opening.rs comment).
DELIVERED 2026-11-16: #683 merged e15caade4 (owner's word in session). INFO 01a1e583 to rust-ui-dev-01 with the merged rule; told them Tab/arrows move focus during the check unless the follow-up PR says otherwise. Carried items are a new job (PR comment 6923656582).

*References: answer-in-spec-before-message, j88-identity-keyboard-answers, lc-pass-before-arm, never-blind-inbox-watch, sr-activate-is-press*

*Observed 2026-11-16 (ux-designer)*
