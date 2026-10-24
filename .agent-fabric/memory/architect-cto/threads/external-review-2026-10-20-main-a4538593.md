---
role: "architect-cto"
class: threads
topic: "external-review-2026-10-20-main-a4538593"
description: "The owner's second external stage review (InterWeave main a4538593, 2026-10-20): what was already landed, what is on #313, the two real findings routed (DNS unavailable rule, bridge line ceiling), the README fix mine"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ecaae1ab30a8114f
---

## The owner's second external stage review (InterWeave main a4538593, 2026-10-20): what was already landed, what is on #313, the two real findings routed (DNS unavailable rule, bridge line ceiling), the README fix mine

Triage against origin/main 5121d46d (2026-10-20 evening):
- P2-1 rollback ignores overlays: landed in #318 (merged 5121d46d) — the profile-format marker, ADR-0028 A 2026-10-20; limits stand (pre-marker builds unprotected, no packaged build tested).
- P2-2 Android READY on UserNotAuthenticated, P3-3 Recents screenshot control: true on main, fixed on rust-ui-dev-01's #313 (d350cb6a, 0598c912..04605e70), and the device run the review demands is already on it (53c34f7a, device-runs/2026-10-20-gate-b.md: cold launch prompts before bind, cancel, Unlock, grant, return, process death). The PR body's "nothing on the device yet" predates that commit.
- P3-1 DNS Unknown keeps the old resolver: REAL and my rule (§14 A 2026-10-14). The aggregator yields the same empty facts for no-default, lost-default and default-with-addresses-and-no-DNS; runtime/mod.rs:875 returns before installing. DECISION to p2p-network-dev-02: unobserved (keep) vs unavailable (empty resolver + ResolverUnavailable); §14 text is my supply onto their branch.
- P3-2 bridge reads an unbounded stdin line before JSON parse (serve.rs:286/298, mcp.rs:83; payload cap only after, tools.rs:307): REAL; OBSERVATION to rust-services-dev-01.
- README lines 27 ("Stages 18 and 19 follow") and 36 ("does not host a transport yet"): mine, fixed on a fresh branch.

**Why:** a third pasting of a review should start from this, not re-triage.
**How to apply:** check the two routed items' replies (jobs), then `git merge-base --is-ancestor` for the fixes. Related: [[external-review-2026-10-04-already-landed]].
- README fix: c930aca0 on BlueTeam-OU/InterWeave#326 (1 work, unarmed; the Stage 17 closing record joins that branch). Jobs: j204 (DNS rule supply), j205 (bridge ceiling reply).

- 2026-10-21: P3-2 fixed on rust-services-dev-01 #330 (1 work) as host_input.rs MAX_HOST_LINE_BYTES = 1 MiB; the record is plugin/SECURITY.md "Host input", supplied 70ac7bd1 on for/rust-services-dev-01/j55-host-line-ceiling-record (REPLY 01a15b6a). Ruling: a bound on the MCP host stdin lives in plugin/SECURITY.md, not LOCAL-IPC (daemon socket) nor CHANNEL-EVENT (event shape). The word on #330 is asked on its finished head.

**Update 2026-10-21 19:48Z:** P3-2 (the bridge's host-line ceiling)
LANDED in #330 (2a415b91, merged 2026-10-20 23:43Z): `HostLines` in
apps/claude-channel/src/host_input.rs, MAX_HOST_LINE_BYTES = 1 MiB on raw
bytes before UTF-8/JSON, over-long line drained and unanswered, const
assert ties it to 6*MAX_PAYLOAD_BYTES + 64 KiB; plugin/SECURITY.md "Host
input". My REQUEST 01a15ff5-d126 of 2026-10-21 asked for it again because
I grepped main for `MAX_LINE|line_ceiling|max_line` and read the empty
result as absence. Lesson: before re-asserting an earlier finding, read
the file at the cited line on origin/main (serve.rs:299 now calls
HostLines), not a name grep; and check PRs merged since the finding.

*References: external-review-2026-10-04-already-landed*

*Observed 2026-10-21 (architect-cto)*
