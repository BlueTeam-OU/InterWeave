// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Contacts built from accepted cards, and each identity's own card
//! (plan §29 step 2; ADR-0059 decisions 2-4, ADR-0061 decision 4,
//! ADR-0063 decision 4; STATE.md's `contacts`, `contact_routes`, `own_cards`).
//!
//! Content-free metadata, outside ADR-0044's three states: what a person
//! accepted about someone, and what they wrote about themselves. The
//! rules a writer keeps, which no constraint can:
//!
//! - a card names only its own route (`{source_peer, source_endpoint}`);
//!   the contact holding that exact route is updated, and no other;
//! - a card on another endpoint of a `PeerId` a contact holds makes a
//!   contact of its own, since a contact holds at most one route per
//!   `PeerId` (the table's `UNIQUE(contact_id, peer_id)`);
//! - a value the person wrote is never overwritten by a card: a name or
//!   photo whose source is the person's, or unknown (every contact from
//!   before v9 was typed), stays;
//! - a route's identity key is recorded by the first accepted card and
//!   replaced only by an accepted card with another key, which the
//!   person accepted as a key change, on that route alone.

use rusqlite::{OptionalExtension as _, params};

use interweave_transport_api::{EndpointId, TransportIdentity};

use super::{HumanStore, sql_timestamp, stored_ms};
use crate::StoreError;

/// Where a contact's value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueSource {
    /// An accepted card.
    Card,
    /// The person: typed, or edited after a card.
    Person,
}

impl ValueSource {
    const fn as_sql(self) -> &'static str {
        match self {
            Self::Card => "card",
            Self::Person => "person",
        }
    }

    /// A NULL source is a value from before v9, which was typed.
    fn from_sql(value: Option<&str>) -> Self {
        match value {
            Some("card") => Self::Card,
            _ => Self::Person,
        }
    }
}

/// One route of a contact: a remote identity of one node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactRoute {
    /// The authenticated `PeerId`.
    pub peer: TransportIdentity,
    /// The remote identity's `EndpointId`.
    pub endpoint: EndpointId,
    /// The identity key the route's accepted card recorded, if any.
    pub identity_key: Option<[u8; 32]>,
}

/// A contact and its routes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contact {
    /// The store's id for it.
    pub contact_id: String,
    /// The name, then a space and the surname when there is one.
    pub display_name: String,
    /// Where the name came from.
    pub name_source: ValueSource,
    /// When the name was last set, if known.
    pub name_at: Option<u64>,
    /// Where the photo came from, if there is one.
    pub photo_source: Option<ValueSource>,
    /// When the photo was last set or removed, if known.
    pub photo_at: Option<u64>,
    /// The photo's encoded bytes as accepted or set. The store never
    /// decodes them: Accept passes a card's photo only when it decodes
    /// within 256 × 256, and the view decodes it again to show it.
    pub photo: Option<Vec<u8>>,
    /// Its routes.
    pub routes: Vec<ContactRoute>,
}

/// What [`HumanStore::delete_route`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteDeleted {
    /// The route and its recorded key are gone; the contact stays.
    Deleted,
    /// The contact's only route: nothing was written.
    LastRoute,
    /// The contact holds no such route, or there is no such contact.
    NoSuchRoute,
}

/// A card the person accepted, as the parser gave it.
#[derive(Debug, Clone, Copy)]
pub struct AcceptedCard<'a> {
    /// The route it arrived on: the authenticated source peer.
    pub peer: &'a TransportIdentity,
    /// The route it arrived on: the source endpoint.
    pub endpoint: &'a EndpointId,
    /// The card's name, after NFC.
    pub name: &'a str,
    /// The card's surname, after NFC.
    pub surname: Option<&'a str>,
    /// The card's identity key.
    pub identity_key: [u8; 32],
    /// The card's photo as received, if it carried one its caller found
    /// decodes within the card's bounds; the store writes it as given.
    pub photo: Option<&'a [u8]>,
}

impl AcceptedCard<'_> {
    /// `display_name`: the name, then a space and the surname.
    #[must_use]
    pub fn display_name(&self) -> String {
        self.surname
            .map_or_else(|| self.name.to_owned(), |s| format!("{} {s}", self.name))
    }
}

/// How a card's key stands against its route's recorded one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStanding {
    /// The route records no key: a new route, a typed one, or one from
    /// before ADR-0063.
    Unrecorded,
    /// The card carries the recorded key: the same identity.
    Same,
    /// The card carries another key: a key change, the person's to decide.
    Changed {
        /// The key the route records.
        recorded: [u8; 32],
    },
}

