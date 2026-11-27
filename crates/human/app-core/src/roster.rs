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
//! - an identity a pending-add record names is not held, so no session
//!   leases it and no row is written under it
//!   (`an_id_a_pending_add_record_names_is_not_held_and_opens_no_session`);
//!   it becomes one at its record's delete and not before (ADR-0061
//!   decision 7);
//! - an identity made while the roster runs is held at once, with no
//!   restart, through the [`AddFactory`] a root gives it: its client is
//!   built as a held one is, so its unadvertised assertion runs before its
//!   lease, and it is told with the others
//!   (`an_identity_made_while_the_roster_runs_is_asserted_leased_and_told`);
//!   a roster given no factory holds no more, and the bound holds at
//!   runtime too (`a_roster_given_no_factory_holds_no_more`,
//!   `the_bound_holds_at_runtime_too`); one whose hold is refused after
//!   its record's delete is told not held and held at a later poll
//!   (`an_identity_whose_hold_is_refused_is_told_and_held_at_a_later_poll`);
//! - an add form's Create, its leaving and the settle of pending-add
//!   records run here, polled at each turn and never awaited
//!   ([`Roster::add`] has what each is held to and its test);
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
//! the root hands it, and what it holds later through the root's
//! [`AddFactory`]: the desktop hands it the profile's held list
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
use std::sync::Arc;

use interweave_human_client_api::{ClientEvent, SessionProblem, SessionState};
use interweave_human_store::identity_key::KeyCipher;
use interweave_human_store::{HumanStore, OwnCard, StoreError};
use interweave_human_transport_client::{
    AddOutcome, AssertingBinding, ClientConfig, FirstSeed, FormAdd, Local, NodePeer, SeedAsk,
    SessionBinding, Settle, Settled, TransportClient,
};
use interweave_human_ui_model::{HeldIdentity, IDENTITY_CAP, IdentityKey, IdentityState};
use interweave_local_client_api::AdminBinding;
use interweave_profile_config::paths::ProfilePaths;
use interweave_transport_api::{ChannelId, EndpointId};

use crate::facade::FacadeSide;
use crate::profile::KeySource;
use crate::protocol::{AddAct, AddTold, CardSaved, Command, Start, Update};

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

/// What gives a new identity's client its bindings.
pub type Bindings<B, A> = Box<dyn FnMut(&EndpointId) -> (B, A) + Send>;

/// What a roster needs to hold an identity made while it runs, given by
/// the root that composes it ([`Roster::adding`]): what `Roster::new`'s
/// closure gives each identity, the platform's key custody and where the
/// profile keeps its keys.
pub struct AddFactory<B, A> {
    /// A new identity's bindings, as `Roster::new`'s closure gives each
    /// held identity its own.
    pub bindings: Bindings<B, A>,
    /// The administrative binding an add form's and a settle's calls
    /// each open a connection of their own on.
    pub admin: Arc<A>,
    /// The profile whose `endpoint-keys/` holds each identity's key file.
    pub paths: ProfilePaths,
    /// The platform's key custody: the desktop's filesystem, Android's
    /// Keystore. An identity held later opens its key under it.
    pub cipher: Arc<dyn KeyCipher + Send + Sync>,
}

/// Why [`Roster::hold_added`] held nothing.
#[derive(Debug)]
pub enum HoldRefused {
    /// The roster was given no [`AddFactory`]: it holds what it was built
    /// with and no more.
    NoFactory,
    /// The identity is held already.
    Held,
    /// A pending-add record names it: it is no identity yet.
    Pending,
    /// As many identities are held as a client holds ([`IDENTITY_CAP`]).
    OverBound,
    /// The store could not be read: the pending-add record, or the
    /// pending rows a new client reads at construction.
    Store(StoreError),
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
    /// The node's `PeerId` cell, a clone given to every held client.
    node_peer: NodePeer,
    /// What every client is built with, kept for an identity held later.
    client_kind: String,
    /// The endpoints a client may lease: those given, and each identity
    /// held later.
    endpoints: Vec<EndpointId>,
    max_payload_bytes: usize,
    wall: Arc<dyn Fn() -> u64 + Send + Sync>,
    /// What holds an identity made while the roster runs; with none, the
    /// roster holds what it was built with and no more.
    factory: Option<AddFactory<B, A>>,
    /// The identities held since the root last took them
    /// ([`Self::take_held_more`]).
    held_more: Vec<EndpointId>,
    /// The add form, the settle, and what is told of them.
    adding: Adding<A>,
}

