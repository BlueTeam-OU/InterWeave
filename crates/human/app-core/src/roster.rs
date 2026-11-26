// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The identities one client holds, each with its own facade side (plan
//! §30 step 1; ADR-0061; architect-cto's rulings A1, A2, A5 and A7 of
//! 2026-11-25, `human-client-ui.md` §13 and STATE.md's "one client
//! process, several identities"): one [`FacadeSide`], one
//! [`TransportClient`] and one store handle per held identity, the
//! handles all of one open (`HumanStore::handle_speaking_as`).
//!
//! What the roster decides, and what pins each:
//! - the held order: the first identity, then the others by their own
//!   card's creation, the `EndpointId` breaking a tie
//!   (`the_held_order_is_the_first_then_card_creation_then_id`);
//! - one status reader: the first identity's client reads status and
//!   every other is built `without_status`, and the first's connectivity
//!   is what every identity is told
//!   (`one_status_reader_whatever_the_number_held`);
//! - the first identity's session joins the profile's channels, the
//!   others join none (`the_first_identity_alone_joins_the_channels`);
//! - the open order: the identity in front, then the default, then the
//!   rest in held order (`sessions_open_front_then_default_then_held_order`);
//! - an identity whose open is answered `Overloaded` retries at the pace
//!   its client reconnects at, the others acting meanwhile, and the
//!   roster closes no identity's session to make room for another's
//!   (`an_overloaded_identity_retries_at_its_pace_and_the_others_act`);
//! - the identity in front at start, from the store's last-front row
//!   (`the_front_at_start_is_the_last_held_else_the_default_else_the_first`);
//! - a change of the identity in front swaps it and writes the last-front
//!   row, and a change to the identity in front or to one not held writes
//!   nothing
//!   (`set_front_swaps_and_writes_the_row_and_an_unchanged_front_writes_nothing`).
//!
//! It is the facade half: a model per identity is the model side's
//! ([`crate::ModelSide`]), which runs a switch's own steps on its thread
//! and sends the roster the new front after the old front's commands
//! ([`crate::ModelSide::switch`]). What the roster holds is what
//! the root hands it: in the batch this lands in, the composition hands it
//! the first identity alone (plan §30, the note of 2026-11-25), and a
//! roster of one behaves as the facade side `main` composed
//! (`a_roster_of_one_behaves_as_its_facade_side`).
//!
//! Turns and commands run one identity after another, each awaited before
//! the next begins: a trust or address act opens its own short admin
//! connection, so one client at a time asks
//! (`TransportClient::without_status`'s note).

use interweave_human_client_api::ClientEvent;
use interweave_human_store::{HumanStore, StoreError};
use interweave_human_transport_client::{ClientConfig, SessionBinding, TransportClient};
use interweave_human_ui_model::IDENTITY_CAP;
use interweave_local_client_api::AdminBinding;
use interweave_transport_api::{ChannelId, EndpointId};

use crate::facade::FacadeSide;
use crate::protocol::{Command, Update};

/// What a roster is built from: what `ClientConfig` carries on `main`,
/// with the identities besides the first and the profile's default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterConfig {
    /// The client kind every session declares.
    pub client_kind: String,
    /// The endpoints the client may lease, as `ClientConfig::endpoints`:
    /// every identity's client is given the same list, and each leases
    /// the identity its store handle speaks as.
    pub endpoints: Vec<EndpointId>,
    /// The profile's channels: the first identity's session joins them,
    /// no other's does (ruling A2).
    pub channels: Vec<ChannelId>,
    /// The transport's effective payload limit.
    pub max_payload_bytes: usize,
    /// The identities held besides the first, in any order; the roster
    /// orders them. The first identity, or one given twice, is taken once.
    pub others: Vec<EndpointId>,
    /// The profile's default endpoint, if any: in front when the store
    /// names no identity held, if it is held itself.
    pub default: Option<EndpointId>,
}

/// Why a roster was not built.
#[derive(Debug)]
pub enum RosterError {
    /// The store could not be read: the pending rows a client reads at
    /// construction, or a card's creation time.
    Store(StoreError),
    /// More identities than a client holds ([`IDENTITY_CAP`]).
    OverBound,
}

impl From<StoreError> for RosterError {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}

/// What each identity answered, by identity, in the order they ran.
pub type Routed = Vec<(EndpointId, Vec<Update>)>;

/// What a change of the identity in front did.
#[derive(Debug)]
pub enum SwitchOutcome {
    /// `to` is in front already, or is not held: nothing changed, and the
    /// last-front row was not written.
    Unchanged,
    /// `to` is in front. `remembered` is the write of the last-front row:
    /// a failed write leaves the switch done and the next start in front
    /// of what the row said before.
    Switched {
        /// The row's write.
        remembered: Result<(), StoreError>,
    },
}

