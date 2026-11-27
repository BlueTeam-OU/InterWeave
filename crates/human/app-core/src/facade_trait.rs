// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The facade a root drives, as one trait: the roster a client composes,
//! or one facade side alone. Held here, not in a root, so another client
//! can name the same seam; the desktop does today. No tokio and no platform code.

use interweave_human_store::StoreError;
use interweave_human_transport_client::{AssertingBinding, SessionBinding};
use interweave_human_ui_model::HeldIdentity;
use interweave_local_client_api::AdminBinding;
use interweave_transport_api::EndpointId;

use crate::facade::FacadeSide;
use crate::protocol::{AddAct, AddTold, Command, Start};
use crate::roster::{Roster, Routed, SwitchOutcome};

/// What a root's facade thread drives: the roster a client composes
/// (the desktop's `ipc.rs`), or one facade side alone, as a test builds one. Each
/// answers by identity, so what crosses to the window names the identity
/// it is for.
pub trait Facade {
    /// The identities held, in held order.
    fn held(&self) -> Vec<EndpointId>;
    /// The identity in front.
    fn front(&self) -> EndpointId;
    /// Each identity's windowed start, in held order.
    fn start(&mut self) -> Vec<(EndpointId, Result<Start, StoreError>)>;
    /// One turn of every identity.
    fn turn(&mut self, now: u64) -> impl Future<Output = Routed>;
    /// Carry out `command` as `identity`; `None` when it is not held.
    fn execute(
        &mut self,
        identity: &EndpointId,
        command: Command,
        now: u64,
    ) -> impl Future<Output = Option<Routed>>;
    /// Bring `to` in front.
    fn set_front(&mut self, to: &EndpointId) -> SwitchOutcome;
    /// Close every session.
    fn close(&mut self) -> impl Future<Output = ()>;
    /// Whether the row naming the identity last in front could not be
    /// read, so the start fell back to another: said once, by class.
    fn front_unread(&self) -> bool {
        false
    }
    /// The held identities as the view is told them, when they changed
    /// since last told; `None` otherwise, and from a facade that names
    /// none.
    fn held_identities(&mut self) -> Option<Vec<HeldIdentity>> {
        None
    }
    /// The identities held since this was last asked, each with its
    /// windowed start: one made while the facade runs, held with no
    /// restart. None from a facade that holds no more than it was built
    /// with.
    fn take_held_more(&mut self) -> Vec<(EndpointId, Result<Start, StoreError>)> {
        Vec::new()
    }
    /// Carry out what the person did with the add form or the list of
    /// identities. A facade that cannot add does nothing and tells
    /// nothing, so no Create is offered.
    fn add(&mut self, act: AddAct, now: u64) {
        let _ = (act, now);
    }
    /// What came of the add form's acts and of the settles since last
    /// asked; none from a facade that cannot add.
    fn take_add_told(&mut self) -> Vec<AddTold> {
        Vec::new()
    }
}

impl<B: SessionBinding, A: AssertingBinding> Facade for Roster<B, A> {
    fn held(&self) -> Vec<EndpointId> {
        Roster::held(self).cloned().collect()
    }
    fn front(&self) -> EndpointId {
        Roster::front(self).clone()
    }
    fn start(&mut self) -> Vec<(EndpointId, Result<Start, StoreError>)> {
        let held = Facade::held(self);
        held.into_iter()
            .filter_map(|identity| {
                let start = self.side_mut(&identity)?.start();
                Some((identity, start))
            })
            .collect()
    }
    async fn turn(&mut self, now: u64) -> Routed {
        Roster::turn(self, now).await
    }
    async fn execute(
        &mut self,
        identity: &EndpointId,
        command: Command,
        now: u64,
    ) -> Option<Routed> {
        Roster::execute(self, identity, command, now).await
    }
    fn set_front(&mut self, to: &EndpointId) -> SwitchOutcome {
        Roster::set_front(self, to)
    }
    async fn close(&mut self) {
        Roster::close(self).await;
    }
    fn front_unread(&self) -> bool {
        Roster::front_unread(self).is_some()
    }
    fn held_identities(&mut self) -> Option<Vec<HeldIdentity>> {
        Roster::held_if_changed(self)
    }
    fn take_held_more(&mut self) -> Vec<(EndpointId, Result<Start, StoreError>)> {
        Roster::take_held_more(self)
    }
    fn add(&mut self, act: AddAct, now: u64) {
        Roster::add(self, act, now);
    }
    fn take_add_told(&mut self) -> Vec<AddTold> {
        Roster::take_add_told(self)
    }
}

/// One facade side alone: one identity, the one its store speaks as,
/// always in front.
impl<B: SessionBinding, A: AdminBinding> Facade for FacadeSide<B, A> {
    fn held(&self) -> Vec<EndpointId> {
        vec![self.speaks_as().clone()]
    }
    fn front(&self) -> EndpointId {
        self.speaks_as().clone()
    }
    fn start(&mut self) -> Vec<(EndpointId, Result<Start, StoreError>)> {
        vec![(self.speaks_as().clone(), FacadeSide::start(self))]
    }
    async fn turn(&mut self, now: u64) -> Routed {
        let updates = FacadeSide::turn(self, now).await;
        vec![(self.speaks_as().clone(), updates)]
    }
    async fn execute(
        &mut self,
        identity: &EndpointId,
        command: Command,
        now: u64,
    ) -> Option<Routed> {
        if identity != self.speaks_as() {
            return None;
        }
        let updates = FacadeSide::execute(self, command, now).await;
        Some(vec![(identity.clone(), updates)])
    }
    fn set_front(&mut self, _to: &EndpointId) -> SwitchOutcome {
        SwitchOutcome::Unchanged
    }
    async fn close(&mut self) {
        FacadeSide::close(self).await;
    }
}
