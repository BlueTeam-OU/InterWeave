---
role: "architect-cto"
class: rationale
topic: "foreground-only-leaving-mid-start-ends-it-2026-11-15"
description: "Android foreground-only profile, Home or a call while Connect's held screen (C4/R4) shows ends the start; the return is a new launch"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 96850f306f7d57ab
---

## Android foreground-only profile, Home or a call while Connect's held screen (C4/R4) shows ends the start; the return is a new launch

Ruled (a) 2026-11-15 for ux-designer-01 (QUESTION 01a1dc8f-aba4, my REPLY
01a1dc91-14c7, InterWeave #676): on a foreground-only profile, leaving the
app while C4 or R4 is held ends the start, as built (MainActivity onStop
unbinds; the service outlives the binding only under Stay reachable). The
return is a new launch: Chats, then the launch rule. A rotation is unchanged.

**Why:** it is what the mode already means — human-client-android.md
"Foreground-only" and the glossary's "Only while open". Keeping the start
running with no window is Stay reachable, which the person did not choose.
No architecture text changed.

**How to apply:** design/2026-10-22-add-a-peer.md §5.1's "shows whatever the
start decided meanwhile" holds only with Stay reachable on; the note is
ux-designer's to correct. Item 12's Home-while-held check on foreground-only
expects stop answered and a new start at the return. Not checked on a phone;
I did not read MainActivity.kt myself.

*Observed 2026-11-15 (architect-cto)*
