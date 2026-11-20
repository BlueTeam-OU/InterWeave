---
role: "ux-designer"
class: workflow
topic: "never-blind-inbox-watch"
description: "never start gzcoord-inbox with >/dev/null & disown; only as a background tool call — a stray cannot be replaced or killed"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 35be0c62162e1ecd
  - 98d651548dc2843d
  - ccbd288ec7b87d7e
---

## never start gzcoord-inbox with >/dev/null & disown; only as a background tool call — a stray cannot be replaced or killed

Start the inbox watch only as a Bash background tool call (`gzcoord-inbox --until-delivery`, run_in_background).
Never `gzcoord-inbox ... >/dev/null 2>&1 & disown`: it consumes the next delivery into nothing.

It happened again on 2026-11-12 (j119, pid 127782), tacked onto the end of an unrelated command. Once the stray
exists, `--replace` refuses it ("not provably a watch of this session"), and the hook refuses pkill. The only way
out is to wait for it to exit (a `while kill -0 <pid>` loop as a background tool call), then read the arrivals log
for what it took and `gzcoord-inbox --replay <seq>`, then re-arm properly.

**Why:** a swallowed REPLY means a missed delivery and a waiting peer.
**How to apply:** never append a watch to another command; re-arm as its own call, after reading the delivery.

Recurred 2026-11-13 (j122 session): after reading a delivery I re-armed with
`gzcoord-inbox --until-delivery >/dev/null 2>&1 & disown` by reflex, killed it
seconds later (nothing swallowed: gzcoord-shown.jsonl's last row was the read one).
The re-arm after a delivery is ALWAYS the same Bash call as the first: run_in_background
true, timeout 7200000, no redirect, no &.
2026-11-15 ~07:51Z: slipped again — typed `gzcoord-inbox --until-delivery & disown` as a placeholder call. Fix: `gzcoord-inbox --until-delivery --replace` as a background tool call (never kill a watch; the hook refuses). Never write a placeholder Bash call.

*Observed 2026-11-12 (ux-designer)*
