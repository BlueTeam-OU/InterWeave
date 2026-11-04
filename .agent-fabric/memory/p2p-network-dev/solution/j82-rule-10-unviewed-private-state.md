---
role: "p2p-network-dev"
class: solution
topic: "j82-rule-10-unviewed-private-state"
description: "j82 — rule 10 second case: a bound private IP no platform view names is off-local (runtime half); ruling, what is owed, branch; the Pixel failure behind it"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 5ae6fff2cd45eda6
---

## j82 — rule 10 second case: a bound private IP no platform view names is off-local (runtime half); ruling, what is owed, branch; the Pixel failure behind it

Origin (2026-10-27): rust-ui-dev-01 OBSERVATION 01a17bb8. On Pixel 9a at main bdc7ac89 (#465 in), Show
listed both carrier 10.x. The bridge's listen callback (INTERNET) never reports background cellular
(rmnet16, no FOREGROUND) nor the restricted IMS network (rmnet17). Code read on origin/main:
known = view ∪ bound IPs (CONNECTIVITY.md §14 says so), and off_local holds only the view's own
classified IPs. So an unviewed private IP was shown, advertised, offered, and counted as a rule-3
own listener on all four surfaces. That is my j80 device row failing.

Ruling: architect-cto-01 REQUEST 01a17bbc-32a4 (= job j82). Where a view is present, a bound
private-range IP the view does not hold is on no local network for rule 10's four consequences. It
stays bound and in the known set. A view that later names it lifts it; a departure drops it.

Mine: network_change.rs derives off_local = view classification ∪ bound private IPs absent from the
view, while a view is present. Make it order-independent ([[domain-a-withhold-set-holds-both-orders]]).
Owed by name: a test where a listener on a private address no view ever named is absent from
own_addresses, from Identify listen_addrs and from a CONNECT, and is no rule-3 own listener, with a
no-view (desktop) control. Wire tests are network_change.rs / dcutr.rs in tests/connectivity, as in #465.

Folds: architect-cto's records arrive on a for/ branch (ADR-0052 r10 A 2026-10-27, ADR-0062 d4,
CONNECTIVITY §14, DCUTR §6, human-client-android.md). rust-ui-dev-01 widens the bridge listen and
supplies commits. Both fold unrebased; one PR, mine. Gate (e) passes only after rust-ui-dev-01
re-runs the Pixel row.

Open point I raised (01a17bbb, 01a17bbd): a hotspot host or Wi-Fi Direct group owner. Its AP
interface may not be in the view even when widened, so the ruling would hide a LAN address there.
Unmeasured; the build does not wait on it.

Branch develop-qzapp/p2p-network-dev-02/feat/j82-rule-10-second-case-a-bound, worktree
…/InterWeave-j76-show-on-the-pixel-offers-its-j82-rule-10-second-case-a-bound, off origin/main.

State 2026-10-27: amendment 857c6fc2 folded (a3c9ff97). Runtime half pushed at 7d082119:
NetworkSet stores `classified` and derives `off_local` = classified ∪ direct_listeners outside the
view (`listen_scope::is_outside_the_view`, the one predicate) after both observations;
ListenScope holds the view and judges NewListenAddr itself (else the crate tells handlers the
address before the runtime's withhold); runtime `withhold_not_offered` funnel after every view AND
listener observation. Mutations: M1 (rule off) caught by unit + both wire tests; M2 (door ignores
view) only by the listen_scope unit test -- the wire cannot see that ordering; the funnel's DCUtR
re-offer is immediacy only (retry tick re-offers), so its comment claims a rebuilt crate only.
Existing tests a_departed_off_local_ip_stays_withheld_while_bound and the widenings test changed
meaning under the new rule and were updated. Waiting: rust-ui-dev-01's bridge commits (INFO
01a17bd5 sent), then PR, review, full workspace tests, Pixel row by rust-ui-dev-01.

2026-10-27 later: #474 opened, two blind reviews (7d082119: F1 P2 departed half untestable under
private IPs -> global-address test; 7d09a2b3: one P3 on the bridge comment/doc). Folded
architect 951282fc (rule 3) and rust-ui-dev-01 76a6b728 (listen drops INTERNET). Armed on the
owner's word (5 work, 3 fix < 8), queued. Carried to my NEXT branch (j84, CGNAT 100.64/10 ruled
unconditional by architect 01a17c13): fold rust-ui-dev-01's cc12f4a8 on
for/p2p-network-dev-02/adr-0052-listen-comment (the F1 comment). Hotspot host address shows only
at targetSdk 37 (ENABLE_MATCH_NON_THREAD_LOCAL_NETWORKS); a product decision put to the owner.

*References: domain-a-withhold-set-holds-both-orders*

*Anchors: `crates/transport/libp2p/src/runtime/network_change.rs::observe_view`*

*Observed 2026-10-27 (p2p-network-dev)*
