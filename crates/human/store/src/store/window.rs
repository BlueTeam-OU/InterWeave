// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The start window's reads (`design/2026-10-27-start-window.md`; STATE.md
//! schema v10): one conversation's shown rows a page at a time, newest
//! first, and the counts the Chats list needs, so that opening the app
//! reads counts and pages, never every row. Each is served by a v10 index:
//! `the_window_reads_use_their_indexes` reads the pages' plans and
//! `the_counts_read_a_covering_index_alone` the counts'.

use rusqlite::params;

use interweave_transport_api::{ChannelId, EndpointId, TransportIdentity};

use super::{HumanStore, InboundRow, sql_timestamp};
use crate::StoreError;
use crate::records::{BackupTable, Page, PageLimits, RowId, StoredInbound};

/// A conversation as the store's rows name it: a direct one by the peer and
/// the endpoint it asserted, a channel by its id alone, whoever published.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StoreConversation {
    /// A peer's endpoint.
    Direct {
        /// The peer.
        peer: TransportIdentity,
        /// The endpoint it sent from.
        endpoint: EndpointId,
    },
    /// A broadcast channel.
    Channel(ChannelId),
}

/// Where a conversation's page ended: the next page is the rows older than
/// this one. The table is named because unread and kept rows have row ids
/// of their own (as a [`crate::BackupCursor`] names it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowCursor {
    received_at: u64,
    table: BackupTable,
    row_id: RowId,
}

/// The order a page is read in: newest first from a point downward, or
/// oldest first from a point upward (a gap read from above, design §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// Newest first.
    NewestFirst,
    /// Oldest first.
    OldestFirst,
}

/// The rows a page may come from: older than one cursor, newer than
/// another, either unbounded at `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PageRange {
    /// Rows older than this one.
    pub older_than: Option<WindowCursor>,
    /// Rows newer than this one.
    pub newer_than: Option<WindowCursor>,
}

/// What one page of a conversation may hold: its records and bytes, and
/// at most `unread` unread rows when it is set -- a page shortened to the
/// room the session has left (design §7.1). Kept rows are not counted
/// there: reading them makes nothing session-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowLimits {
    /// Records and bytes.
    pub page: PageLimits,
    /// Unread rows, when bounded.
    pub unread: Option<usize>,
}

impl WindowCursor {
    /// The place of `row`, a row a page gave: kept when it has a keep time.
    #[must_use]
    pub const fn of(row: &StoredInbound) -> Self {
        Self {
            received_at: row.received_at,
            table: if row.kept_at.is_some() {
                BackupTable::Kept
            } else {
                BackupTable::Unread
            },
            row_id: row.row_id,
        }
    }
}

/// A cursor as a bound in `table`'s own terms, `(received_at, row_id)`
/// compared strictly: a row of a table ranked below the cursor's is on the
/// cursor's side at its instant too, one ranked above only past it -- the
/// same whichever side the bound is on.
fn bound(table: BackupTable, cursor: WindowCursor) -> Result<(i64, i64), StoreError> {
    let at = sql_timestamp("cursor received_at", cursor.received_at)?;
    Ok(match rank(table).cmp(&rank(cursor.table)) {
        std::cmp::Ordering::Equal => (at, cursor.row_id.get()),
        std::cmp::Ordering::Less => (at, i64::MAX),
        std::cmp::Ordering::Greater => (at, 0),
    })
}

/// A conversation's unread count as the badge counts it (STATE.md schema
/// v10, A 2026-10-27 (ii)): its unread rows of a kind this build shows,
/// one per row. The table's key already makes two rows of one direct
/// conversation two messages, and two publishers reusing one id on a
/// channel are two items, so no distinct count is taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationUnread {
    /// The conversation.
    pub conversation: StoreConversation,
    /// Its unread messages and cards.
    pub unread: u64,
}

/// Rows of a kind this build cannot show, counted apart: never an item,
/// never in a badge, and the hidden-rows line's count with the pending
/// rows that do not decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HiddenInbound {
    /// Unread.
    pub unread: u64,
    /// Kept.
    pub kept: u64,
}