/// What a received card would do, for its proposal (`human-client-ui.md`
/// §3): the contact it would update, the contacts that share its node,
/// the contacts with its name on other nodes, and how its key stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardStanding {
    /// The contact holding the card's exact route, which Accept updates.
    pub updates: Option<Contact>,
    /// Contacts holding another endpoint of the card's `PeerId`: Accept
    /// makes a contact of its own, and the proposal names these.
    pub shares_node: Vec<Contact>,
    /// Contacts of the same display name on other `PeerIds`: the same-name
    /// note.
    pub same_name: Vec<Contact>,
    /// The key against the route's record.
    pub key: KeyStanding,
    /// A contact holding this card's key on another `PeerId`: the same
    /// identity on another node, which the proposal says (ADR-0063
    /// decision 4) and which groups nothing on its own.
    pub key_matches: Vec<Contact>,
}

/// What Accept did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accepted {
    /// The contact it created or updated.
    pub contact_id: String,
    /// Whether it created one.
    pub created: bool,
    /// Whether the contact's name was the person's and stayed.
    pub name_kept: bool,
    /// Whether the contact's photo was the person's and stayed.
    pub photo_kept: bool,
    /// How the key stood before.
    pub key: KeyStanding,
    /// What [`HumanStore::undo_accept`] writes back.
    pub undo: AcceptUndo,
}

/// What an Accept changed, kept by the caller for the person's Undo
/// (contact-cards design section 5.4.2): a created contact is deleted
/// with its route, an updated one gets back the values it had and its
/// route the key it had. Held in memory only: a process that ends inside
/// the window leaves the Accept as it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptUndo(UndoOf);

#[derive(Debug, Clone, PartialEq, Eq)]
enum UndoOf {
    Created {
        contact_id: String,
    },
    Updated {
        contact_id: String,
        peer: String,
        endpoint: String,
        before: Box<ContactValues>,
        key: Option<Vec<u8>>,
    },
}

/// A contact the person deleted, as it was stored, kept by the caller for
/// the person's Undo (contact-cards design section 6.3): its row and its
/// routes' rows, every column as read, so the Undo writes back the same
/// values and provenance. Held in memory only, like [`AcceptUndo`]: a
/// process that ends inside the window leaves the contact deleted.
#[derive(Clone, PartialEq)]
pub struct DeletedContact {
    contact_id: String,
    contact: StoredRow,
    routes: Vec<StoredRow>,
}

impl DeletedContact {
    /// The deleted contact's id.
    #[must_use]
    pub fn contact_id(&self) -> &str {
        &self.contact_id
    }
}

// Names and photos are the person's: a log line says which contact only.
impl std::fmt::Debug for DeletedContact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeletedContact")
            .field("contact_id", &self.contact_id)
            .field("routes", &self.routes.len())
            .finish_non_exhaustive()
    }
}

/// What [`HumanStore::restore_contact`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactRestored {
    /// The contact and its routes are back as they were.
    Restored,
    /// A route it held is under another contact now: nothing is written,
    /// since one route is never under two contacts.
    RouteTaken,
    /// A contact of its id exists: nothing is written.
    Exists,
}

/// A contact's photo the person removed, as it was stored, for the
/// person's Undo (design section 6.3): the bytes, where they came from
/// and when, and the time the removal wrote. Held in memory only.
#[derive(Clone, PartialEq, Eq)]
pub struct RemovedPhoto {
    contact_id: String,
    photo: Vec<u8>,
    photo_source: Option<String>,
    photo_at: Option<i64>,
    updated_at: i64,
    removed_at: i64,
}

impl RemovedPhoto {
    /// The contact whose photo it was.
    #[must_use]
    pub fn contact_id(&self) -> &str {
        &self.contact_id
    }
}

impl std::fmt::Debug for RemovedPhoto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemovedPhoto")
            .field("contact_id", &self.contact_id)
            .finish_non_exhaustive()
    }
}

/// One stored row, every column as read: what an Undo writes back.
#[derive(Debug, Clone, PartialEq)]
struct StoredRow {
    columns: Vec<String>,
    values: Vec<rusqlite::types::Value>,
}

impl StoredRow {
    /// The rows of `table` whose `key` column is `id`.
    fn read(
        tx: &rusqlite::Transaction<'_>,
        table: &str,
        key: &str,
        id: &str,
    ) -> rusqlite::Result<Vec<Self>> {
        let mut stmt = tx.prepare(&format!("SELECT * FROM {table} WHERE {key} = ?1"))?;
        let columns: Vec<String> = stmt
            .column_names()
            .iter()
            .map(|c| (*c).to_owned())
            .collect();
        let rows = stmt.query_map(params![id], |row| {
            (0..columns.len())
                .map(|i| row.get::<_, rusqlite::types::Value>(i))
                .collect::<rusqlite::Result<Vec<_>>>()
        })?;
        rows.map(|values| {
            values.map(|values| Self {
                columns: columns.clone(),
                values,
            })
        })
        .collect()
    }

