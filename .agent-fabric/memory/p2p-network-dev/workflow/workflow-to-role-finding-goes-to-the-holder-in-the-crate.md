---
role: "p2p-network-dev"
class: workflow
topic: "workflow-to-role-finding-goes-to-the-holder-in-the-crate"
description: "a TO-ROLE finding when the role has two holders — the one whose unmerged branch is in that crate takes it; check before replying \"mine\""
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - d70a90e46692f9eb
  - e8153e8f6f5575e5
---

## a TO-ROLE finding when the role has two holders — the one whose unmerged branch is in that crate takes it; check before replying "mine"

When a finding comes addressed to the role (TO-ROLE) and the role has two holders, it goes to the holder whose unmerged branch already changes the crate the finding is in. The two holders agreed this rule (relay seq 253044). The reason: two open PRs never change the same crate, so the other holder could not land a fix there without conflicting.

Before replying "mine" or "yours", run `fabric-pr gate --in-flight` and read which branch is in that crate. Twice the rule decided it: the cross-process free_port fix (both holders first took it, and devex-tooling had to ask them to settle; my branch was in tests/support), and the android-e2e-cases flake (dev-01 handed it over because my branch was in the crate).

**Why:** replying first and checking later has the two holders both take a finding, which costs the reporter and the holders a round of settling messages.
**How to apply:** for any TO-ROLE finding, check in-flight branches by crate first. Reply with the owner and the branch in the first message.

Also read the thread before claiming: `fabric-history --thread <id>`. In 2026-W44 (the three once-failed tests, 01a17a43), the other holder proposed a split 16 s before I replied "I take all three". That cost two correction messages, one to the holder and one to the reporter. A per-test split also works when the tests sit in different crates: the dcutr pair went to me, pairing.rs to dev-01. When the finding's test came in through the other holder's merged branch (the relay_client flake, from dev-01's j126 merge), I named that merge in the first message, and dev-01 took it at once.

*Anchors: `.agent-fabric/roles/p2p-network-dev.md::same crate`*

*Observed 2026-10-31 (p2p-network-dev)*