/// Shown rows of one table, by conversation, between two bounds, in
/// `order`. `{table}`, `{read}` and `{direction}` are literals of this
/// module; `{conversation}` is one of the two filters below.
fn page_sql(table: &str, conversation: &str, order: Order) -> String {
    let direction = match order {
        Order::NewestFirst => "DESC",
        Order::OldestFirst => "ASC",
    };
    let read = if table == "kept_inbound" {
        "read_at, kept_at"
    } else {
        "NULL, NULL"
    };
    format!(
        "SELECT row_id, app_message_id, source_peer, source_endpoint, channel_id,
                media_type, payload, received_at, {read}, kind
           FROM {table}
          WHERE {conversation} AND kind IN ('text', 'card')
            AND (received_at, row_id) < (?3, ?4)
            AND (received_at, row_id) > (?5, ?6)
          ORDER BY received_at {direction}, row_id {direction}
          LIMIT ?7"
    )
}

/// A direct conversation: the peer and its endpoint key, served by the
/// `_by_conversation` index. A broadcast row's endpoint key is empty, and a
/// direct row's never is, so no channel row matches.
const DIRECT: &str = "source_peer = ?1 AND source_endpoint_key = ?2";

/// A channel: its key alone, served by the `_by_channel` index. `?2` is
/// unused and bound empty so both filters take the same parameters.
const CHANNEL: &str = "channel_key = ?1 AND ?2 = ''";

/// The badge's count of direct conversations, and of channels, and the
/// hidden counts: each read from a `_by_kind` index alone, no row read
/// (schema v10), over the tables' own columns rather than their generated
/// `_key` forms, which SQLite recomputes from the row and so never reads
/// from an index alone, and named rather than left to the planner, which with no
/// statistics picks a narrower index and reads the rows. A kind this build
/// does not know -- a newer build's -- is
/// neither text nor card, so it is hidden rather than counted.
const DIRECT_COUNTS: &str = "SELECT source_peer, source_endpoint, COUNT(*)
   FROM unread_inbound INDEXED BY unread_inbound_by_kind
  WHERE channel_id IS NULL AND source_endpoint IS NOT NULL AND kind IN ('text', 'card')
  GROUP BY source_peer, source_endpoint";
const CHANNEL_COUNTS: &str = "SELECT channel_id, COUNT(*)
   FROM unread_inbound INDEXED BY unread_inbound_by_kind
  WHERE channel_id IS NOT NULL AND kind IN ('text', 'card')
  GROUP BY channel_id";
const HIDDEN: &str =
    "SELECT (SELECT COUNT(*) FROM unread_inbound INDEXED BY unread_inbound_by_kind WHERE kind NOT IN ('text', 'card')),
            (SELECT COUNT(*) FROM kept_inbound INDEXED BY kept_inbound_by_kind WHERE kind NOT IN ('text', 'card'))";

/// The conversations with a shown row: a direct one by its peer and
/// endpoint, a channel by its id, from each inbound table's kind index.
const SHOWN_CONVERSATIONS: &str = "SELECT DISTINCT
        CASE WHEN channel_id IS NULL THEN source_peer END,
        CASE WHEN channel_id IS NULL THEN source_endpoint END,
        channel_id
   FROM unread_inbound INDEXED BY unread_inbound_by_kind
  WHERE kind IN ('text', 'card')
 UNION
 SELECT DISTINCT
        CASE WHEN channel_id IS NULL THEN source_peer END,
        CASE WHEN channel_id IS NULL THEN source_endpoint END,
        channel_id
   FROM kept_inbound INDEXED BY kept_inbound_by_kind
  WHERE kind IN ('text', 'card')";

/// A peer's distinct endpoints: its conversations by a range on the
/// index's key, its contacts' routes by the route index, never a scan of
/// either (`the_endpoint_count_reads_its_indexes`).
const ENDPOINT_COUNT: &str = "SELECT COUNT(*) FROM (
     SELECT endpoint_id FROM conversation_index
      WHERE conversation_id > ?1 AND conversation_id < ?2
     UNION
     SELECT endpoint_id FROM contact_routes WHERE peer_id = ?3)";

/// The bounds of `peer`'s direct conversation ids, `direct/<peer>/...`,
/// for a range read on the index's key: `/` sorts just before `0`, so
/// every id of this peer, and no other's, is between them.
fn conversation_id_range(peer: &TransportIdentity) -> (String, String) {
    (
        format!("direct/{}/", peer.as_str()),
        format!("direct/{}0", peer.as_str()),
    )
}

