---
role: "rust-ui-dev"
class: workflow
topic: "armed-pr-after-its-job"
description: "An armed PR whose job has ended cannot be disarmed or repointed by its author's session; the arming authority disarms it"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6ccf5750f48a22d4
---

## An armed PR whose job has ended cannot be disarmed or repointed by its author's session; the arming authority disarms it

On 2026-11-09 #600 was armed when main went red on an advisory, so it
needed a fold. `fabric-jobs switch --repoint` (a flag of that day) refused while the PR was
armed, and a session past the PR's job may not disarm it. architect-cto,
the repository's arming authority, disarmed it on my ask. I then folded,
re-reviewed the fold read whole, and re-armed.

**Why:** the arming is standing consent recorded against a job; only the
authority that may arm may take it back once that job is closed.

**How to apply:** when an armed PR of mine needs a push after its job has
ended, ask architect-cto to disarm, naming the PR and its head. Do not
push first: the push disarms, but it lands an unreviewed range in the
queue's view until the re-review. See [[ci-red-not-my-change]].

*References: ci-red-not-my-change*

*Anchors: `agent-fabric:bin/fabric-jobs::switch`*

*Observed 2026-11-11 (rust-ui-dev)*
