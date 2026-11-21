---
role: "architect-cto"
class: threads
topic: "gzcoord-interweave-only-cutover-2026-11-17"
description: "owner 2026-11-17, GZCoord moves to InterWeave only (data and control plane); job j753 state, what part 1 found, the two REQUESTs out, and the final REPLY still owed"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 15aa7e27060ecf1a
---

## owner 2026-11-17, GZCoord moves to InterWeave only (data and control plane); job j753 state, what part 1 found, the two REQUESTs out, and the final REPLY still owed

The owner's word of 2026-11-17, relayed by fabric-coordinator (REQUEST 01a1e873-da80): GZCoord switches to InterWeave only, data and control plane, as soon as possible; the InterWeave (rust) work is mine to scope and distribute. Job j753 (interweave, high).

**Done 2026-11-17.**
- Part 1 (13 misses, 516 duplicates of agent-fabric docs/live-checks/2026-11-09-interweave-parity-computed.md): no class is on InterWeave's side. 12 misses are the pair (a broadcast, its own sender), which the relay echoes and the carrier does not carry; 1 (01a1824e, mine to p2p-network-dev-02, 2026-10-28) has no carrier row and its cause is not recoverable (Adapter.post logs to stderr only). 477 of 516 duplicates are one broadcast, 01a18c45 of 2026-10-30: the carrier's resends at the 300 s cap while no receipt came; the daemon's duplicate window is 5 minutes (ADR-0019), so each copy passed by rule. python-dev-03 confirmed from the 30-account collection (REPLY 01a1e8ab to my REQUEST 01a1e8aa) and fixed the parity tool on their branch.
- Part 2 read answers sent to fabric-coordinator in the second REPLY, 01a1e8b5-ccd7 (sessions per daemon, size, reconnect, burst), from InterWeave main 773b2c685.
- Two things must change before the cutover: v0.3.0 holds a peer that failed to answer for 30 s doubling to 5 minutes (main: 30 s flat, ADR-0011 A 2026-11-14); and interweave-human drops its own healthy session when a draining peer's refusal comes back as BackendUnavailable (crates/human/transport-client/src/problem.rs ends_session).

**Out, awaiting replies.**
- REQUEST 01a1e8b4-b918 to p2p-network-dev-01: 30-peer fan-in (k = 1, 4, 9, 16 parts of 32 KiB) and reach after a daemon restart on v0.3.0 and main, as a run record under tests/automated-client/runs/; which commit of main to tag.
- REQUEST 01a1e8b4-bfaf to rust-ui-dev-01: confirm and fix the session drop with a test; time a fan-out of 30 input lines with 0, 1, 3 peers down (listen sends one line at a time); one `outbound` line per answer or the README says two.

**Owed.** The final REPLY to 01a1e873 with the measured figures, what changed, and the release to pin by tag and commit: not 0.3.0, the next tag cut from main once the session-drop fix is on it (number decided at the cut under ADR-0065; the owner tags by hand, ADR-0066). A defect that lands is an InterWeave PR under my arming word.

Related: [[control-plane-on-interweave-cli-2026-10-27]], [[stage-20-fleet-carrier-2026-10-19]], [[request-repository-line-files-the-job-re-point-before-switch]].

**Release question, 2026-11-17.** The owner asked in session whether to release 0.4.0. My answer: yes, cut from main once rust-ui-dev-01's session-drop fix (their j288) is on it, so the fleet moves its pin once; fallback if j288 drags past a day: cut 0.4.0 now for the 30 s redial and follow with 0.4.1. Not checked: that 0.4.0 is the number ADR-0065 gives. The owner has not answered. Both rust developers took their REQUESTs (p2p-network-dev-01 j174, rust-ui-dev-01 j288). The owner also gave the word to arm #690 and gzapp #1265 the same day: jobs j754 and j755.

*References: control-plane-on-interweave-cli-2026-10-27, request-repository-line-files-the-job-re-point-before-switch, stage-20-fleet-carrier-2026-10-19*

*Observed 2026-11-17 (architect-cto)*
