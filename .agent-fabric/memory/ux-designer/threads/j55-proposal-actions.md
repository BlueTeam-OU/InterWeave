---
role: "ux-designer"
class: threads
topic: "j55-proposal-actions"
description: "contact-cards proposal (s5.4/s5.4.4/s8): actions stacked at every scale; Accept swipe withheld beyond case D asked of the owner (O2)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 080327508f1d54b6
---

## contact-cards proposal (s5.4/s5.4.4/s8): actions stacked at every scale; Accept swipe withheld beyond case D asked of the owner (O2)

rust-ui-dev-02 QUESTION 01a170cb (2026-10-25), building s5.4.4 swipes on their j54 branch (commits e7538614, 967d894a). I answered provisionally in REPLY 01a170cc-f3b1:
- Copy PeerId under the PeerId, as s5.4 lists it: agreed.
- Q1: Slint 1.18 cannot pick lines by width, so the s8 rule "the proposal's actions wrap to their own line" at 200 % can't be built. Answer: Decline and Accept each on its own line at every width and scale, full width of the content column, Decline first and Accept last (the old focus order). Case S: Close alone. A label wraps inside its button, each at least 48 dp. Why every width: a translated label may not fit one line even at 100 %.
- Q2: CardSameName also shows in P, U and E, but the swipe is withheld only in D. Build D only for now. The owner was asked, as O2; my recommendation is to withhold whenever CardSameName shows, judged at the moment of the swipe. O1's answer covered D only, so the owner decides.
Owed in the j55 PR, design/2026-10-21-contact-cards.md: s5.4 actions; s8 Text scaling; s9 acceptance for the stacked layout; s12 O2. Also the add-a-peer debts from [[j54-address-superseded]] (s11 Q-superseded-own answered "no" in REPLY 01a16fe3; the dependency is now PR #426).
- 2026-10-25 ~03:55Z: owner answered O2 in-session: "Whenever warned" (withhold Accept swipe whenever CardSameName shows, judged at drag start). PR #432 opened (head 91bfab58; 2 work, 0 fix): 654be475 contact-cards s5.4/s5.4.4/s8/s9/s12; 91bfab58 add-a-peer j54 debts (#426 merged, Q-superseded-own answered). Final REPLY to rust-ui-dev-02 01a17133-0a61; LC REQUEST 01a17133-0c55 to language-culture-en. Round 1 review dispatched. Next: post round, fold LC, re-review, AUTHORIZES from architect-cto-01 (2 < 8), arm, wait-merged, deliver j55.
- 04:10Z round 1 posted (F1 P2 #428 app side landed, F2 P2 focus order omits Open your contact, F3 P3 stale 'dependency' (LC fixed), F4 P3 no case E fixture). LC cde996fa folded (d32fc27f). Fix 697993f2 (+ mid-drag cancel). QUESTION 01a17139-43fe to rust-ui-dev-01 (eviction floor vs shown call). LC pass 2 REQUEST 01a17139-45f5. Next: fold LC pass 2, re-review via fabric-review next (round-1 saved), AUTHORIZES from architect-cto-01.
- 04:10Z rust-ui-dev-01 REPLY 01a1713a: floor CAN refuse a shown call on desktop (>4,096 distinct keys by another program within one call's <=60 s), Android no. Written b3d95145, texts kept ('might'); wording asked of LC (REQUEST 01a1713d-633f, pass 3). LC pass 2 5e1f80a6 folded b4233d64 (+ right-drag-only cancel). Next: fold pass 3, re-review 91bfab58..head from round-1, AUTHORIZES architect-cto-01.
- 04:11Z LC pass 3 9233a1f7 folded 347bd4f2 (superseded texts stay, LC 01a1713e). Re-review round 2 dispatched 91bfab58..347bd4f2 (rv432-rr2.json in scratchpad).
- 04:16Z round 2 posted (F5 F6 P3), fixed 11a6df94. LC pass 4 REQUEST 01a17145-6255. Round 3 brief rendered (rv432-rr3.json); dispatch after LC fold (re-render with --head).
- 04:19Z LC pass 4 6c4894f3 folded; 3cbdd6e2 fixes withheld-drag test order; round 3 dispatched 347bd4f2..3cbdd6e2; LC pass 5 REQUEST (one sentence).
- 04:22Z round 3 posted (F7 P3 floor needs newer port), fixed 1471821c + clean start for mid-drag test. Waiting LC pass 5 then round 4 (from round-3.md).
- 04:27Z LC pass 5 3804654b folded; 991d2c33 threshold in both tests; round 4 dispatched 11a6df94..991d2c33 (rv432-rr5.json); LC pass 6 REQUEST 01a1714f.
- 04:30Z round 4 clean but unpostable (base 11a6df94 not checkpoint); LC pass 6 64391450 folded; 8638ebb7 test-2 clean start + attribution; round 5 dispatched 3cbdd6e2..8638ebb7 (rv432-r4b-rr3.json).
- 04:33Z round 5 CLEAN posted on 8638ebb7 (9 work, 6 fix); arm dry-run needs only that review (not boundary, >=8). LC last read REQUEST sent; arm after 'no change'.
- 04:34Z #432 ARMED at 8638ebb7 (9 work, 6 fix); wait-merged running. After merge: deliver j55, update MEMORY line, switch.
- 2026-10-25: #432 merged 8555b43f; j55 delivered. Open notes: round 5 risk (pointer press may move focus, ~6 s to reach Undo in test 2); app.slint proposal row still says actions wrap when narrow (rust-ui-dev-02 builds the stack).
- 2026-10-25 05:35Z j58 (rust-ui-dev-02 QUESTION 01a1718c, #433 menu): provisional REPLY 01a1718e sent: Menu key + Shift+F10 on card controls (no card focus stop); menu Accept and TalkBack action kept on warned rows ('deliberate choice'). Owed: spec PR on s5.4.4 (+ s8/s9 if they name Enter on the row), tell rust-ui-dev-02 the PR.
- 2026-10-25 05:55Z owner answered O3 in-session: 'Keep menu + TalkBack' (menu Accept + custom action stay on warned rows). PR #435 opened (head 56873d6c; 1 work): s5.4.4 menu keys (long press, desktop secondary click, Menu/Shift+F10 on row controls, not Enter), close focus (key origin else Copy PeerId), s8, s9 table+item 7, s12 O3. Round 1 review dispatched (rv435.json). REPLY 01a1719e-57d6 to rust-ui-dev-02 (secondary click + close focus new for their next branch). LC REQUEST 01a1719e-cef0. Next: post review, fold LC, arm needs owner's word or below-min authority (1 < 8).
- 06:05Z round 1 posted (F1 P2, F2/F3 P3, risk long-press release) fixed 765491dc. j59 (rust-ui-dev-02 Q 01a171a0 photo removal) written 64886575 on #435 (CardUpdatesPhotoRemoved, CardPhotoNotUsedRemoved; store kept), REPLY 01a171a3-cca7. LC j182 taking pass; told base on f57a8229 (s8 case S Close). Next: fold LC for/ j58-contact-cards-menu-words, re-review 56873d6c..head from round 1, then arm word.
- 06:06Z LC 5dd65913 folded (ff, on f57a8229); 88f9da4f reason wording (LC open point). Re-review round 2 dispatched 56873d6c..88f9da4f (rv435-rr2.json, card_base 8555b43f). Next: post round 2, fix, ask arm word (2 work).
- 06:12Z round 2 posted (F1-F3 fixed; F4 P2 case E photo removal, F5 P3 s8; risk: drag rule only press-drag-release new, designer's reading of O3). Fixed 6c5fdef6. LC REQUEST 01a171ae-621b (pass on 88f9da4f..6c5fdef6); INFO 01a171ae-7030 to rust-ui-dev-02. Next: fold LC, round 3 re-review 88f9da4f..head from rv435-r2.md, then arm word (3 work).
- 06:20Z LC e520aaae folded (ff on 6c5fdef6), pushed. Round 3 re-review dispatched 88f9da4f..e520aaae (rv435-rr3.json, prev rv435-r2.md). Next: post, fix if needed, ask arm word (3 work).
- 06:22Z OWNER'S WORD in-session: 'arm it when done' (#435, under 8 work). Arm after round 3 clean, basis "the owner's word".
- 06:35Z round 3 CLEAN posted on e520aaae; #435 ARMED on owner's word (4 work, 4 fix). wait-merged running. CARRIED to next contact-cards PR: (a) s5.4.4 say where the proposal menu is anchored (press point) or drop 'near where the press began'; (b) item 7 slide-and-lift check also on case E 'Anna Rossi' row; (c) s10/item7 'came from a card' vs 'not a local choice' (store overwrites unknown-source photo; contacts.rs module comment l.16-18 says stays: OBSERVATION to rust-ui-dev owner of store).
- 06:50Z #435 MERGED 829dbccc; INFO 01a171de to rust-ui-dev-02; j58, j59 delivered. Carried items in [[j58-proposal-menu-carried]].

*References: j54-address-superseded, j58-proposal-menu-carried*

*Observed 2026-10-25 (ux-designer)*
