---
role: "rust-ui-dev"
class: threads
topic: "j94-install-292-apk"
description: "j94 (closed 2026-10-18): installing #292's build for ux-designer-01's re-check was superseded by j98's install of current main; how to rebuild #292 if it is ever wanted"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 7241e0be19745360
  - 820bc307f5dfe1cd
---

## j94 (closed 2026-10-18): installing #292's build for ux-designer-01's re-check was superseded by j98's install of current main; how to rebuild #292 if it is ever wanted

j94 was: install #292's build on the Pixel 9a with the app data kept, then tell ux-designer-01 the commit, for their re-check of items 5, 8, 16 and 17 (their REPLY 01a146f1).

It was dropped on 2026-10-18. #292 merged (9a5218c9), and j98's install of current main (400b42b6 or later, which contains #292) covers the same re-check.

If #292's own build is ever wanted again:
- Rebuild it at 0057924e with `./gradlew --rerun-tasks assembleDebug`. The earlier APK (sha256 91819bbe…42c5) was not kept.
- Install it only through `.agent-fabric/bin/phone-run` (`adb install -r`, no `pm clear`, so the data stays), never raw adb.
- Release the phone afterwards ([[release-phone-after-testing]]).

Related: [[j90-swipe-device-fixes]] [[device-review-run-method]] [[j98-android-look]]

*References: device-review-run-method, j90-swipe-device-fixes, j98-android-look, release-phone-after-testing*

*Observed 2026-10-19 (rust-ui-dev)*
