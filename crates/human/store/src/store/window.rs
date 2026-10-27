// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The start window's reads (`design/2026-10-27-start-window.md`; STATE.md
//! schema v10): one conversation's shown rows a page at a time, newest
//! first, and the counts the Chats list needs, so that opening the app
//! reads counts and pages, never every row. Each is served by a v10 index;
//! `the_window_reads_use_their_indexes` reads their plans.

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

/// A conversation's unread count as the badge counts it (STATE.md schema
/// v10, architect-cto's A3): distinct message ids among its unread rows of
/// a kind this build shows, so a message stored twice counts once.
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

/// Shown rows of one table, by conversation, older than a bound, newest
/// first. `{table}` and `{read}` are literals of this module;
/// `{conversation}` is one of the two filters below.
fn page_sql(table: &str, conversation: &str) -> String {
    let read = if table == "kept_inbound" {
        "read_at, kept_at"
    } else {
        "NULL, NULL"
    };
    format!(
        "SELECT row_id, app_message_id, source_peer, source_endpoint, channel_id,
                media_type, payload, received_at, {read}, kind
           FROM {table}
          WHERE {conversation} AND kind <> 'other'
            AND (received_at, row_id) < (?3, ?4)
          ORDER BY received_at DESC, row_id DESC
          LIMIT ?5"
    )
}

/// A direct conversation: the peer and its endpoint key, served by the
/// `_by_conversation` index. A broadcast row's endpoint key is empty, and a
/// direct row's never is, so no channel row matches.
const DIRECT: &str = "source_peer = ?1 AND source_endpoint_key = ?2";

/// A channel: its key alone, served by the `_by_channel` index. `?2` is
/// unused and bound empty so both filters take the same parameters.
const CHANNEL: &str = "channel_key = ?1 AND ?2 = ''";

/// The rank of a table among rows received at one instant: kept rows
/// order after unread ones, newest first, so a cursor names one place.
const fn rank(table: BackupTable) -> u8 {
    match table {
        BackupTable::Unread => 0,
        BackupTable::Kept => 1,
    }
}

impl HumanStore {
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
        let (filter, first, second) = match conversation {
            StoreConversation::Direct { peer, endpoint } => {
                (DIRECT, peer.as_str(), endpoint.as_str())
            }
            StoreConversation::Channel(channel) => (CHANNEL, channel.as_str(), ""),
        };
        let fetch = i64::try_from(limits.max_records().saturating_add(1)).unwrap_or(i64::MAX);
        let mut merged: Vec<(BackupTable, StoredInbound, usize)> = Vec::new();
        for table in [BackupTable::Unread, BackupTable::Kept] {
            let name = match table {
                BackupTable::Unread => "unread_inbound",
                BackupTable::Kept => "kept_inbound",
            };
            // The bound in this table's own terms: a row of a table ranked
            // before the cursor's is older at the cursor's instant too, one
            // ranked after it only before that instant.
            let (at, row) = match before {
                None => (i64::MAX, i64::MAX),
                Some(c) => {
                    let at = sql_timestamp("cursor received_at", c.received_at)?;
                    match rank(table).cmp(&rank(c.table)) {
                        std::cmp::Ordering::Equal => (at, c.row_id.get()),
                        std::cmp::Ordering::Less => (at, i64::MAX),
                        std::cmp::Ordering::Greater => (at, 0),
                    }
                }
            };
            let mut stmt = self.conn.prepare(&page_sql(name, filter))?;
            let rows = stmt.query_map(params![first, second, at, row, fetch], InboundRow::read)?;
            for row in rows {
                let row = row?;
                let bytes = row.payload.len();
                merged.push((table, row.parse()?, bytes));
            }
        }
        merged.sort_by(|(ta, a, _), (tb, b, _)| {
            (b.received_at, rank(*tb), b.row_id.get()).cmp(&(
                a.received_at,
                rank(*ta),
                a.row_id.get(),
            ))
        });
        let mut items = Vec::new();
        let mut bytes = 0usize;
        let mut last: Option<WindowCursor> = None;
        let mut more = false;
        for (table, row, size) in merged {
            if !items.is_empty()
                && (items.len() >= limits.max_records()
                    || bytes.saturating_add(size) > limits.max_bytes())
            {
                more = true;
                break;
            }
            bytes = bytes.saturating_add(size);
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
        let mut direct = self.conn.prepare(
            "SELECT source_peer, source_endpoint, COUNT(DISTINCT app_message_id)
               FROM unread_inbound
              WHERE channel_key = '' AND kind <> 'other'
              GROUP BY source_peer, source_endpoint_key",
        )?;
        let rows = direct.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?;
        for row in rows {
            let (peer, endpoint, count) = row?;
            let Some(endpoint) = endpoint else {
                // A direct row with no endpoint is no row the client wrote.
                continue;
            };
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
        let mut channels = self.conn.prepare(
            "SELECT channel_id, COUNT(DISTINCT app_message_id)
               FROM unread_inbound
              WHERE channel_key <> '' AND kind <> 'other'
              GROUP BY channel_key",
        )?;
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

    /// The inbound rows of a kind this build cannot show, unread and kept.
    ///
    /// # Errors
    /// A storage error.
    pub fn hidden_inbound(&self) -> Result<HiddenInbound, StoreError> {
        let (unread, kept): (i64, i64) = self.conn.query_row(
            "SELECT (SELECT COUNT(*) FROM unread_inbound WHERE kind = 'other'),
                    (SELECT COUNT(*) FROM kept_inbound WHERE kind = 'other')",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
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
    use super::{CHANNEL, DIRECT, page_sql};
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
                .prepare(&format!("EXPLAIN QUERY PLAN {}", page_sql(table, filter)))
                .expect("plan")
                .query_map(rusqlite::params!["p", "e", 1, 1, 1], |r| {
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
}