/// The add form open now, the settle running, and what is told of them
/// ([`Roster::add`]).
struct Adding<A> {
    /// One form at a time.
    form: Option<OpenForm<A>>,
    settle: Option<Settle<A>>,
    /// A settle moment came, and no settle started since.
    owed: bool,
    /// The first turn ran: its settle moment is past.
    started: bool,
    /// The identities a Create made or a settle found whose hold was
    /// refused after their record was deleted, with their alias: held
    /// at a later poll. At most [`IDENTITY_CAP`]: no more can be held.
    unheld: Vec<(EndpointId, Option<String>)>,
    told: Vec<AddTold>,
}

impl<A> Adding<A> {
    const fn new() -> Self {
        Self {
            form: None,
            settle: None,
            owed: false,
            started: false,
            unheld: Vec::new(),
            told: Vec::new(),
        }
    }

    /// Queue `endpoint` for a later hold. Whether it is queued: an id
    /// already queued is, one that finds the queue at [`IDENTITY_CAP`]
    /// is not (`the_unheld_queue_stops_at_the_cap_and_says_so`).
    fn queue_unheld(&mut self, endpoint: EndpointId, alias: Option<String>) -> bool {
        if self.unheld.iter().any(|(e, _)| *e == endpoint) {
            return true;
        }
        if self.unheld.len() < IDENTITY_CAP {
            self.unheld.push((endpoint, alias));
            return true;
        }
        false
    }
}

/// What [`Roster::hold_again`] came to.
#[derive(Debug, PartialEq, Eq)]
enum HoldOutcome {
    /// Held now, or held already.
    Held,
    /// Refused and queued for a later poll.
    Queued,
    /// Refused and not queued: the queue is at [`IDENTITY_CAP`].
    Dropped,
}

/// An open add form: its add, the card of its running Create, and its
/// seed ask while one runs.
struct OpenForm<A> {
    add: FormAdd<A>,
    card: Option<NewCard>,
    seed: Option<SeedAsk>,
}

/// The card a Create gives the identity it makes.
struct NewCard {
    name: String,
    surname: Option<String>,
    photo: Option<Vec<u8>>,
}

