---
role: "rust-ui-dev"
class: threads
topic: "j95-plan"
description: "j95 long-message second batch - design choices (cut module, counting drawn lines, reader as overlay, custom ListView virtualization), commit plan, what is owed"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6a6ef6db3af74ac4
---

## j95 long-message second batch - design choices (cut module, counting drawn lines, reader as overlay, custom ListView virtualization), commit plan, what is owed

j95 (started 2026-11-05) builds design/2026-11-04-long-message.md (#561 on main; #564 is ux-designer-01's
provisional revision). Branch develop-qzapp/rust-ui-dev-02/feat/j95-long-messages-second-batch-the-cut.

Design choices so far:
- `ui-model/src/cut.rs`: `formatted(rendered, image) -> Drawn { body, shown_percent }` and
  `source(text) -> (pieces, Option<u8>)`. `body.rs` gained `whole()` (unpieced lines, each link with
  its line index) and `pieced()`.
- Lines are counted as DRAWN lines: one per block, plus one per line break inside it. The contract
  counts "a paragraph" as one line, so a hard-broken paragraph of 1,000 lines at 2,000 chars would be
  whole and past the ceiling. This is stricter than the contract, and owes an OBSERVATION to
  architect-cto.
- Line breaks are not counted as characters.
- The ReplacedCharacter label is deferred to A12's measurement (no text has been proposed for it).
- Reader: plan to overlay the conversation pane, keeping the pane alive (visible:false) so its scroll
  and focus survive.
- Lazy layout: Slint 1.18 virtualizes a `for` whose parent type is named `ListView` and has
  content-y/content-height/content-width/visible-height/visible-width (i-slint-compiler
  object_tree.rs ~2892). std-widgets' ScrollView cannot be used (an image feature is missing), so
  the plan is a custom `component ListView` over a plain Flickable with its own thumb.
- Copy: the existing hidden-TextInput copy pattern (peers-copy), plus `CopyFeedback` (Snackbar or
  Platform). A refused copy cannot be detected through TextInput, which is a known limit.

Related: [[j92-progress]] [[slint-software-renderer-i16-height]]

Progress 2026-11-05 (session 9abb7e68):
- Commits: e30c8b3ad (cut), dafede9e3 (the ellipsis counts in no figure), d231d794a (MessageCopied snackbar).
- ux-designer-01 REPLY 01a1aac0, provisional on #564 cd89698ba: a word boundary counts only in the
  last quarter (else the cut falls between characters), and the ellipsis is uncounted. A2 checks C2
  and "key: " plus base64.
- The OBSERVATION on the drawn-line count went to architect-cto-01 as 01a1aabf-1504 (they were
  planning, so the reply is pending).
- Uncommitted: the row's cut line, the Read button, menu items, the reader overlay (GridLayout cell
  over the conversation pane), reader.rs, and tests/long_message.rs.
- Testing backend: the clipboard is kept, but there is no public reader for it. Copy is tested
  through invoke_message_source.
- No DateFull label exists in the build, so ReaderSent's spoken form equals the shown one. That is
  owed (A8).
- The reader progress bar is not built: the open is synchronous. A5 measures whether it is ever
  needed.

Measurements 2026-11-05 (probe tests/long_probe.rs, uncommitted; run the debug binary under with_display.sh):
- A4 desktop: at scale 2, 200 %, 360 wide, every shape draws, the tallest being headings at 22,286 physical px.
  At scale 3 the headings shape panics (33,429). OBSERVATION 01a1ab0f-c665 to architect-cto-01 asks them to name
  the supported desktop scale, lower the figures, or let me draw the row's fills in pieces.
- A3 at 412 wide in 747 dp viewports: the cut table is 2.2 at 100 % and 7.8 at 200 %, over figures of 2 and 7.
  Every other shape is within its figure. OBSERVATION 01a1ab0f-d2a0 went to ux-designer-01.
- Source mode is shorter than formatted for every shape.
- Commits since: a5de0fdca (view), the contrast commit, the eviction test, e4587d155. That is 9 work commits,
  pushed.

