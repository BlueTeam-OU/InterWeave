---
role: "devex-tooling"
class: threads
topic: "threads-2026-11-14-fabric-h2-probe"
description: "OPEN: fabric-h2 public forward probe (195.69.140.18, TCP 47100-47199) waits on the provider's confirmation, then fabric-coordinator's probe REQUEST"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "devex-tooling"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 84ed4b380de4aa37
---

## OPEN: fabric-h2 public forward probe (195.69.140.18, TCP 47100-47199) waits on the provider's confirmation, then fabric-coordinator's probe REQUEST

Open as of 2026-11-14. fabric-coordinator (login `user`) merged agent-fabric#597 (ADR-064 rule 1 any-source forward; ADR-069 rule 2 fabric-h2 at 195.69.140.18, TCP 47100-47199, any source), distributed at d1de71d4. Thread root is my message 01a1d3c7-8b7d-718f-bb6e-c77c11fd9f70; their replies are 01a1d843 and 01a1d898.

- No roster row names fabric-h2 until my probe reads 195.69.140.18 back.
- Next step: when the provider confirms the forward, fabric-coordinator sends a probe REQUEST naming the port. Their last reply said "tell me" when the provider confirms, so whoever hears first tells the other.
- Per #597's review, InterWeave's SPIKE-004 deferred limits for a public listener must be answered before fabric-h2 carries an account. The forward has no relay fallback.

**Why:** the probe is the gate on fabric-h2 joining the roster; a fresh session would otherwise not know it is mine.
**How to apply:** on a REQUEST naming the port, probe the forward from outside the LAN and report what reads back.

*Observed 2026-11-14 (devex-tooling)*
