---
role: "rust-ui-dev"
class: solution
topic: "j271-elided-label-clip"
description: "Slint 1.18.1 clips a Text's ink to its box only under overflow clip; an elided label spills stacked marks — InterWeave draws elided lines in OneLine (clip box), guarded by compile.rs and a desktop pixel test"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6f119723d68c15fc
---

## Slint 1.18.1 clips a Text's ink to its box only under overflow clip; an elided label spills stacked marks — InterWeave draws elided lines in OneLine (clip box), guarded by compile.rs and a desktop pixel test

j271 (2026-11-13), on #658's branch, commits 5c241324d (fix + tests) and 2d46d7508 (run record in past tense).

- Cause, read in i-slint-core 1.18.1 `textlayout/sharedparley.rs` draw_text: the clip to the Text's box is applied only for `overflow: clip` (or an elided first line taller than the box by LINE metrics, which marks do not change). Both renderers (desktop software, Android Skia) go through it, so the desktop spills too: at scale 1 a 32-code-point character in the Chats preview ran 49 px up through the name to the row's clip; on the A40 the unclipped title ran through the status bar.
- Wrapped Texts default to `overflow: clip`, so they were already clipped (the "about two marks" in the run file).
- Fix: `component OneLine inherits Rectangle { clip: true; HorizontalLayout { Text { overflow: elide } } }` (its colour property is `ink`: Rectangle already has `color`). FocusHeading's text sits in the same clip box inside its layout, so the focus scope itself is outside the clip (TAB skips controls under a non-Flickable clip, see [[j270-proposal-row-clip-tab]]).
- Guard: compile.rs `elide_scan`: each `overflow:` naming elide is in a Text whose parent is a *Layout whose parent binds `clip: true`. Pixel test: one_line_clip.rs draws Chats with preview "a" then the bound character as a second message (the same RowId re-received is NOT redrawn, so use a new row) and requires the changed pixel rows to span < 24 px × scale (12 measured clipped, 49 mutant).
- Correction from the A40 shots (review F5, 2026-11-13): the Chats row's existing clip (Rectangle clip:true radius 12) HELD on Skia; its marks stopped at the row top. Only the title (FocusHeading, no clipping ancestor) went through the status bar. The run record had merged the two; fixed cb3be645d/2364439de/5808600ce. Read a screenshot before restating a run's prose.
- #658 state 15:30Z: review r4 of 2d46d7508 posted (F5-F9; F8 = ux-designer's doc, told in REPLY 01a1d4bb), re-review of ..5808600ce clean and posted. Red tree checks = RUSTSEC-2026-0534, devex-tooling j336 bumps gossipsub; after it lands fold origin/main, review the fold, then arm (9 work).
- 16:40Z: folded origin/main (1fb70d8c3, #660), full review from merge-base clean (posted 5894612203), ARMED with F8 deferred (ux-designer doc). Follow-up j273: NameField single-line TextInput unclipped (review risk 1).
- Not run on a phone since.
- 2026-11-13: #658 MERGED 4add2e9f8; j267, j268, j270, j271 done.

*References: j270-proposal-row-clip-tab*

*Anchors: `crates/human/ui-slint/ui/app.slint::component OneLine`, `crates/human/ui-slint/tests/compile.rs::every_elided_text_is_clipped_to_its_box`, `crates/human/ui-slint/tests/one_line_clip.rs`*

*Observed 2026-11-13 (rust-ui-dev)*