#565 opened 2026-11-05. Round 1 review on e4587d155: F1 and F2, both P3, both judged real. Fixed in e451795f1 (F1)
and 49fe5a185 (F2), each with `Answers:`.
- architect-cto DECISION 01a1ab12: lines are drawn lines; no drawn item spans more than 8,000 logical px at any
  scale; the figures stay. Supplied on for/rust-ui-dev-02/drawn-lines-and-item-bound: c416bea40 and e3bca3bba (a
  hard break is any UAX #14 mandatory break), folded unrebased as eebd95960 and fdce41b1c.
- ux-designer REPLY 01a1ab10: the cut table takes the headings' A3 figures, 4 and 12.
- cb101cdd3 is the tiling:
  - Tile.extent is 8000px.
  - Tiled: the slices.
  - Plain: a Rectangle with no fill, border or radius, which the software renderer never casts (the
    `color.alpha > 0` guard).
  - TallRing: the focus ring as strips.
  - SwipeReveal inherits Plain; its radius is `corner-radius`.
  - row-box stays a Rectangle, because `clip` needs one; the extent test exempts it by id.
  - A Plain without a Rectangle base broke child sizing (bubbles moved).
- The desktop probe at scale 3 and 4 draws every shape at the bound.
- Owed next: a CI-style run on cb101cdd3, then a re-review of e4587d155..cb101cdd3 that reads the fold whole (no
  out_of_scope), then an update of the PR body.

State on pausing for j96 (2026-11-05 ~12:50Z): #565 head cb101cdd3, 12 work, 2 fix, 2 merges, unarmed, pushed;
working tree clean.
- CI: `rust` and both Android jobs pass. `tree checks` and `tool self-tests` are red ONLY on RUSTSEC-2026-0441
  (pin-utils unmaintained), which devex-tooling's #566 ignores. Fold main once #566 lands.
- Local failures that are NOT mine:
  - window_faults at_the_session_bound_the_limit_row_asks... fails on this host at c9ba8c215 (main) too.
  - desktop-e2e window::a_conversation_opens_at_its_newest... failed once under with_display.
  - CI's rust job passed both on cb101cdd3.
- Owed:
  - A re-review of e4587d155..cb101cdd3. The range holds the two fold merges, so it must read them whole, with no
    out_of_scope. Start it with `fabric-review next review-565.yaml --pr 565 --head <head>`; review-565.yaml and
    review-565-r1.md are in the scratchpad of session 9abb7e68.
  - Answer F1/F2 on the PR (fabric-pr reply).
  - Update the PR body (tiling, the fold, the A3 table rule, the item bound).
  - The A40 is free for me (p2p-network-dev-02 INFO 01a1aba9, a private-network address:39991, user-presence identity, so the
    owner's PIN is asked on each start). The device runs come after j96.
- The round-1 request (review-565.yaml), report (review-565-r1.md), PR body draft (pr-body.md) and the probe
  (long_probe.rs) are copied to ~/agent-fabric-tmp/j95-review/.

2026-11-05 16:10Z (ux-designer-01, INFO 01a1ac73): #564 MERGED as a66d295a4, so the long-message note is binding. These
answers now bind: REPLY 01a1aac0 (4.1's cut), INFO 01a1aac8 (word boundary, step 3), REPLY 01a1ab10 (A3's table
figures 4 and 12), and the screen reader's ReplacedCharacter rule (INFO 01a1aaa9-8b1a). architect-cto's DECISION
01a1ab12-5459 stays marked pending until #565 brings c416bea40 and e3bca3bba to main.

2026-11-05 ~20:00Z (j90 session): #565 now carries j90 too (owner's choice): head 623c26939 = cb101cdd3 + origin/main +
the j90 branch, pushed from the j90 copy. 14 work, 2 fix, 5 merge; CI green; workspace suite green on it (human_app
needs tools/ci/with_display.sh). The j95 worktree's local branch is still at cb101cdd3: `git pull --ff-only` first.
main has moved ~60 commits since: fold again, suite, then the review over e4587d155..head (j90's commits included),
then F1/F2, then arm. Title and body have a j90 section.

2026-11-06 (session e4b2fd47): folded main (6a1debeba, pushed); workspace suite green but window_faults session-bound (known host-only). Full review 62ba5cf01..6a1debeba posted (F3 P2 judged NOT real: zero-size TextInput never shaped; F4 partly, F5 real). Fixes 7c9a22039 (row-gone copy confirms nothing), b7cc6e9eb (reader::sent test), f14e35165 (doc move); F1/F2 answered by PR comment. Round 1 saved with `fabric-review save 565 1`; re-review request is the scratch review-565-full-rr2.json. PR body updated. Next: post re-review, arm (17 work >= floor), then device runs (A40/Pixel acceptance) remain owed.
2026-11-06 ~01:20Z: #565 ARMED at 8fef90396 (14 work, 6 fix, 6 merge); rounds 2-3 posted (F6 fixed with copy-field-width/height out properties: the testing tree omits invisible elements, so find_by_element_id cannot see peers-copy). Waiting on merge (fabric-pr wait-merged). After merge, j95 still owes: A40/Pixel acceptance runs (A5 timing, A6 TalkBack, A8), headings at real scale 2, 40 link controls, non-UAX-29 clusters, ReplacedCharacter (A12), labels to language-culture-en; then j93 phone handover.
2026-11-06: #565 MERGED as 135d7cd05. INFO 01a1ae8d to architect-cto-01 (contract on main; labels await vocabulary read). Next j95 work goes on a NEW branch off main: device runs A5/A6/A8, measurements, ReplacedCharacter.
2026-11-06 ~02:30Z: new branch develop-qzapp/rust-ui-dev-02/feat/j95-long-messages-device-runs (pushed) in the same worktree: 48bdb2760 MessageNotCopied + Try again (architect-cto REPLY 01a1aea5; other six stand; ReaderSent key rename optional, not done). INFO 01a1aea6 to ux-designer-01 (design row 514). Device runs await the owner (A40 PIN).
2026-11-06 ~02:50Z: j99 (lc-en final labels) delivered as 4177c66a3 on the device-runs branch: ReadWholeMessage renamed OpenWholeMessage (+open-whole callbacks), MessageNotCopied final, glossary §6. REPLY 01a1aebe to lc-en, INFO to ux-designer (#581 row 514 follows the superseded Try again). Branch has 2 work commits, no PR yet.
2026-11-07: folded main (90512c566; #578 used the old ReadWholeMessage key -> fae25042c). A12 built: b867eb655 (ui-model ReplacedCharacter placeholder + cluster::spoken), 75417e2f2 (ui-slint: BodyLine.spoken, MessageRow.source-spoken, label/source_label/Chats description/reader source through spoken; test long_message a_replaced_character_is_spoken_as_words, 7 mutations caught). REQUEST 01a1b63e-2772 to lc-en (final English), QUESTION 01a1b63e-3c7c to ux-designer-01 (names/titles in scope? row.title is drawn and spoken). Branch: 5 work + merge, pushed, no PR. Still owed: device runs (owner PIN), scale-2 headings, 40 links, non-UAX-29 measurements.
2026-11-07 later: A12 widened to names/titles (ux REPLY 01a1b63f-125e, note 95c44ffc1/ec7dd4cd0 on #581, unmerged). Final en ", character not shown," 8906341cc (Supplier-Review lc-en). 565dffd20 no space before punctuation-led words (unicode-general-category, workspace dep). 6a5394c86: Drawn.spoken pure callback wraps ALL 161 accessible-label/description (replaces 75417e2f2's fields); wire_drawn for AppWindow AND RefusalWindow (unset pure callback = "" => empty labels; refusal tests catch it); tests/compile.rs every_accessible_text_goes_through_spoken. Test helpers find rows by spoken label. Branch 8 work + 1 merge, pushed, no PR. Remaining: device runs, scale-2 headings, 40 links, non-UAX-29.
2026-11-07 measurements (probe ~/agent-fabric-tmp/j95-review/long_probe_scripts.rs, untracked copy in ui-slint/tests/long_probe.rs): 40 link controls fine (39 x path 16 whole, text items <=524 px). Khmer coeng, Myanmar stacker, Devanagari conjunct, emoji ZWJ, Hangul L*V, a+U+0903 chains are each ONE UAX29 cluster (unicode-segmentation 1.13.3) -> drawn as one mark. FOUND: cluster drawn size unbounded: whole 2,000 x U+FDFD row 87,236 px (~117 viewports, not cut); items past 8,000: W/CJK 9,981, emoji 13,947, FDFD >=17,434 (lower bounds, tree omits clipped). No panic at scale 4. OBSERVATIONs sent: architect-cto-01 01a1b6b2-3e5e (extent/Bounds, options a/b/c), ux-designer-01 01a1b6b2-4049 (row height, A3/A4). Waiting; piece size unchanged.
2026-11-07 15:50Z ux REPLY 01a1b6b4-1aba PROVISIONAL (waits architect-cto): weight 2 for EAW W/F base or emoji presentation in cut + all §6 figures; U+FDFD known limit; A4 gains whole-body figure per shape (~9 viewports paragraph); re-measure after ruling. Sent ux REPLY 01a1b6b4-d9a8 (pieces must use weight; FDFD past extent) and architect-cto INFO 01a1b6b5-26ce.
2026-11-07 ~17:00Z: architect-cto DECISION 01a1b6b7-afbe (Bounds A 2026-11-07): weight 2 (EAW W/F, emoji presentation) in all figures incl. piece ceiling; item extent 8,000 kept BY MEASURE (split by layout), U+FDFD known limit of figures only. Contract supply 0b0b11c1a folded (eba07688c, also main 20ce43c8c). Built: 79b71474e ui-model cluster::weight via icu_properties ~2.3 (already in graph); 8f15369b3 body::halves; 75c2b7f59 ui-slint measurer (app.slint measure-text at 320px-window floor, measure-rem/width key -> View::measured_for clears bodies), by_measure/pieces_by_measure in drawn_body + reader::forms. A4 test: W/CJK/emoji/FDFD whole + 2 cut wide, reader both modes; all wiring mutations caught. #583 OPEN DRAFT (11 work + 2 merges after these). ux INFO 01a1b6eb (viewport record). Next: workspace suite, blind review, then device runs (owner PIN) — A5 timing shows measure cost.
2026-11-07 ~18:00Z: #583 rounds 1-3 posted (head 3f70ca5d3; 12 work, 4 fix): F1 reader not re-measured + no wake on in-place fontScale -> 895379f80 (measure-changed callback, reader_forms rebuild); F2 doc 1358643aa; F3 rename dddb69645; F4 wire_measure 3f70ca5d3. All fixed, round 3 Finding: none. Rounds saved (fabric-review save 583 1..3); next re-review from scratch review-583-rr4.json. OPEN: architect-cto OBSERVATION 01a1b725-2e57 (link labels past 8,000 px: W 8,390, FDFD 74,823; proposed (a) no control when label measures past extent). PR still DRAFT; then device runs (owner PIN).
2026-11-07 ~18:20Z: owner "arm 583" -> #583 ARMED at 3f70ca5d (12 work, 4 fix, 2 merge), queued. DO NOT push the device-runs branch until merged (push disarms). Local only, NOT pushed: 4caed8b41 fold of architect-cto 64fdca2d3 (label pieces by measure, A 2026-11-07 (ii)) + UNCOMMITTED label work (LinkButton pieces/Plain/TallRing, label_pieces, Measured enum, measure-label, ReaderRow/LinkRow pieces, a4 label shapes; 4 mutations caught). After merge: new branch for next PR carrying that + device runs.
2026-11-07 ~18:40Z: #583 MERGED 902495dda (12w/4f). Next PR #585 DRAFT on develop-qzapp/rust-ui-dev-02/feat/j95-link-labels-and-device-runs (same worktree): 64fdca2d3 (architect-cto) + 0640e1c7b label pieces + fold c9cbc8b81. REPLY 01a1b751 to architect-cto. Owed on #585: device runs (owner PIN at A40), then full suite, blind review, arm (under 8 work -> owner's word).

2026-11-07 late: #585 round 1 (c9cbc8b8) F1 P1 (pressed StateLayer/Ripple past extent) and F2 P2 (no pieces test)
judged real; fixed in 9938ff7c0 (TapArea: Tiled layer past extent, no ripple; Tile.ripple-reach 1000px cap;
test a_press_on_a_link_control_past_the_extent_is_drawn_in_pieces, 4 mutations caught). architect-cto holds the
owner's "arm 585" and arms the reviewed head from its session (--any-owner) once re-review is clean: send it the sha.
Device runs MOVED to the next batch (architect-cto 01a1b7c4). A40 was in use by rust-ui-dev-01.
2026-11-07 ~21:30Z: #585 round 2 F2 partly/F3/F4 fixed in 1d57352e4; round 3 clean (no finding). Full suite green
(322 ok). Head sent to architect-cto (REPLY 01a1b7f3) to mark ready and arm with the owner's word. Next batch:
device runs on A40 + Pixel (A40 was rust-ui-dev-01's at the time).
2026-11-07 22:24Z: #585 ARMED+QUEUED by architect-cto at a4e73dd16 (owner's word "arm 585 when rust is green");
round 4 F5 (P3) DEFERRED to next batch. My F5 fix is LOCAL commit 1d5daa0a8 on the j95 link-labels branch
(push refused: branch queued). Next batch: new branch off main after #585 merges, carry 1d5daa0a8 (cherry-pick),
plus the A40 device record. A40 APK built at a4e73dd16: scratchpad j95-a4e73dd16-debug.apk (sha 0b61c6585e1c5a75).
A40 TalkBack checks (A6 android, A8, A12 spoken, reader focus) are BLOCKED: no AccessKit adapter on main.
2026-11-07 22:41Z: #585 MERGED f34ca71e9 (2w/3f). Next batch branch develop-qzapp/rust-ui-dev-02/feat/j95-device-runs
off f34ca71e9 with F5 fix 3a531ccdb (cherry-pick of 1d5daa0a8), local, unpushed. Owed: A40 run (address, owner's
uninstall word, PIN), device record, then PR.

2026-11-08: phone order (fabric-coordinator REQUEST 01a1ba04, job j100): my step 3a = j95 then j97 on one
samsung-a40 lease (<2 h) after rust-ui-dev-01's step 2 (debug APK at main on both phones, j95 code included, so
no uninstall of mine); tell ux-designer-01 when the A40 is free. A40 last address a private-network address:39716 (port moves).
j97 fix fe0c55e8f pushed on feat/j95-device-runs (front() refuses behind keyguard; launch_to_identity unlocks first;
2 mutations caught). app_lifecycle.rs same gap: rust-ui-dev-01 takes it after #590. No PR open yet.

*References: j92-progress, slint-software-renderer-i16-height*

*Observed 2026-11-05 (rust-ui-dev)*
