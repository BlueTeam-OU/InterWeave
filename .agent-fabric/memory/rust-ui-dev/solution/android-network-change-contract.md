---
role: "rust-ui-dev"
class: solution
topic: "android-network-change-contract"
description: "§20 step 5 (j44): what the Android Service's network callback owes EmbeddedHost::network_changed, and the wildcard-listener rule"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 1cee9453e2dbc444
  - c55ddf1091c15b6c
---

## §20 step 5 (j44): what the Android Service's network callback owes EmbeddedHost::network_changed, and the wildcard-listener rule

From p2p-network-dev-01's INFO 01a1214d-e7f0-77a0-a087-f3bf18bb2221 (2026-10-09). Runtime is on
develop-qzapp/p2p-network-dev-01/feat/network-change-binding @ 915491ba, not merged yet. The
ruling is architect-cto's (seq 33736).
- Call: `host.network_changed(NetworkView { addresses: Vec<IpAddr> })`; NetworkView is
  re-exported from interweave_transport_embedded.
- Pass a SNAPSHOT of every usable address from ConnectivityManager callbacks with LinkProperties,
  never a delta. An empty view means offline. Addresses only: no network id, interface or carrier.
- Call it when the addresses change. A new network with the same addresses needs no call.
- It never blocks, from any thread; the latest view replaces one not yet read. A call while the
  host is stopping goes nowhere.
- Until the first view, the runtime sees addresses only through the wildcard listener's 10 s
  getifaddrs poll, which may see nothing on Android. So send the first view at Service start.
- An embedded-android profile may list only wildcard listeners (/ip4/0.0.0.0, /ip6/::); anything
  else is refused at load as AndroidListenerNotWildcard. human-android.yaml already complies.
Check this against the merged code before building on it. See [[embedded-host-seam]].
- 2026-10-14: still nothing in apps/human-android calls ServiceHost::network_changed (service.rs:453); j44 taken up. Proposed to p2p-network-dev-02 (01a139ef-fb72): NetworkView gains dns_servers (LinkProperties.getDnsServers) and search_domains (getDomains split on commas), empty = unknown; build addresses first, add DNS when their field lands.
- 2026-10-14 BUILT on branch develop-qzapp/rust-ui-dev-01/feat/j44-interweave-20-step-5-the-android (head 1e7f1a55, pushed, no PR, 4 work commits): android-platform `network.rs` (Links union per network handle, NetworkSlot with its OWN lock — ServiceHost::start holds `running` for the whole start, so a callback through it would block; attach at start replays the current view; detach before stop or try_unwrap fails "a waiter still held"), `addresses_from_wire` (len byte + octets), JNI networkWatching/Link/Lost/Unwatched, Kotlin callback on a HandlerThread (INTERNET cap, NOT_VPN removed). The "never waits" test needed a barrier + 5 ms settle to be load-bearing (mutation passed 1 in 3 without it). p2p-network-dev-02 (01a139f3): build addresses-only now; dns_servers/search_domains/private_dns come in their part 2 PR, NetworkView gets #[non_exhaustive] + constructor. Owed: device run (Wi-Fi off/on, logcat "the runtime was told the networks: N addresses"), needs the owner's phone IP:port.
- 2026-10-14 (01a13bb7, architect-cto DECISION): search_domains DROPPED (FQDN only, ADR-0052 rule 2): never send getDomains. Two fields: dns_servers (empty = unknown), private_dns (strict mode only). NetworkView goes #[non_exhaustive] with NetworkView::new(addresses) + setters in p2p-network-dev-02's j30 PR: j44's Links::view literal must switch to new() when that lands.
- 2026-10-15: #280 merged (NetworkView #[non_exhaustive], new()+with_dns_servers+with_private_dns; contract: the ACTIVE network's DNS, a view when addresses OR DNS differ). j44 head 534cd243 (pushed, no PR): NetworkReport::Link{network, link: Link{addresses,dns_servers,private_dns}}, NetworkReport::Default{network}, Told{addresses,dns_servers,strict_private_dns} counts; Kotlin registerDefaultNetworkCallback on the same HandlerThread, privateDns = isPrivateDnsActive ? privateDnsServerName : null; JNI networkDefault(0 = none). Pixel run 1: runtime started with 4 addresses (wlan0's). Sequence script (scratchpad j44-net-run.sh: Private DNS strict dns.google then RESTORED, airplane, wifi) writes /data/local/tmp/j44-net.log; the first one ran 2026-10-14 19:39Z with the old build, unread.
- 2026-10-16: device run on 534cd243 passed (strict DNS on/off, airplane 0 then back, wifi off/on, settings restored). After #283 merged, folded origin/main (68cac96d: conflicts in jni.rs start(), lib.rs re-exports and tests/service.rs, each two independent additions; around_start wraps start, network_told logged on Ok). Opened BlueTeam-OU/InterWeave#285 (9 work). The Android check needs JAVA_HOME=/opt/android-sdk/jdk, and the script has no +x bit, so run it with `bash`.
- 2026-10-16: #285 review: 3 comment P3s fixed in fdde4072, clean re-review. Armed at 9 work, 1 fix. Codex is additional_reviews in policies/arming.json, not required. A re-review brief whose base is not on main needs `card_base:` (the first round's base). Open risks from the review, NOT fixed here: one process-global slot, so a late Unwatched from a destroyed Service instance could clear a new instance's view; INTERNET without VALIDATED admits captive portals; linkAddresses omits 464XLAT stacked links (an architecture question); no instrumented Kotlin test.

*References: embedded-host-seam*

*Observed 2026-10-09 (rust-ui-dev)*
