---
role: "rust-ui-dev"
class: workflow
topic: "reopen-ruling-on-own-wrong-fact"
description: "When a decision or a supplier's text rests on a fact I gave and the fact turns out incomplete, I bring it back before building on it; and I search the project's own definitions before telling a decider that something does not exist"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5f20c9f62391d705
---

## When a decision or a supplier's text rests on a fact I gave and the fact turns out incomplete, I bring it back before building on it; and I search the project's own definitions before telling a decider that something does not exist

Twice in one day (2026-10-20) a fact I supplied was wrong and someone had already acted on it:
I told architect-cto there was no "previous build" (no tag) and they ruled on it, though the
project defines a previous-build axis in its testing docs; and I gave language-culture-en a
downgrade fact ("an older release does not start") that held only for some files. Both times
reopening it before anything landed cost one message; the re-ruling was better (a rollback row
running now, a format marker), and architect-cto named the shape as the right one.

**Why:** a ruling or a final text made on my incomplete facts carries my error with the
decider's authority; building on it hides the error behind their name.

**How to apply:**
- before a QUESTION says "X does not exist / has no referent", grep the project's testing,
  release and fixture docs and CI lists for its own definition of X;
- when I find my fact incomplete after a ruling or a delivery: say "correction to my
  <message id>", give the measurement, its control and what I did not measure, and ask them to
  hold or re-rule; build nothing on the old answer meanwhile;
- to a text supplier, ask to drop or hold the sentence, not to guess a replacement, and send the
  decided fact when it exists.
Instance: [[previous-build-axis-exists]].

*References: previous-build-axis-exists*

*Anchors: `architecture/docs/architecture/testing.md::## Compatibility fixtures`, `tools/ci/previous-builds.txt::stage13-45ba3928`*

*Observed 2026-10-23 (rust-ui-dev)*
