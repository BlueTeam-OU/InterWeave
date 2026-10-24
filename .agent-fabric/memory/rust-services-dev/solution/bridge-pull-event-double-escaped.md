---
role: "rust-services-dev"
class: solution
topic: "bridge-pull-event-double-escaped"
description: "claude-channel pull mode escapes an event twice on the host line; any size bound on an event must measure both modes"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - d0a3fb5ac346cac5
---

## claude-channel pull mode escapes an event twice on the host line; any size bound on an event must measure both modes

In claude-channel's pull mode, `receive` serialises `{events:[{kind,content,meta}],…}` to a string and returns it as the TEXT of a tool result (`mcp::tool_result_line`), so every event is JSON-escaped twice on the host line: a control character costs 6 bytes pushed, 7 pulled; a quote 2 pushed, 4 pulled in meta. Since CHANNEL-EVENT.md requires a pulled event to carry exactly the push's content and meta, a size rule has to be one measure for both modes — the larger of the push line and the twice-escaped pull event. `channel-core`'s `host_event_bytes` (PR BlueTeam-OU/InterWeave#376, 2026-10-22) is that measure, and `serve.rs`'s `the_measure_is_the_lines_own_length` keeps it equal to what the server writes. The reply token must be in meta before measuring and minted after (review finding F1/F2 on #376).

*Anchors: `crates/claude/channel-core/src/host_event.rs::host_event_bytes`, `apps/claude-channel/src/mcp.rs::tool_result_line`*

*Observed 2026-10-22 (rust-services-dev)*
