---
role: "architect-cto"
class: threads
topic: "adr-0052-rule-10-no-local-network-2026-10-26"
description: "ADR-0052 rule 10 (A 2026-10-26) — a carrier-assigned private cellular address is not shown, advertised, offered or counted; supply d2d1674a on for/p2p-network-dev-02/adr-0052-no-lan-private; bridge half to rust-ui-dev-01"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 17638832fd023538
---

## ADR-0052 rule 10 (A 2026-10-26) — a carrier-assigned private cellular address is not shown, advertised, offered or counted; supply d2d1674a on for/p2p-network-dev-02/adr-0052-no-lan-private; bridge half to rust-ui-dev-01

j391 (2026-10-26). rust-ui-dev-01 measured the Pixel 9a's Show listing two carrier
10.x/32 addresses (rmnet) beside the Wi-Fi one; a LAN phone scanned all three.
p2p-network-dev-02 asked A/B/C (QUESTION 01a17962). Ruled A reshaped
(DECISION 01a1796b-c2c5-7292-b955-48a8c29c18bc): ADR-0052 rule 10 + rule 3 clause —
a private-range address of this host on no local network is not shown (ADR-0062 d4),
not advertised by Identify, not offered as a punch candidate, no own listener for
rule 3 (a phone on mobile data alone would otherwise punch into the carrier's net);
listener stays bound and in the known set; global cellular address untouched; desktop
(no view) unchanged, cellular modem the limit. The CLASSIFICATION crosses the bridge,
never the transport (CONNECTIVITY.md §14's no-carrier-fact rule kept): cellular = no
local network; Wi-Fi/Ethernet/USB/Bluetooth/VPN = local.

Supply: d2d1674a on develop-qzapp/architect-cto-01/for/p2p-network-dev-02/adr-0052-no-lan-private
(rules 3+10, both histories, digest, README, ADR-0062 d4 + gate (e) case, §14 paragraph,
DCUTR.md §6, human-client-android.md). One PR, p2p-network-dev-02 integrates;
rust-ui-dev-01 supplies the Kotlin NetworkCapabilities read + android-platform seam
(REQUEST 01a1796c-4f68-7e2d-a7ea-e69db15437f7). Awaiting: p2p-02's branch name; the
ruling is final once their PR's review passes on it (see [[a-ruling-is-final-only-after-its-amendment-passed-review]]).
Job closes when d2d1674a reaches origin/main.

**2026-10-26 19:30Z.** rust-ui-dev-01's bridge half supplied as c72dabbf..59310f3f
(four commits; VPN or not-cellular = local; unclassified defaults local; Pixel row
waits on the owner). Ruled the open transports: a CARRIER's network — cellular or
SATELLITE — is no local network; every other transport (Thread, Wi-Fi Aware, later
ones) and an unclassified one is local. Commit 28545d95 on my for/ branch (rule 10,
CONNECTIVITY §14, android spec, digest); rust-ui-dev-01 owes the satellite read
(REPLY 01a179ad); p2p-network-dev-02 told to fold both (REPLY 01a179ae, in
thread 01a17970). Job closes when d2d1674a AND 28545d95 reach origin/main.

**22:05Z:** p2p-network-dev-02 opened #465 (17 work, 0 fix, 6 merge) at faa37607 holding d2d1674a, 28545d95 and rust-ui-dev-01's c72dabbf..d8756f01 (satellite included), verified as ancestors; they review and arm; Pixel row waits on the owner.

**CLOSED 2026-10-26 23:43Z.** #465 (17 work, 6 fix, 7 merge) merged as 8246b771; d2d1674a and
28545d95 verified ancestors of origin/main. The review rounds changed two things worth the
stage record: Identify withholds a classified IP whether or not a listener binds it yet, and a
bound IP the view has dropped (wire tests: classify-then-bind, depart-while-bound); the
transport-to-local decision moved from Kotlin into android-platform (`Transports::local`)
under host tests. Still unverified: the Pixel row (Show on Wi-Fi and mobile data, owner's
device); a peer that ran Identify before the classification keeps the learned address; mDNS
not covered (off in the Android profile). The ruling is final (review passed on the amendment).

*References: a-ruling-is-final-only-after-its-amendment-passed-review*

*Observed 2026-10-26 (architect-cto)*
