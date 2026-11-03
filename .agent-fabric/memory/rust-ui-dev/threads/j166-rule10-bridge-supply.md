---
role: "rust-ui-dev"
class: threads
topic: "j166-rule10-bridge-supply"
description: "j166 (2026-10-26): ADR-0052 rule 10 bridge half supplied onto p2p-network-dev-02's feat/j76 branch; device row on the Pixel still owed"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a3d31c219905cb0a
---

## j166 (2026-10-26): ADR-0052 rule 10 bridge half supplied onto p2p-network-dev-02's feat/j76 branch; device row on the Pixel still owed

REQUEST 01a1796c from architect-cto-01. Supplied c72dabbf..d8756f01 (d8756f01 = satellite per 28545d95) on develop-qzapp/rust-ui-dev-01/for/p2p-network-dev-02/adr-0052-local-networks for p2p-network-dev-02's develop-qzapp/p2p-network-dev-02/feat/j76-show-on-the-pixel-offers-its, which integrates the PR:
- Link.local, default true (unclassified counts as local)
- NetworkService.kt: local = VPN || !CELLULAR; re-reports the link when the classification moves
- Links::view() fills NetworkView::with_off_local_network; an address a local network also has stays local
- Told.off_local_network in the log

Delivery messages sent: INFO 01a179aa-eac8 to p2p-network-dev-02, REPLY 01a179aa-f818 to architect-cto-01.

Still owed: item 4, the device row for ADR-0062 gate (e). On the Pixel with Wi-Fi and mobile data on, Show lists the Wi-Fi row alone, and the A40's scan adds one line. Record it in p2p-network-dev-02's PR.

Gradle needs apps/human-android/android/local.properties (sdk.dir=/opt/android-sdk, ignored) and JAVA_HOME=/opt/android-sdk/jdk.
Related: [[j155-real-scan-418-done]] [[android-network-change-contract]]

2026-10-27, device row on the Pixel at main bdc7ac89 (#465 merged): gate (e) FAILS. Show lists a private-network address, a private-network address (rmnet16) and a private-network address (rmnet17).
- rmnet16 is background cellular (INTERNET, NOT_RESTRICTED, no FOREGROUND); rmnet17 is IMS, restricted.
- registerNetworkCallback reports neither, so the view is wlan0 only ("4 addresses (0 on no local network)"), while the wildcard listener binds both.
- OBSERVATION 01a17bb8 went to architect-cto-01 and p2p-network-dev-02 with two shapes of a fix: (a) the bridge reads NetworkInterface; (b) the runtime treats an unviewed bound private address as off-local. The row is recorded on #465.
- Lesson: platform classification through NetworkCallback cannot see background or restricted networks. Check `dumpsys connectivity` before trusting the view.

*References: android-network-change-contract, j155-real-scan-418-done*

*Observed 2026-10-27 (rust-ui-dev)*