struct Held<B: SessionBinding, A: AdminBinding> {
    endpoint: EndpointId,
    side: FacadeSide<B, A>,
}

/// The identities a client holds, the first first.
pub struct Roster<B: SessionBinding, A: AdminBinding> {
    /// In held order; index 0 is the first identity.
    held: Vec<Held<B, A>>,
    front: EndpointId,
    default: Option<EndpointId>,
    /// A handle of the one open, for the client's own rows: the last
    /// front and the cards' creation.
    store: HumanStore,
    /// Why the last-front row was unreadable at start, kept beside the
    /// fallback it caused ([`Self::front_unread`]).
    front_unread: Option<StoreError>,
}

impl<B: SessionBinding, A: AdminBinding> Roster<B, A> {
    /// The roster over `store`, whose identity is the first identity, and
    /// the identities `config` names besides it. `bindings` gives each
    /// identity's client its bindings; `wall` is the wall clock in Unix
    /// milliseconds, `now` the caller's monotonic clock. Opens nothing: the
    /// first [`Self::turn`] does, in the open order.
    ///
    /// # Errors
    /// [`RosterError::OverBound`] for more than [`IDENTITY_CAP`]
    /// identities (`more_identities_than_the_bound_are_refused`), or the
    /// store's error.
    pub fn new<W>(
        config: RosterConfig,
        store: HumanStore,
        mut bindings: impl FnMut(&EndpointId) -> (B, A),
        wall: W,
        now: u64,
    ) -> Result<Self, RosterError>
    where
        W: Fn() -> u64 + Clone + Send + Sync + 'static,
    {
        let first = store.identity().clone();
        let mut others: Vec<EndpointId> = Vec::new();
        for endpoint in config.others {
            if endpoint != first && !others.contains(&endpoint) {
                others.push(endpoint);
            }
        }
        if others.len() + 1 > IDENTITY_CAP {
            return Err(RosterError::OverBound);
        }
        let own = store.handle_speaking_as(first.clone());
        let mut keyed = Vec::with_capacity(others.len());
        for endpoint in others {
            let created = own.own_card_created_at(&endpoint)?;
            keyed.push((created, endpoint));
        }
        // An identity with no card yet goes after those with one; with
        // none at all, after a discard, the id alone orders them.
        keyed.sort_by(|(a, x), (b, y)| {
            (a.is_none(), a, x.as_str()).cmp(&(b.is_none(), b, y.as_str()))
        });
        let mut held = Vec::with_capacity(keyed.len() + 1);
        let order = std::iter::once((first.clone(), store)).chain(keyed.into_iter().map(
            |(_, endpoint)| {
                let handle = own.handle_speaking_as(endpoint.clone());
                (endpoint, handle)
            },
        ));
        for (endpoint, handle) in order {
            let is_first = endpoint == first;
            let (binding, admin) = bindings(&endpoint);
            let client = TransportClient::new(
                binding,
                admin,
                handle,
                ClientConfig {
                    client_kind: config.client_kind.clone(),
                    endpoints: config.endpoints.clone(),
                    channels: if is_first {
                        config.channels.clone()
                    } else {
                        Vec::new()
                    },
                    max_payload_bytes: config.max_payload_bytes,
                },
                Box::new(wall.clone()),
                now,
            )?;
            let client = if is_first {
                client
            } else {
                client.without_status()
            };
            held.push(Held {
                endpoint,
                side: FacadeSide::new(client, wall.clone()),
            });
        }
        let mut roster = Self {
            held,
            front: first,
            default: config.default,
            store: own,
            front_unread: None,
        };
        // An unreadable row is read as no row, so a start is not refused
        // over it (`the_front_at_start_is_the_last_held_else_the_default_else_the_first`);
        // the error is kept for the root to report
        // ([`Self::front_unread`]).
        let last = match roster.store.last_front() {
            Ok(last) => last,
            Err(e) => {
                roster.front_unread = Some(e);
                None
            }
        };
        roster.front = roster.resolve(last);
        Ok(roster)
    }

    /// Why the last-front row could not be read at start, when it could
    /// not: the start went on without it (the default if held, else the
    /// first), and the root says so once.
    #[must_use]
    pub const fn front_unread(&self) -> Option<&StoreError> {
        self.front_unread.as_ref()
    }

    /// The identity in front at start: `last` when it is held, else the
    /// default when it is held, else the first.
    fn resolve(&self, last: Option<EndpointId>) -> EndpointId {
        last.filter(|e| self.holds(e))
            .or_else(|| self.default.clone().filter(|e| self.holds(e)))
            .unwrap_or_else(|| self.first().clone())
    }

    fn holds(&self, endpoint: &EndpointId) -> bool {
        self.held.iter().any(|h| &h.endpoint == endpoint)
    }

