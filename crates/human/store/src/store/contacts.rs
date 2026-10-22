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
    /// The photo's encoded bytes as accepted or set. The store never
    /// decodes them: Accept passes a card's photo only when it decodes
    /// within 256 × 256, and the view decodes it again to show it.
    pub photo: Option<Vec<u8>>,
    /// Its routes.
    pub routes: Vec<ContactRoute>,
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
                "SELECT display_name, name_source, name_at, photo_source, photo
                   FROM contacts WHERE contact_id = ?1",
                params![contact_id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<i64>>(2)?,
                        r.get::<_, Option<String>>(3)?,
                        r.get::<_, Option<Vec<u8>>>(4)?,
                    ))
                },
            )
            .optional()?;
        let Some((display_name, name_source, name_at, photo_source, photo)) = row else {
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
