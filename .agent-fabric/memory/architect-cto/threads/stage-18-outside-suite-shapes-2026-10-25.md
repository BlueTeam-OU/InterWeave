---
role: "architect-cto"
class: threads
topic: "stage-18-outside-suite-shapes-2026-10-25"
description: "2026-10-25 j368: four adversarial test shapes relayed by fabric-coordinator judged against plan §21; three covered by name, the misbehaving-relay one handed to p2p-network-dev-01 (REQUEST 01a1736a-446d) as test or limit on S21-07"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 588e5bc609bc7928
---

## 2026-10-25 j368: four adversarial test shapes relayed by fabric-coordinator judged against plan §21; three covered by name, the misbehaving-relay one handed to p2p-network-dev-01 (REQUEST 01a1736a-446d) as test or limit on S21-07

On 2026-10-25 the owner relayed through fabric-coordinator (OBSERVATION
01a17360-c4fc) four test shapes from an outside conformance suite and
asked whether plan §21 covers them. Judged at origin/main 14161365:

- **Misbehaving double** (TAKEN 2026-10-25 by p2p-network-dev-01, REPLY 01a1736c-4229, their j126, branch test/j126-misbehaving-relay; items 1-4 and S8 AutoNAT counterpart as tests, limit cells where not testable): covered for a direct peer
  (`tests/direct-v2/tests/stage6_malformed_frames.rs`), a broadcast
  publisher (S21-03) and a daemon in front of a client (facade tests).
  NOT whole for a relay in front of a real daemon: stalled RESERVE,
  undecodable hop frame, relay failing mid-circuit
  (`relay_failover.rs` header names it unproved), plus the hop gate's
  late drop RELAY.md says no test shuts. Handed to p2p-network-dev-01
  (S7's holder) as REQUEST 01a1736a-446d: a test or a `limit` cell on
  S21-07, reply expected.
- **Crash between record and reply**: nothing durable before
  `AcceptedV2` by decision (ADR-0018/0019); pinned by
  `automated_client.rs::a_withheld_lease_is_retried_under_one_id_and_lands_once`
  (receiver restart, second delivery collapsed), the store's duplicate
  refusal and `read_pairs`; TM-05 records the residual.
- **Resend under one id**: TM-05 and the same automated-client test.
- **Unsupported vs failed**: the repository decided the opposite for
  tests (fail by name, ADR-0053 rule 9, §21 decided (4)/(7), P2); what
  cannot be graded is a matrix `limit` cell.

No tree change: CLAUDE.md §7 keeps the suite's name out of project
files, and the one open item lands in the matrix row by its holder.
REPLY to fabric-coordinator 01a1736a-4822.

**How to apply:** when p2p-network-dev-01 answers, check S21-07 (and
S21-08 if taken) on their PR names test or limit per item; then close
j368.

**2026-10-25 15:58Z p2p-network-dev-01 REPLY 01a173c8-7ded** (branch
`develop-qzapp/p2p-network-dev-01/test/j126-misbehaving-relay`, head
d0c6ce59, no PR yet): every item a test or a limit, S21-07/S21-08 rows at
3aed1dde read and found right. Tests: `tests/security/tests/relay_double.rs`
(stalled RESERVE holds the ask 60 s, the crate's STREAM_TIMEOUT, then the
spare; garbage RESERVE/CONNECT answers fail at once; a relay stopped under
a circuit bounds the send at DIRECT_TIMEOUT and is found by the reserving
end's keepalive), `relay_hop_gate.rs` late drop, `autonat_misbehaving_server.rs`
(S8, crate surface). Limits on S21-07: (a) the DIALLING end holds no
reservation on the relay, is never pinged there, keeps the peer Connected;
(b) a relay that does not speak ping is found dead by neither end. Their
own defect fixed in the same branch: the reconnect scheduler re-dialled an
infrastructure-only relay under its own origin after a failed reservation
dial (7af22528, 7ba67dda).

**My REPLY 01a173cc-8454 (reply expected: the build branch name):**
supplied 2e1028a9 on `for/p2p-network-dev-01/relay-md-late-drop-test`
(RELAY.md §8 step-6 note names the hop-gate test). Ruled, each with a
CONNECTIVITY.md §14 item 5 amendment (A 2026-10-25) I supply onto the
build branch: (a) the keepalive extends to every RELAYED connection, pinged
end to end toward the data-plane peer by both ends, a missed liveness
closing it (check the ping bytes against the per-circuit byte budget first);
(b) a control peer's refusal of the ping protocol is a missed liveness at
both ends, closed and reported by cause; a relay that cannot be found dead
is not a usable relay, required not preferred. The keepalive rule has no
ADR (the owner's "keepalive yes" 2026-09-26 lives in CONNECTIVITY.md §14);
if (a)/(b) grow a third rule, promote the three into a record.
- 16:04Z p2p-network-dev-01 REPLY 01a173ce-7abe: (a)+(b) built on the same j126 branch after its CI run; budget verified 64 MiB/1 h (`default_circuit_bytes`), ping ≈0.02 %, no floor. Supplied e4a8c227 on `for/p2p-network-dev-01/connectivity-14-keepalive-relayed` (§14 A 2026-10-25 + contracts/CONNECTIVITY.md rules-after-freeze sentence), REPLY 01a173d1-b964. j368 closes when the j126 PR merges carrying 2e1028a9 and e4a8c227.
- j372 2026-10-25: #447 (6 work, 4 fix) AUTHORIZED @4a302ee0 (REPLY 01a173cf-f97b to ux-designer-01): 5.2 as decided; carry: cites 4df2fa26 by branch name.

*Observed 2026-10-25 (architect-cto)*
