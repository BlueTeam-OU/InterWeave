---
role: "rust-services-dev"
class: solution
topic: "print-session-can-miss-registration"
description: "A -p --channels session can start with no channel: the handler registers only at a turn's wiring; channel_enable avoids it; init status is no sign"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 79f2ad49085c07ba
---

## A -p --channels session can start with no channel: the handler registers only at a turn's wiring; channel_enable avoids it; init status is no sign

SPIKE-001 S37 (InterWeave #686, Claude Code 2.1.322) found 4 of 30 `-p --channels` print sessions with no channel. S38
(j78, 2026-11-16, 2.1.322 and 2.1.325) established the cause. The runs are under `spikes/spike-001/runs/s38-*`, and the
read is in `runs/s38-read/`.

- **Cause:** the print runner registers a channel handler only in `wireTurn()`, at each turn's start, for the clients
  connected at that moment. The reconnect watcher, the only other caller, skips `stdio` servers. So a bridge that
  connects after turn 1's wiring has no handler. Its event is dropped and cannot start a turn. The first turn does not
  wait for a pending plugin server.
- **Proof:** in s38_late.py's gated-stub cells, a late connection lost event 1 in 34 of 34 runs. The next user turn
  re-registered it, and event 2 was delivered, 34 of 34.
- **channel_enable avoids it:** 10 of 10. It answers `not connected` first, so retry; `success` registers at once.
- **Rate on the release bridge:** 2.1.322 gave 2 of 30 here, plus S37's 4 of 30. 2.1.325 gave 1 of 30. It is a race
  that depends on host load. Connect time alone does not predict it (registered runs reached 4,271 ms).
- **Init's mcp_servers status is NOT a sign.** All three real losses showed `connected`, because init is written after
  the wiring.

**How to apply:** when a spike run gets no reply, grep its evidence for `Channel notifications` before blaming the
model. A program driving a print session should send `channel_enable`. Related:
[[print-session-registers-at-reconnect-site]], [[settle-waits-out-a-running-turn]].

*References: print-session-registers-at-reconnect-site, settle-waits-out-a-running-turn*

*Anchors: `interweave:spikes/spike-001/README.md::130.`, `interweave:spikes/spike-001/README.md::## S38`, `interweave:spikes/spike-001/s38_late.py`*

*Observed 2026-11-16 (rust-services-dev)*