/// The rank of a table among rows received at one instant: kept rows rank
/// above unread ones, so in a newest-first page a kept row comes first
/// (`a_conversation_is_paged_newest_first_with_every_row_once`), and a
/// cursor names one place.
const fn rank(table: BackupTable) -> u8 {
    match table {
        BackupTable::Unread => 0,
        BackupTable::Kept => 1,
    }
}

/// Index the direct conversation of `peer`'s `endpoint`, if it is not
/// already: written with the conversation's first row, in its
/// transaction (STATE.md, the per-peer endpoint bound).
pub(crate) fn index_direct(
    tx: &rusqlite::Connection,
    peer: &TransportIdentity,
    endpoint: &EndpointId,
) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT OR IGNORE INTO conversation_index (conversation_id, peer_id, endpoint_id)
         VALUES (?1, ?2, ?3)",
        params![
            crate::direct_conversation_id(peer, endpoint),
            peer.as_str(),
            endpoint.as_str()
        ],
    )?;
    Ok(())
}

/// The version of the index's backfill, kept in `settings` as
/// `conversation_index_version` (STATE.md): a store without it -- every
/// store before this build -- has the direct conversations of its stored
/// rows indexed at open, once.
const CONVERSATION_INDEX_VERSION: &str = "1";

/// At open: index the direct conversation of every stored pending, unread
/// and kept row, once per version, in one transaction with the version
/// row. Read from the rows' own columns, never their payloads.
///
/// # Errors
/// A storage error; the open then fails as a migration's would.
pub(crate) fn backfill_conversation_index(
    conn: &mut rusqlite::Connection,
) -> Result<(), StoreError> {
    use rusqlite::OptionalExtension as _;
    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'conversation_index_version'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if stored.as_deref() == Some(CONVERSATION_INDEX_VERSION) {
        return Ok(());
    }
    let tx = conn.transaction()?;
    // `direct/<peer>/<endpoint>`, as `direct_conversation_id` writes it.
    for (table, peer, endpoint, channel) in [
        (
            "unread_inbound",
            "source_peer",
            "source_endpoint",
            "channel_id",
        ),
        (
            "kept_inbound",
            "source_peer",
            "source_endpoint",
            "channel_id",
        ),
        (
            "pending_outbound",
            "destination_peer",
            "destination_endpoint",
            "channel_id",
        ),
    ] {
        tx.execute(
            &format!(
                "INSERT OR IGNORE INTO conversation_index (conversation_id, peer_id, endpoint_id)
                 SELECT DISTINCT 'direct/' || {peer} || '/' || {endpoint}, {peer}, {endpoint}
                   FROM {table}
                  WHERE {channel} IS NULL AND {endpoint} IS NOT NULL"
            ),
            [],
        )?;
    }
    tx.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('conversation_index_version', ?1)",
        params![CONVERSATION_INDEX_VERSION],
    )?;
    tx.commit()?;
    Ok(())
}

impl HumanStore {
    /// How many distinct endpoints of `peer` the store holds: its indexed
    /// direct conversations and the routes kept under contacts, together
    /// (HUMAN-CHAT.md's malformed rule; STATE.md). Two indexed reads.
    ///
    /// # Errors
    /// A storage error.
    pub fn remote_endpoints(&self, peer: &TransportIdentity) -> Result<u64, StoreError> {
        let (from, to) = conversation_id_range(peer);
        let count: i64 =
            self.conn
                .query_row(ENDPOINT_COUNT, params![from, to, peer.as_str()], |r| {
                    r.get(0)
                })?;
        Ok(u64::try_from(count).unwrap_or(0))
    }

