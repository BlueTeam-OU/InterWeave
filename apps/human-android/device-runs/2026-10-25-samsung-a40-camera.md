# Samsung A40 device run, 2026-10-25: the camera permission

Plan §29 exit gate (e)'s camera-permission row: CAMERA is asked at the
first Scan a code and at no other moment, and a refusal leaves the add
page offering typing. A dated device-run record, as the request that
named the row allows (architect-cto's REQUEST 01a1716e, the owner's
"complete stage 21").

Device: **Samsung A40**, SM-A405FN, Android 11, **API 30**, at the
owner's word for the one destructive step ("go ahead", 2026-10-25):
`pm revoke org.interweave.human android.permission.CAMERA`. API 30's
`pm` has no `clear-permission-flags`; it was not needed, since the
grant read before the run carried no `USER_SET` or `USER_FIXED` flag
and the app's data held no `shared_prefs` (so its own "refused before"
mark was unset): the app had never been answered on CAMERA. The
profile was not cleared.

Build: the debug APK built from this branch's 96cc5791 (sha256
beginning d95d4464), installed with `adb install -r`. Every adb step
ran on this login's own adb server under the `samsung-a40` lease, with
`ANDROID_SERIAL` set; no adb process of the login was left after it.
Times are UTC, 15:21 to 15:28. Navigation was by `input tap` on
screenshots; focus was read from `dumpsys window`'s `mCurrentFocus`
and each permission request from `ActivityTaskManager`'s
`REQUEST_PERMISSIONS` start and `GrantPermissionsViewModel`'s line.

| step | what was seen | result |
|---|---|---|
| launch (`am start`, after `force-stop`) | one `REQUEST_PERMISSIONS`, naming POST_NOTIFICATIONS only ("None of [android.permission.POST_NOTIFICATIONS] in {android.permission-group.CAMERA=[android.permission.CAMERA]}"): API 30 has no such runtime permission, so the request draws a zero-height window and asks nothing; focus back on `MainActivity`, Conversations shown | CAMERA not asked |
| the app menu, Peers, Add an address | focus `MainActivity` throughout; Peers with Show my address and Add an address, then the add page with the field, its helper, Scan a code and Add; no request | CAMERA not asked |
| first press of Scan a code | `REQUEST_PERMISSIONS` from the app; focus `GrantPermissionsActivity`; the dialog "Allow InterWeave to take pictures and record video?" with While using the app, Only this time and Don't allow | asked |
| Don't allow | focus back on `MainActivity`; CAMERA `granted=false` with `USER_SET`; the add page still open, the field holding its caret, and under Scan a code "The camera is needed only to scan a code. You can type or paste the address instead." | refused, typing offered |
| typing after the refusal | `input text "/ip4/"` lands in the field | typing works |
| restart (`force-stop`, `am start`), menu, Peers, Add an address, then Back to Conversations | the launch's POST_NOTIFICATIONS request as before and no other; CAMERA still `granted=false, USER_SET`; `dumpsys media.camera` lists no active client | CAMERA not asked again |

The screenshots (eight, from launch to the add page after the
restart) are kept in the run's scratch, not committed; the address
field in them is empty or holds `/ip4/` only.

What this run does not show: a second press of Scan a code after the
refusal, where the platform may ask once more and a lasting refusal
draws the line that names Settings in place of this one
(`MainActivity.kt`'s `refusedBefore && !asksAgain`,
`AddNote::CameraDeniedSettings`); a grant and a real scan (#418's device
check); a phone without a camera, where Scan a code is not offered
(the manifest's `android.hardware.camera` and `camera.any` both
`required="false"`, `tools/checks/check_android_contacts_and_backup.py`);
TalkBack's reading of the dialog and the refusal line, a person's
check; and the Pixel's API 37.

An oddity seen in passing, not part of the row: at API 30 the app
requests POST_NOTIFICATIONS at every launch, a permission that exists
from API 33. It asks nothing and costs a transient system activity.
