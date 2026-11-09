---
role: "architect-cto"
class: workflow
topic: "j490-s28-propagation-and-reply-shaped-request"
description: "j490 (2026-11-03): SPIKE-001 S28 propagation into architecture was mine because #532 touched only spikes/; a REPLY carrying a REQUEST section is not queued as a job, record it with fabric-jobs add --request <id>"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 20e7b3d1dde17464
---

## j490 (2026-11-03): SPIKE-001 S28 propagation into architecture was mine because #532 touched only spikes/; a REPLY carrying a REQUEST section is not queued as a job, record it with fabric-jobs add --request <id>

j490 (2026-11-03): rust-services-dev-02's SPIKE-001 S28 PR (InterWeave #532)
touched only `spikes/spike-001/`, so the facts' propagation into
`architecture/plugin/CLAUDE-CODE-CHANNEL.md` §Delivery condition, plan §19's
SPIKE-001 carried list and `SPIKES.md`'s carried-measurement paragraph was
architect-cto's: InterWeave #538 (1 work commit, owner's word asked), mirroring
the shape #531 used for S26/S27. `packaging/claude-plugin/INSTALL.md`'s copy of
the same "not run" sentence is rust-services-dev's; OBSERVATION 01a1a165-aff2
→ their j69, PR #539, armed on my word (REPLY 01a1a170-b204, AUTHORIZES header).

**Why:** the README's "Where the architecture disagrees" list names the exact
architecture sentences a spike disproves; the spike's PR lands the facts, the
architecture's owner lands the amendments.

**How to apply:** after a spike PR lands, read that README list for sentences
in my lane and amend them the same day. A REPLY-type message with a REQUEST
section (an arming ask) is not queued as a job on delivery; record it with
`fabric-jobs add --request <message-id>` (title taken from the message) and
close it with `fabric-jobs done` once the authorizing reply is sent. See
[[arming-words-2026-10-27-ii]].

**Closed 2026-11-03:** #538 merged 07b0af1f5 at 17:13Z on the owner's word ("arm 538"). `--defer` names a finding by the label of the review round that raised it (b54ab6ea:F3), not by the current head: `51c21895:F3` was refused as "names no open finding".

*References: arming-words-2026-10-27-ii*

*Observed 2026-11-03 (architect-cto)*
