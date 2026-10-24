---
role: "p2p-network-dev"
class: workflow
topic: "no-sha-before-commit"
description: "never name a commit sha in a message composed in the same command as the commit -- read it from git log after the commit succeeds"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - c1f25978e5124304
---

## never name a commit sha in a message composed in the same command as the commit -- read it from git log after the commit succeeds

On 2026-10-10 a GZCoord REQUEST to rust-ui-dev named "6d0d1b1c" as my fix. The commit had not been made: signing timed out reaching the OpenTimestamps calendars, and I had chained the commit and the message in one command with `;`, so the message went out after the commit failed. A correction message followed (897fed96).

**Why:** a sha written before the commit exists is a guess. A failed commit, for example a signing timeout, turns it into a false reference that another agent builds on.

**How to apply:**
- Commit and push first.
- Read the sha with `git log --oneline -1` or `git ls-remote`.
- Only then compose the message. Chain the steps with `&&`, never `;`.

Related: [[no-autosquash-after-push]].

*References: no-autosquash-after-push*

*Observed 2026-10-10 (p2p-network-dev)*
