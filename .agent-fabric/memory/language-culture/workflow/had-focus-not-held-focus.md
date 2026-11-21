---
role: "language-culture"
class: workflow
topic: "had-focus-not-held-focus"
description: "beside \"held\" controls (identity §8), a control \"had focus\", never \"held focus\"; a fallback named only in history needs \"that fallback\" after it is stated"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - f7b5157bfef9a1d0
---

## beside "held" controls (identity §8), a control "had focus", never "held focus"; a fallback named only in history needs "that fallback" after it is stated

In design/2026-10-19-android-identity.md, "held" is a defined state: a control that does nothing while R2's check runs. So "the control that held it (focus)" and "whatever held focus" read as that state. On #693 (cd0ba9741, 2026-11-16) they became "had focus". The same collision remains outside that diff at §7 ("that held it when Back was pressed"), and I raised it with ux-designer.

**Why:** the cold read named it as the worst word-sense clash on that page.

**How to apply:** when a text defines "held", "hold" or "holds" for a state, write focus as something a control "has". Also: a revision note that says what a rule replaced must first state the rule ("X. Before this revision, that fallback named only Y"). A bare "not only Y" in a spec's body contrasts with text its reader cannot see. Related: [[held-and-waits-read-as-queued]], [[word-against-every-state]].

*References: held-and-waits-read-as-queued, word-against-every-state*

*Anchors: `design/2026-10-19-android-identity.md::focus stays on the control that had focus`*

*Observed 2026-11-16 (language-culture)*
