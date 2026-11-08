---
role: "rust-ui-dev"
class: threads
topic: "j155-real-scan-418-done"
description: "#418 camera Scan verified on both phones 2026-10-26 (owner's hands): both directions, trust step, permission re-ask; evidence a scratch path"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 39470fc8720927d3
---

## #418 camera Scan verified on both phones 2026-10-26 (owner's hands): both directions, trust step, permission re-ask; evidence a scratch path

2026-10-26 ~10:40-10:56 local, owner holding the phones; Pixel runs main 05d3f9cd (sha 14cb5e0f), A40 an older build (sha d95d4464, 10-25 19:22).
- A40 Show -> Pixel Scan: Pixel holds a private-network address:31823 (A40 `ss`: LISTEN 0.0.0.0:31823).
- Pixel Show -> A40 Scan: A40 added 3 lines :12538 = wlan0 a private-network address + rmnet16 a private-network address + rmnet17 a private-network address (carrier /32s) -> OBSERVATION 01a176fa to p2p-network-dev-02.
- Camera after an earlier refusal: Android asked again at Scan (not "for good"); CameraDenied line not on this path.
- Redo on the A40 with the Pixel untrusted+removed: "Added 3 addresses for ...gEYmzkDh.", trust block with full PeerId, Copy PeerId, Trust/Not now; Trust closed the page to Peers. State restored (trusted, 3 rows).
- Screenshots a scratch path. Pairing: both phones paired to rust-ui-dev-01 on 2026-10-26 (Pixel connect 44599, A40 connect 39813; A40 via its lease + own adb port).
Related: [[j134-camera-scan-pr418]] [[a40-own-adb-server]]

*References: a40-own-adb-server, j134-camera-scan-pr418*

*Observed 2026-10-26 (rust-ui-dev)*
