---
role: "rust-ui-dev"
class: threads
topic: "j94-install-292-apk"
description: "j94 (2026-10-17): #292's APK at 0057924e is built and waiting in [host path]; install it with adb install -r when the owner re-pairs the Pixel, then tell ux-designer-01 the commit"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 820bc307f5dfe1cd
---

## j94 (2026-10-17): #292's APK at 0057924e is built and waiting in [host path]; install it with adb install -r when the owner re-pairs the Pixel, then tell ux-designer-01 the commit

j94 has two steps: install #292's build on the Pixel 9a and keep the app data, then tell ux-designer-01 the commit. They then re-check items 5, 8, 16 and 17 under the lease (their REPLY 01a146f1).

State on 2026-10-17 00:35Z:
- **APK built** from #292's head 0057924e, in the j85 worktree, with `./gradlew --rerun-tasks assembleDebug`. Cargo recompiled ui-slint, transport-client, app-core and human-android.
- The APK is at `[host path]`, sha256 91819bbe…42c5.
- **The phone was not reachable.** `adb devices` listed nothing and `adb mdns services` found nothing. The adb server was released afterwards.
- Blocked on the owner, who has to re-enable wireless debugging and read out the IP:port.

When the link is back:
1. Rebuild #292's APK at 0057924e (the saved copy is gone), then run `.agent-fabric/bin/phone-run adb install -r <apk>`: every run on the Pixel goes through phone-run (workflow/release-phone-after-testing). Use `-r` with no `pm clear`, so the app data stays.
2. Release the phone (see [[release-phone-after-testing]]).
3. Send ux-designer-01 a REPLY to 01a146f1 naming 0057924e and the sha.

If #292's head has moved, rebuild first.

Related: [[j90-swipe-device-fixes]] [[device-review-run-method]]

*References: device-review-run-method, j90-swipe-device-fixes, release-phone-after-testing*

*Observed 2026-10-17 (rust-ui-dev)*
