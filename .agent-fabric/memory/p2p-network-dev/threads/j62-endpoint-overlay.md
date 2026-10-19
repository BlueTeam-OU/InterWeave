---
role: "p2p-network-dev"
class: threads
topic: "j62-endpoint-overlay"
description: "Plan §20 gate (e): endpoint overlay persisted per architect-cto DECISION seq 109465 (A); branch feat/endpoint-overlay; what was built, limits, what the PR must say"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-18"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 80c69e3c22497342
---

## Plan §20 gate (e): endpoint overlay persisted per architect-cto DECISION seq 109465 (A); branch feat/endpoint-overlay; what was built, limits, what the PR must say

architect-cto ruled (A) persist on 2026-10-11 (DECISION seq 109465; supply 7d6edc53 + 3dd2dda8 folded, reply seq 109568 confirmed note 2 and restated the predicate on the EFFECTIVE state).
Built on develop-qzapp/p2p-network-dev-01/feat/endpoint-overlay (off a3b76b55), one commit per package:
profile-config endpoint_overlay.rs (MAX_ENDPOINT_OVERLAY_BYTES 16 KiB, duplicate endpoint key refused by a visitor -- serde's BTreeMap is last-wins);
local-client-api EndpointAdminView.persisted; composition driver routes set_enabled/set_default (write-before-publish, restore, ahead), OverlayFault::Clean added;
IPC 2.5 (ENDPOINT_PERSISTED_SINCE_MINOR, IPC_MAX_MINOR 5, endpoint-list 1.1.0), server refuses unpersisted at 2.5, client refuses the other minor's shape both ways;
embedded no-file test; fake keeps endpoints across restart; conformance + ipc-v2 runners supply the endpoint store.
Limit found (put in PR body): embedded translate re-validates the COMPOSED profile, so an overlay disabling runtime.android.endpoint refuses the embedded start as ProfileInvalid (fail-closed; nothing writes the file on Android). Daemon validation only reads the default, which normalisation keeps enabled.
Not built (architect: carried by name in the closing record): a per-row `source` on admin.endpoints.list.
Lesson: a Request routed through the composition driver needs every test runner that serves IPC to carry the store, or 2.5 rows are refused Internal.

#261 opened 2026-10-11. Review F1 (P2): composed profile can fail enabled-dependent validation (max_advertised, android lease endpoint) -> set-time validate + EndpointOverlayConflicts at start (0937006e). Re-review N1 (P2) open: ADR-0028 'no analogue of MAX_ALLOWED_PEERS' sentence + failure-model.md:84 are architect-cto's, asked REQUEST 01a12997-cb1d (also (a) fatal vs (b) normalise enables). Arm only after their commit lands + re-review of the new range.
LANDED #261 as e7397ea8 (2026-10-11), 20 work 8 fix, armed at 20 on the standing word. Closing record (architect-cto's) carries: embedded ProfileInvalid limit, no per-row source. rust-ui-dev builds the desktop endpoint choice from admin.endpoints.list after this. threat-model, ADR, plan brackets all folded.

*Observed 2026-10-11 (p2p-network-dev)*
