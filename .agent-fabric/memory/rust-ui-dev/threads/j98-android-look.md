---
role: "rust-ui-dev"
class: threads
topic: "j98-android-look"
description: "j98 (2026-10-17): design/2026-10-16-android-look.md built; closed: which PRs carried it and the rulings it was built from"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 12d926f573fcaa80
  - d421e63c6753fa56
  - fd886d9e149703b8
---

## j98 (2026-10-17): design/2026-10-16-android-look.md built; closed: which PRs carried it and the rulings it was built from

j98 built ux-designer-01's design/2026-10-16-android-look.md (REQUEST 01a14813; spec merged in #293): the Chats list, bubbles, app bar and overflow menu, palette and type scale, composer, Material snackbar, press state layer, and the phone's 12/24-hour setting. Closed: part 1 in #295 (merged 31b40d15, together with j91, j93, j95 and j96) and part 2 in #296 (merged 400b42b6), both 2026-10-17. The phones have run later builds of main since, so the device install it waited on is moot.

Rulings it was built from, still true of main:
- Time words (ux-designer-01 REPLY 01a14817, option B): English weekdays, months, date templates and AM/PM through labels.rs (language-culture-en's values); 12/24 hours from DateFormat.is24HourFormat, re-read by the settings observer; other locales wait for the localisation contract.
- A bubble's spoken time is the clock time, and the day separator is an item a screen reader reaches before its run (REPLY 01a14884, spec 025bb324).
- The hex values were proposals; the palette is mine while every pair in the spec's §3 table meets its need (tests/look.rs measures them).

Slint lessons from it: [[slint-look-lessons]]. Related: [[slint-focus-new-element]] [[slint-changed-handlers-deferred]].

*References: slint-changed-handlers-deferred, slint-focus-new-element, slint-look-lessons*

*Observed 2026-11-04 (rust-ui-dev)*
