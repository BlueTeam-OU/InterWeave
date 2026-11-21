---
role: "rust-ui-dev"
class: threads
topic: "j95-plan"
description: "j95 long messages, second batch: what is merged, what A5 run is still owed, where the rig is"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 46f20c5d9391751e
  - 6a6ef6db3af74ac4
---

## j95 long messages, second batch: what is merged, what A5 run is still owed, where the rig is

j95 builds design/2026-11-04-long-message.md. Five PRs have merged: #565,
#583, #585, #600 and #643 (the open-cost batch, 6e52954cb, 2026-11-12;
12 work and 3 fix commits). The design, rulings and figures are in
[[j95-design-choices]], [[j95-cluster-measurements]],
[[j95-reader-open-cost]], and the repository's
apps/human-android/device-runs/2026-11-12-samsung-a40-pixel-9a.md.

The opening on main builds for 0.4 s at the first go, then in 50 ms turns. The bar's
0.5 s timer starts before the first go. An opening's end goes through
`Shared.reader_built` and stays outside the input queue (#643 F1/F2).
ux-designer-01 ruled to land it (01a1cfc3). Their section 5.4 text is on #642.

Still owed for A5, as later runs, not failures:
- release-build open times on the A40;
- Back about 0.6 s on heading autolinks and reference link, and Escape on the
  desktop at 0.2 s and 0.6 s, each with its Back-to-close time and how long
  the first screen showed (ux INFO 01a1d006).

Known limits:
- No test pins the bar-before-first-go order, because the first go runs on a
  real `Instant`.
- Show source during an opening does nothing.
- Opening as source builds the source in one go.

The rig for the runs is in [[j95-a40-ab-rig]]. ux-designer-01 was told that #643
landed and what F2 changed (INFO 01a1d097).

2026-11-12 ~17:00Z: release run (owner: "take the A40 and do the release run") on branch develop-qzapp/rust-ui-dev-02/test/j95-a40-release-run off main 6e52954cb. A real release APK has no trace/case/stand-ins, so the build is the debug variant with [profile.release] settings via CARGO_PROFILE_DEV_* env (lto thin, cgu 1, debug 1, overflow ON, assertions OFF): ab/release-profile app 48833554, test 4db8c691 (build-release-profile.sh). The A40 held a release 0.3.0 (signer a7f38b5e, 3b89be89, installed 16:42); uninstalled on the owner's word. 5 rounds running -> ab/release-runs. Then: ab-summary, device record, PR.
2026-11-12 ~23:30 phone time: release-profile timed case 5 complete rounds (ab/release-complete): medians = A/B candidate within 4 ms (paragraph 146, list 370, code 355, table 358, headings 371, images 403, heading autolinks 513, reference 752, distinct headings 1652, distinct links 1311), End <= 65 ms, 1 frame. One round lost to adb exit 255 (wireless). Back 0.6 s (ab/back600, 3 rounds x 4 shapes, back-times.py): `input` start-up ~0.47 s under screenrecord; heading autolinks/reference link alone open in 0.25/0.41 s so Back lands after rows (close 82-205 ms after Back). Stress shapes: Back inside the opening (~0.67 s), bar shown and moving, but close only at the build's end, 0.87-0.96 s after Back -> FINDING: Android does not take Back between turns (testing backend does). Fix owed (my lane). TM-57 (j106) on this branch too; architect-cto told (REPLY 01a1d12d-cde7).

*References: j95-a40-ab-rig, j95-cluster-measurements, j95-design-choices, j95-reader-open-cost*

*Observed 2026-11-12 (rust-ui-dev)*
