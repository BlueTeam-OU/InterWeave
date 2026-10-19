---
role: "rust-ui-dev"
class: threads
topic: "j88-device-review-run"
description: "j88 delivered 2026-10-16: Pixel 9a recorded run of swipe spec §11 on main 761d72b3; results, findings now j90, where the evidence is"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 34be93ec3c37069d
  - 51ba55279522d9a2
---

## j88 delivered 2026-10-16: Pixel 9a recorded run of swipe spec §11 on main 761d72b3; results, findings now j90, where the evidence is

j88 answered the owner's REQUEST 01a14643. Delivered as REPLY 01a14692 to develop-qzapp/user, and REPLY 01a14692-a94f to ux-designer-01 (phone free, state left).
- Evidence: a local run directory (README, log, per-item video/input/png/haptics), gone with the 2026-10-17 /var/tmp wipe; the findings are InterWeave #292's commits. Main's §11 has items 1-18; 12 and 14 were left out.
- Pass: 1 2 4 6 7 9 10 11 15 18. At the limit: 13 (row buttons drawn 125.5 px = 47.8 dp). Fail: 3 (the known fling rule, j87/#292), 5, 17. Partial: 8, 16.
- The findings are mine, queued as **j90**:
  - Item 17 (worst). Releasing a horizontal drag on a Chats row that offers no swipe acts as a tap. It selects the dragged conversation and marks its unread messages read, while the pane shows the previously opened one. After a restart those messages are gone, never shown.
  - Item 5. On a kept row, the first left drag after a right drag (a direction the row does not offer) does not track.
  - Item 8. One Undo of Cancel in three left the restored row off screen 2.6 s or more after Undo.
  - Item 16. The snackbar does not mirror in RTL.
  - Item 13. Row buttons are drawn at 47.8 dp.
- Questions left with ux-designer-01: the "Dismissed" vs "Hidden" wording, Retry never showing "Sending", and Force RTL not reaching the app when set by adb.
Related: [[device-review-run-method]] [[j87-swipe-spec-fixes]] [[release-phone-after-testing]]

*References: device-review-run-method, j87-swipe-spec-fixes, release-phone-after-testing*

*Observed 2026-10-16 (rust-ui-dev)*