    fn index(&self, endpoint: &EndpointId) -> Option<usize> {
        self.held.iter().position(|h| &h.endpoint == endpoint)
    }

    /// The first identity.
    #[must_use]
    pub fn first(&self) -> &EndpointId {
        &self.held[0].endpoint
    }

    /// The identity in front.
    #[must_use]
    pub const fn front(&self) -> &EndpointId {
        &self.front
    }

    /// The identities held, in held order.
    pub fn held(&self) -> impl ExactSizeIterator<Item = &EndpointId> {
        self.held.iter().map(|h| &h.endpoint)
    }

    /// The identities in the order their sessions open and their turns
    /// run: the identity in front, then the default, then the rest in
    /// held order.
    #[must_use]
    pub fn open_order(&self) -> Vec<EndpointId> {
        self.order()
            .into_iter()
            .map(|i| self.held[i].endpoint.clone())
            .collect()
    }

    fn order(&self) -> Vec<usize> {
        let mut order = Vec::with_capacity(self.held.len());
        let lead = [Some(&self.front), self.default.as_ref()];
        for endpoint in lead.into_iter().flatten() {
            if let Some(i) = self.index(endpoint)
                && !order.contains(&i)
            {
                order.push(i);
            }
        }
        for i in 0..self.held.len() {
            if !order.contains(&i) {
                order.push(i);
            }
        }
        order
    }

    /// An identity's facade side.
    #[must_use]
    pub fn side(&self, endpoint: &EndpointId) -> Option<&FacadeSide<B, A>> {
        self.index(endpoint).map(|i| &self.held[i].side)
    }

    /// An identity's facade side, to start it or give it its key.
    pub fn side_mut(&mut self, endpoint: &EndpointId) -> Option<&mut FacadeSide<B, A>> {
        self.index(endpoint).map(|i| &mut self.held[i].side)
    }

    /// One turn of every identity, in the open order. The connectivity
    /// every identity is told is the first identity's: the others read no
    /// status.
    pub async fn turn(&mut self, now: u64) -> Routed {
        let mut routed = Vec::with_capacity(self.held.len());
        for i in self.order() {
            let updates = self.held[i].side.turn(now).await;
            routed.push((i, updates));
        }
        self.route(routed)
    }

    /// Carry out `command` as `identity`. `None` when it is not held.
    pub async fn execute(
        &mut self,
        identity: &EndpointId,
        command: Command,
        now: u64,
    ) -> Option<Routed> {
        let i = self.index(identity)?;
        let updates = self.held[i].side.execute(command, now).await;
        Some(self.route(vec![(i, updates)]))
    }

    /// Each identity's updates, the first's connectivity given to every
    /// identity held and any other's dropped. An identity that is not the
    /// first is built `without_status` and says `Unknown` once, at
    /// construction, which is not the client's connectivity.
    fn route(&self, ran: Vec<(usize, Vec<Update>)>) -> Routed {
        let is_connectivity =
            |u: &Update| matches!(u, Update::Client(ClientEvent::Connectivity(_)));
        let told: Vec<Update> = ran
            .iter()
            .filter(|(i, _)| *i == 0)
            .flat_map(|(_, updates)| updates.iter().filter(|u| is_connectivity(u)).cloned())
            .collect();
        let mut routed: Routed = Vec::with_capacity(self.held.len());
        for (i, mut updates) in ran {
            if i != 0 {
                updates.retain(|u| !is_connectivity(u));
            }
            routed.push((self.held[i].endpoint.clone(), updates));
        }
        if !told.is_empty() {
            for held in self.held.iter().skip(1) {
                match routed.iter_mut().find(|(e, _)| e == &held.endpoint) {
                    Some((_, updates)) => updates.extend(told.iter().cloned()),
                    None => routed.push((held.endpoint.clone(), told.clone())),
                }
            }
        }
        routed
    }

    /// Bring `to` in front and write the last-front row: the facade's
    /// half of a switch. The model side's half runs before it, on its own
    /// thread, and sends the old front's commands ahead of this
    /// ([`crate::ModelSide::switch`]); each command names the identity it
    /// runs as, whatever is in front when it arrives
    /// (`crossings_run_as_the_identity_they_name_whatever_is_in_front`;
    /// `set_front_swaps_and_writes_the_row_and_an_unchanged_front_writes_nothing`).
    pub fn set_front(&mut self, to: &EndpointId) -> SwitchOutcome {
        if !self.holds(to) || to == &self.front {
            return SwitchOutcome::Unchanged;
        }
        self.front = to.clone();
        SwitchOutcome::Switched {
            remembered: self.store.set_last_front(to),
        }
    }

    /// Close every identity's session: their leases are released.
    pub async fn close(&mut self) {
        for held in &mut self.held {
            held.side.close().await;
        }
    }
}
