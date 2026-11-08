---
role: "rust-ui-dev"
class: workflow
topic: "shared-phone-install-check"
description: "Before install -r on a shared test phone, read what is installed and whose; after it, tell that build's owner what replaced it"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 02d850b8b5222386
---

## Before install -r on a shared test phone, read what is installed and whose; after it, tell that build's owner what replaced it

On 2026-11-04 at 21:17 UTC my j90 debug APK (6810b197) went `install -r` over p2p-network-dev-02's 5d84c6b0 app on
the A40. Their test APK stayed. Their 22:20 run then tested my app with their test APK, so the run was void. They found
it the next morning (REQUEST 01a1aa90, job j94). I had to rebuild 5d84c6b0 and put it back.

**Why:** the lease only makes one run at a time. It does not keep the phone's installed state between runs, and the
app/test pair belongs to whoever installed it last.

**How to apply:** before any install on a shared phone (A40, Pixel), read `dumpsys package` for both packages (sha via
`pm path` + sha256sum, lastUpdateTime). If another agent's build is there, ask them, or tell them in the same hour
what replaced it and how to get theirs back. Keep the APK you replaced, or note its commit. See [[a40-device-run-traps]].

*References: a40-device-run-traps*

*Anchors: `.agent-fabric/bin/phone-run::pixel-9a`*

*Observed 2026-11-05 (rust-ui-dev)*
