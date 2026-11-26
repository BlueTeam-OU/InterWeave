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
//!   (`set_front_swaps_and_writes_the_row_and_an_unchanged_front_writes_nothing`);
//! - an identity other than the first whose key cannot be opened is
//!   asserted and leased as the first is, and given no key in its place
//!   (`an_added_identity_without_its_key_is_leased_and_given_no_key`);
//! - every identity's client asks the FIRST identity's trust seed on each
//!   trust connection it opens, the first's alone, one record of whether
//!   one was answered shared by them all ([`Roster::first_seeded`];
//!   `every_identitys_trust_connection_seeds_the_first_alone`);
//! - what each identity's row says is told at start and when it changes
//!   (`the_held_identities_are_told_at_start_and_on_change_alone`), in
//!   identities §5.2's states
//!   (`each_session_state_is_told_as_its_identity_state`), a Try again's
//!   start kept until an attempt is answered
//!   (`a_try_again_reopens_its_identity_alone_and_is_told_starting`).
//!
//! It is the facade half: a model per identity is the model side's
//! ([`crate::ModelSide`]), which runs a switch's own steps on its thread
//! and sends the roster the new front after the old front's commands
//! ([`crate::ModelSide::switch`]). What the roster holds is what
//! the root hands it: the desktop hands it the profile's held list
//! ([`crate::held_list`]), and a roster of one behaves as the facade side
//! `main` composed (`a_roster_of_one_behaves_as_its_facade_side`).
//!
//! Turns and commands run one identity after another, each awaited before
//! the next begins: a trust or address act opens its own short admin
//! connection -- a trust act made as the identity in front, its endpoint
//! named -- so one client at a time asks
//! (`TransportClient::without_status`'s note). Each identity's client is
//! a graphical one (`TransportClient::new`), which asserts its endpoint
//! unadvertised before every lease; that assertion is polled at each of
//! its turns and never awaited, so one the daemon does not answer holds
//! no other identity's turn (transport-client's
//! `an_assertion_that_blocks_takes_no_lease_until_it_answers`).

use std::collections::{BTreeMap, BTreeSet};

use interweave_human_client_api::{ClientEvent, SessionProblem, SessionState};
use interweave_human_store::{HumanStore, StoreError};
use interweave_human_transport_client::{
    AssertingBinding, ClientConfig, FirstSeed, SessionBinding, TransportClient,
};
use interweave_human_ui_model::{HeldIdentity, IDENTITY_CAP, IdentityKey, IdentityState};
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
    /// Each held identity's alias, where its entry has one; an identity
    /// absent here has none, and is told with an empty alias, never one
    /// made up (identities §6.1).
    pub aliases: BTreeMap<EndpointId, String>,
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
    aliases: BTreeMap<EndpointId, String>,
    /// The identities a Try again re-opened, while that re-open runs:
    /// told `Starting { after_try_again: true }` (identities §5.2).
    tried_again: BTreeSet<EndpointId>,
    /// The held identities as last told ([`Self::held_if_changed`]).
    told: Option<Vec<HeldIdentity>>,
    /// Something ran that may change what is told: a turn with updates,
    /// or a command. Set at build, so the first ask tells.
    stale: bool,
    /// The first identity's seed, a clone given to every held client.
    seed: FirstSeed,
}

