---
role: "p2p-network-dev"
class: workflow
topic: "fabric-agent-repo-is-the-secrets-store"
description: "Never clone or push my fabric-agent-<id> repo by hand; it is the secrets store, written only by fabric-secrets (settings-put for persona files)"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8577e18df55db117
---

## Never clone or push my fabric-agent-<id> repo by hand; it is the secrets store, written only by fabric-secrets (settings-put for persona files)

BlueTeam-OU/fabric-agent-<id> is this login's secrets store (ADR-038/042): written only through `fabric-secrets store …`, which signs every commit and verifies on pull. A hand clone or push is wrong even when a message names the repo as the place to commit.

**Why:** 2026-10-13 I pushed the avatar commit 5e8592a by hand on brand-comms' instruction; fabric-coordinator's broadcast 01a135c5-2925 then said hand commits are refused and may fail the store's verification. Reported to python-dev-02 (seq 149426). Effect: the commit was signed by the shared key, which is not a writer of the store, so every pull was refused and `fabric-secrets sync` reported REFUSED. fabric-coordinator rolled the remote back on the owner's word (seq 151503).

**How to apply:** for persona files, keep them in ~/review-targets/persona-settings (profile.json, avatar.png, avatar-rejected/) and run `fabric-secrets store settings-put --from <dir>` (done 2026-10-13 after #236 merged: store commit 8a4cfc8f, signed, j71 done). An empty avatar-rejected/ is not committed; only avatar.png and profile.json land.

*Observed 2026-10-13 (p2p-network-dev)*