    /// Insert it into `table` as it was.
    fn insert(&self, tx: &rusqlite::Transaction<'_>, table: &str) -> rusqlite::Result<()> {
        let marks = vec!["?"; self.columns.len()].join(", ");
        tx.execute(
            &format!(
                "INSERT INTO {table} ({}) VALUES ({marks})",
                self.columns.join(", ")
            ),
            rusqlite::params_from_iter(self.values.iter()),
        )?;
        Ok(())
    }

    /// The text value of `column`, if it has one.
    fn text(&self, column: &str) -> Option<&str> {
        let i = self.columns.iter().position(|c| c == column)?;
        match &self.values[i] {
            rusqlite::types::Value::Text(t) => Some(t.as_str()),
            _ => None,
        }
    }
}

/// A contact's values an Accept may overwrite, as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ContactValues {
    display_name: String,
    name_source: Option<String>,
    name_at: Option<i64>,
    photo: Option<Vec<u8>>,
    photo_source: Option<String>,
    photo_at: Option<i64>,
    updated_at: i64,
}

impl AcceptUndo {
    /// The contact the Accept created or updated.
    #[must_use]
    pub fn contact_id(&self) -> &str {
        match &self.0 {
            UndoOf::Created { contact_id } | UndoOf::Updated { contact_id, .. } => contact_id,
        }
    }
}

/// An identity's own card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnCard {
    /// The identity.
    pub endpoint: EndpointId,
    /// The name.
    pub name: String,
    /// The surname, if any.
    pub surname: Option<String>,
    /// The photo, re-encoded and stripped, if any.
    pub photo: Option<Vec<u8>>,
}

/// The `conversation_index` id of a direct conversation: one per route.
#[must_use]
pub fn direct_conversation_id(peer: &TransportIdentity, endpoint: &EndpointId) -> String {
    format!("direct/{}/{}", peer.as_str(), endpoint.as_str())
}

fn new_contact_id() -> String {
    format!("{:032x}", rand::random::<u128>())
}

fn key_of(blob: Option<Vec<u8>>) -> Result<Option<[u8; 32]>, StoreError> {
    blob.map(|b| {
        <[u8; 32]>::try_from(b.as_slice())
            .map_err(|_| StoreError::Corrupt("a route's identity key is not 32 bytes".to_owned()))
    })
    .transpose()
}

impl HumanStore {
    /// Every contact, with its routes, ordered by name.
    ///
    /// # Errors
    /// A storage error, or [`StoreError::Corrupt`] for an unreadable row.
    pub fn contacts(&self) -> Result<Vec<Contact>, StoreError> {
        let ids: Vec<String> = self
            .conn
            .prepare("SELECT contact_id FROM contacts ORDER BY display_name, contact_id")?
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        ids.iter()
            .map(|id| {
                self.contact(id)?
                    .ok_or_else(|| StoreError::Corrupt("a contact vanished".to_owned()))
            })
            .collect()
    }

