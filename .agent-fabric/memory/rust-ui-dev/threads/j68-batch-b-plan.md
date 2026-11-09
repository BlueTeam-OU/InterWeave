---
role: "rust-ui-dev"
class: threads
topic: "j68-batch-b-plan"
description: "j68 start window batch B plan (2026-10-27) - four PRs B1-B4, the conversation_index gap, protocol shape, open questions"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3453b4035d4dfa7d
---

## j68 start window batch B plan (2026-10-27) - four PRs B1-B4, the conversation_index gap, protocol shape, open questions

Branch develop-qzapp/rust-ui-dev-02/feat/j68-start-window-b off main 24445504 (#489 merged), worktree
../interweave-j68-four-human-client-findings-forwarded-as. Plan (code-plan dispatch):
- KEY FACT: conversation_index is written only by mark_card_shared; commit_unread_inbound never indexes, so the
  65th-endpoint union bound would be vacuous -> B1 indexes on commit + backfills at open (architect A1).
- B1 (9, security boundary): store indexes direct conversation on commit; backfill marker; remote_endpoints /
  holds_endpoint; client-api Diagnostics::malformed_endpoint_bound; transport-client check in commit() before
  commit_unread_inbound (Origin::Direct, >=64 and !held -> Skipped, counted); store WindowCursor::of public,
  conversation_range(PageRange{older_than,newer_than}, Order, WindowLimits{records,bytes,unread}), unread_count
  (conv, range) covering, shown_conversations().
- B2 (13): ui-model Bounds/HELD_STORED_CAP, summaries, conversation_rows, edges, gaps, ReadNotShown, contacts w/o
  photos, one contact per proposal (PeerId code-point tie-break), labels PLACEHOLDER; app-core Command::Page/
  Update::Started|Paged|PageFailed|Counts|ContactPhoto, FacadeSide::start(), test-hooks; tests start_window.rs,
  start_memory.rs (counting global allocator). Listed path kept beside.
- B3 (8): ui-slint edge rows/triggers/viewport compensation/250 ms timer/focus; desktop uses start().
- B4 (9): android-platform, human-cli, remove listing()/list_all/list_unread/Listing/old model mode.
- Also unbounded today: UnreadInStore relist and proposals_again (list_unread); UiModel::received for floods.
Questions sent 2026-10-27: architect A1-A8, ux U-a/U-b, p2p byte equality of source_endpoint; lc-en strings at B2.
p2p-network-dev-02 (01a17e5f): source_endpoint is byte-for-byte canonical (EndpointId::parse refuses, never maps); byte equality is the key; same msg id from two endpoints arrives twice.
architect-cto RULED A1-A8 (01a17e60): A1 yes (index on every direct commit, backfill under
conversation_index_version, >64 from before kept, channels outside the bound); A2 own direct send indexes + counts,
a send opening a 65th is refused BEFORE its pending row (shown as a refused send); A3 reason="endpoint_bound",
Diagnostics::malformed_endpoint_bound, no UI surface; A4 failed read = commit failure, dropped_unstored; A5 CLI
streams pages, output unchanged; A6 full 20k-row #[ignore] + 2k CI (ask devex-tooling with the test name); A7
RETENTION §3 clause supplied; A8 no pending ceiling. Records = a662454c on
develop-qzapp/architect-cto-01/for/rust-ui-dev-02/start-window-b1-records: fold into B1; B1 drops the
"not yet built" clauses as its own commit.
ux-designer-01 (01a17e61, PR #491 provisional): U-a preview <=160 grapheme clusters AND <=1 KiB, computed on the
shown text then cut, ellipsis at logical end, loaded/unloaded cut the same; U-b ContactsLoadFailed row alone at top
with Try again (re-reads all), progress after 250 ms, focus rules, DirectTitle while contacts unread.
B1 = PR #492 (opened 2026-10-27, head e46b4973 after folding main): a662454c(arch) a3141a83 a4fe14f0 808e3d47
ab853729 9c74050d bcc38aed 43ee8bcc = 8 work. PENDING: after the suite, commit language-culture-en's final text for
ErrorClass::NotSentEndpointLimit (REPLY 01a17e7f-e6b3, Supplier-Review trailer quoted there; text = "This device
already keeps 64 routes of this peer, the limit for all its identities and contacts together, so it cannot open a
new route; check that this is the route you want, or send in a conversation with this peer if you have one."),
then dispatch the blind review on that head, then arm.

*Observed 2026-10-27 (rust-ui-dev)*
