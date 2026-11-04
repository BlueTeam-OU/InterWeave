---
role: "rust-ui-dev"
class: threads
topic: "j68-p2-1-plan"
description: "j68 P2-1 start window plan (2026-10-27) - batches A/B/C, schema v10 contents, protocol shape, open questions per role"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 1e44b1685d648b54
---

## j68 P2-1 start window plan (2026-10-27) - batches A/B/C, schema v10 contents, protocol shape, open questions per role

Branch develop-qzapp/rust-ui-dev-02/feat/j68-start-window (worktree ../interweave-j68-four-human-client-findings-forwarded-as),
off main 3b6db4e7. Design: design/2026-10-27-start-window.md (binding). Plan from a code-plan dispatch:

- Batch A (first PR, ~9 work): store schema v10 = contacts.name_key column (filled in-migration by a Rust
  loop) + settings row contacts.name_key_version (recompute on mismatch); indexes contacts_by_name_key,
  contact_routes_by_route(peer_id,endpoint_id), contact_routes_by_key(identity_key),
  unread/kept_inbound_by_route(source_peer,source_endpoint_key,received_at,row_id), *_by_channel; verify_shape;
  every contacts writer sets name_key. card_standing as ~5 indexed statements returning ContactName (no photos),
  added beside the old, app-core moved, old removed. conversation_page(StoreConversation, WindowCursor{received_at,
  table,row_id}), unread_inbound_row, contact_directory_page + contact_photos(<=4). EXPLAIN QUERY PLAN tests;
  query-count test via rusqlite trace (K=50,C=500, <=5K+C).
- Batch B (lands whole, ~16): FacadeSide::start() emits PendingListed / Summaries(<=128) / Hidden pages;
  OpenConversation/LoadEarlier/CloseConversation commands, Window/Earlier/EarlierFailed/SummaryChanged updates;
  model windows (MAX_WINDOWS 2, STORED_HELD_CAP 1024), edge state machine, evict skips focused row, labels;
  CLI, desktop, android-platform consumers; Slint edge rows + focus table; remove Listing/list_all last.
- Batch C: contact directory paging, photo cache 64, SlowDocuments androidTest provider + photo e2e cases.
- STATE.md row is architect-cto's (for/ branch), NOT mine. Questions sent 2026-10-27: architect-cto A1 (decode
  walk vs kind column), A2 (persist name_key), A3 (badge counts rows or items), A4 (pending ceiling);
  ux-designer U1-U4 (STORED_HELD_CAP behaviour, photo placeholder, cap same-name lists, limit row early by <=64);
  p2p-network-dev (UnreadInStore origins; 65th-endpoint refusal in transport-client); language-culture keys in Batch B.

Answers so far (2026-10-27): ux-designer-01 took U1-U4 as its j77 (answer in a design revision after #480;
build none on a guess). p2p-network-dev-02 (01a17dcb): both transport-client items are MINE --
(1) UnreadInStore may carry origins: union across coalesced events (queue.rs:38), bounded, an "and more" =
re-list-all fallback, origins of refused rows only (take_the_rest client.rs:1266); (2) 65th source endpoint
is unbuilt and unwritten: check in transport-client's commit path before the store, counted on
human_inbound_malformed_total{reason}; rule text is architect-cto's (lifetime of "distinct" asked, A5).

architect-cto RULED (01a17dcc, 2026-10-27): A2 yes (name_key + name_key_version settings row, recompute at open
before any standing; RETENTION §5 admits it); A1 = `kind` column (text|card|other) on unread_inbound and
kept_inbound, written at commit by the listing's classification, versioned by inbound_kind_version and
reclassified at open; badge = one indexed count, hidden = the rest, NO start-up walk; A3 badge counts distinct
(conversation_id, app_message_id) among decodable unread rows (an unread copy of a kept item counts once);
A4 no pending ceiling. STATE.md v10 + HUMAN-CHAT 65th-endpoint rule = 92f56bed, folded (ff) into my branch.
So schema v10 = name_key, kind x2, two version settings rows, indexes. A5 RULED (01a17dcd, ef435647 folded): distinct = conversation_index rows per peer; 65th refused in transport-client commit path, one fixed reason.

PROGRESS on feat/j68-start-window (pushed): folded architect-cto 92f56bed, ef435647, 86708be2, ad1d2f78 (ff);
0ca9c0da store schema v10 + name_key writers + refresh_name_keys at open (NAME_KEY_VERSION "1", pin test);
bd4dd3a7 InboundKind required on NewInbound, kept through Keep, reclassify_inbound(version, classify).
Notes: an ALTER ADD COLUMN ... CHECK failed under SQLCipher here (dropped the CHECK). Tests faking older
schemas use common::UNDO_V10. NEXT: app-core reclassify at open (listing classifier + version const);
card_standing indexed (ContactName, no photos) beside old -> app-core -> remove old; conversation_page,
unread_inbound_row, contact_directory_page; EXPLAIN + query-count tests.

ux-designer-01 ANSWERED U1-U4 (01a17df6) in PR #488 (design note revision, head 14c652c3, binding once merged):
U1 at STORED_HELD_CAP release held stored rows silently (closed conversations first, then the open one's farthest
from the person's place; never pending, newest, focused, in-view, Undo rows), "gap" reload from either side (§5.1);
U2 placeholder photos OK in Contacts/Chats/app bar, proposals show no contact photo, contact page waits (tell if
>100 ms); U3 a proposal names ONE contact (contact-cards §5.4.1 rule + §6.2 code-point tie-break); U4 page shortened
to room left, limit row only at the bound (§7.1). Pending rows are OUTSIDE the ceiling's count (confirm in batch B).

BATCH A = PR #489 (opened 2026-10-27, head 17cf88c3 after folding main 17cf88c3): + 035b2690 window reads
(conversation_page/WindowCursor, unread_counts A3, hidden_inbound, unread_inbound_row), 66cd3959 contact directory
(ContactListing, contact_photo), eddacd37 upgrade-tree classification test, ba20682b README; architect 20afc55a
folded by merge 7bba3aa9 (65th-endpoint bound = conversation_index UNION contact_routes per PeerId). 65th check is
batch B's. Review + suite dispatched. Old review worktrees still unswept.
#489 round 1 findings F1-F4 answered (71337686, 01a8df60) + architect 8abb329a, 6485eba1, e308cf65 folded (4f6387d9); re-review + suite running. Covering index must use STORED columns (virtual generated _key cols never cover).

#489 MERGED 2026-10-27 17:00Z (24445504), head 9205d553 (19 work, 3 fix; 3 review rounds, F1-F4 fixed; architect 8abb329a 6485eba1
e308cf65 00516e52 c45bbbbb folded). Arm warned "over 16: smaller batches next time". NEXT after merge: batch B on a
NEW branch (window + 65th-endpoint check in transport-client commit path counting conversation_index UNION
contact_routes per PeerId, fixed reason; drop the "not yet built" parenthesis in HUMAN-CHAT/STATE via architect);
wait for ux #488 to merge first (U1-U4 binding). Keep batch B under 16 work commits.

*Observed 2026-10-27 (rust-ui-dev)*
