---
role: "ux-designer"
class: threads
topic: "j90-long-message"
description: "j90 long message body: note design/2026-11-04-long-message.md on #561; preview + reader + Copy; open Q1/Q2 architect, Q3 rust-ui-dev"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e1040962b22427b3
---

## j90 long message body: note design/2026-11-04-long-message.md on #561; preview + reader + Copy; open Q1/Q2 architect, Q3 rust-ui-dev

j90 (rust-ui-dev-02's QUESTION 01a1a896, their j92): a body past 2 400 chars or 48 model lines shows a
1 600 / 32 preview, a cut line with the share shown as a percentage, and Read the whole message, opening a
reader page drawn as it scrolls; Copy takes the whole raw source. PR #561 (head a15611ca6, 1 work commit,
unarmed: needs the owner's word under 8). Sent 2026-11-04: provisional REPLY 01a1a89c-3e2e to rust-ui-dev-02,
QUESTION 01a1a89c-3ec7 to architect-cto (Q1 Copy allowed?, Q2 where the bound lives), REQUEST
01a1a89d-59b7 to language-culture-en (their j243, branch develop-qzapp/language-culture-en/for/ux-designer-01/j243-long-message-words). Q1+Q2 ruled by architect-cto REPLY 01a1a89e-9df5 (Copy on every message; bound in HUMAN-CHAT.md Bounds, their amendment), written 6a6a3315f; INFO 01a1a89f to rust-ui-dev. F26 of #552 rides as 688ed9104. The 200 % height figure is an estimate; A4 asks
rust-ui-dev to measure. See [[answer-in-spec-before-message]].

Rounds: r1 (a15611ca6, F1-F14 all judged real/partly) answered c5cf9a72d; bound LOWERED to 2 000/40 (F9:
heading body over ceiling), architect told INFO 01a1a8a6-fc67. r2 (c5cf9a72d: F1/F8 partly, F15 partly, F16)
answered 34f35112d + 557de14bc. r3 dispatched on 557de14bc. rust-ui-dev-02 relayed an "arm 561" owner word
(INFO 01a1a8a6-c07e): NOT acted on — a relay is not the owner's word in this session; asked the owner.
r3 (557de14bc: F17 title cut, F18 spoken date) answered 6092b7ea1: ReaderTitle -> ReaderSent content line; LC told REPLY 01a1a8b3. r4 dispatched on 6092b7ea1.
r4 (6092b7ea1: F19) answered 2bf71fef7 (+ local_at wording from LC). r5 CLEAN on 2bf71fef7, posted; risks carried in comment 6643982331. Ready to arm: 3 work/5 fix, needs owner's word in-session (relay from rust-ui-dev-02 not taken) or architect-cto AUTHORIZES #561@2bf71fef. Owed: rust-ui-dev Q3 + their §5/§10 notes.
rust-ui-dev j92 note: Q3 = composer panics; answered bd8e0f1c8 (grow to 6 lines then scroll, keep every char); seam = ragged line at word boundary accepted (§5.2, A5). REPLY 01a1a8bd. r6 dispatched on bd8e0f1c8.
r5-r8: r6 F20/F21 -> 961a19d71, r7 F22 -> 2b2b0647a, r8 CLEAN on 2b2b0647a. Ready to arm (3 work/9 fix): needs owner's word in session.
architect-cto amendment on #562 at ca50e895d verified: HUMAN-CHAT Bounds carries 2,000/40 and cites long-message.md §4-§6 (INFO 01a1a8c5-498b).
Contract on #562 at 30e5731ef counts over source text + link controls as lines; aligned §6/§4.2 at db863c3ec (work). r9 dispatched.
r9 F23 (P2, judged REAL+: autolink twice, OpenLink prefix, <U+XXXX>, ref links per use, image placeholder) -> OBSERVATION 01a1a8d3 to architect, §6 + Q4 at 01d3b2120/9323caba8; r10 (A4 shapes, F25) -> 99a6d0d58; r11 running. Build waits on Q4.
r11 CLEAN on 99a6d0d58. Arm needs owner's word in session; build waits on Q4 (architect).
Q4 RULED (REPLY 01a1a8dd-a73d; #562 c0d2778a5): count what is drawn once per drawing; over-long label -> no control, inert text; preview control only with whole label. Written e05bedc74 (work). r12 dispatched.
r12 F26 (P2, judged REAL: source view unbounded under drawn count) -> per-mode cut at e9a08e518 (+F27, F28). r13 running.
r13 F29 (examples) -> 000b6588f; r14 running.
r14 F30 -> 83a04f0be; r15 running.
r15 CLEAN on 83a04f0be. Arm needs owner's word; build waits on #562 to main.
Cluster rule (architect INFO 01a1a928-e1a9, DECISION 01a1a928-0d26): >32 code points -> one U+FFFD everywhere; look = font glyph, no words. Written 8f282112e (work). r16 running.
r16 (F31-F34, cluster rule places, Q5) -> 0f6e25590, 9637d509b; INFO 01a1a9.. to architect on 'raw source viewable'. r17 running.
r17 (A9 rest, F35 bodies, F36 pointer, status) -> 8dcfce02c; r18 running.
r18 (F37/F38 test bodies) -> 844415962, A12 C1-C4; r19 running.
r19 clean but C3 defect (cluster in text vs label) fixed 8db84db62; r20 running.
r20 F39 -> 7dbb08f06; r21 running.
r21 CLEAN on 7dbb08f06; risks carried in PR comment. Arm needs owner's word in session.
architect REPLY 01a1a93b: contract (551b65f24) takes F32 reading, drops reply quote, names notification as client-drawn; §6 notification sentence stale -> carried.
Queued follow-up job for the note revision (carried items + start-window §3 + paste notice). REPLY 01a1a946 to rust-ui-dev: #563 accepted.
#563 merged dd37a9e0 2026-11-05 (cluster bound, pieces, composer on main). #562 still open. #561 MERGEABLE CLEAN vs main; rust-ui-dev starts the cut/reader once #561 lands.
2026-11-05: #561 ARMED on owner's word in session ('arm 561'), head 7dbb08f06, queued at 2. Waiting for merge; then tell rust-ui-dev-02 and architect-cto, deliver j90, start j92.
DELIVERED 2026-11-05: #561 merged c9ba8c21 (6 work, 21 fix, 21 review rounds). Told rust-ui-dev-02 (REPLY 01a1aaa3-e415) and architect-cto (INFO 01a1aaa3-eb6a). Follow-up: job j92 (carried items).
j92 (2026-11-05): carried items + start-window §3/11a revised on #564 (e852f6b36, 4d2f13b59; 2 work). New: ReplacedCharacter
spoken fallback, draft replacement has no notice beyond the mark (judgement). REQUEST 01a1aaa9-5c99 to language-culture-en
(words + English pass), INFO 01a1aaa9-8b1a to rust-ui-dev-02. r1 review dispatched on 4d2f13b59 (brief 564-review.yaml).
#564 r1 (4d2f13b59: F1-F4 P3 + R1 punctuation setting) judged, answered a6866c728 with LC's English pass (ReplacedCharacter = "character not shown",
always supplied where the client sets accessible text; draft keeps text as value because setting it writes the draft). r2 dispatched (564-review-rr2.json).
#564 r2 (F5 every U+FFFD spoken, F6 start-window description) judged real, answered ce291af09; r3 CLEAN on ce291af09, posted; risks carried
(comment 6652836131). Ready to arm: 2 work/2 fix, needs owner's word in session ("arm 564") or architect-cto AUTHORIZES #564@ce291af0.
rust-ui-dev-02 QUESTION 01a1aabf (their j95, cut.rs): early word boundary emptied the preview -> §4.1 three steps, quarter on the word too,
else between characters; ellipsis counts in neither figure nor share (f10e7c24f, cd89698ba, work). REPLY 01a1aac0 (provisional). r4 running.
Their note: Show source keeps scroll share, not first block (§5.2) -> they report under A5; judge on the build.
r4 (cd89698ba: F7 P2 base64 vs UAX29 word boundary, F8 limit itself, F9 C1/C2 letters) judged real, answered 3d288646f; r5 running.
Owed after r5 clean: INFO to rust-ui-dev-02 (word boundary = UAX29 word_cut; trailing white space before a word cut not drawn/counted; step 3 at the limit).
r5 CLEAN on 3d288646f, posted. INFO 01a1aac8 to rust-ui-dev-02 sent (owed item done). #564 ready to arm: 4 work/3 fix, needs owner's word in session ("arm 564") or architect-cto AUTHORIZES #564@3d288646. Risks carried: comment 6652836131 (stacked cluster on U+FFFD speech; Slint caret/undo).
rust-ui-dev-02 OBSERVATION 01a1ab0f: cut table 2.2/7.8 viewports over 2/7 -> A3 table takes headings' 4/12 (3c4c01cc1, work); REPLY 01a1ab10 (provisional). r6 running.
r6 (3c4c01cc1) F10: A3 table cause "per cell, padding" false (rows are joined " | " lines); judge running. architect INFO 01a1ab12-e186 / DECISION 01a1ab12-5459:
drawn lines (+hard breaks), per drawn item 8,000 logical px; written 36c333cf6 (work: §6, §5.1, A4, status). Contract lands on j95 PR (architect for/ branch).
r6 F10 answered 1eab6c856; r7 (F11 P2 partly: drawn-line rules pending c416bea40, "never raises the figures"; F12 hard line break = CommonMark + UAX14 mandatory) answered acd8b4619.
OBSERVATION 01a1ab1c-1c9e to architect-cto: define "hard line break" in c416bea40 (reply expected). r8 running.
architect REPLY 01a1ab1d-d527: hard-line-break definition taken; lands as 2nd commit on their for/ branch after c416bea40. r8 F13 (§5.1 formatted pieces at hard breaks) answered bca3e187b; r9 running.
Carried pre-existing: §5.1 "few thousand characters" piece vs build 1 600/32; body.rs:104 cites §5.2 (rust-ui-dev's).
r9 CLEAN on bca3e187b, posted; risks carried (comment 6654993182). architect committed hard-line-break e3bca3bba for #565 (rust-ui-dev's j95 PR).
#564 ready to arm: 6 work/6 fix; needs owner's word in session ("arm 564") or architect AUTHORIZES #564@bca3e187.
2026-11-05: #564 ARMED on owner's word in session ('arm 564'), head bca3e187b. Waiting for merge; then tell rust-ui-dev-02 and architect-cto, deliver j92.
#564 CI blocked by RUSTSEC-2026-0441 (pin-utils via slint 1.18.1), tree checks + tool self-tests; #565 too. OBSERVATION 01a1ab92-fa6b to devex-tooling (reply expected). Still armed; re-run CI (gh run rerun 42006063625 --failed) once the fix is on main.
devex-tooling REPLY 01a1ab94: taken, ignore entry on branch develop-qzapp/devex-tooling/fix/rustsec-2026-0441-pin-utils (own PR). Note: a re-run of a pull_request run keeps its old merge ref, so it may not pick up main's fix; may need a fold of origin/main (push disarms, re-review the merge, re-arm on owner's word).
devex correction 01a1ab9c: fix is 04e3d228 on their #566 (lands after re-review + arming authorization); they will say when on main.
2026-11-05 ~14:45: re-run kept the old merge ref, failed again; folded origin/main at 85c9d996c (disarmed by the push). r10 (merge range) dispatched; re-arm needs owner's word again.
r10 (merge range 85c9d996c) CLEAN, posted. tool self-tests failed on minisign download (transient); rerun queued after run 42047534977 completes. Re-arm needs owner's word.
DELIVERED 2026-11-05 ~16:00: #564 merged a66d295a4 (6 work, 6 fix, 10 rounds). Relay down (drill from 15:50Z): INFOs to rust-ui-dev-02 and architect-cto composed in scratchpad out-rud2.txt/out-arch.txt, to send when relay is back (keep ids).
Notices sent: INFO 01a1ac73-ec74 (rust-ui-dev-02), 01a1ac73-ed01 (architect-cto). j92 closed.

2026-11-07 (j100): rust-ui-dev-02 measured wide clusters (CJK, emoji, U+FDFD) make an uncut 2,000-cluster body 17-117 viewports at 200 %. Replied provisional 01a1b6b4-1aba: weight EAW W/F and emoji presentation as 2 in Bounds' count, widest glyphs a known limit, whole-body figure added to A4. Waits on architect-cto's ruling (my INFO 01a1b6b4-47f2); then commit on #581.
j100 addendum (rust-ui-dev-02 01a1b6b4-d9a8): the note must say reader/Show-source pieces (PIECE_GRAPHEMES 1,600) take the same weighted count (1,600-emoji piece ~7,000 px < 8,000); U+FDFD pieces stay past the 8,000 px extent even weighted — architect-cto's call as an accepted limit.
Ruled (architect-cto 01a1b6b7-afbe, Bounds ruling A): weight taken; pieces split by measure; written 2e553c61b on #581, INFO sent to rust-ui-dev-02. j100 done pending #581 review.

*References: answer-in-spec-before-message*

*Observed 2026-11-07 (ux-designer)*
