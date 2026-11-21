---
role: "rust-services-dev"
class: workflow
topic: "inbox-watch-only-as-tracked-background"
description: "arm gzcoord-inbox only through the harness's run_in_background; a shell '&' leaves an untracked watch that cannot be killed or replaced and swallows deliveries; --help is not a help flag"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 48c508e666e80025
---

## arm gzcoord-inbox only through the harness's run_in_background; a shell '&' leaves an untracked watch that cannot be killed or replaced and swallows deliveries; --help is not a help flag

On 2026-11-12, during j69's idle tail, `gzcoord-inbox --until-delivery >/dev/null 2>&1 & true` inside a Bash call left a detached watch holding the address cursor.

- The fabric's hook refuses any kill of an inbox watch, `kill -0` included.
- `--replace` refused it as "not provably a watch of this session".
- So the session had no visible inbox until the stray reached its 115-minute deadline. Any delivery in that window would have gone to /dev/null.
- To wait for it, poll `[ -d /proc/<pid> ]` in a background loop.

Separately, `gzcoord-inbox --help` is not a help flag: on 2026-11-13 it ran a drain and delivered (consumed) a waiting REQUEST.

**Why:** the cursor is per address. A watch nobody reads eats messages, and none of the safe tools can remove it.

**How to apply:** start the watch only as Bash `run_in_background: true` with the bare command `gzcoord-inbox --until-delivery`, and read its output file on the notification. Never run `gzcoord-inbox` with an unknown flag to learn its usage; read its source or SPEC instead.

*Observed 2026-11-13 (rust-services-dev)*
