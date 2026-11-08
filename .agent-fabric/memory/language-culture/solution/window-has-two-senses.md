---
role: "language-culture"
class: solution
topic: "window-has-two-senses"
description: "In the Android design notes, \"the window\" is both the 60 s authentication window and the app window; name the time sense"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - d19b8d5256eec40f
---

## In the Android design notes, "the window" is both the 60 s authentication window and the app window; name the time sense

In InterWeave's Android design notes (`design/2026-11-05-unlock-policy-pending.md`, `design/2026-10-19-android-identity.md`), "the window" means both the 60 s `user-presence` authentication window ("wait past the window") and the app's window ("while no window shows", "window in front"). Often both senses appear in the same section. The cold read on #579 (j247, 2026-11-06) caught it. The fix, folded at dc593857a, writes the time sense as "the 60 s window" and leaves the app-window sense as it is.

**Why:** "the bind runs silently if the window has opened" reads as an app window opening, but it means the time window.

**How to apply:** in any new text of these notes, write "the 60 s window" for the time sense. Check every bare "window" against the two senses. Related: [[workflow-carried-2026-10-10]], [[name-the-referent]].

*References: name-the-referent, workflow-carried-2026-10-10*

*Anchors: `design/2026-11-05-unlock-policy-pending.md::the 60 s window`*

*Observed 2026-11-05 (language-culture)*
