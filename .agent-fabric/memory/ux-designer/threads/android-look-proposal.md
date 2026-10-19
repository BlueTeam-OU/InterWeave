---
role: "ux-designer"
class: threads
topic: "android-look-proposal"
description: "Owner asked (2026-10-16) for the Android client to look like an Android app with colour, not a prebeta; Material 3 proposal awaiting owner decisions"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 395dc9566d3fa2a8
  - a4864a6089350af5
---

## Owner asked (2026-10-16) for the Android client to look like an Android app with colour, not a prebeta; Material 3 proposal awaiting owner decisions

2026-10-16 the owner said: "improve a bit that ux with colours - look like a prebeta not like an android application".
Proposal published as a mockup (claude.ai artifact N66eN3T2pVAKYdQt8MAVYL): Material 3, colour roles from the icon
palette (navy #232c63, teal #2ec4b6, amber #ffb547; assets/README.md), app bars, list items with id avatars and amber
unread badge, bubbles, chips 32 dp drawn / 48 dp touch, inverse snackbar, 3 dp focus ring. Contrast measured AA.
Open owner decisions: bubbles, id avatars, last-message preview. Next: spec under design/, roles named for
rust-ui-dev (palette in app.slint is theirs while brand-comms has no holder here).

Related: [[swipe-actions-j1]], [[focus-table-in-spec]]

2026-10-17: owner chose bubbles, id avatars, last-message subtitle. Spec is design/2026-10-16-android-look.md on InterWeave #293
(branch develop-qzapp/ux-designer-01/docs/android-look), reviewed in 3 rounds + fixes, language-culture-en folded 3 times.
Done follow-up (293494a2 on the swipe for/ branch, 2026-10-17): the look spec gives the dark neutral swipe fill its own label role (swipe-on-neutral, #11152a on #9aa1bb);
swipe spec §7 still names one swipe-on-fill for all four fills — change it once #293 lands. Arming #293 needs the owner's word (under 8 work commits).

2026-10-17 04:15Z: #293 merged (62cbce3a) on the owner's "arm 293". Handed to rust-ui-dev-01 to build (REQUEST 01a14813-2baa); my device review follows their install.
2026-10-17: ruled B on rust-ui-dev-01's QUESTION 01a14816-d3a1 (list times): English words via labels.rs (weekdays, months, date and AM/PM templates, language-culture-en finalises) plus the phone's 12/24-hour setting (DateFormat.is24HourFormat). TODO: amend android-look §5.2 and §9 to say so, on the for/ branch once #286 has merged (needs main's copy of the file).
- 2026-10-18 j22: #300 (025bb324 + language-culture-en F1-F3 + round 1) answered at 0f7db13d on for/user/swipe-followups (REPLY 01a150d4-67a3); separator names the day as the list does.

*References: focus-table-in-spec, swipe-actions-j1*

*Observed 2026-10-18 (ux-designer)*