/// What an identity's row says it can do now (identities §5.2;
/// architect-cto's ruling B of 2026-11-26 and ux U4), from its session,
/// whether that session re-opens because the daemon had no room for it,
/// and whether a Try again started the re-open. The key's state is not
/// an input: an identity whose key cannot be used is leased as any
/// other, and its page names the key (`KeyProblem`, identity-key §4.2)
/// (`each_session_state_is_told_as_its_identity_state`).
pub(crate) const fn identity_state(
    session: &SessionState,
    for_room: bool,
    after_try_again: bool,
) -> IdentityState {
    match session {
        SessionState::Ready { .. } => IdentityState::Ready,
        // No room at the daemon: no words of its own and no Try again,
        // since the client already retries.
        SessionState::Reconnecting { .. } if for_room => IdentityState::Unavailable,
        SessionState::Reconnecting { .. } => IdentityState::Starting { after_try_again },
        SessionState::Refused {
            problem: SessionProblem::NotHidden,
        } => IdentityState::NotHidden,
        SessionState::Refused { .. } | SessionState::StorageDegraded | SessionState::Closed => {
            IdentityState::Unavailable
        }
    }
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
        A: AssertingBinding,
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
        // The first identity's, whichever identity a client speaks as
        // (ADR-0067 decision 6): an added identity is given its own set
        // at its add and is never seeded.
        let seed = FirstSeed::new(first.clone());
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
            let client = client.seeding(seed.clone());
            let client = if is_first {
                client
            } else {
                client.without_status()
            };
            let side = FacadeSide::new(client, wall.clone());
            held.push(Held { endpoint, side });
        }
        let mut roster = Self {
            held,
            front: first,
            default: config.default,
            store: own,
            front_unread: None,
            aliases: config.aliases,
            tried_again: BTreeSet::new(),
            told: None,
            stale: true,
            seed,
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

    /// Whether the first identity's trust seed was answered -- `seeded`
    /// or `present` -- on any client's trust connection in this process
    /// run: the first identity's set exists. Kept in memory only; a
    /// failed seed leaves it false.
    #[must_use]
    pub fn first_seeded(&self) -> bool {
        self.seed.answered()
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
            self.stale |= !updates.is_empty();
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
        let reopen = matches!(command, Command::Reopen);
        let updates = self.held[i].side.execute(command, now).await;
        if reopen
            && matches!(
                self.held[i].side.client().session_state(),
                SessionState::Reconnecting { .. }
            )
        {
            self.tried_again.insert(identity.clone());
        }
        self.stale = true;
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

    /// The held identities as the view is told them
    /// ([`interweave_human_ui_model::Identities::held_changed`]), in held
    /// order, when they may have changed since last told and did: `None`
    /// otherwise. Read again only after a turn that brought updates or a
    /// command, so an idle client reads and tells nothing
    /// (`the_held_identities_are_told_at_start_and_on_change_alone`).
    pub fn held_if_changed(&mut self) -> Option<Vec<HeldIdentity>> {
        if !std::mem::take(&mut self.stale) {
            return None;
        }
        let now = self.held_identities();
        if self.told.as_ref() == Some(&now) {
            return None;
        }
        self.told = Some(now.clone());
        Some(now)
    }

    /// Each held identity as its row reads it: its card's name and unread
    /// count from the store (a read that fails keeps what was last told),
    /// its alias, its state ([`identity_state`]), whether it is the first
    /// or the default, whether a message of it was not sent.
    fn held_identities(&mut self) -> Vec<HeldIdentity> {
        let mut out = Vec::with_capacity(self.held.len());
        for (i, held) in self.held.iter().enumerate() {
            let endpoint = &held.endpoint;
            let client = held.side.client();
            let session = client.session_state();
            if !matches!(session, SessionState::Reconnecting { .. }) {
                self.tried_again.remove(endpoint);
            }
            let key = IdentityKey::new(endpoint.as_str());
            let before = self
                .told
                .as_ref()
                .and_then(|told| told.iter().find(|h| h.key == key));
            let card_name = match self.store.own_card(endpoint) {
                Ok(card) => card.as_ref().and_then(crate::cards::chosen_card_name),
                Err(_) => before.and_then(|h| h.card_name.clone()),
            };
            let unread = match self
                .store
                .handle_speaking_as(endpoint.clone())
                .unread_counts()
            {
                Ok(counts) => {
                    u32::try_from(counts.iter().map(|c| c.unread).sum::<u64>()).unwrap_or(u32::MAX)
                }
                Err(_) => before.map_or(0, |h| h.unread),
            };
            out.push(HeldIdentity {
                key,
                card_name,
                alias: self.aliases.get(endpoint).cloned().unwrap_or_default(),
                state: identity_state(
                    session,
                    client.retrying_for_room(),
                    self.tried_again.contains(endpoint),
                ),
                is_first: i == 0,
                is_default: self.default.as_ref() == Some(endpoint),
                unread,
                not_sent: held.side.not_sent(),
            });
        }
        out
    }

    /// Close every identity's session: their leases are released.
    pub async fn close(&mut self) {
        for held in &mut self.held {
            held.side.close().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each session state is told as identities §5.2 names it: Ready
    /// ready; re-opening, Starting, after a Try again or not; re-opening
    /// because the daemon had no room, unavailable with no Try again;
    /// refused `NotHidden`, could not start; any other refusal, a store
    /// that cannot keep messages and a closed session, unavailable.
    /// Mutations: each arm moved to its neighbour's state --
    /// the row for it fails.
    #[test]
    fn each_session_state_is_told_as_its_identity_state() {
        let ready = SessionState::Ready { endpoint: None };
        let reconnecting = SessionState::Reconnecting {
            attempt: 1,
            next_at: 9,
        };
        let refused = |problem| SessionState::Refused { problem };
        let starting = |after_try_again| IdentityState::Starting { after_try_again };
        let cases = [
            (&ready, false, false, IdentityState::Ready),
            (&reconnecting, false, false, starting(false)),
            (&reconnecting, false, true, starting(true)),
            (&reconnecting, true, false, IdentityState::Unavailable),
            (&reconnecting, true, true, IdentityState::Unavailable),
            (
                &refused(SessionProblem::NotHidden),
                false,
                false,
                IdentityState::NotHidden,
            ),
            (
                &refused(SessionProblem::EndpointInUse),
                false,
                false,
                IdentityState::Unavailable,
            ),
            (
                &refused(SessionProblem::NotAvailableToThisClient),
                false,
                false,
                IdentityState::Unavailable,
            ),
            (
                &SessionState::StorageDegraded,
                false,
                false,
                IdentityState::Unavailable,
            ),
            (
                &SessionState::Closed,
                false,
                false,
                IdentityState::Unavailable,
            ),
        ];
        for (session, room, again, told) in cases {
            assert_eq!(
                identity_state(session, room, again),
                told,
                "{session:?} room {room} again {again}"
            );
        }
    }
}
