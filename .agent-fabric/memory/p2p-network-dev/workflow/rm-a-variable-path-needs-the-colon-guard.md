---
role: "p2p-network-dev"
class: workflow
topic: "rm-a-variable-path-needs-the-colon-guard"
description: "rm -f $S/*.md is refused by the harness safety check with no prompt; write \"${S:?}\"/*.md"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4e9a21aff5ed1b79
---

## rm -f $S/*.md is refused by the harness safety check with no prompt; write "${S:?}"/*.md

The harness refuses `rm` of a target built from a shell variable (`rm -f $S/*.md`) as possibly `rm -rf /`. In a session that cannot prompt, this fails the whole compound command. Write `"${S:?}"/*.md` or use a literal absolute path. Seen in j111 on 2026-10-23, cleaning scratchpad notices after a real-notices comparison.

**Why:** the check fires before anything runs, so every command chained with `&&` before the `rm` is lost too.
**How to apply:** guard every variable-rooted removal with `:?`, also inside scripts.

*Observed 2026-10-22 (p2p-network-dev)*
