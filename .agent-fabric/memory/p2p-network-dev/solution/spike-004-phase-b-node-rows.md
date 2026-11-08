---
role: "p2p-network-dev"
class: solution
topic: "spike-004-phase-b-node-rows"
description: "SPIKE-004 phase B's five items as node rows on the podman NAT matrix (PR #127) — the four harness facts a row needs before it measures anything, and the six findings (1 of 2; rest in spike-004-phase-b-node-rows-part-2)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6c6774366f178edc
  - ddaecd6a8180e249
---

## SPIKE-004 phase B's five items as node rows on the podman NAT matrix (PR #127) — the four harness facts a row needs before it measures anything, and the six findings (1 of 2; rest in spike-004-phase-b-node-rows-part-2)

`spikes/spike-004/phase-b/nodes.sh` + `node/` (PR #127, 2026-09-26; recorded run
`REPRODUCTION-2026-09-26.log`, nodes.sh at 8ee1ec5f, pin 6500391e).

**Harness facts, each found by a row that measured the wrong thing first:**
- AutoNAT probes only a public literal (`is_probeable_address`), and a relay hands out
  reservation addresses only once AutoNAT verified one → public side on `PUB_SUBNET`
  11.0.0.0/24 (rootless netns, routes nowhere on the host); two relays verify EACH OTHER.
- Rootless podman routes between every bridge (forwarding on): punches "succeeded" over
  LAN private addresses (ADR-0052 admits them beside a private listener). Blackhole the LAN
  pool on routers and public nodes.
- A Linux router RSTs an unsolicited SYN → every TCP simultaneous open refused. Drop it
  silently (RFC 5382 REQ-4) on the routers' public interface.
- SELinux enforces on this host: bind mounts need `:z`, else `podman exec -d` fails silently.
- An `absent`/`await` about "after X" needs a window starting at X (`mark`), or a pre-X line
  (address-less reservation refusals) matches.
- **A punch trial needs a port of its own**: router conntrack outlives a trial; with every
  trial on 4001 the eim router went endpoint-dependent from trial 2 (eim 2/10 → 10/10 with
  `4100+i`). Measured wrong, then withdrawn — the #127 review's risk B.
- **Never `cmd | grep -q` (or `| head`) under `pipefail`**: grep's early exit SIGPIPEs cmd,
  the pipeline reports 141, and a present match reads as absent — it fired on `nft list |
  grep -q`. Match in one awk over the file (pattern via ENVIRON, since `-v` unescapes `\(`),
  or capture first, then match.
- **A burst that never reaches the relay decides nothing**: late dialers without `--data`
  for their target were refused by their own gate; guard every "count of X" with "X happened".

**Findings sent to architect-cto (seq 5558):** relay serves before AutoNAT verification and
holds unusable reservations against its ceiling; a network change rebuilds nothing and there
is no keepalive (contract says rebuild); RELAY.md §8 misreads the relay rate limiter (bucket
60, +1/min per IP, not 60/min); 128 circuits unreachable from 2 addresses; pre-Noise 30/min per
source bounds seating; eim TCP punches 2/10 — WITHDRAWN (harness, see above; corrected to
architect-cto seq 5962). Final record bd0ab554 (nodes.sh fa62ad86): eim 10/10, eds 0/10; early
control (ceiling 3) denies nobody; rate limiter 60 then 4 of 10 four minutes later.

Related: [[replacing-a-libp2p-behaviour-leaks-its-tasks]], [[reproduction-logs-commit-beside-the-pin]].

**architect-cto's rulings (seq 5561, 2026-09-26):** RELAY.md §8 limiter text is theirs (defaults
kept); network change: the CODE is wrong — at the NetworkChanged event CLOSE every connection
whose local address is in `removed` (mine, next code PR); a ping on relay control connections is
the owner's call (architect recommends yes, not built until the owner says so); the relay offers
hop only while ServedAddresses holds ≥1 verified direct address, withdrawn like an unauthorized
class otherwise (mine, same code PR, cites RELAY.md §8's new rule text). Order: #127 merges →
tell architect-cto → their docs PR (limiter, hop rule, CONNECTIVITY §14 note, verdict) → my code
PR (2a + 3). Verdict: interface change NOT MET until 2(a) lands.

**2026-09-26, the owner: "keepalive yes"** — 2(b) decided: the crate's ping at its default, relay
control connections only, built in the same code PR as 2(a) and 3 (told architect-cto).

**#127 MERGED 2026-09-26 as ef09e31a** on the owner's "land 127" (told architect-cto seq 6140).
Carried to the next PR (P3s, comment on #127): the eim punch cause is the port LIKELY, not
isolated (router A's public iface eth1 in both failing runs, eth0 in the passing one) — fix the
README/port-control header wording or run a control with the interface order fixed; `fail`
guards after `x=$(grep|awk)` unreachable under set -e; late-burst guard counts non-x outcomes;
usage line omits early/ratelimit. Next: architect-cto's docs PR, then my code PR (2a, 2b, 3).

**Rule text for the code PR (architect-cto #128, seq 6143/6158):** RELAY.md §8 step-6 Note — hop
offered only while ServedAddresses holds a verified direct address; withdrawn before the first
verdict and whenever the set empties, as for an unauthorized class; withdrawn hop refuses NEW
reservations AND RENEWALS; Status::Enable stays forced below. CONNECTIVITY.md (transport) §14
step-10 note item (5): close every connection whose LOCAL endpoint IP equals the host IP of any
address in `removed` (key on host, not full multiaddr — outbound conns use ephemeral ports);
keepalive = the bounded ping at the crate's default interval on relay control connections only;
a missed liveness closes and takes the ladder. Re-read tests/connectivity/tests/dcutr.rs "the kept
reservation across a removal" — it may pin the wrong thing now. #128 arms on the owner's word.

Continued in [[spike-004-phase-b-node-rows-part-2]].

*References: replacing-a-libp2p-behaviour-leaks-its-tasks, reproduction-logs-commit-beside-the-pin, spike-004-phase-b-node-rows-part-2*

*Observed 2026-09-26 (p2p-network-dev)*
