// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The endpoint set's methods (IPC 3.0, `LOCAL-IPC.md` §The endpoint set's
//! methods): `admin.endpoints.add`, `remove`, `set_advertise` and
//! `set_alias`, each one runtime function that the IPC binding and the
//! embedded host's in-process port both reach through the driver.
//!
//! Every write is the endpoint overlay's, through
//! [`Driver::set_endpoints_noting`]: the composed profile validated, the
//! overlay written whole and synced, then the change published to the
//! substrate, the previous overlay put back when the publish fails, and an
//! overlay that could not be put back left AHEAD and answered `Internal`.
//! A refusal is an answer, not an error, and writes nothing
//! (`tests/endpoint_set.rs`: each refusal compares the overlay's bytes
//! before and after it).

use interweave_local_client_api::{
    AliasChange, AliasRefusal, EndpointAddChange, EndpointAddRefusal, EndpointRemoveChange,
    EndpointRemoveRefusal, EndpointSource, Generation,
};
use interweave_profile_config::ClientKind;
use interweave_profile_config::endpoint_overlay::{
    self as overlay, Added, EndpointOverlay, EndpointOverlayError, Removed, Renamed,
};
use interweave_transport_api::{EndpointId, TransportError};
use interweave_transport_libp2p::runtime::DirectEndpoints;

use super::{AUDIT_TARGET, Driver};

const fn add_refusal(reason: overlay::AddRefusal) -> EndpointAddRefusal {
    match reason {
        overlay::AddRefusal::IdNotDerived => EndpointAddRefusal::IdNotDerived,
        overlay::AddRefusal::AliasInvalid => EndpointAddRefusal::AliasInvalid,
        overlay::AddRefusal::KeyMissing => EndpointAddRefusal::KeyMissing,
        overlay::AddRefusal::KeyMismatch => EndpointAddRefusal::KeyMismatch,
        overlay::AddRefusal::Exists => EndpointAddRefusal::Exists,
        overlay::AddRefusal::AliasTaken => EndpointAddRefusal::AliasTaken,
        overlay::AddRefusal::Bound => EndpointAddRefusal::Bound,
    }
}

const fn remove_refusal(reason: overlay::RemoveRefusal) -> EndpointRemoveRefusal {
    match reason {
        overlay::RemoveRefusal::Authored => EndpointRemoveRefusal::Authored,
        overlay::RemoveRefusal::Default => EndpointRemoveRefusal::Default,
    }
}

const fn alias_refusal(reason: overlay::AliasRefusal) -> AliasRefusal {
    match reason {
        overlay::AliasRefusal::Authored => AliasRefusal::Authored,
        overlay::AliasRefusal::AliasInvalid => AliasRefusal::AliasInvalid,
        overlay::AliasRefusal::AliasTaken => AliasRefusal::AliasTaken,
    }
}

/// The audit line of an add, a remove or a rename: the method, the
/// endpoint when one is known, the outcome and a refusal's reason, on the
/// trust record's target, as `admin.trust.set`'s (LOCAL-IPC.md: they
/// create and destroy an admission source).
fn audit(method: &'static str, endpoint: Option<&EndpointId>, outcome: &str, refused: &str) {
    tracing::info!(
        target: AUDIT_TARGET,
        endpoint = endpoint.map_or("", EndpointId::as_str),
        outcome,
        refused,
        "{method}"
    );
}

/// The outcome a write's answer is audited under, as the trust set's.
fn written(
    published: &Result<impl Sized, TransportError>,
    unwritten: bool,
    done: &'static str,
) -> &'static str {
    match published {
        Ok(_) => done,
        Err(TransportError::InvalidArgument) => "refused",
        Err(_) if unwritten => "unwritten",
        Err(_) => "failed",
    }
}

