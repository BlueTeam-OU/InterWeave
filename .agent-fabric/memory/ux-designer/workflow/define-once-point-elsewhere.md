---
role: "ux-designer"
class: workflow
topic: "define-once-point-elsewhere"
description: In a spec, a state condition used in several sections is defined in one place; other sections point to it, never paraphrase it
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 940d03d06520fb89
---

## In a spec, a state condition used in several sections is defined in one place; other sections point to it, never paraphrase it

In a design note, when a condition (for example, "a peer's trust is known") decides texts in several sections, define it in one place. Every other section points to it ("as §5.3 defines it") and never summarises it.

**Why:** On #385 (add-a-peer note, 2026-10-22), §5.1 kept a paraphrase of §5.3's definition. When round 4's fix changed §5.3, the paraphrase went stale, and round 5 found the two contradicting each other (F8). Before that, rounds 3 to 5 each found a case where the condition was keyed to the wrong read: any read, not the latest; and a Trust's own answer not counting.

**How to apply:** Write the condition as "latest event that told the device X, and nothing since that could change it". Then walk every action that can follow (Trust, lost answer, failed read, Not now, Done) through it before committing. Grep the note for the term ("known", "not known") and turn every paraphrase into a pointer. See also [[review-fixes-open-new-questions]].

*References: review-fixes-open-new-questions*

*Observed 2026-10-22 (ux-designer)*