/// The identities a settle found.
fn identities(settled: Vec<Settled>) -> impl Iterator<Item = (EndpointId, Option<String>)> {
    settled.into_iter().filter_map(|s| match s {
        Settled::Identity { endpoint, alias } => Some((endpoint, alias)),
        Settled::Discarded(_) | Settled::Gone(_) | Settled::Kept { .. } => None,
    })
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
    /// the identities `config` names besides it but those a pending-add
    /// record names: such an id is no identity yet, so it is not held, no
    /// session leases it and nothing is written under it (ADR-0061
    /// decision 7). `bindings` gives each
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
        // Read before anything is built: a record that cannot be read
        // refuses the start rather than holding an id it may name.
        let pending = store.pending_adds()?;
        let mut others: Vec<EndpointId> = Vec::new();
        for endpoint in config.others {
            if endpoint != first && !others.contains(&endpoint) && !pending.contains(&endpoint) {
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
        // One node, so one PeerId: an added identity whose own open was
        // refused still names it (`a_refused_identity_reads_the_node_peer_from_its_sibling`).
        let node_peer = NodePeer::new();
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
            let client = client
                .seeding(seed.clone())
                .sharing_node_peer(node_peer.clone());
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
            node_peer,
            client_kind: config.client_kind,
            endpoints: config.endpoints,
            max_payload_bytes: config.max_payload_bytes,
            wall: Arc::new(wall),
            factory: None,
            held_more: Vec::new(),
            adding: Adding::new(),
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

    /// The roster, able to hold an identity made while it runs
    /// ([`Self::hold_added`]) with what `factory` supplies.
    #[must_use]
    pub fn adding(mut self, factory: AddFactory<B, A>) -> Self {
        self.factory = Some(factory);
        self
    }

    /// Hold `endpoint`, made while the roster runs -- by an add form's
    /// Create, or found by a settle -- at once and with no restart, named
    /// by `alias` until its card is read. Its client is built as a held
    /// identity's is ([`TransportClient::new`], `without_status`, the
    /// first's seed, the node's `PeerId` cell, no channel), so its
    /// unadvertised assertion runs before its lease; its key is opened
    /// under the factory's custody, and one that cannot be is leased as
    /// any other with no key. It is held last, and the root takes it with
    /// [`Self::take_held_more`]. The identity in front does not change.
    ///
    /// # Errors
    /// [`HoldRefused`]: no factory, held already, named by a pending-add
    /// record, the bound reached, or the store unread. Nothing is held.
    pub fn hold_added(
        &mut self,
        endpoint: EndpointId,
        alias: Option<String>,
    ) -> Result<(), HoldRefused>
    where
        A: AssertingBinding,
    {
        if self.holds(&endpoint) {
            return Err(HoldRefused::Held);
        }
        if self
            .store
            .pending_adds()
            .map_err(HoldRefused::Store)?
            .contains(&endpoint)
        {
            return Err(HoldRefused::Pending);
        }
        if self.held.len() + 1 > IDENTITY_CAP {
            return Err(HoldRefused::OverBound);
        }
        let Some(factory) = self.factory.as_mut() else {
            return Err(HoldRefused::NoFactory);
        };
        let mut endpoints = self.endpoints.clone();
        if !endpoints.contains(&endpoint) {
            endpoints.push(endpoint.clone());
        }
        let (binding, admin) = (factory.bindings)(&endpoint);
        let wall = Arc::clone(&self.wall);
        let client = TransportClient::new(
            binding,
            admin,
            self.store.handle_speaking_as(endpoint.clone()),
            ClientConfig {
                client_kind: self.client_kind.clone(),
                endpoints: endpoints.clone(),
                channels: Vec::new(),
                max_payload_bytes: self.max_payload_bytes,
            },
            Box::new(move || wall()),
            0,
        )
        .map_err(HoldRefused::Store)?
        .seeding(self.seed.clone())
        .sharing_node_peer(self.node_peer.clone())
        .without_status();
        let wall = Arc::clone(&self.wall);
        let mut side = FacadeSide::new(client, move || wall());
        side.set_key_source(KeySource::new(
            factory.paths.clone(),
            endpoint.clone(),
            Box::new(Arc::clone(&factory.cipher)),
        ));
        self.endpoints = endpoints;
        if let Some(alias) = alias {
            self.aliases.insert(endpoint.clone(), alias);
        }
        self.held.push(Held {
            endpoint: endpoint.clone(),
            side,
        });
        self.held_more.push(endpoint);
        self.stale = true;
        Ok(())
    }

    /// The identities held since this was last asked
    /// ([`Self::hold_added`]), in the order held, each with its windowed
    /// start ([`FacadeSide::start`]): what the root names to its model
    /// side ([`crate::ModelSide::hold_more`]) and applies there.
    pub fn take_held_more(&mut self) -> Vec<(EndpointId, Result<Start, StoreError>)> {
        let added = std::mem::take(&mut self.held_more);
        added
            .into_iter()
            .filter_map(|endpoint| {
                let start = self.side_mut(&endpoint)?.start();
                Some((endpoint, start))
            })
            .collect()
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
    /// status. Then an add form's Create, its seed ask and the settle go
    /// as far as they go without waiting ([`Self::add`]); the first turn
    /// is a settle moment.
    pub async fn turn(&mut self, now: u64) -> Routed
    where
        A: AssertingBinding,
    {
        if !self.adding.started {
            self.adding.started = true;
            self.adding.owed = true;
        }
        let mut routed = Vec::with_capacity(self.held.len());
        for i in self.order() {
            let updates = self.held[i].side.turn(now).await;
            self.stale |= !updates.is_empty();
            routed.push((i, updates));
        }
        self.poll_adding(now);
        self.route(routed)
    }

    /// Carry out what the person did with the add form or the list of
    /// identities (`design/2026-11-04-identities.md` §6.2; ADR-0061
    /// decision 7; architect-cto's rulings of 2026-11-27), then go as far
    /// as it goes without waiting; what came of it is
    /// [`Self::take_add_told`]'s. A roster given no [`AddFactory`] does
    /// nothing and tells nothing, so no Create is ever offered.
    ///
    /// - [`AddAct::Opened`] opens one form, its key not made yet. Create
    ///   is offered once a first-identity seed answered in this run
    ///   ([`Self::first_seeded`]); until then the form asks for the seed
    ///   on a connection of its own, waiting at most `ADD_WAIT_MS`, and
    ///   the answer is the run's
    ///   (`add_is_offered_only_once_the_first_seed_answered`).
    /// - [`AddAct::Create`] runs the form's add with the card held to a
    ///   card's bounds, never while Create is not offered. An identity it
    ///   makes is given its card -- one that cannot be written still ends
    ///   Created, marked not saved -- and held at once, the front
    ///   unchanged
    ///   (`a_create_that_lands_deletes_the_record_holds_the_identity_and_writes_its_card`,
    ///   `a_card_that_cannot_be_written_still_ends_created_with_the_card_not_saved`).
    ///   Nothing is written under its id while a record names it
    ///   (`while_a_record_names_the_id_nothing_is_written_under_it`), and
    ///   the add is polled, never awaited, so it holds no identity's turn
    ///   (`a_create_never_holds_another_identitys_turn`). A Create that
    ///   ends Created closes the form, Done being a way of leaving it: a
    ///   settle moment, and the next [`AddAct::Opened`] opens a new form
    ///   with a key of its own
    ///   (`done_closes_the_form_and_the_next_opens_a_new_one`). A Create
    ///   that does not start -- no form open, or one running -- is told
    ///   [`AddTold::NotStarted`].
    /// - [`AddAct::Left`] leaves the form and [`AddAct::ListOpened`] is
    ///   the list's opening: each a settle moment, as the first turn is
    ///   and a record written at the bound is (that one inside the form's
    ///   add) (`the_settle_runs_at_the_first_turn_at_the_lists_opening_and_at_the_bound`).
    ///   A settle covers every record but the open form's, so a form's
    ///   leaving settles its own
    ///   (`a_forms_closing_settles_its_own_record_with_one_identity_held`),
    ///   and an identity it finds is held at once
    ///   (`an_identity_the_start_settle_finds_is_held_without_a_restart`).
    pub fn add(&mut self, act: AddAct, now: u64)
    where
        A: AssertingBinding,
    {
        let Some(factory) = self.factory.as_ref() else {
            return;
        };
        match act {
            AddAct::Opened => {
                let form = self.adding.form.get_or_insert_with(|| OpenForm {
                    add: FormAdd::new(
                        Arc::clone(&factory.admin),
                        vec![self.client_kind.clone()],
                        crate::alias::alias_for,
                    ),
                    card: None,
                    seed: None,
                });
                if self.seed.answered() {
                    self.adding.told.push(AddTold::Offered);
                } else {
                    form.seed
                        .get_or_insert_with(|| SeedAsk::new(&factory.admin, &self.seed, now));
                    self.adding.told.push(AddTold::WaitsSeed);
                }
            }
            AddAct::Create {
                name,
                surname,
                photo,
            } => {
                let Some(form) = self.adding.form.as_mut() else {
                    self.adding.told.push(AddTold::NotStarted);
                    return;
                };
                if !self.seed.answered() {
                    self.adding.told.push(if form.seed.is_some() {
                        AddTold::WaitsSeed
                    } else {
                        AddTold::NoSeed
                    });
                    return;
                }
                let draft = interweave_human_chat_protocol::CardDraft {
                    name: &name,
                    surname: surname.as_deref(),
                    photo: photo.as_deref().map(|p| ("image/jpeg", p)),
                };
                if interweave_human_chat_protocol::check_draft(draft).is_err() {
                    self.adding.told.push(AddTold::OutOfBounds);
                    return;
                }
                if form.add.create(&name, &[]) {
                    form.card = Some(NewCard {
                        name,
                        surname,
                        photo,
                    });
                } else {
                    self.adding.told.push(AddTold::NotStarted);
                }
            }
            AddAct::Left => {
                let Some(form) = self.adding.form.take() else {
                    return;
                };
                let mut local = Local {
                    store: &mut self.store,
                    paths: &factory.paths,
                    cipher: &*factory.cipher,
                };
                // Whatever leaving did, the settle that follows decides
                // what it kept.
                let _ = form.add.leave(&mut local);
                self.adding.owed = true;
            }
            AddAct::ListOpened => self.adding.owed = true,
        }
        self.poll_adding(now);
    }

    /// What came of the add form's acts and of the settles since this
    /// was last asked, in order.
    pub fn take_add_told(&mut self) -> Vec<AddTold> {
        std::mem::take(&mut self.adding.told)
    }

    /// The seed ask, the running Create and the settle, as far as each
    /// goes without waiting; a settle owed starts when none runs.
    fn poll_adding(&mut self, now: u64)
    where
        A: AssertingBinding,
    {
        if self.factory.is_none() {
            return;
        }
        for (endpoint, alias) in std::mem::take(&mut self.adding.unheld) {
            self.hold_again(endpoint, alias);
        }
        let Some(factory) = self.factory.as_ref() else {
            return;
        };
        let mut found: Vec<(EndpointId, Option<String>)> = Vec::new();
        let mut ended: Option<(AddOutcome, Option<NewCard>)> = None;
        let mut local = Local {
            store: &mut self.store,
            paths: &factory.paths,
            cipher: &*factory.cipher,
        };
        if let Some(form) = self.adding.form.as_mut() {
            if let Some(ask) = form.seed.as_mut()
                && let Some(answered) = ask.poll(now)
            {
                form.seed = None;
                self.adding.told.push(if answered {
                    AddTold::Offered
                } else {
                    AddTold::NoSeed
                });
            }
            if let Some(end) = form.add.poll(now, &mut local) {
                found.extend(identities(end.settled));
                ended = Some((end.outcome, form.card.take()));
            }
        }
        if self.adding.settle.is_none() && self.adding.owed {
            self.adding.owed = false;
            let open: Vec<EndpointId> = self
                .adding
                .form
                .as_ref()
                .and_then(|f| f.add.endpoint().cloned())
                .into_iter()
                .collect();
            // A record that cannot be read starts no settle: nothing is
            // touched, and the next moment reads it again.
            if let Ok(settle) = Settle::of(&factory.admin, &local, &open) {
                self.adding.settle = Some(settle);
            }
        }
        if let Some(settle) = self.adding.settle.as_mut()
            && let Some(settled) = settle.poll(now, &mut local)
        {
            self.adding.settle = None;
            found.extend(identities(settled));
        }
        if let Some((outcome, card)) = ended {
            self.ended(outcome, card);
        }
        for (endpoint, alias) in found {
            self.hold_told(endpoint, alias);
        }
    }

    /// A Create ended: one that made its identity writes its card, then
    /// holds it.
    fn ended(&mut self, outcome: AddOutcome, card: Option<NewCard>)
    where
        A: AssertingBinding,
    {
        let AddOutcome::Created { endpoint, alias } = &outcome else {
            self.adding.told.push(AddTold::Ended {
                outcome,
                card: None,
            });
            return;
        };
        let (endpoint, alias) = (endpoint.clone(), alias.clone());
        // Done is a way of leaving the form (identities §6.2, contact
        // cards §5.2): it closes, which is a settle moment, and the next
        // opening makes a new form and a new key. A done form kept open
        // would start no Create again, and tell nothing of one.
        self.adding.form = None;
        self.adding.owed = true;
        // The record is deleted before `Created` is given, so this is the
        // first row written under the id.
        let saved = card.is_some_and(|card| {
            self.store
                .set_own_card(
                    &OwnCard {
                        endpoint: endpoint.clone(),
                        name: card.name,
                        surname: card.surname,
                        photo: card.photo,
                    },
                    (self.wall)(),
                )
                .is_ok()
        });
        self.adding.told.push(AddTold::Ended {
            outcome,
            card: Some(if saved {
                CardSaved::Saved
            } else {
                CardSaved::NotSaved
            }),
        });
        self.hold_told(endpoint, alias);
    }

    /// Hold an identity a Create made or a settle found, and tell it.
    /// Its record is deleted already, so a refusal -- the store unread,
    /// the bound -- is told [`AddTold::NotHeld`] and the hold is tried
    /// again at each later poll until it is held
    /// (`an_identity_whose_hold_is_refused_is_told_and_held_at_a_later_poll`);
    /// one the full queue cannot take is told
    /// [`AddTold::NotHeldUntilRestart`] and never tried again.
    fn hold_told(&mut self, endpoint: EndpointId, alias: Option<String>)
    where
        A: AssertingBinding,
    {
        match self.hold_again(endpoint.clone(), alias) {
            HoldOutcome::Held => {}
            HoldOutcome::Queued => self.adding.told.push(AddTold::NotHeld(endpoint)),
            HoldOutcome::Dropped => self
                .adding
                .told
                .push(AddTold::NotHeldUntilRestart(endpoint)),
        }
    }

    /// Try to hold `endpoint` and tell it held; refused, queue it for the
    /// next poll. What it returns: `Held` when it was held or already is,
    /// `Queued` when refused and queued, `Dropped` when refused and the
    /// queue is full, so no later poll will try it.
    fn hold_again(&mut self, endpoint: EndpointId, alias: Option<String>) -> HoldOutcome
    where
        A: AssertingBinding,
    {
        match self.hold_added(endpoint.clone(), alias.clone()) {
            Ok(()) => {
                self.adding.told.push(AddTold::Held(endpoint));
                HoldOutcome::Held
            }
            Err(HoldRefused::Held) => HoldOutcome::Held,
            Err(_) => {
                if self.adding.queue_unheld(endpoint, alias) {
                    HoldOutcome::Queued
                } else {
                    HoldOutcome::Dropped
                }
            }
        }
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

    fn id(n: u8) -> EndpointId {
        EndpointId::from_ed25519_public(&[n; 32])
    }

    /// The queue of unheld identities takes at most [`IDENTITY_CAP`], says
    /// when it did not, and counts an id it already holds as queued.
    /// Mutations: `<` to `<=` -- one over the cap is queued and the
    /// length assert fails; the full queue answering queued -- the last
    /// assert fails; a repeat pushed again -- the length assert fails.
    #[test]
    fn the_unheld_queue_stops_at_the_cap_and_says_so() {
        let mut adding = Adding::<()>::new();
        for n in 0..IDENTITY_CAP {
            let n = u8::try_from(n).expect("a small cap");
            assert!(adding.queue_unheld(id(n), None), "room for {n}");
        }
        assert_eq!(adding.unheld.len(), IDENTITY_CAP);
        assert!(adding.queue_unheld(id(0), None), "a repeat is queued");
        assert_eq!(adding.unheld.len(), IDENTITY_CAP, "a repeat adds none");
        let over = u8::try_from(IDENTITY_CAP).expect("a small cap");
        assert!(!adding.queue_unheld(id(over), None), "no room");
        assert_eq!(adding.unheld.len(), IDENTITY_CAP, "nothing past the cap");
    }

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
