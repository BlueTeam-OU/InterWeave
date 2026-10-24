---
role: "p2p-network-dev"
class: workflow
topic: "host-address-classes-bound-what-a-wire-test-can-prove"
description: "Before calling a wire test \"owed, needs a bigger harness\", check the host's address classes — develop-qzapp has only lo (no MULTICAST) and one RFC 1918 /32, so AutoNAT verdicts and mDNS exchanges cannot happen on it at any harness size…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - bf3f9abbd4944620
---

## Before calling a wire test "owed, needs a bigger harness", check the host's address classes — develop-qzapp has only lo (no MULTICAST) and one RFC 1918 /32, so AutoNAT verdicts and mDNS exchanges cannot happen on it at any harness size; never borrow the shared eth0

Twice in two days (2026-09-19/20) a test I had written up as "owed,
needs a larger harness" turned out not constructible on this host at
all, and the wording invited someone to try to build it:

- A real direct-inbound verdict needs a real AutoNAT probe;
  `is_probeable_address` admits only a public `/ip4`/`/ip6` literal.
  `ip -4 -o addr` here: `lo` on loopback, `eth0` one RFC 1918 /32 — no
  candidate is probeable, so no verdict arrives at any harness size.
  (DCUtR's boundary admits a private pair, which is why dcutr.rs runs
  here and the AutoNAT one cannot.)
- mDNS conformance needs a multicast domain: `ip -o link` shows `lo`
  without the MULTICAST flag and no multicast route. eth0 has the flag
  but is a shared network this session does not control: announcing on
  it is outward-facing, and a pass there is inherited, not chosen.

**How to apply:** for any wire test, first ask which address class or
link property the mechanism needs (public literal, private pair,
multicast, a NAT) and read `ip -o addr` / `ip -o link` / `ip route`
against it. If the host cannot supply it, write "owed, and not
constructible on this host", name the predicate that excludes it, and
place the test where it can run — the rootless-podman environments of
SPIKE-004 phase B (NAT, public-range subnet) and SPIKE-010 (multicast),
whose environment is configured and then measured with a negative
control. Never widen the production address rule or add a test-only
knob to make the wire move: that proves a lookalike. Related:
[[routing-tests-run-on-the-hosts-private-address]],
[[spike-004-phase-b-node-rows]].

*References: routing-tests-run-on-the-hosts-private-address, spike-004-phase-b-node-rows*

*Anchors: `crates/transport/runtime/src/reachability.rs::is_probeable_address`*

*Observed 2026-10-18 (p2p-network-dev)*
