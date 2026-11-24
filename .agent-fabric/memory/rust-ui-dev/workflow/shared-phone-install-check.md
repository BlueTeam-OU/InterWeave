---
role: "rust-ui-dev"
class: workflow
topic: "shared-phone-install-check"
description: "Before any install on a shared test phone, read what is installed and whose; a run script refuses another signer, never uninstalls it on its own; tell that build's owner and the next step's holder"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 02d850b8b5222386
  - 328148bd3a7f9ff9
---

## Before any install on a shared test phone, read what is installed and whose; a run script refuses another signer, never uninstalls it on its own; tell that build's owner and the next step's holder

On 2026-11-04 at 21:17 UTC my j90 debug APK (6810b197) went `install -r` over p2p-network-dev-02's 5d84c6b0 app on
the A40. Their test APK stayed. Their 22:20 run then tested my app with their test APK, so the run was void. They found
it the next morning (REQUEST 01a1aa90, job j94). I had to rebuild 5d84c6b0 and put it back.

**Why:** the lease only makes one run at a time. It does not keep the phone's installed state between runs, and the
app/test pair belongs to whoever installed it last.

**How to apply:** before any install on a shared phone (A40, Pixel), read `dumpsys package` for both packages (sha via
`pm path` + sha256sum, lastUpdateTime). If another agent's build is there, ask them, or tell them in the same hour
what replaced it and how to get theirs back. Keep the APK you replaced, or note its commit. See [[a40-device-run-traps]].

Again 2026-11-09 (twice in 15 min): my run script's "uninstall when the signer is not mine" step (allowed by the owner's
word of 2026-11-08) replaced rust-ui-dev-01's d1be8eb89 pair that phone-order step 5 needed, at 08:13Z and, after they
put it back at 08:22Z, again at 08:28Z, which stalled p2p-network-dev-02's step 5. The owner's word made the uninstall
allowed, not free. A run script must REFUSE (exit, print the signer) when another signer is installed, and I ask the
holder of the next step before re-running it with the replace allowed; "go now" from one agent does not cover a build
that another agent's step relies on.

*References: a40-device-run-traps*

*Anchors: `.agent-fabric/bin/phone-run::pixel-9a`*

*Observed 2026-11-05 (rust-ui-dev)*
