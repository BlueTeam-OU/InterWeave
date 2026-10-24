---
role: "p2p-network-dev"
class: threads
topic: "j66-stage-21-device-rows"
description: "j66 Stage 21 step 5 device rows on the Wi-Fi harness (added address survives restart; two fresh phones connect after trust): what is needed, branch, what waits (2026-10-24)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 2012c2afda65a60b
---

## j66 Stage 21 step 5 device rows on the Wi-Fi harness (added address survives restart; two fresh phones connect after trust): what is needed, branch, what waits (2026-10-24)

architect-cto REQUEST 01a16c3a (owner 2026-10-24: "complete stage 21"); my REPLY 01a16c3c, INFO 01a16c3d (branch name).
Branch develop-qzapp/p2p-network-dev-02/test/j66-stage-21-close-the-two-device off main 93b63a39, pushed, empty;
worktree ../InterWeave-j66-stage-21-close-the-two-device. PR opens only after j65's lands (one open PR per agent;
j65 already 22 work); both change tests/android-e2e-cases, fold main after j65.

Facts the plan rests on:
- This host (VM a private-network address behind NAT) cannot be dialled by a phone, so row 1 (the phone dials an added address)
  needs a second phone as far end, or an owner-opened inbound port to this VM.
- No phone-side case adds an address or waits for an un-added peer: new case bodies in
  tests/android-e2e-cases/src/lib.rs (dispatch in `run`, cases::*), so a new APK built and installed by the
  installed signer (rust-ui-dev-01, install -r) on BOTH phones; ask for the sha256s. See
  [[workflow-shared-device-build-by-the-installed-signer]].
- The embedded host persists the bootstrap overlay under its root (crates/transport/embedded/tests/host.rs ~592).
- "Fresh" = pm clear of the app: needs the owner's word (shared Pixel).
Asked of the owner via architect-cto: Pixel 9a + A40 on one non-isolating Wi-Fi with wireless adb to this host,
the word to clear app data, optionally the inbound port.
Run files follow tests/android-e2e/runs/2026-10-22-pixel-9a-api37-wifi.md's shape; send paths to architect-cto.
Related: [[j63-wifi-device-run]], [[j65-superseded-state]].

2026-10-24 (session 301d0a8c): architect-cto REPLY 01a16c4b agreed the plan, owner asked for phones/clear/port.
Cases pushed at 173f4dfb (j63 harness merged unrebased 373a9937): added_address, await_peer, pair in
tests/android-e2e-cases/src/pairing.rs (NEED_A_RUNTIME, no app change); driver tests/android-e2e/tests/pairing.rs,
AdbDevice::connect_to(serial), wifi::reversed_on. Stand-ins green over the private IP; mutations: overlay not fed
to provider -> row 1 fails after restart; B trusting in apart step -> fails. Measured: A trusting alone, A's row ends
last_outcome connected (B closes at establishment) -- assert no message accepted, not no connection. Sent REPLY
01a16c61 with the commit. Next: owner places phones -> rust-ui-dev-01 builds/installs 173f4dfb (or later) on both,
sha256s; pm clear both (owner's word); run row 2 then row 1 with SERIAL_A/_B, WIFI_ADDRESS_A/_B; write two run files.
Device apart window 15 s. PR after j65 lands.

*References: j63-wifi-device-run, j65-superseded-state, workflow-shared-device-build-by-the-installed-signer*

*Observed 2026-10-24 (p2p-network-dev)*
