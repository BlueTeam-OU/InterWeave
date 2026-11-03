---
role: "rust-ui-dev"
class: threads
topic: "phones-lan-2026-10-30"
description: "Where the test phones are since the LAN moved on 2026-10-30, their lease names, and whose adb key the Pixel accepts"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 97c6999e2d9679bd
---

## Where the test phones are since the LAN moved on 2026-10-30, their lease names, and whose adb key the Pixel accepts

On 2026-10-30 the LAN moved from a private-network address to a private-network address. This is from fabric-coordinator's INFOs 01a18b21, 01a18b22 and 01a18b2f.

- The Pixel 9a is at a private-network address:43463. Its wireless-debugging port moves; 43462 refused.
- The Samsung A40 is at a private-network address:37099.
- Lease names are `pixel-9a` and `samsung-a40`. Never `a40`: that name would let two agents on the phone at once.
- Three agents share the two phones.
- That day the owner paired only p2p-network-dev-02's key with the Pixel. If adb refuses rust-ui-dev-01's key, send fabric-coordinator a REPLY quoting the exact adb output. The coordinator puts the re-pair to the owner.

This answers the address half of j158's blocker.

Update 2026-10-30 ~06:10 local: the Pixel ACCEPTED rust-ui-dev-01's key. j177 installed both APKs at aec72790 with `-r` through phone-run (app sha256 d8b293eb…, androidTest 425c686e…; REPLY 01a18b37). So the Pixel half of j158's blocker is cleared; the A40 key is still untested.
Related: [[pixel-install-r-keeps-data]].

Update 2026-10-30 later: the A40 accepts rust-ui-dev-01's key too (j158 install -r + screens run). The Pixel drops adb now and then: a retry after ~20 s worked.

*References: pixel-install-r-keeps-data*

*Observed 2026-10-30 (rust-ui-dev)*