    /// One contact by id.
    ///
    /// # Errors
    /// A storage error, or [`StoreError::Corrupt`] for an unreadable row.
    pub fn contact(&self, contact_id: &str) -> Result<Option<Contact>, StoreError> {
        let row = self
            .conn
            .query_row(
                "SELECT display_name, name_source, name_at, photo_source, photo, photo_at
                   FROM contacts WHERE contact_id = ?1",
                params![contact_id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<i64>>(2)?,
                        r.get::<_, Option<String>>(3)?,
                        r.get::<_, Option<Vec<u8>>>(4)?,
                        r.get::<_, Option<i64>>(5)?,
                    ))
                },
            )
            .optional()?;
        let Some((display_name, name_source, name_at, photo_source, photo, photo_at)) = row else {
            return Ok(None);
        };
        let routes = self
            .conn
            .prepare(
                "SELECT peer_id, endpoint_id, identity_key FROM contact_routes
                  WHERE contact_id = ?1 ORDER BY peer_id, endpoint_id",
            )?
            .query_map(params![contact_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<Vec<u8>>>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|(peer, endpoint, key)| {
                Ok(ContactRoute {
                    peer: TransportIdentity::parse(&peer).map_err(|_| {
                        StoreError::Corrupt("a route's PeerId does not parse".to_owned())
                    })?,
                    endpoint: EndpointId::parse(&endpoint).map_err(|_| {
                        StoreError::Corrupt("a route's EndpointId does not parse".to_owned())
                    })?,
                    identity_key: key_of(key)?,
                })
            })
            .collect::<Result<_, StoreError>>()?;
        Ok(Some(Contact {
            contact_id: contact_id.to_owned(),
            display_name,
            name_source: ValueSource::from_sql(name_source.as_deref()),
            name_at: name_at.map(|t| stored_ms("name_at", t)).transpose()?,
            photo_source: photo_source
                .as_deref()
                .map(|s| ValueSource::from_sql(Some(s))),
            photo_at: photo_at.map(|t| stored_ms("photo_at", t)).transpose()?,
            photo,
            routes,
        }))
    }

    /// The contact holding exactly this route, if any.
    ///
    /// # Errors
    /// A storage error.
    pub fn contact_for_route(
        &self,
        peer: &TransportIdentity,
        endpoint: &EndpointId,
    ) -> Result<Option<Contact>, StoreError> {
        let id: Option<String> = self
            .conn
            .query_row(
                "SELECT contact_id FROM contact_routes WHERE peer_id = ?1 AND endpoint_id = ?2",
                params![peer.as_str(), endpoint.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        id.map_or(Ok(None), |id| self.contact(&id))
    }

    /// What `card` would do if accepted, for its proposal.
    ///
    /// # Errors
    /// A storage error.
    pub fn card_standing(&self, card: &AcceptedCard<'_>) -> Result<CardStanding, StoreError> {
        let updates = self.contact_for_route(card.peer, card.endpoint)?;
        let key = match updates
            .as_ref()
            .and_then(|c| {
                c.routes
                    .iter()
                    .find(|r| &r.peer == card.peer && &r.endpoint == card.endpoint)
            })
            .and_then(|r| r.identity_key)
        {
            None => KeyStanding::Unrecorded,
            Some(k) if k == card.identity_key => KeyStanding::Same,
            Some(recorded) => KeyStanding::Changed { recorded },
        };
        let others = self.contacts()?;
        let not_updated = |c: &&Contact| {
            updates
                .as_ref()
                .is_none_or(|u| u.contact_id != c.contact_id)
        };
        let shares_node = others
            .iter()
            .filter(not_updated)
            .filter(|c| c.routes.iter().any(|r| &r.peer == card.peer))
            .cloned()
            .collect();
        let display = card.display_name();
        let same_name = others
            .iter()
            .filter(not_updated)
            .filter(|c| c.display_name == display && c.routes.iter().all(|r| &r.peer != card.peer))
            .cloned()
            .collect();
        let key_matches = others
            .iter()
            .filter(not_updated)
            .filter(|c| {
                c.routes
                    .iter()
                    .any(|r| &r.peer != card.peer && r.identity_key == Some(card.identity_key))
            })
            .cloned()
            .collect();
        Ok(CardStanding {
            updates,
            shares_node,
            same_name,
            key,
            key_matches,
        })
    }

    /// Accept `card`: update the contact holding its route, or create one.
    ///
    /// The person accepted what [`HumanStore::card_standing`] showed them,
    /// a key change included, so the route's key becomes the card's. A
    /// name or photo the person wrote stays.
    ///
    /// # Errors
    /// A storage error; nothing is written then.
    pub fn accept_card(
        &mut self,
        card: &AcceptedCard<'_>,
        at_ms: u64,
    ) -> Result<Accepted, StoreError> {
        let at = sql_timestamp("at", at_ms)?;
        let display = card.display_name();
        let result = (|| -> Result<Accepted, StoreError> {
            let tx = self.conn.transaction()?;
            let held: Option<(String, Option<Vec<u8>>)> = tx
                .query_row(
                    "SELECT contact_id, identity_key FROM contact_routes WHERE peer_id = ?1 AND endpoint_id = ?2",
                    params![card.peer.as_str(), card.endpoint.as_str()],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            let accepted = if let Some((contact_id, recorded)) = held {
                let before: ContactValues = tx.query_row(
                    "SELECT display_name, name_source, name_at, photo, photo_source, photo_at, updated_at
                       FROM contacts WHERE contact_id = ?1",
                    params![contact_id],
                    |r| {
                        Ok(ContactValues {
                            display_name: r.get(0)?,
                            name_source: r.get(1)?,
                            name_at: r.get(2)?,
                            photo: r.get(3)?,
                            photo_source: r.get(4)?,
                            photo_at: r.get(5)?,
                            updated_at: r.get(6)?,
                        })
                    },
                )?;
                let undo = AcceptUndo(UndoOf::Updated {
                    contact_id: contact_id.clone(),
                    peer: card.peer.as_str().to_owned(),
                    endpoint: card.endpoint.as_str().to_owned(),
                    before: Box::new(before),
                    key: recorded.clone(),
                });
                let key = match key_of(recorded)? {
                    None => KeyStanding::Unrecorded,
                    Some(k) if k == card.identity_key => KeyStanding::Same,
                    Some(recorded) => KeyStanding::Changed { recorded },
                };
                let (name_source, photo_source): (Option<String>, Option<String>) = tx.query_row(
                    "SELECT name_source, photo_source FROM contacts WHERE contact_id = ?1",
                    params![contact_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?;
                let name_kept =
                    ValueSource::from_sql(name_source.as_deref()) == ValueSource::Person;
                if !name_kept {
                    tx.execute(
                        "UPDATE contacts SET display_name = ?2, name_at = ?3, updated_at = ?3
                          WHERE contact_id = ?1",
                        params![contact_id, display, at],
                    )?;
                }
                // A photo the person set stays. Unlike a name, a contact
                // may hold no photo at all, so one with no recorded source
                // is no person's choice, and the card's photo, or its
                // absence, is the contact's.
                let photo_kept = photo_source.as_deref() == Some(ValueSource::Person.as_sql());
                if !photo_kept {
                    tx.execute(
                        "UPDATE contacts SET photo = ?2, photo_source = ?3, photo_at = ?4, updated_at = ?5
                          WHERE contact_id = ?1",
                        params![
                            contact_id,
                            card.photo,
                            card.photo.map(|_| ValueSource::Card.as_sql()),
                            card.photo.map(|_| at),
                            at
                        ],
                    )?;
                }
                tx.execute(
                    "UPDATE contact_routes SET identity_key = ?3 WHERE peer_id = ?1 AND endpoint_id = ?2",
                    params![card.peer.as_str(), card.endpoint.as_str(), card.identity_key.as_slice()],
                )?;
                Accepted {
                    contact_id,
                    created: false,
                    name_kept,
                    photo_kept,
                    key,
                    undo,
                }
            } else {
                let contact_id = new_contact_id();
                tx.execute(
                    "INSERT INTO contacts (contact_id, display_name, name_source, name_at,
                                           photo, photo_source, photo_at, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?4, ?4)",
                    params![
                        contact_id,
                        display,
                        ValueSource::Card.as_sql(),
                        at,
                        card.photo,
                        card.photo.map(|_| ValueSource::Card.as_sql()),
                        card.photo.map(|_| at)
                    ],
                )?;
                tx.execute(
                    "INSERT INTO contact_routes (contact_id, peer_id, endpoint_id, identity_key)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        contact_id,
                        card.peer.as_str(),
                        card.endpoint.as_str(),
                        card.identity_key.as_slice()
                    ],
                )?;
                Accepted {
                    undo: AcceptUndo(UndoOf::Created {
                        contact_id: contact_id.clone(),
                    }),
                    contact_id,
                    created: true,
                    name_kept: false,
                    photo_kept: false,
                    key: KeyStanding::Unrecorded,
                }
            };
            tx.commit()?;
            Ok(accepted)
        })();
        result.map_err(|e| self.note_store_failure(e))
    }

    /// Undo an Accept: delete the contact it created, with its route, or
    /// write back the values the contact it updated had and the key its
    /// route had. Whether there was a contact to undo on: a contact
    /// deleted since is not made again.
    ///
    /// # Errors
    /// A storage error; nothing is written then.
    pub fn undo_accept(&mut self, undo: &AcceptUndo) -> Result<bool, StoreError> {
        let UndoOf::Updated {
            contact_id,
            peer,
            endpoint,
            before,
            key,
        } = &undo.0
        else {
            return self.delete_contact(undo.contact_id());
        };
        let result = (|| -> Result<bool, StoreError> {
            let tx = self.conn.transaction()?;
            let restored = tx.execute(
                "UPDATE contacts SET display_name = ?2, name_source = ?3, name_at = ?4,
                        photo = ?5, photo_source = ?6, photo_at = ?7, updated_at = ?8
                  WHERE contact_id = ?1",
                params![
                    contact_id,
                    before.display_name,
                    before.name_source,
                    before.name_at,
                    before.photo,
                    before.photo_source,
                    before.photo_at,
                    before.updated_at
                ],
            )?;
            tx.execute(
                "UPDATE contact_routes SET identity_key = ?4
                  WHERE contact_id = ?1 AND peer_id = ?2 AND endpoint_id = ?3",
                params![contact_id, peer, endpoint, key],
            )?;
            tx.commit()?;
            Ok(restored == 1)
        })();
        let restored = result.map_err(|e| self.note_store_failure(e))?;
        if restored {
            // The card's photo, overwritten, leaves the log as a deleted
            // contact's does.
            self.release_done();
        }
        Ok(restored)
    }

    /// The person's own edit of a contact's name: it is theirs from now
    /// on, and no card overwrites it.
    ///
    /// # Errors
    /// A storage error. Whether there was such a contact.
    pub fn set_contact_name(
        &mut self,
        contact_id: &str,
        name: &str,
        at_ms: u64,
    ) -> Result<bool, StoreError> {
        let at = sql_timestamp("at", at_ms)?;
        let changed = self
            .conn
            .execute(
                "UPDATE contacts SET display_name = ?2, name_source = ?3, name_at = ?4, updated_at = ?4
                  WHERE contact_id = ?1",
                params![contact_id, name, ValueSource::Person.as_sql(), at],
            )
            .map_err(StoreError::from)
            .map_err(|e| self.note_store_failure(e))?;
        Ok(changed == 1)
    }

    /// The person's own photo for a contact, or its removal: theirs from
    /// now on, and no card replaces it.
    ///
    /// # Errors
    /// A storage error. Whether there was such a contact.
    pub fn set_contact_photo(
        &mut self,
        contact_id: &str,
        photo: Option<&[u8]>,
        at_ms: u64,
    ) -> Result<bool, StoreError> {
        let at = sql_timestamp("at", at_ms)?;
        let changed = self
            .conn
            .execute(
                "UPDATE contacts SET photo = ?2, photo_source = ?3, photo_at = ?4, updated_at = ?4
                  WHERE contact_id = ?1",
                params![contact_id, photo, ValueSource::Person.as_sql(), at],
            )
            .map_err(StoreError::from)
            .map_err(|e| self.note_store_failure(e))?;
        Ok(changed == 1)
    }

    /// Delete a contact: its row and its routes, and with them the name,
    /// the photo and every identity key its routes recorded -- the person
    /// removes a contact only by deleting it (ADR-0059 decision 3). The
    /// routes are deleted by name as well as by the cascade, so the rule
    /// does not rest on the connection's pragma. What was deleted leaves
    /// the log as released content does (`release_done`): before this
    /// returns (`a_deleted_contact_leaves_both_files_before_the_call_returns`),
    /// unless another reader holds the log then, and at the next release,
    /// open or close if so. A card from one of its routes is a new
    /// proposal afterwards, its key unrecorded.
    ///
    /// # Errors
    /// A storage error. Whether there was such a contact.
    pub fn delete_contact(&mut self, contact_id: &str) -> Result<bool, StoreError> {
        Ok(self.delete_contact_for_undo(contact_id)?.is_some())
    }

    /// [`HumanStore::delete_contact`], returning what it deleted for the
    /// person's Undo ([`HumanStore::restore_contact`]): the rows are read
    /// in the transaction that deletes them, so the Undo writes back what
    /// was deleted and nothing else. `None` when there was no such
    /// contact.
    ///
    /// # Errors
    /// A storage error; nothing is deleted then.
    pub fn delete_contact_for_undo(
        &mut self,
        contact_id: &str,
    ) -> Result<Option<DeletedContact>, StoreError> {
        let result = (|| -> Result<Option<DeletedContact>, StoreError> {
            let tx = self.conn.transaction()?;
            let contact = StoredRow::read(&tx, "contacts", "contact_id", contact_id)?;
            let routes = StoredRow::read(&tx, "contact_routes", "contact_id", contact_id)?;
            tx.execute(
                "DELETE FROM contact_routes WHERE contact_id = ?1",
                params![contact_id],
            )?;
            let deleted = tx.execute(
                "DELETE FROM contacts WHERE contact_id = ?1",
                params![contact_id],
            )?;
            tx.commit()?;
            Ok(contact
                .into_iter()
                .next()
                .filter(|_| deleted == 1)
                .map(|contact| DeletedContact {
                    contact_id: contact_id.to_owned(),
                    contact,
                    routes,
                }))
        })();
        let deleted = result.map_err(|e| self.note_store_failure(e))?;
        if deleted.is_some() {
            self.release_done();
        }
        Ok(deleted)
    }

    /// The person's Undo of a delete (design section 6.3): the contact
    /// and its routes written back with the values and provenance they
    /// had, in one transaction -- unless a route it held is under another
    /// contact now, or a contact of its id exists, when nothing is
    /// written.
    ///
    /// # Errors
    /// A storage error; nothing is written then.
    pub fn restore_contact(
        &mut self,
        deleted: &DeletedContact,
    ) -> Result<ContactRestored, StoreError> {
        let result = (|| -> Result<ContactRestored, StoreError> {
            let tx = self.conn.transaction()?;
            let exists: Option<i64> = tx
                .query_row(
                    "SELECT 1 FROM contacts WHERE contact_id = ?1",
                    params![deleted.contact_id],
                    |r| r.get(0),
                )
                .optional()?;
            if exists.is_some() {
                return Ok(ContactRestored::Exists);
            }
            for route in &deleted.routes {
                let taken: Option<i64> = tx
                    .query_row(
                        "SELECT 1 FROM contact_routes WHERE peer_id = ?1 AND endpoint_id = ?2",
                        params![route.text("peer_id"), route.text("endpoint_id")],
                        |r| r.get(0),
                    )
                    .optional()?;
                if taken.is_some() {
                    return Ok(ContactRestored::RouteTaken);
                }
            }
            deleted.contact.insert(&tx, "contacts")?;
            for route in &deleted.routes {
                route.insert(&tx, "contact_routes")?;
            }
            tx.commit()?;
            Ok(ContactRestored::Restored)
        })();
        result.map_err(|e| self.note_store_failure(e))
    }

    /// The person's removal of a contact's photo, as
    /// [`HumanStore::set_contact_photo`] with no photo writes it, returning
    /// what it removed for the person's Undo
    /// ([`HumanStore::restore_contact_photo`]). `None` when there was no
    /// such contact or it had no photo, and nothing is written then.
    ///
    /// # Errors
    /// A storage error; nothing is written then.
    pub fn remove_contact_photo(
        &mut self,
        contact_id: &str,
        at_ms: u64,
    ) -> Result<Option<RemovedPhoto>, StoreError> {
        let at = sql_timestamp("at", at_ms)?;
        let result = (|| -> Result<Option<RemovedPhoto>, StoreError> {
            let tx = self.conn.transaction()?;
            let before = tx
                .query_row(
                    "SELECT photo, photo_source, photo_at, updated_at FROM contacts
                      WHERE contact_id = ?1 AND photo IS NOT NULL",
                    params![contact_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .optional()?;
            let Some((photo, photo_source, photo_at, updated_at)) = before else {
                return Ok(None);
            };
            tx.execute(
                "UPDATE contacts SET photo = NULL, photo_source = ?2, photo_at = ?3, updated_at = ?3
                  WHERE contact_id = ?1",
                params![contact_id, ValueSource::Person.as_sql(), at],
            )?;
            tx.commit()?;
            Ok(Some(RemovedPhoto {
                contact_id: contact_id.to_owned(),
                photo,
                photo_source,
                photo_at,
                updated_at,
                removed_at: at,
            }))
        })();
        result.map_err(|e| self.note_store_failure(e))
    }

    /// The person's Undo of a photo removal (design section 6.3): the
    /// bytes written back with where they came from and when -- only
    /// while the contact still shows the removal, so an Undo never
    /// overwrites a photo set since. Whether it was written.
    ///
    /// # Errors
    /// A storage error; nothing is written then.
    pub fn restore_contact_photo(&mut self, removed: &RemovedPhoto) -> Result<bool, StoreError> {
        let changed = self
            .conn
            .execute(
                "UPDATE contacts SET photo = ?2, photo_source = ?3, photo_at = ?4, updated_at = ?5
                  WHERE contact_id = ?1 AND photo IS NULL AND photo_at = ?6",
                params![
                    removed.contact_id,
                    removed.photo,
                    removed.photo_source,
                    removed.photo_at,
                    removed.updated_at,
                    removed.removed_at
                ],
            )
            .map_err(StoreError::from)
            .map_err(|e| self.note_store_failure(e))?;
        Ok(changed == 1)
    }

    /// Delete one route of a contact, with the identity key it recorded
    /// (ADR-0059 decision 9: deleting a contact's routes): the contact
    /// stays with its others. A contact's last route is refused in the
    /// same transaction and nothing is written, since a contact with no
    /// route is a delete of the contact, which is the person's
    /// [`HumanStore::delete_contact`]. What was deleted leaves the log as a
    /// deleted contact does
    /// (`a_deleted_route_leaves_both_files_and_the_last_is_refused`). A
    /// card from the route is a new proposal afterwards, its key
    /// unrecorded.
    ///
    /// # Errors
    /// A storage error; nothing is written then.
    pub fn delete_route(
        &mut self,
        contact_id: &str,
        peer: &TransportIdentity,
        endpoint: &EndpointId,
    ) -> Result<RouteDeleted, StoreError> {
        let result = (|| -> Result<RouteDeleted, StoreError> {
            let tx = self.conn.transaction()?;
            let routes: i64 = tx.query_row(
                "SELECT COUNT(*) FROM contact_routes WHERE contact_id = ?1",
                params![contact_id],
                |r| r.get(0),
            )?;
            let held: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM contact_routes
                                WHERE contact_id = ?1 AND peer_id = ?2 AND endpoint_id = ?3)",
                params![contact_id, peer.as_str(), endpoint.as_str()],
                |r| r.get(0),
            )?;
            if !held {
                return Ok(RouteDeleted::NoSuchRoute);
            }
            if routes <= 1 {
                return Ok(RouteDeleted::LastRoute);
            }
            tx.execute(
                "DELETE FROM contact_routes
                  WHERE contact_id = ?1 AND peer_id = ?2 AND endpoint_id = ?3",
                params![contact_id, peer.as_str(), endpoint.as_str()],
            )?;
            tx.commit()?;
            Ok(RouteDeleted::Deleted)
        })();
        let deleted = result.map_err(|e| self.note_store_failure(e))?;
        if deleted == RouteDeleted::Deleted {
            self.release_done();
        }
        Ok(deleted)
    }

    /// Record that the conversation on this route shared its identity's
    /// card at `at_ms` (`conversation_index.card_shared_at`).
    ///
    /// # Errors
    /// A storage error.
    pub fn mark_card_shared(
        &mut self,
        peer: &TransportIdentity,
        endpoint: &EndpointId,
        at_ms: u64,
    ) -> Result<(), StoreError> {
        let at = sql_timestamp("at", at_ms)?;
        self.conn
            .execute(
                "INSERT INTO conversation_index (conversation_id, peer_id, endpoint_id, card_shared_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(conversation_id) DO UPDATE SET card_shared_at = excluded.card_shared_at",
                params![direct_conversation_id(peer, endpoint), peer.as_str(), endpoint.as_str(), at],
            )
            .map_err(StoreError::from)
            .map_err(|e| self.note_store_failure(e))?;
        Ok(())
    }

    /// Whether any conversation ever recorded a share of the identity's
    /// card: what tells a first start from a key file lost after
    /// counterparties got its key. `card_shared_at` is cleared by nothing
    /// short of a discard of the whole store.
    ///
    /// # Errors
    /// A storage error.
    pub fn card_ever_shared(&self) -> Result<bool, StoreError> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM conversation_index WHERE card_shared_at IS NOT NULL)",
            [],
            |r| r.get(0),
        )?)
    }

    /// Every direct conversation that recorded a share of the identity's
    /// card, most recently shared first: whose counterparts may hold its
    /// key (design/2026-10-24-identity-key.md section 5.3).
    ///
    /// # Errors
    /// A storage error, or [`StoreError::Corrupt`] for an unreadable row.
    pub fn cards_shared(&self) -> Result<Vec<(TransportIdentity, EndpointId, u64)>, StoreError> {
        let rows: Vec<(String, String, i64)> = self
            .conn
            .prepare(
                "SELECT peer_id, endpoint_id, card_shared_at FROM conversation_index
                  WHERE card_shared_at IS NOT NULL
                  ORDER BY card_shared_at DESC, conversation_id",
            )?
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<_, _>>()?;
        rows.into_iter()
            .map(|(peer, endpoint, at)| {
                Ok((
                    TransportIdentity::parse(&peer)
                        .map_err(|_| StoreError::Corrupt("a shared card's peer".to_owned()))?,
                    EndpointId::parse(&endpoint)
                        .map_err(|_| StoreError::Corrupt("a shared card's endpoint".to_owned()))?,
                    stored_ms("card_shared_at", at)?,
                ))
            })
            .collect()
    }

    /// When the conversation on this route last shared its card.
    ///
    /// # Errors
    /// A storage error.
    pub fn card_shared_at(
        &self,
        peer: &TransportIdentity,
        endpoint: &EndpointId,
    ) -> Result<Option<u64>, StoreError> {
        let at: Option<Option<i64>> = self
            .conn
            .query_row(
                "SELECT card_shared_at FROM conversation_index WHERE conversation_id = ?1",
                params![direct_conversation_id(peer, endpoint)],
                |r| r.get(0),
            )
            .optional()?;
        at.flatten()
            .map(|t| stored_ms("card_shared_at", t))
            .transpose()
    }

    /// An identity's own card, if it has one.
    ///
    /// # Errors
    /// A storage error, or [`StoreError::Corrupt`] for an unreadable row.
    pub fn own_card(&self, endpoint: &EndpointId) -> Result<Option<OwnCard>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT name, surname, photo FROM own_cards WHERE endpoint_id = ?1",
                params![endpoint.as_str()],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<Vec<u8>>>(2)?,
                    ))
                },
            )
            .optional()?
            .map(|(name, surname, photo)| OwnCard {
                endpoint: endpoint.clone(),
                name,
                surname,
                photo,
            }))
    }

    /// Write an identity's own card, as the person edited it.
    ///
    /// # Errors
    /// A storage error.
    pub fn set_own_card(&mut self, card: &OwnCard, at_ms: u64) -> Result<(), StoreError> {
        let at = sql_timestamp("at", at_ms)?;
        self.conn
            .execute(
                "INSERT INTO own_cards (endpoint_id, name, surname, photo, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5)
                 ON CONFLICT(endpoint_id) DO UPDATE SET
                    name = excluded.name, surname = excluded.surname, photo = excluded.photo,
                    updated_at = excluded.updated_at",
                params![
                    card.endpoint.as_str(),
                    card.name,
                    card.surname,
                    card.photo,
                    at
                ],
            )
            .map_err(StoreError::from)
            .map_err(|e| self.note_store_failure(e))?;
        Ok(())
    }

    /// The identity's own card, made with its route label as its name if
    /// it has none (STATE.md: a store recreated after a discard makes one
    /// per identity this way).
    ///
    /// # Errors
    /// A storage error.
    pub fn ensure_own_card(
        &mut self,
        endpoint: &EndpointId,
        at_ms: u64,
    ) -> Result<OwnCard, StoreError> {
        if let Some(card) = self.own_card(endpoint)? {
            return Ok(card);
        }
        let card = OwnCard {
            endpoint: endpoint.clone(),
            name: endpoint.as_str().to_owned(),
            surname: None,
            photo: None,
        };
        self.set_own_card(&card, at_ms)?;
        Ok(card)
    }
}
