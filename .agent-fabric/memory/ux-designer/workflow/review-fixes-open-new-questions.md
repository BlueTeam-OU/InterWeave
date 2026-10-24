---
role: "ux-designer"
class: workflow
topic: "review-fixes-open-new-questions"
description: In a spec review loop, each fix of a rule (a new case, an invariant) breeds a P2/P3 in the paths it did not touch; grep every path that writes the same state before committing the fix
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5028ce78ce6e8380
---

## In a spec review loop, each fix of a rule (a new case, an invariant) breeds a P2/P3 in the paths it did not touch; grep every path that writes the same state before committing the fix

On InterWeave #334 (contact-card spec, 2026-10-21) the review took seven rounds. Most later findings came from my own fixes:
- adding case P removed a warning that case D used to give (F11);
- declaring "a PeerId is never split" left Details' Add as a contact (F13) and two Undos (F13, F14) still able to split one;
- keeping Details open hid the snackbar's Undo (F17).

**Why:** a fix that states a new invariant or a new case changes every path that writes the same state, and I checked only the path the finding named.

**How to apply:** before committing a fix that adds a rule, list every action AND every Undo that writes the same state (create, add route, move, delete, each one's undo, the CLI), and check each against the rule. Also check whether the fix's UI sits under a modal or its focus target disappears. Related: [[focus-table-in-spec]], [[undo-by-reverse-not-deferral]].

Again on #339 (j30, 2026-10-21, eight rounds): rounds 4-7 each found one more place where a state I had just added for one count (one identity held, no default, one identity off) reached a string written for another count ("names the identity"). Each fix opened the next.
**How to apply, for count-dependent states:** when a fix makes something reachable with one identity (or with none, or with a default absent), list every string and control reachable in that state — its success snackbar, its refusal text, its helper, its line on the page — and give each the form that state needs, or show the state cannot be reached; do it in the same commit.

Again on the add-a-peer note (j34, 2026-10-22, rounds 4-6): each fix that let two sources share one control (a scanned code and typed text in one field, Ctrl+V with text or image) opened a mixed state the other rules did not cover. Round 6 ended the chain by removing the mixed state (a scan runs only from an empty field), not by specifying it.
**How to apply:** when a review keeps finding gaps around one new state, ask first whether the state needs to exist; forbidding it at its source is usually one sentence, specifying it is one per rule it touches.

*References: focus-table-in-spec, undo-by-reverse-not-deferral*

*Observed 2026-10-22 (ux-designer)*