    /// Whether the store already holds `endpoint` of `peer`, as an indexed
    /// conversation or a contact's route.
    ///
    /// # Errors
    /// A storage error.
    pub fn holds_endpoint(
        &self,
        peer: &TransportIdentity,
        endpoint: &EndpointId,
    ) -> Result<bool, StoreError> {
        Ok(self.conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM conversation_index WHERE conversation_id = ?1)
                 OR EXISTS (SELECT 1 FROM contact_routes WHERE peer_id = ?2 AND endpoint_id = ?3)",
            params![
                crate::direct_conversation_id(peer, endpoint),
                peer.as_str(),
                endpoint.as_str()
            ],
            |r| r.get(0),
        )?)
    }

    /// One page of `conversation`'s rows of a kind this build shows, unread
    /// and kept together, newest first, older than `before` (the newest at
    /// `None`): within `limits`, and the cursor of the next page when there
    /// are older rows. A row this build cannot show is never in a page
    /// ([`Self::hidden_inbound`] counts it).
    ///
    /// # Errors
    /// A storage error, or a row this build cannot read back.
    pub fn conversation_page(
        &self,
        conversation: &StoreConversation,
        before: Option<WindowCursor>,
        limits: PageLimits,
    ) -> Result<Page<StoredInbound, WindowCursor>, StoreError> {
        self.conversation_range(
            conversation,
            PageRange {
                older_than: before,
                newer_than: None,
            },
            Order::NewestFirst,
            WindowLimits {
                page: limits,
                unread: None,
            },
        )
    }

    /// One page of `conversation`'s shown rows within `range`, in `order`,
    /// unread and kept together: within `limits`, and the cursor of the
    /// next page in the same order when the range holds more. A page holds
    /// at least one row, unless its unread room is used up and the next
    /// row is unread: then it holds none, and the cursor says more rows
    /// remain (design §7.1).
    ///
    /// # Errors
    /// A storage error, or a row this build cannot read back.
    pub fn conversation_range(
        &self,
        conversation: &StoreConversation,
        range: PageRange,
        order: Order,
        limits: WindowLimits,
    ) -> Result<Page<StoredInbound, WindowCursor>, StoreError> {
        let (filter, first, second) = match conversation {
            StoreConversation::Direct { peer, endpoint } => {
                (DIRECT, peer.as_str(), endpoint.as_str())
            }
            StoreConversation::Channel(channel) => (CHANNEL, channel.as_str(), ""),
        };
        let fetch = i64::try_from(limits.page.max_records().saturating_add(1)).unwrap_or(i64::MAX);
        let mut merged: Vec<(BackupTable, StoredInbound, usize)> = Vec::new();
        for table in [BackupTable::Unread, BackupTable::Kept] {
            let name = match table {
                BackupTable::Unread => "unread_inbound",
                BackupTable::Kept => "kept_inbound",
            };
            let (older_at, older_row) = match range.older_than {
                None => (i64::MAX, i64::MAX),
                Some(c) => bound(table, c)?,
            };
            let (newer_at, newer_row) = match range.newer_than {
                None => (i64::MIN, i64::MIN),
                Some(c) => bound(table, c)?,
            };
            let mut stmt = self.conn.prepare(&page_sql(name, filter, order))?;
            let rows = stmt.query_map(
                params![
                    first, second, older_at, older_row, newer_at, newer_row, fetch
                ],
                InboundRow::read,
            )?;
            for row in rows {
                let row = row?;
                let bytes = row.payload.len();
                merged.push((table, row.parse()?, bytes));
            }
        }
        merged.sort_by(|(ta, a, _), (tb, b, _)| {
            let ka = (a.received_at, rank(*ta), a.row_id.get());
            let kb = (b.received_at, rank(*tb), b.row_id.get());
            match order {
                Order::NewestFirst => kb.cmp(&ka),
                Order::OldestFirst => ka.cmp(&kb),
            }
        });
        let mut items = Vec::new();
        let mut bytes = 0usize;
        let mut unread = 0usize;
        let mut last: Option<WindowCursor> = None;
        let mut more = false;
        for (table, row, size) in merged {
            let past_room =
                table == BackupTable::Unread && limits.unread.is_some_and(|room| unread >= room);
            if past_room && items.is_empty() {
                // Nothing taken: the next page starts at this row, so the
                // cursor stands just before it in the page's order -- a
                // first page has no bound to hand back, and no cursor
                // would read as a finished conversation (#492 review F1).
                last = Some(WindowCursor {
                    received_at: row.received_at,
                    table,
                    row_id: RowId::from_stored(match order {
                        Order::NewestFirst => row.row_id.get().saturating_add(1),
                        Order::OldestFirst => row.row_id.get().saturating_sub(1),
                    }),
                });
            }
            if past_room
                || (!items.is_empty()
                    && (items.len() >= limits.page.max_records()
                        || bytes.saturating_add(size) > limits.page.max_bytes()))
            {
                more = true;
                break;
            }
            bytes = bytes.saturating_add(size);
            if table == BackupTable::Unread {
                unread += 1;
            }
            last = Some(WindowCursor {
                received_at: row.received_at,
                table,
                row_id: row.row_id,
            });
            items.push(row);
        }
        Ok(Page {
            items,
            next: if more { last } else { None },
        })
    }

    /// Every conversation's unread count, as the badge counts it
    /// ([`ConversationUnread`]): one grouped count per table filter, no row
    /// decoded. A conversation with no unread row it shows is absent.
    ///
    /// # Errors
    /// A storage error, or a row this build cannot read back.
    pub fn unread_counts(&self) -> Result<Vec<ConversationUnread>, StoreError> {
        let mut out = Vec::new();
        let mut direct = self.conn.prepare(DIRECT_COUNTS)?;
        let rows = direct.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?;
        for row in rows {
            let (peer, endpoint, count) = row?;
            out.push(ConversationUnread {
                conversation: StoreConversation::Direct {
                    peer: TransportIdentity::parse(peer)
                        .map_err(|e| StoreError::Corrupt(e.to_string()))?,
                    endpoint: EndpointId::parse(endpoint)
                        .map_err(|e| StoreError::Corrupt(e.to_string()))?,
                },
                unread: u64::try_from(count).unwrap_or(0),
            });
        }
        let mut channels = self.conn.prepare(CHANNEL_COUNTS)?;
        let rows = channels.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for row in rows {
            let (channel, count) = row?;
            out.push(ConversationUnread {
                conversation: StoreConversation::Channel(
                    ChannelId::parse(channel).map_err(|e| StoreError::Corrupt(e.to_string()))?,
                ),
                unread: u64::try_from(count).unwrap_or(0),
            });
        }
        Ok(out)
    }

    /// Every conversation with a row this build shows, unread or kept: the
    /// conversations the Chats list holds from the store, a kept-only one
    /// included, which [`Self::unread_counts`] does not name. Each table's
    /// peer and channel columns are read, never a payload.
    ///
    /// # Errors
    /// A storage error, or a row this build cannot read back.
    pub fn shown_conversations(&self) -> Result<Vec<StoreConversation>, StoreError> {
        let mut out = std::collections::BTreeSet::new();
        let mut stmt = self.conn.prepare(SHOWN_CONVERSATIONS)?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })?;
        for row in rows {
            let conversation = match row? {
                (Some(peer), Some(endpoint), None) => StoreConversation::Direct {
                    peer: TransportIdentity::parse(peer)
                        .map_err(|e| StoreError::Corrupt(e.to_string()))?,
                    endpoint: EndpointId::parse(endpoint)
                        .map_err(|e| StoreError::Corrupt(e.to_string()))?,
                },
                (_, _, Some(channel)) => StoreConversation::Channel(
                    ChannelId::parse(channel).map_err(|e| StoreError::Corrupt(e.to_string()))?,
                ),
                // Neither an endpoint nor a channel: no row the client wrote.
                _ => continue,
            };
            out.insert(conversation);
        }
        Ok(out.into_iter().collect())
    }

    /// The inbound rows of a kind this build cannot show, unread and kept.
    ///
    /// # Errors
    /// A storage error.
    pub fn hidden_inbound(&self) -> Result<HiddenInbound, StoreError> {
        let (unread, kept): (i64, i64) = self
            .conn
            .query_row(HIDDEN, [], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(HiddenInbound {
            unread: u64::try_from(unread).unwrap_or(0),
            kept: u64::try_from(kept).unwrap_or(0),
        })
    }

    /// One unread row by its id, if it is still unread.
    ///
    /// # Errors
    /// A storage error, or a row this build cannot read back.
    pub fn unread_inbound_row(&self, row_id: RowId) -> Result<Option<StoredInbound>, StoreError> {
        use rusqlite::OptionalExtension as _;
        self.conn
            .query_row(
                "SELECT row_id, app_message_id, source_peer, source_endpoint, channel_id,
                        media_type, payload, received_at, NULL, NULL, kind
                   FROM unread_inbound WHERE row_id = ?1",
                params![row_id.get()],
                InboundRow::read,
            )
            .optional()?
            .map(InboundRow::parse)
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::{CHANNEL, CHANNEL_COUNTS, DIRECT, DIRECT_COUNTS, ENDPOINT_COUNT, HIDDEN, page_sql};
    use crate::{HumanStore, StoreOptions};

    /// Each window read is planned on its v10 index, never a scan of an
    /// inbound table. Mutation: the conversation index dropped from
    /// migration 10.
    #[test]
    fn the_window_reads_use_their_indexes() {
        let store = HumanStore::open_in_memory(StoreOptions::default()).expect("store");
        for (table, filter, index) in [
            ("unread_inbound", DIRECT, "unread_inbound_by_conversation"),
            ("kept_inbound", DIRECT, "kept_inbound_by_conversation"),
            ("unread_inbound", CHANNEL, "unread_inbound_by_channel"),
            ("kept_inbound", CHANNEL, "kept_inbound_by_channel"),
        ] {
            let plan: Vec<String> = store
                .conn
                .prepare(&format!(
                    "EXPLAIN QUERY PLAN {}",
                    page_sql(table, filter, super::Order::NewestFirst)
                ))
                .expect("plan")
                .query_map(rusqlite::params!["p", "e", 1, 1, 0, 0, 1], |r| {
                    r.get::<_, String>(3)
                })
                .expect("rows")
                .collect::<Result<_, _>>()
                .expect("plan rows");
            assert!(
                plan.iter().any(|step| step.contains(index)),
                "{table} {filter}: {index} not used: {plan:?}"
            );
        }
    }

    /// The badge's and the hidden counts read an index alone, never a row:
    /// each plan names a covering `_by_kind` index (#489 review F1).
    /// Mutation: the unread kind index dropped from migration 10.
    #[test]
    fn the_counts_read_a_covering_index_alone() {
        let store = HumanStore::open_in_memory(StoreOptions::default()).expect("store");
        for (sql, indexes) in [
            (
                DIRECT_COUNTS,
                &["COVERING INDEX unread_inbound_by_kind"][..],
            ),
            (
                CHANNEL_COUNTS,
                &["COVERING INDEX unread_inbound_by_kind"][..],
            ),
            (
                HIDDEN,
                &[
                    "COVERING INDEX unread_inbound_by_kind",
                    "COVERING INDEX kept_inbound_by_kind",
                ][..],
            ),
        ] {
            let plan: Vec<String> = store
                .conn
                .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
                .expect("plan")
                .query_map([], |r| r.get::<_, String>(3))
                .expect("rows")
                .collect::<Result<_, _>>()
                .expect("plan rows");
            for index in indexes {
                assert!(
                    plan.iter().any(|step| step.contains(index)),
                    "{index} not used: {plan:?}"
                );
            }
            assert!(
                !plan
                    .iter()
                    .any(|step| step.starts_with("SCAN unread_inbound")
                        && !step.contains("COVERING")),
                "a row read: {plan:?}"
            );
        }
    }

    /// A peer's endpoint count reads the conversation index by a range on
    /// its key and the routes by their index, never a scan of either table.
    /// Mutation: the range replaced by a filter on `peer_id`.
    #[test]
    fn the_endpoint_count_reads_its_indexes() {
        let store = HumanStore::open_in_memory(StoreOptions::default()).expect("store");
        let plan: Vec<String> = store
            .conn
            .prepare(&format!("EXPLAIN QUERY PLAN {ENDPOINT_COUNT}"))
            .expect("plan")
            .query_map(rusqlite::params!["a", "b", "p"], |r| r.get::<_, String>(3))
            .expect("rows")
            .collect::<Result<_, _>>()
            .expect("plan rows");
        assert!(
            plan.iter()
                .any(|s| s.starts_with("SEARCH conversation_index USING")),
            "{plan:?}"
        );
        assert!(
            plan.iter().any(|s| s.contains("contact_routes_by_route")),
            "{plan:?}"
        );
        assert!(
            !plan.iter().any(|s| s.starts_with("SCAN conversation_index")
                || s.starts_with("SCAN contact_routes")),
            "{plan:?}"
        );
    }
}
