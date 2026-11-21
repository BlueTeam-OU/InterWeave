---
role: "language-culture"
class: workflow
topic: "test-step-words-that-collide"
description: "In InterWeave Android test steps, \"time from\", \"put back\", \"in Settings\", \"A run without X checked\" read wrongly (#676 j322)"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 984203fece4dee2b
---

## In InterWeave Android test steps, "time from", "put back", "in Settings", "A run without X checked" read wrongly (#676 j322)

In device-run steps of design/ specs (add-a-peer s8 item 12, PR #676, commit b09b16202), four phrasings read two ways:
- "time from the first line" (imperative "time" reads as a noun) -> "count the seconds from".
- "put the mode back" sits beside the Back key -> "set the mode to its earlier value". Do not write "set it to what it was after that run": that reads as the value after the run.
- "in Settings" is the app's Settings page -> "the phone's own settings" for the system.
- "A run without both halves checked no window" (participle garden path) -> "A run that does not show both halves has checked no window".
Also, when a list loses an item in the author's edit, check the "and" before the last item that is left.

**Why:** found in the first pass and by the cold read on #676. A second-language reader takes the wrong meaning.
**How to apply:** check every imperative step and every "back"/"Settings" in a spec's test text. See [[workflow-carried-2026-10-10]] and [[below-list-collides]].

*References: below-list-collides, workflow-carried-2026-10-10*

*Anchors: `design/2026-10-22-add-a-peer.md::count the seconds from`*

*Observed 2026-11-15 (language-culture)*