impl Driver {
    /// `admin.endpoints.add {key, alias, allowed_client_kinds}`: refused in
    /// the contract's order with nothing written, `unchanged` for a retry,
    /// else the entry and its two empty sets written, and the endpoint
    /// registered with its sets in one substrate step before the answer.
    ///
    /// `unchanged` writes nothing and STILL PUBLISHES the entry the
    /// overlay holds, as every set the overlay already records does
    /// (`Driver::set_endpoints`): an add left ahead -- written, its
    /// publish and its restore failed -- leaves the overlay holding an
    /// endpoint the runtime never registered, and the retry is what
    /// registers it. A registered endpoint keeps what it has (the
    /// registry's insert never replaces), and the sources re-apply the
    /// sets the overlay holds. `unchanged` is answered only once that
    /// publish succeeded (`a_retried_add_left_ahead_registers_the_endpoint`).
    pub(super) async fn add_endpoint(
        &mut self,
        key: String,
        alias: String,
        kinds: Vec<String>,
    ) -> Result<EndpointAddChange, TransportError> {
        const METHOD: &str = "admin.endpoints.add";
        let kinds = kinds
            .into_iter()
            .map(ClientKind::parse)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| TransportError::InvalidArgument)?;
        // Without a state directory (a test construction) the overlay
        // reads no key file: its order still decides every refusal.
        let added = self.endpoint_overlay.add(
            &self.authored.endpoints,
            self.profile_paths.as_ref(),
            &key,
            &alias,
            kinds,
        );
        let (moved, endpoint) = match added {
            Added::Refused {
                reason,
                endpoint,
                detail,
            } => {
                // The id the key's name derives, absent only when it
                // derives none (`endpoint_add_audit.rs`).
                let reason = add_refusal(reason);
                audit(METHOD, endpoint.as_ref(), "refused", reason.as_str());
                return Ok(EndpointAddChange::Refused { reason, detail });
            }
            Added::Unchanged(endpoint) => {
                let held = self.endpoint_overlay.clone();
                let published = match self.publish_added(&held, &endpoint) {
                    Ok(publish) => publish.await.map(|_| ()),
                    Err(refused) => Err(refused),
                };
                audit(
                    METHOD,
                    Some(&endpoint),
                    match &published {
                        Ok(()) => "unchanged",
                        Err(TransportError::InvalidArgument) => "refused",
                        Err(_) => "failed",
                    },
                    "",
                );
                published?;
                return Ok(EndpointAddChange::Unchanged(endpoint));
            }
            Added::Added { overlay, endpoint } => (overlay, endpoint),
        };
        let publish = match self.publish_added(&moved, &endpoint) {
            Ok(publish) => publish,
            Err(refused) => {
                audit(METHOD, Some(&endpoint), "refused", "");
                return Err(refused);
            }
        };
        let before = self.admission();
        let (published, unwritten) = self
            .set_endpoints_noting("add", Ok(Some(moved)), publish)
            .await;
        audit(
            METHOD,
            Some(&endpoint),
            written(&published, unwritten, "added"),
            "",
        );
        published?;
        self.trust_changed(before);
        Ok(EndpointAddChange::Added(endpoint))
    }

    /// The substrate step that registers the added `endpoint` as
    /// `overlay` holds it, with the trust sources `overlay` derives: the
    /// add's publish, and an `unchanged` retry's. `Internal` when the
    /// overlay does not compose the entry, `InvalidArgument` when its
    /// sources are past their bound.
    fn publish_added(
        &mut self,
        overlay: &EndpointOverlay,
        endpoint: &EndpointId,
    ) -> Result<impl Future<Output = Result<bool, TransportError>> + use<>, TransportError> {
        let entry = overlay
            .apply(&self.authored.endpoints)
            .entries
            .into_iter()
            .find(|e| &e.id == endpoint)
            .ok_or(TransportError::Internal)?;
        let sources = self.sources(self.trust.clone(), overlay)?;
        let registered = DirectEndpoints::registered(&entry, EndpointSource::Added);
        let fault = self.take_publish_fault();
        let commander = self.swarm.commander();
        let id = endpoint.clone();
        Ok(async move {
            if fault {
                return Err(TransportError::BackendUnavailable);
            }
            commander
                .add_endpoint(id, registered, sources)
                .await
                .map_err(|_| TransportError::BackendUnavailable)
        })
    }

    /// `admin.endpoints.remove {endpoint}`: refused `authored`, then
    /// `default`, nothing written; else the entry and every overlay member
    /// keyed by the id gone in one write, and the endpoint unregistered
    /// with the trust sources that no longer hold its sets in one
    /// substrate step -- its lease revoked, a peer no set holds closed.
    ///
    /// An endpoint the overlay holds and the runtime does not register --
    /// an add left ahead -- is removed too: the registry's not-registered
    /// is agreement, no lease to revoke and `revoked_epoch` absent, and
    /// the sources are still adopted, so the answer is `removed` and the
    /// next start does not bring the endpoint back
    /// (`a_remove_of_an_add_left_ahead_is_removed_and_gone_at_the_restart`).
    pub(super) async fn remove_endpoint(
        &mut self,
        endpoint: EndpointId,
    ) -> Result<EndpointRemoveChange, TransportError> {
        const METHOD: &str = "admin.endpoints.remove";
        let next = match self
            .endpoint_overlay
            .remove(&self.authored.endpoints, &endpoint)
        {
            Ok(Removed::Removed(next)) => next,
            Ok(Removed::Refused(reason)) => {
                let reason = remove_refusal(reason);
                audit(METHOD, Some(&endpoint), "refused", reason.as_str());
                return Ok(EndpointRemoveChange::Refused {
                    reason,
                    detail: match reason {
                        EndpointRemoveRefusal::Authored => {
                            "a config.yaml entry is turned off, not removed".to_owned()
                        }
                        EndpointRemoveRefusal::Default => {
                            "the default endpoint: move the default first".to_owned()
                        }
                    },
                });
            }
            Err(EndpointOverlayError::Unknown) => {
                audit(METHOD, Some(&endpoint), "unknown", "");
                return Err(TransportError::EndpointUnknown);
            }
            Err(_) => return Err(TransportError::Internal),
        };
        let sources = self.sources(self.trust.clone(), &next)?;
        let fault = self.take_publish_fault();
        let commander = self.swarm.commander();
        let id = endpoint.clone();
        let publish = async move {
            if fault {
                return Err(TransportError::BackendUnavailable);
            }
            match commander
                .remove_endpoint(id, sources.clone())
                .await
                .map_err(|_| TransportError::BackendUnavailable)?
            {
                Err(TransportError::EndpointUnknown) => commander
                    .set_trust(sources)
                    .await
                    .map(|_| None)
                    .map_err(|_| TransportError::BackendUnavailable),
                answer => answer,
            }
        };
        let before = self.admission();
        let (published, unwritten) = self
            .set_endpoints_noting("remove", Ok(Some(next)), publish)
            .await;
        audit(
            METHOD,
            Some(&endpoint),
            written(&published, unwritten, "removed"),
            "",
        );
        let revoked_epoch: Option<Generation> = published?;
        self.trust_changed(before);
        Ok(EndpointRemoveChange::Removed { revoked_epoch })
    }

    /// `admin.endpoints.set_advertise {endpoint, advertise}`: the overlay's
    /// delta or the added entry's member written, then the directory's
    /// answer source set, both before the answer. Not audited, as
    /// `set_enabled` is not.
    pub(super) async fn set_endpoint_advertise(
        &mut self,
        endpoint: EndpointId,
        advertise: bool,
    ) -> Result<(), TransportError> {
        let moved: Result<Option<EndpointOverlay>, EndpointOverlayError> = self
            .endpoint_overlay
            .set_advertise(&self.authored.endpoints, &endpoint, advertise);
        let fault = self.take_publish_fault();
        let commander = self.swarm.commander();
        let publish = async move {
            if fault {
                return Err(TransportError::BackendUnavailable);
            }
            commander
                .set_endpoint_advertise(endpoint, advertise)
                .await
                .map_err(|_| TransportError::BackendUnavailable)?
        };
        self.set_endpoints("set_advertise", moved, publish).await
    }

    /// `admin.endpoints.set_alias {endpoint, alias}`: refused `authored`,
    /// `alias_invalid`, `alias_taken` in that order, `unchanged` for the
    /// alias the entry has, else written and the runtime's list told.
    pub(super) async fn set_endpoint_alias(
        &mut self,
        endpoint: EndpointId,
        alias: String,
    ) -> Result<AliasChange, TransportError> {
        const METHOD: &str = "admin.endpoints.set_alias";
        let next =
            match self
                .endpoint_overlay
                .set_alias(&self.authored.endpoints, &endpoint, &alias)
            {
                Ok(Renamed::Renamed(next)) => next,
                Ok(Renamed::Unchanged) => {
                    audit(METHOD, Some(&endpoint), "unchanged", "");
                    return Ok(AliasChange::Unchanged);
                }
                Ok(Renamed::Refused { reason, detail }) => {
                    let reason = alias_refusal(reason);
                    audit(METHOD, Some(&endpoint), "refused", reason.as_str());
                    return Ok(AliasChange::Refused { reason, detail });
                }
                Err(EndpointOverlayError::Unknown) => {
                    audit(METHOD, Some(&endpoint), "unknown", "");
                    return Err(TransportError::EndpointUnknown);
                }
                Err(_) => return Err(TransportError::Internal),
            };
        let fault = self.take_publish_fault();
        let commander = self.swarm.commander();
        let id = endpoint.clone();
        let publish = async move {
            if fault {
                return Err(TransportError::BackendUnavailable);
            }
            commander
                .set_endpoint_alias(id, Some(alias))
                .await
                .map_err(|_| TransportError::BackendUnavailable)?
        };
        let (published, unwritten) = self
            .set_endpoints_noting("set_alias", Ok(Some(next)), publish)
            .await;
        audit(
            METHOD,
            Some(&endpoint),
            written(&published, unwritten, "renamed"),
            "",
        );
        published?;
        Ok(AliasChange::Renamed)
    }
}
