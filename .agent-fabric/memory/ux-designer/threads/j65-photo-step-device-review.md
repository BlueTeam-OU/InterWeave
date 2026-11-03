---
role: "ux-designer"
class: threads
topic: "j65-photo-step-device-review"
description: "j65 contact cards 5.2: citation fix afe99571; Pixel review of #449's photo picker waits on rust-ui-dev-01's install (data kept)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6666cfcb16729852
---

## j65 contact cards 5.2: citation fix afe99571; Pixel review of #449's photo picker waits on rust-ui-dev-01's install (data kept)

j65 (2026-10-26), branch develop-qzapp/ux-designer-01/feat/j65-contact-cards-5-2-photo-step.
- afe99571: 5.2 cites card_photo.rs "at 4df2fa26, which #449 brought to main" (branch name dropped; comment unchanged on main since).
- Pixel now at a private-network address:44599; the owner re-paired this login 2026-10-26 ~03:39Z. The re-pair left an adb server
  of my uid on port 5037 orphaned to init; I killed it by pid (no lease held, phone-run never uses 5037).
- Installed app updated 2026-10-25 06:58 (pre-#449). rust-ui-dev-02 has NO debug keystore (REPLY 01a17614):
  the phone's app is signed by rust-ui-dev-01's key. Install REQUEST 01a17615 to rust-ui-dev-01 for main 05d3f9cd.
- Owner's word on test photos: push synthetic images, pick only those, delete afterwards; crop/delete any picker
  shot that shows the owner's photos.
Related: [[j64-photo-subsampling]], [[workflow-carried-2026-10-18]], [[device-run-check-focus]].
- j67 on the same branch: b33384e6 (5.2 "Leaving with unsaved changes": every way off asks; LC English; stacked; item 26).
  Provisional REPLY 01a17618-9d08 to rust-ui-dev-02; LC REQUEST 01a17618-f4ed. Pushed, no PR yet; review not yet dispatched.
- j68 on the same branch: 88b5fa40 (add-a-peer: ShowTooManyForCode in the code's place, no subset code; Paste an image only where
  read within bounds, Linux today). Provisional REPLY 01a1761b-2bab to rust-ui-dev-01; LC REQUEST 01a1761b-3d4a.
- LC passes folded (55a8e9ed, 5b74db56); f6d7a483, 63a3859d answer LC's questions. Round 1 review on 05d3f9cd..63a3859d: F1 F2 P2, F3-F6 P3,
  pre-existing dialog Escape/Back; saved as scratchpad review1.md (post once the PR exists). Adjudication dispatched (adj1.md).
- Judged (adj1): F1-F5 real, F6 partly, pre-existing real. Fixed in 772e6b83 (review-fix). PR #456 opened; round 1 posted on 63a3859d,
  judgement comment posted. Round 2 re-review dispatched (brief2.md, 63a3859d..772e6b83). LC pass 3 REQUEST sent.
  Counts: 5 work commits (afe99571 b33384e6 88b5fa40 f6d7a483 63a3859d) + LC's 2 + 1 fix: under 8, arming needs the owner's word or architect-cto's AUTHORIZES.
  The Pixel review (j65's other half) still waits on rust-ui-dev-01's install (REQUEST 01a17615).
- LC pass 3 faa2a25f folded ff and pushed. Round 2 (63a3859d..772e6b83) posted on 772e6b83: F1-F6 fixed; F7 P3 (notification focus case unchecked),
  F8 P3 (way-of-leaving definition covers Save and Remove this identity); risk: row 1712 widened conversation-open focus to every open.
  Adjudication adj2 dispatched. Next: fix (limit definition: other than Save and Remove; limit row to after Close without saving + check both),
  round 3 re-review on 772e6b83..<fix> (card_base 05d3f9cd needed in briefs), LC pass, then AUTHORIZES from architect-cto (7 work < 8).
- 03:09Z: installed main 05d3f9cd myself (rust-ui-dev-01's APK sha 14cb5e0f, their key; signer matched installed, install -r Success, data kept);
  rust-ui-dev-01 cannot reach the Pixel (not paired). Round 2 judged F7 F8 real; fixed b7b0b59a; judgement comment posted; round 3 dispatched (brief3.md); LC pass 4 asked.
- Rounds 3-6: F9 (P3) then its remainder fixed (50d394b5, 756000fc); LC passes 2e881c9d, e8b78f62 folded; round 6 CLEAN on e8b78f62.
  ARMED #456 at e8b78f62 (10 work, 4 fix) ~03:27Z; wait-merged running.
- Device run on 05d3f9cd (07:12-07:30 phone time): Photo Picker opens (PhotoPickerActivity, bottom sheet); picker Albums show only
  Camera/Screenshots/Downloads, so pick by uiautomator tree (desc "Photo taken on 26 Oct 2026, 6:41:54") and crop only my tiles.
  PASS: preparing (dimmed circle + ring, buttons unchanged), EXIF upright, transparency over white, noise accepted, broken -> PhotoUnusable
  with previous photo kept, picker cancel changes nothing, Remove -> initials + Add a photo, Back with no net change closes with no dialog.
  DEVIATION: PhotoUnusable shows under the buttons (right column), spec says under the circle. Note: Name field shifts ~20 px when 2 buttons show.
  Still to run: Back with a change -> dialog (stacked), Continue editing, Close without saving, Save + reopen, small image; then cleanup:
  delete /sdcard/Pictures/InterWeave-test and scan, card back to no photo.
- Device run finished ~07:31 phone time: dialog stacked as #453 PASS; Back on dialog = Continue editing PASS; Close without saving PASS;
  Save writes (reopen shows photo) PASS. FINDINGS -> OBSERVATION 01a17643 to rust-ui-dev-02: (1) Save lands on "Select a conversation"
  placeholder, extra Back; (2) PhotoUnusable under the buttons, not under the circle; (3) own avatar teal before own.peer known, amber after
  (cards.rs map_or(0)), PeerId line appears only later. Cleanup done: card no photo, test images deleted (media store 0), DND restored 0.
  Owed: the review record (design/ file) on a NEW branch after #456 merges (it is armed; a push would disarm it); then deliver j65.
- #456 MERGED 6e68549c. j67, j68 delivered and swept closed; final REPLYs 01a17659-e7cb (rust-ui-dev-02), 01a17659-f1a5 (rust-ui-dev-01).
- Review record PR #457 (branch docs/j65-photo-step-pixel-review, worktree ../interweave-j65-photo-review, head 3559060e; 2 work commits).
  rust-ui-dev-02 took all 3 findings (REPLY 01a17644, branch feat/j54-proposal-in-time). Round 1 review dispatched (brief7.md); LC REQUEST 01a1765a.
  Next: post review, fold LC, re-review, AUTHORIZES from architect-cto (2 work < 8), arm; deliver j65. Re-walk the 3 on their build later.
- #457 round 1 (posted on 3559060e): F1 P2 (buttons BESIDE circle passed; 5.2 field 1 says UNDER - my miss), F2-F4 P3; all judged real (adj3).
  LC cfdf8921 folded; c0b05cb8 answered LC Qs; fixes 0a287596. 4th finding + avatar restated (5.1: hue of the EndpointId, build uses PeerId)
  sent OBSERVATION 01a17660. Round 2 re-review dispatched (brief8.md). Lesson: check each element's PLACEMENT against the field list,
  not only its behaviour, before calling a row pass.
- rust-ui-dev-02 took #4 and restated #3 (REPLY 01a17665); recorded in 457 at the head after 4f770835. Waiting LC pass (01a17663), then one re-review, then AUTHORIZES.
- #457 round 3 CLEAN on ac79f4dc (posted). AUTHORIZES REQUEST sent to architect-cto-01; arm with --authorization <id> when it comes, then wait-merged, deliver j65.
- AUTHORIZES 01a17669 from architect-cto-01; ARMED #457 at ac79f4dc (6 work, 2 fix) ~04:14Z; wait-merged running. After merge: fabric-jobs deliver j65 #457 (+#456), switch.
- #457 MERGED 2025672d. j65 delivered (#456 + #457). Follow-up job queued: re-walk on rust-ui-dev-02's fixed build.
- 2026-10-26: rust-ui-dev-02's fixes are PR #458 (11 work, green, reviewed, unarmed at 0d18560d). j70 blocked on pr #458; after merge ask rust-ui-dev-01 for a main APK (their key), install -r myself, re-walk.
- 2026-10-26: #458 merged as d1367025 (11 work, 6 fix); j70 started; asked rust-ui-dev-01 for a debug APK of d1367025 (REQUEST 01a179d5). Last handover was a host scratch path, signer cert 18f39ab0...; install script pattern in scratchpad install.sh.
- 2026-10-27 00:34 phone: j70 walked on d1367025 (APK f1d91e04, rust-ui-dev-01 key); all 4 findings closed; Remove-of-saved-then-Back walked, pass. Phone moved to a private-network address:43815 (owner). Card restored to no photo; test images removed. Record: PR #462 (branch docs/j70-photo-step-rewalk, worktree interweave-j70-photo-step-rewalk); review dispatched; LC REQUEST 01a179f5-87c1; INFO to rust-ui-dev-02 01a179f5-8b02. Arming under 8 work commits needs owner or architect-cto AUTHORIZES.
- #462 rounds: R1 F1,F2 real (fixed fe860b4b), LC b48837bf folded; R2 F3 real (LC wording overclaimed walked steps; fixed b75eb278); R3 clean on b75eb278. Then LC pass asked (lc-req9), then AUTHORIZES from architect-cto-01 (template scratchpad auth.txt).
- #462 R4 F4 (LC narrowed a 5.2 condition to screen readers) + F5 pre-existing misquote, fixed 1e5821eb verbatim; R5 clean posted. AUTHORIZES asked of architect-cto-01 (01a17a09-01a8). Pitfall: when dispatching with a short prompt pointing at the brief file, keep the full header incl. "config", or post-review refuses; fix by restoring the rendered brief header (same dispatch_id).
- 2026-10-26: j70 delivered: #462 merged 919f3073 (3 work, 3 fix), armed on architect-cto AUTHORIZES 01a17a0a. Photo-step findings all closed on main d1367025. Still unchecked: focus/announcements (no a11y tree), TalkBack (person), wide layout, large photo.

*References: device-review-pixel-lease, device-run-check-focus, j64-photo-subsampling*

*Observed 2026-10-26 (ux-designer)*
