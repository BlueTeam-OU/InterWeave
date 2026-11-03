---
role: "rust-ui-dev"
class: threads
topic: "j139-human-chat-wifi-run"
description: "j139 human_chat Wi-Fi device run — built as cde318b5 on #451, device run waits on the Pixel's address"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 9e5f749016e2347b
---

## j139 human_chat Wi-Fi device run — built as cde318b5 on #451, device run waits on the Pixel's address

j139 (REQUEST 01a1653c from p2p-network-dev-02, j63's Wi-Fi run): both asks were answered 2026-10-22
(the 15:02 install was be4ca6b4, APK sha 9433e6ff…; the androidTest APK at be4ca6b4 was installed, sha 1d289d62…).
My half is commit cde318b5 on #451 (2026-10-26):
`over_wifi_the_phone_chats_with_the_desktop_dialling_its_lan_address` in tests/android-e2e/tests/human_chat.rs.
It uses main's src/wifi.rs (b9df66a1), so nothing more is needed from p2p-network-dev-02 (INFO 01a1757b).
The per-side exchange is now `chat()`, which the stand-in run shares; that run passes.

Still owed: the device run, which needs the Pixel's current address:port from the owner.
Run it through `.agent-fabric/bin/phone-run` with ANDROID_SERIAL, INTERWEAVE_ANDROID_INSTRUMENTATION
and INTERWEAVE_ANDROID_WIFI_ADDRESS set, using `--ignored --test-threads 1`.
First build the daemon: `cargo build -p interweave-transport-daemon`.
Whatever build is installed must carry the human_chat case (any main build does). Install nothing unless asked.
The facade retries a send made before D's dial ([[j138-store-answers-pr451]]); if the phone's first send ends NeedsAttention, that is the finding.

2026-10-27: the device run PASSES on the Pixel (61 s). The first run failed with CapabilityDenied: the case asked for admin.peers alone, and peers() needs admin.status. Fixed in 50778fb9 on the local branch test/j155-drop-store-key-hook. The package is interweave-android-e2e-tests and the instrumentation is org.interweave.human.test/org.interweave.human.test.E2eCase. Told p2p-network-dev-02.

*References: j138-store-answers-pr451*

*Observed 2026-10-25 (rust-ui-dev)*
