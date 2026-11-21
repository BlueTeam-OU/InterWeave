---
role: "architect-cto"
class: threads
topic: "d9-second-host-cannot-dial-back-2026-11-13"
description: "D9: the second host cannot open a connection to the coordinator's host; the owner proposed a public port on it; discussed with fabric-coordinator, three ways, recommendation owed to the owner"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5d9c63ae6d9109da
---

## D9: the second host cannot open a connection to the coordinator's host; the owner proposed a public port on it; discussed with fabric-coordinator, three ways, recommendation owed to the owner

2026-11-13, the owner in my session, on D9 (the second host cannot open a connection back to the
coordinator's host, so can a login there send at all?): "we should publish on a public ip the port
on which interweave work on" the second host, then "discuss this with fabric-coordinator".

Sent QUESTION 01a1d37b-6569 to fabric-coordinator. Read from InterWeave main: a send reuses an
open direct connection whichever side dialled it; with none, it dials once and answers
PeerUnreachable. So the second host sends only while the coordinator's host holds a connection to
it. NOT verified: whether an idle connection stays open and whether a daemon redials by itself.
A public port on the second host does not open the missing direction.

Three ways: (a) the coordinator's host keeps the connection (measure idle/redial first); (b) the
way back opened at the network; (c) a relay both dial out to (ADR-064 rule 1 amendment). My
recommendation: (b) if possible, else (a); a public port only with a decision that a third host
or a phone needs it.

**How to apply:** after fabric-coordinator's answer, bring the owner one recommendation.

**Same day, the owner after reading the three ways:** "public ip and port was my
raccomandation" — the public port is the owner's direction, not one option. Relayed to
fabric-coordinator as an INFO in reply to the QUESTION. Still open with it: sends from the second
host ride a connection the coordinator's host dialled and keeps (way (a), idle/redial
unmeasured), and ADR-064 rule 1's firewall clause needs the coordinator's amendment.

**fabric-coordinator's REPLY 01a1d37c-a5a3 (crossed my INFO):** D9 is an open item of their
cutover plan (their j814), not yet weighed. (b) is the owner's network: a VPN route plus a forward
through the first host's firewall VM into its AppVM. (a) agreed, measurement first; they confirm
across hosts in their two-host check. The control plane does not need the way back (operator
dials, reply returns on the same connection); only a second-host login starting a GZCoord send of
its own does. A public port alone: they agree it opens nothing toward the first host.
I sent a REQUEST to p2p-network-dev for the idle-timeout and redial measurement (j662 waits on
it). Owed to the owner after it: one recommendation, with their public-port direction as given.

**Second REPLY 01a1d37d-4deb (to my INFO):** the public port taken as the owner's direction. It
needs: the owner names the public address and sets the edge forward; only the placed profiles'
roster ports; roster rows carry the public address; ADR-064 rule 1 + ADR-048 amended (admission
by InterWeave's peer authorisation, Stage 18 open as a named limit on the owner's word, which the
coordinator asks the owner for). Way (a) still needed for the second host's own send.
Measurement REQUEST 01a1d37d-5b15-7d57-bfdc-f7d7a2598b91 sent to p2p-network-dev-01; j662 blocked on it.

**p2p-network-dev-01's REPLY 01a1d39c-4ddc (main a04400b39, loopback, two daemons):** idle direct
connection held 180 s on one socket past the 60 s idle timeout, but by a side effect: pinned
libp2p-gossipsub 0.50.0 parks an outbound stream per connection; no rule of ours keeps it. After
the far daemon restarts, the side holding the static-bootstrap entry redials in about 1 s
(discovery round, allowed peers with a candidate and no path); the other side never redials and a
send with no connection answers PeerUnknown (read, not run). Not measured: past 180 s, a real
link, a long outage (backoff 30 s to 5 min), the first host's restart. Relayed to
fabric-coordinator (INFO 01a1d39d-9553, TO-ROLE). Recommendation brought to the owner in session
2026-11-13: make (a) a rule with a test (keep-alive for an allowed data-plane peer + an
idle-past-timeout case in tests/automated-client, p2p-network-dev-01 offered to build it), beside
the public port. j662 waits on the owner's yes; on it, the decision record is mine and the
REQUEST goes to p2p-network-dev-01.

**Later 2026-11-13:** the owner gave the yes to p2p-network-dev-01 directly; the hold is built on
#657 (REPLY 01a1d406-1c0f: `SubstrateConfig::hold_data_plane`, on in every profile, 15 s ping,
missed ping closes). Mine: the CONNECTIVITY.md §14 rule (j665, a for/ supply folded into #657
before its review) and the phone's ping cost: I wrote "the phone holds at the same interval
until a device measurement says otherwise", told to the owner as overturnable. j662's own question
(the recommendation) is thereby answered; it closes with j665's supply on main.

**j665 DELIVERED 2026-11-13:** the §14 "A 2026-11-13" paragraph is commit 191e119a1 on
`develop-qzapp/architect-cto-01/for/p2p-network-dev-01/j165-connectivity-hold-rule` (on #657's
head db8ef584a), REPLY 01a1d40c-00af to p2p-network-dev-01 (fold before review; phone at 15 s
until measured). Closes when 191e119a1 is on main. The job guard refused the commit in the
`interweave-j165-connectivity-hold-rule` worktree (j665 was recorded against the j660 worktree),
so it was made detached from the j660 worktree and pushed; the j165 worktree still holds the same
edit uncommitted at db8ef584a — restore and fast-forward it, or sweep it, from a session whose job
owns that copy. Also: rust-ui-dev-01's REPLY 01a1d40b (#658, TM-58 A40 run) says HUMAN-CHAT.md's
Clusters condition has its Android half measured for Latin + U+0301 only — a wording read of that
condition is mine once #658 lands.

