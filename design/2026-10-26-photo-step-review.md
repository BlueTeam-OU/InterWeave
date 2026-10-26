# The own card's photo step on Android: a design review of #449

A design review of the photo step in the card editor
(`2026-10-21-contact-cards.md` §5.2, "The photo" and "Focus and speech
in the photo step"), as #449 built it, together with the
unsaved-changes dialog #453 built, run by ux-designer on the phone on
2026-10-26. It is the device run #449 named as still owed ("the
Android picker on the phone"). Findings go to the builder,
rust-ui-dev-02, in OBSERVATION 01a17643.

## 1. Build and device

- **Build:** main 05d3f9cd (#453's merge, which holds #449). Debug
  APK built by rust-ui-dev-01 with their debug key, sha256 beginning
  `14cb5e0f`; signer certificate SHA-256 beginning `18f39ab0`, the
  same as the installed app's, so `adb install -r` kept the app's
  data. Installed by ux-designer through `phone-run`, 2026-10-26
  07:09 phone time.
- **Device:** Pixel 9a, API 37, narrow layout, one identity held,
  dark theme, font scale 1.0. Do Not Disturb was on during each run,
  and afterwards it was set back to its earlier state.
- **Method:** each step was a scripted tap from a screenshot, after
  reading the focused window from `dumpsys window`. The Photo Picker
  is system UI and shows the phone owner's library. With the owner's
  permission, five synthetic images were pushed to a folder of their own
  and picked by their entry in the picker's accessibility tree ("Photo
  taken on 26 Oct 2026, 6:41:54"). Only their five tiles were ever
  cropped from a capture; every other capture of the picker was
  deleted unviewed. The images and their media-store entries were
  removed after the run, and the card was saved back to no photo.

The five images:

| # | image | made to show |
|---|---|---|
| 1 | 1600 × 1000 JPEG, stored rotated, EXIF Orientation 6, an arrow pointing up when shown upright | orientation applied |
| 2 | 600 × 600 PNG, transparent, with a black ring and a blue square | transparency over white |
| 3 | 256 × 256 PNG of random bytes | a picture that fits only at a low quality |
| 4 | 120 × 80 JPEG | a picture smaller than 256 px |
| 5 | a JPEG header followed by random bytes | an image that cannot be prepared |

## 2. What was walked, and the result

| step | seen | 5.2 says | result |
|---|---|---|---|
| Add a photo | `PhotoPickerActivity` (the Android Photo Picker) opens as a sheet, images only, with the line "This app can only access the photos that you select" | Android's Photo Picker, images only, one item | pass for the picker and images only; whether it takes one item was not observed (§3) |
| picture 1 chosen, 0.3 s later | the circle dimmed under a progress ring; Add a photo still shown, and unchanged | the circle shows a progress indicator; the buttons stay as they were | pass |
| picture 1 prepared | the arrow points up; Change photo and Remove photo in a column **beside** the circle | turned upright by its orientation; field 1: **under** the circle, Change photo and Remove photo | orientation: pass. **Mismatch: the buttons are beside the circle, not under it** |
| picture 2 by Change photo | a black ring and blue square on white | transparency laid over plain white | pass |
| picture 3 | the noise shows, with no `PhotoUnusable` | prepared and shown, since it fits at some quality from 90 down to 20; an image that fits at none is unusable | pass: accepted; the quality it was prepared at was not checked (§3) |
| picture 5 | "This photo cannot be used. Choose another one."; picture 3 still shown | `PhotoUnusable` shows **under the circle**; the card stays as it was | **mismatch: the line is in the right-hand column, under the buttons**; the line's place follows from the buttons' column (the picture 1 row) |
| Change photo, then Back in the picker | nothing changes; the unusable line stays | a cancel changes nothing; the line goes when another picture is chosen or the photo is removed | pass |
| Remove photo | the initials, Add a photo; the line gone | the circle shows the initials, Add a photo replaces both buttons | pass |
| Back, on a card saved with no photo | the editor closes to Conversations with no dialog | a photo added then removed on a card with no photo is no change | pass |
| picture 4, then Back | `CardDiscardChanges` ("Close without saving your changes?"); Close without saving (text) above Continue editing (filled), each the dialog's width | the dialog, its actions stacked, Close without saving above Continue editing | pass |
| Back on the dialog | the dialog closes; picture 4 still in the editor | Back on the dialog is Continue editing | pass |
| Back, then Close without saving; reopen Your card | Conversations; reopened, no photo | the changes dropped; the saved card unchanged | pass |
| picture 1, Save (the Chats screen 5.2 names is the screen titled Conversations) | a page titled "Select a conversation" with a back arrow and nothing else; one more Back reaches Conversations | the page closes back to where it was opened from (the Chats screen) | **mismatch: after Save, the empty-pane placeholder shows** |
| Your card reopened | picture 1 shown | Save writes the photo | pass |
| Remove photo, Save | no photo on reopening; whether a snackbar showed was not captured | the removal written on Save, with no snackbar | pass: the removal was written; the snackbar not observed (§3) |

Also seen, against §5.2 field 1 outside the photo step:

- **The own avatar's hue is not the identity's.** Field 1 draws the
  initials "on the identity's hue (§5.1)", and §5.1 makes that the hue
  of the identity's EndpointId, so that two identities of one PeerId
  do not look alike. Before the first Save of the run the initials
  circle was teal and the page had no PeerId line; after it the circle
  was amber, with a PeerId line under the key fingerprint. In the code
  (read, not traced at run time), `crates/human/ui-slint/src/cards.rs`
  gives the editor's hue as teal (`map_or(0, ..)`) while the own
  `PeerId` is not known, and otherwise `Avatar::of`'s hue, which
  `crates/human/ui-model/src/look.rs` takes from the `PeerId` alone.
  So neither colour is taken from the EndpointId, and the circle changes
  colour once the `PeerId` is known.
- **The rows below the photo move.** The Name field sits about 20 px
  lower while two buttons stand beside the circle than while one
  does. With the buttons under the circle, as field 1 places them,
  the layout differs, so this is left to the fix for the buttons' place.

## 3. What this run could not show

- **One item in the picker.** Nothing in what was seen shows whether
  the picker would take a second item; each run tapped one.
- **Remove photo of a saved photo, then Back.** This could be tried
  once picture 1 was saved, but it was not walked: §5.2 says Back then opens
  `CardDiscardChanges` and Close without saving keeps the saved photo,
  with no snackbar (§9 item 11). Nor was it captured whether a
  snackbar showed after Remove photo and Save.
- **The prepared bytes.** The run saw that each picture was accepted
  and how it looked, not the quality, size or bytes it was prepared
  at, nor the bytes a share would send; those are §9 item 11's test
  on captured bytes.
- **Focus and announcements.** The app draws through Slint, which
  gives Android no accessibility tree (`uiautomator` sees one node).
  So where focus lands after the picker closes, after Remove photo
  and on the dialog, and whether `PhotoAdded` and `PhotoUnusable` are
  announced once, are blocked here, not passed (§9 item 1's rows for
  the photo step).
- **TalkBack** needs a person to check it.
- **A large photograph** from a camera, where preparation takes long
  enough to try Remove photo during preparation (§9 item 1's row). The run's pictures were prepared within a second.
- **The wide layout** (the dialog on a list pick or a menu page,
  §5.2's ways of leaving), and the desktop build.

## 4. Findings and who owns them

| finding | 5.2's line it misses | owner |
|---|---|---|
| Change photo and Remove photo beside the circle | field 1: "under it **Add a photo** … or … **Change photo** … and **Remove photo**" | rust-ui-dev-02 |
| after Save, "Select a conversation" shows | "the page closes back to where it was opened from" | rust-ui-dev-02 |
| `PhotoUnusable` under the buttons (the same column) | "`PhotoUnusable` shows under the circle" | rust-ui-dev-02 |
| the own avatar's hue is the `PeerId`'s, or teal before the `PeerId` is known, not the identity's | field 1: "on the identity's hue (§5.1)", the hue of the identity's EndpointId | rust-ui-dev-02 |

No data was lost in any of them. The second, third and fourth went in
OBSERVATION 01a17643 (2026-10-26), each with its path, the fourth then
framed as a colour change; rust-ui-dev-02 accepted all three (REPLY
01a17644) on develop-qzapp/rust-ui-dev-02/feat/j54-proposal-in-time,
each with a test that fails before its fix. The first was found in
this record's review, and the fourth was restated there against field
1 and §5.1; both findings were sent in OBSERVATION 01a17660, and
rust-ui-dev-02 accepted both on the same branch (REPLY 01a17665). The
steps above will be walked again on the
build that has the fixes.
