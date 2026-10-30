---
role: "architect-cto"
class: threads
topic: "adr-0052-rule-10-outside-the-view-2026-10-27"
description: "ADR-0052 rule 10 second case (A 2026-10-27) — a bound private address the platform's view does not hold is on no local network; Pixel gate (e) failed on the first rule-10 build; supply 857c6fc2 for p2p-network-dev-02's j82; hotspot AP =…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a46263002bd84dc5
---

## ADR-0052 rule 10 second case (A 2026-10-27) — a bound private address the platform's view does not hold is on no local network; Pixel gate (e) failed on the first rule-10 build; supply 857c6fc2 for p2p-network-dev-02's j82; hotspot AP = the named limit

j404 (2026-10-27). Follows [[adr-0052-rule-10-no-local-network-2026-10-26]].

**Measured (rust-ui-dev-01, OBSERVATION 01a17bb8, Pixel 9a at main bdc7ac89, the first build with rule 10):** Show still lists both carrier 10.x/32 addresses. The bridge's listen (`registerNetworkCallback`, INTERNET, VPN admitted) never reported their networks: rmnet16 = cellular held in the background behind validated Wi-Fi (no FOREGROUND), rmnet17 = IMS, restricted (no NOT_RESTRICTED). View = Wi-Fi only; log "4 addresses (0 on no local network)"; the wildcard listener binds both. p2p-network-dev-02 (OBSERVATION 01a17bbb) read all four surfaces on origin/main: known set = view ∪ bound set (§14), off_local = view's classification only, so an address the view never names is shown, advertised, offered and counted for rule 3 (on cellular alone the IMS address keeps the rule-3 probe open).

**Ruled (b), platform-neutral:** where a view is present, a bound private-range address the view does not hold lies on no local network for the four consequences, lifted when a view names it; listener stays bound and in the known set. Reason: the view is every network the platform lets the app use; a withheld network is one the app cannot use and no peer reaches it on, whichever reason. Fail direction deliberately opposite to the unclassified-transport default (hide a missed LAN vs the rule-3 probe + Identify leak). Rejected: (a) bridge enumerating interfaces (classification by absence from interface names, a second source); (c) widen the listen only (IMS withheld regardless). Bridge half (rust-ui-dev-01): listen admits every network the platform reports (clear capabilities, keep VPN) so a no-uplink Wi-Fi stays shown; measure whether a tethering downstream (hotspot AP, Wi-Fi Direct GO) is readable by an app — if yes it enters the view as local, else the hotspot host's unshown address is the recorded limit (the client's code connects the pair one way). Runtime half: p2p-network-dev-02 j82, branch `feat/j82-rule-10-second-case-a-bound`, one PR, both halves folded.

**Supply:** 857c6fc2 on `for/p2p-network-dev-02/adr-0052-rule-10-outside-the-view` (from d514608b): rule 10 + row + note, ADR-0062 d4 + row + note, digest ×2, README row, CONNECTIVITY §14 new paragraph (owed cases named), DCUTR §6, human-client-android.md. Closes when on origin/main; gate (e) passes only on the re-run Pixel row. Ruling final once the PR's review passes on it.

**Lesson:** a platform "view" rule needs its complement stated: what an address outside the view means. The first amendment ruled only classified networks and left the unreported ones at the old default, so the measured device case passed every surface.

**2026-10-27 06:00Z:** p2p-network-dev-02 opened #474 (4 work, 0 fix) at 7d082119 holding 857c6fc2; its review F3: rule 3 named only the first case; fixed 951282fc on `for/p2p-network-dev-02/adr-0052-rule-3-outside-the-view`. rust-ui-dev-01 supplied the bridge half 76a6b728 (listen drops INTERNET, keeps VPN; restricted/untrusted stay out) and measured on a 76a6b728+7d082119 build: Wi-Fi+data PASS (Wi-Fi alone shown, both rmnet withheld), data-alone PASS (nothing shown; cellular reported and classified carrier). Owner's side question: on data alone nothing reachable was shown because relay reservations failed and no AutoNAT address was confirmed — a provisioning/relay question, not rule 10's. Still to measure: no-uplink Wi-Fi, hotspot host, A40.

**06:40Z, #474 review items (p2p-network-dev-02 OBSERVATION 01a17c11):** F1 P3: the tethering clause (human-client-android.md:128, rule 10) reads as built; rides my next batch (j410) once the hotspot measurement lands. CGNAT: ruled rule 10 covers 100.64.0.0/10 UNCONDITIONALLY (no view needed; RFC 6598 is provider space, never a person's LAN); today only Identify advertises it (rule 2 floor refuses it as special-use for candidates/probes, Show refuses, offer sites filter, rule 3 counts private only). p2p-02 builds the predicate in their next PR; texts mine on j410; owed test: a 100.64 listener absent from the advertised set and a CONNECT with no view.

**07:12Z (rust-ui-dev-01 INFO 01a17c33):** no-uplink row PASS on the Pixel joined to the A40's data-less hotspot: shown; the network carried INTERNET (unvalidated), so the old listen heard it too — the widening admits only LOCAL_NETWORK-without-INTERNET networks, i.e. the own hotspot, reported at target 37 only; a background network is never heard (the platform adds FOREGROUND to an app's listen). Comment fix cc12f4a8 on their for/p2p-network-dev-02/adr-0052-listen-comment (from 76a6b728), folded after #474 lands. j410 texts must say: "a local network with no uplink stays in the view" was true before the widening; what the widening admits is the LOCAL_NETWORK case at target 37.

**08:12Z, j84 (p2p-network-dev-02, feat/j84-rule-10-shared-address-space off e1e653d7):** 100.64/10 off-local built (is_shared_address_space, lies_on_no_local_network at the detector and Identify's door; owed case in netns; cc12f4a8 folded). Counter-example to my premise: a WireGuard/Tailscale-style mesh numbers a person's peers from 100.64/10 (and #475's own two-host case uses 100.64.0.1/.2). j410 texts: reason = no peer may learn and dial it (rule 2 floor refuses it as a candidate), so advertising serves nobody; such an address lives in the operator door (rule 9) as a static entry; if the floor ever admits the shared range, the off-local rule moves with it. Lesson: a "never a LAN" premise about an address range is a claim about every user of the range; state the reason in the record's own rules instead.

**09:00Z:** #476 (j84, 100.64/10 off-local, head 78f0282b) armed on the owner's word ("merge 476") after its re-review; my rule 10 texts are NOT folded there (a push disarms) and land on my own j410 batch after it merges. p2p-01 ran two_hosts against main+#476: 2/2 twice.

**09:30Z:** #476 MERGED 740ed001. My batch: PR #479 (2 work, 0 fix) = rule 10 amendments (ii) widened-listen facts + hotspot cause (e420cf69) and (iii) shared address space with the corrected reason (69978cb0); CONNECTIVITY §14, DCUTR §6, Android spec, README row. Review dispatched; arms on the owner's word (under 8). The 5b limits word and j413 (headless custody) can ride #479 as later commits while it is open.

*References: adr-0052-rule-10-no-local-network-2026-10-26*

*Observed 2026-10-27 (architect-cto)*