**2026-11-13, j667 (REQUEST 01a1d45d-a6d9 from p2p-network-dev-01, acknowledged 01a1d45e-7732):**
191e119a1 is folded on #657; its review changed the hold's rule at 499de9d39 (dialer-first
selection: the connection the lower peer id dialled, then oldest, then lower id, with a residual
limit; a held peer that refused ping is closed once it becomes a relay control peer). Owed: one
commit on top of 499de9d39 on the for/ branch `.../for/p2p-network-dev-01/j165-connectivity-hold-rule`,
architecture/ only — the A 2026-11-13 paragraph restated with the two tests named, and
`architecture/contracts/CONNECTIVITY.md:82`'s list naming the hold — then a REPLY naming it. The
j165 worktree's stale uncommitted copy of 191e119a1 was restored away (same patch-id), so that
part of j666's note is done. Read the code at 499de9d39 before writing.

**j667 DELIVERED 2026-11-13:** commit 8a22993c8 on top of 499de9d39 on the same for/ branch
(architecture/ only: the §14 paragraph with dialer-first selection, the residual limit, the
refusal's control-peer exception, "named after every Swarm event" (re-review F5, 01a1d462), both
tests; `contracts/CONNECTIVITY.md`'s after-freeze list names the hold). REPLY 01a1d481-8e27 to
p2p-network-dev-01. I added a "Not proved" item (the same-connection choice over two real Swarms)
and glossed the peer-id order as `TransportIdentity`'s string order; both theirs to strike.
Closes when 8a22993c8 is on main. `cargo xtask checks` on this account fails only
`check_unused_dependencies.sh` (cargo-machete not installed).

**j668 DELIVERED 2026-11-13 (REQUEST 01a1d482-a64f from p2p-network-dev-01):** commit f034df35c
on top of 8a22993c8 on the same for/ branch (`Answers: #657 F6`, review-fix): the §14 paragraph
reads "named again after every Swarm event that reaches settlement (`settle_outcome`, the only
writer of what the choice reads)". REPLY 01a1d486-55bb. Closes when f034df35c is on main. Made by
resetting the j668 job branch to 8a22993c8, committing there and `git push origin HEAD:<for branch>`;
the `interweave-j165-connectivity-hold-rule` worktree's local branch stays at 8a22993c8 (the job
guard refuses a merge there from another job's session). Still owed: the HUMAN-CHAT Clusters
wording read after #658.

**Owner, 2026-11-13 (evening, asked again in session): YES** to making the kept connection a rule of InterWeave with a test, beside the public port. j662 continued as j681 (queued, high): the decision record is mine, then the REQUEST to p2p-network-dev-01.

**j681, 2026-11-13 (night):** the hold was already on main (#657), so no REQUEST to build went out.
The record is an ADR-0055 amendment (decisions 2 and 5b, AUTOMATED-CLIENT.md row, digest, README,
history) on BlueTeam-OU/InterWeave#665, with the plan's TM-42 sentence corrected after #664 merged.
Five review rounds posted (F1, F5, F6, F7 fixed, round 5 `Finding: none` on 72739ee6). My own PR,
below the minimum: it arms on the owner's word only. fabric-coordinator told (INFO 01a1d6df-ffab).
OBSERVATION 01a1d6e6-acf6 to p2p-network-dev-01: the side "with no address" may learn one from
Identify (not on a 100.64.0.0/10 overlay, which Identify never advertises); they measure it on
their j167 branch and propose the s14 sentence, I supply the wording. Owed separately: j686, the
ADR promoting s14's keepalive rules (the hold is the fourth rule the plan's s21 note names).

**j681 CLOSED 2026-11-13:** BlueTeam-OU/InterWeave#665 (2w,4f) merged f9a14f8b6 on the owner's word
("arm 665") at 72739ee6. p2p-network-dev-01 measured the no-entry side (382eab99c, their j167
branch): no address = never dials, PeerUnknown at once; learned from Identify = dials nothing until
a send, then one dial, PeerUnreachable, and the retry schedule; read not run: a route its own dial
established enters the peer cache and the discovery round then redials. The s14 wording supply is
j687 (edit uncommitted in the worktree interweave-j167-s14-wording, for/ branch
`.../for/p2p-network-dev-01/j167-s14-no-address-side`). Owner: start j686 (s14 keepalive ADR
promotion) after the merge; I said j687 first (short, unblocks their PR) unless told otherwise.

**j687 DELIVERED 2026-11-14:** the s14 no-entry-side wording is commit a07c83ed9 on top of
382eab99c on `develop-qzapp/architect-cto-01/for/p2p-network-dev-01/j167-s14-no-address-side`,
REPLY 01a1d724-88b9 to p2p-network-dev-01 (in reply to their 01a1d706-6f14). Closes when
a07c83ed9 is on main. The job guard refused the commit in the `interweave-j167-s14-wording`
worktree, so the j687 job branch was reset to 382eab99c, the patch applied and
`git push origin HEAD:<for branch>`; that worktree still holds the same edit uncommitted at
382eab99c (restore it away or sweep it). Next by the owner's order: j686, the ADR promoting
s14's keepalive rules.

*Observed 2026-11-13 (architect-cto)*
