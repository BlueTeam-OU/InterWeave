// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The admin commands: one method against the profile's running daemon,
//! over its admin socket (plan §16 (10)).
//!
//! An endpoint is named on the command line by its id or its alias. The
//! wire takes the id alone, so an alias is resolved here, before anything
//! is sent, over the profile's `config.yaml` and its endpoint overlay as
//! the configuration's own readers compose them (ENDPOINTS.md section
//! Configured endpoint set, A 2026-11-19): `load_document`, which reads
//! no file under `endpoint-keys/`, and the overlay's read-only
//! `read_within`, which writes nothing back (A 2026-11-21). An id is sent
//! as typed, the daemon judging it, so it needs no readable `config.yaml`.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use interweave_ipc_client::{IpcAdmin, IpcBinding, SocketPaths};
use interweave_local_client_api::{
    AddressChange, AddressSource, AdminBinding as _, AdminCapability, AdminPort as _, TrustChange,
    TrustDirection, TrustSource,
};
use interweave_profile_config::endpoint_overlay::EndpointOverlay;
use interweave_profile_config::{
    EndpointAlias, EndpointSelector, ProfileConfig, ProfileLock, ProfilePaths, XdgRoots,
    resolve_endpoint,
};
use interweave_transport_api::{EndpointId, TransportError};

use crate::Failure;
use crate::cli::Admin;

/// The client kind this tool names itself on the admin socket. Not an
/// authority: on the data socket it buys nothing (ADR-0037).
const CLIENT_KIND: &str = "transportctl";

/// Run `action` against `profile`'s daemon; what to print on success.
///
/// # Errors
/// [`Failure::Unreachable`] when no daemon answers on the admin socket,
/// told apart by the profile lock; [`Failure::Refused`] with the error
/// code when the daemon refuses.
pub(crate) async fn run(profile: &str, action: Admin, json: bool) -> Result<String, Failure> {
    let refused = |what: &str, e: &dyn std::fmt::Display| Failure::Refused(format!("{what}: {e}"));
    let roots = XdgRoots::from_env().map_err(|e| refused("the XDG directories", &e))?;
    let paths = ProfilePaths::resolve(profile, &roots).map_err(crate::profile_refusal)?;
    let sockets = SocketPaths {
        data: paths
            .data_socket()
            .map_err(|e| refused("the data socket", &e))?,
        admin: paths
            .admin_socket()
            .map_err(|e| refused("the admin socket", &e))?,
    };
    // Resolved before the connection: an alias that names nothing is
    // refused with nothing sent.
    let write = resolve_write(&paths, &action)?;
    let trust_endpoint = match &action {
        Admin::TrustList(Some(endpoint)) | Admin::SetTrust(_, _, Some(endpoint)) => {
            Some(resolve(&paths, endpoint)?)
        }
        _ => None,
    };
    let binding = IpcBinding::new(sockets.clone(), CLIENT_KIND);
    let capability = match action {
        Admin::Status | Admin::PeersList => AdminCapability::Status,
        Admin::Shutdown(_) => AdminCapability::Shutdown,
        Admin::TrustList(_) | Admin::SetTrust(..) => AdminCapability::Trust,
        Admin::AddAddress(_)
        | Admin::RemoveAddress(_)
        | Admin::ListAddresses
        | Admin::ShowAddresses => AdminCapability::Peers,
        _ => AdminCapability::Endpoints,
    };
    let port = match binding.admin([capability].into()).await {
        Ok(port) => port,
        Err(TransportError::BackendUnavailable) => {
            return Err(unreachable(&paths, &sockets));
        }
        Err(e) => return Err(code(e)),
    };
    // A 2.0 daemon is not asked for a 2.1 capability, so the port comes
    // back without it: said here, rather than as the refusal a request
    // would draw.
    if !port.port().holds(capability) {
        let minor = if capability == AdminCapability::Peers {
            "2.7"
        } else {
            "2.1"
        };
        return Err(Failure::Refused(format!(
            "the daemon does not grant {capability:?} (it is IPC {minor}'s: is the daemon older?)"
        )));
    }
    if let Some(write) = write {
        return endpoint_write(&port, write).await.map_err(code);
    }
    // A REFUSED MOVE EXITS AS REFUSED, naming the daemon's rule, so a
    // script cannot read "nothing changed" as success.
    if let Admin::AddAddress(address) | Admin::RemoveAddress(address) = &action {
        let change = if matches!(action, Admin::AddAddress(_)) {
            port.add_bootstrap_address(address.clone()).await
        } else {
            port.remove_bootstrap_address(address.clone()).await
        };
        return match change.map_err(code)? {
            AddressChange::Added => Ok("added\n".to_owned()),
            AddressChange::Removed => Ok("removed\n".to_owned()),
            AddressChange::Unchanged => Ok("unchanged\n".to_owned()),
            AddressChange::Refused { reason, detail } => Err(Failure::Refused(format!(
                "refused {}: {detail}",
                reason.as_str()
            ))),
        };
    }
    // And so does a set refused as superseded: another admin
    // connection's later set for the peer stands (LOCAL-IPC.md
    // §Cancellation mapping and request concurrency).
    if let Admin::SetTrust(peer, allowed, _) = &action {
        // For an endpoint, both directions of its own set: the method has
        // no way to write one alone (LOCAL-IPC.md, The endpoint dimension).
        let change = match &trust_endpoint {
            Some(endpoint) => {
                port.set_endpoint_trust(endpoint.clone(), peer.clone(), *allowed)
                    .await
            }
            None => port.set_trust(peer.clone(), *allowed).await,
        };
        return match change.map_err(code)? {
            TrustChange::Set => Ok(format!(
                "{} {}\n",
                if *allowed { "allowed" } else { "revoked" },
                peer.as_str()
            )),
            TrustChange::Refused(reason) => Err(Failure::Refused(format!(
                "refused {}: a later admin connection's set for this peer stands",
                reason.as_str()
            ))),
        };
    }
    if let Some(endpoint) = &trust_endpoint {
        return endpoint_trust_list(&port, endpoint, json)
            .await
            .map_err(code);
    }
    let peers = action == Admin::PeersList;
    let show = action == Admin::ShowAddresses;
    call(&port, &paths, action, json)
        .await
        .map_err(|e| match e {
            // The port refuses a method the connection's version does not
            // have, before any request (`IpcAdmin::speaks`): every one is a
            // 3.0 method, so this is said for a minor that brings a method
            // to come, never for one today.
            TransportError::ProtocolUnsupported if show => Failure::Refused(
                "the daemon does not speak admin.peers.own_addresses: is the daemon older?"
                    .to_owned(),
            ),
            TransportError::ProtocolUnsupported if peers => Failure::Refused(
                "the daemon does not speak admin.peers.list: is the daemon older?".to_owned(),
            ),
            e => code(e),
        })
}

/// An endpoint write with its endpoint resolved to the id the wire takes.
enum EndpointWrite {
    Revoke(EndpointId),
    SetEnabled(EndpointId, bool),
    SetDefault(Option<EndpointId>),
}

/// `action`'s endpoint write, its endpoint resolved; `None` for any other
/// action.
fn resolve_write(paths: &ProfilePaths, action: &Admin) -> Result<Option<EndpointWrite>, Failure> {
    Ok(Some(match action {
        Admin::Revoke(endpoint) => EndpointWrite::Revoke(resolve(paths, endpoint)?),
        Admin::SetEnabled(endpoint, enabled) => {
            EndpointWrite::SetEnabled(resolve(paths, endpoint)?, *enabled)
        }
        Admin::SetDefault(endpoint) => EndpointWrite::SetDefault(
            endpoint
                .as_ref()
                .map(|endpoint| resolve(paths, endpoint))
                .transpose()?,
        ),
        _ => return Ok(None),
    }))
}

/// The id `selector` names: an id as typed, an alias through
/// [`resolve_endpoint`] over the composed configuration. An alias no
/// entry carries, or one given against a configuration that does not
/// load, is refused naming the alias.
fn resolve(paths: &ProfilePaths, selector: &EndpointSelector) -> Result<EndpointId, Failure> {
    let alias = match selector {
        EndpointSelector::Id(id) => return Ok(id.clone()),
        EndpointSelector::Alias(alias) => alias,
    };
    let (config, overlay) = composed(paths).map_err(|why| {
        Failure::Refused(format!(
            "the alias '{alias}' cannot be resolved: {why}; nothing was sent"
        ))
    })?;
    resolve_endpoint(&config.endpoints, &overlay, alias.as_str())
        .map_err(|e| Failure::Refused(format!("{e}; nothing was sent")))
}

/// The profile's configuration, read without its keys, and its endpoint
/// overlay, read without a write back: what an alias is resolved over.
fn composed(paths: &ProfilePaths) -> Result<(ProfileConfig, EndpointOverlay), String> {
    let config = ProfileConfig::load_document(paths)
        .map_err(|e| format!("the profile {}: {e}", paths.config_file().display()))?;
    let (overlay, _) = EndpointOverlay::read_within(
        &EndpointOverlay::path_for(paths),
        &config.endpoints,
        paths.boundary(),
    )
    .map_err(|e| format!("the endpoint overlay: {e}"))?;
    Ok((config, overlay))
}

/// Each configured endpoint's alias, `None` for an entry that carries
/// none, over the composed configuration.
fn aliases(paths: &ProfilePaths) -> Result<BTreeMap<EndpointId, Option<EndpointAlias>>, String> {
    let (config, overlay) = composed(paths)?;
    Ok(overlay
        .apply(&config.endpoints)
        .entries
        .into_iter()
        .map(|entry| (entry.id, entry.alias))
        .collect())
}

/// How a listed endpoint's alias is shown: the alias, `-` for an entry
/// that carries none, and [`UNKNOWN_ALIAS`] when the client cannot tell
/// -- `config.yaml` does not load, or names no such endpoint -- never as
/// absent or empty. Neither stand-in is in the alias grammar, so neither
/// can be read as an alias.
const NO_ALIAS: &str = "-";
/// See [`NO_ALIAS`].
const UNKNOWN_ALIAS: &str = "(unknown)";

async fn endpoint_write(port: &IpcAdmin, write: EndpointWrite) -> Result<String, TransportError> {
    Ok(match write {
        EndpointWrite::Revoke(endpoint) => {
            port.revoke_endpoint(endpoint.clone()).await?;
            format!("revoked {}\n", endpoint.as_str())
        }
        EndpointWrite::SetEnabled(endpoint, enabled) => {
            let revoked = port.set_endpoint_enabled(endpoint.clone(), enabled).await?;
            format!(
                "{} {}{}\n",
                if enabled { "enabled" } else { "disabled" },
                endpoint.as_str(),
                if revoked.is_some() {
                    ", its lease revoked"
                } else {
                    ""
                }
            )
        }
        EndpointWrite::SetDefault(endpoint) => {
            port.set_default_endpoint(endpoint.clone()).await?;
            match endpoint {
                Some(endpoint) => format!("default endpoint {}\n", endpoint.as_str()),
                None => "no default endpoint\n".to_owned(),
            }
        }
    })
}

/// `trust list --endpoint`: both directions of the endpoint's effective
/// set, the inbound pages and then the outbound ones. Text: the set the
/// daemon names, then one row per peer and direction with its source.
/// `--json`: every page as the daemon sent it, one `ipc/trust-list` a
/// line, inbound first; a page without `next` ends a direction, since a
/// page does not carry its direction.
async fn endpoint_trust_list(
    port: &IpcAdmin,
    endpoint: &EndpointId,
    json: bool,
) -> Result<String, TransportError> {
    let inbound = port
        .endpoint_trust_pages(endpoint, TrustDirection::Inbound)
        .await?;
    let outbound = port
        .endpoint_trust_pages(endpoint, TrustDirection::Outbound)
        .await?;
    let mut out = String::new();
    if json {
        for page in inbound.iter().chain(&outbound) {
            out += &serde_json::to_string(page).unwrap_or_default();
            out.push('\n');
        }
        return Ok(out);
    }
    // Each direction's pages carry one `set` (`endpoint_trust_pages`); the
    // two are read in two calls, so a write between them may differ.
    let inbound_set = inbound
        .first()
        .and_then(|page| page.set)
        .map_or("", |set| set.as_str());
    let outbound_set = outbound
        .first()
        .and_then(|page| page.set)
        .map_or("", |set| set.as_str());
    if inbound_set == outbound_set {
        let _ = writeln!(out, "set       {inbound_set}");
    } else {
        let _ = writeln!(
            out,
            "set       inbound {inbound_set}, outbound {outbound_set}"
        );
    }
    for (name, pages) in [("inbound", &inbound), ("outbound", &outbound)] {
        for row in pages.iter().flat_map(|page| &page.allowed) {
            let _ = write!(out, "{name:<9} {}", row.peer.as_str());
            match row.source {
                Some(TrustSource::Configured) => out += "  configured",
                Some(TrustSource::Administered) => out += "  administered",
                None => {}
            }
            out.push('\n');
        }
    }
    out += "every other peer is denied\n";
    Ok(out)
}

/// Tell "no daemon" from "a daemon whose socket is gone" by the lock: a
/// `try_lock` released at once, which the daemon's own one-second retry
/// absorbs, so a probe cannot fail a starting daemon (plan §16 (10)).
fn unreachable(paths: &ProfilePaths, sockets: &SocketPaths) -> Failure {
    match ProfileLock::is_held(paths) {
        Ok(false) => Failure::Unreachable(format!(
            "no daemon is running for profile {:?}",
            paths.profile()
        )),
        Ok(true) => Failure::Unreachable(format!(
            "a daemon holds profile {:?}'s lock, but its admin socket {} does not answer",
            paths.profile(),
            sockets.admin.display()
        )),
        Err(e) => Failure::Unreachable(format!(
            "the admin socket {} does not answer, and the lock could not be probed: {e}",
            sockets.admin.display()
        )),
    }
}

/// A daemon's refusal: its error code, as the wire names it.
fn code(e: TransportError) -> Failure {
    Failure::Refused(format!("refused: {e:?}"))
}

/// A result object as `--json` prints it: the method's own shape, so the
/// output validates against its schema (plan §16 (11)).
fn to_json<T: serde::Serialize>(result: &T) -> String {
    // A result that came off the wire as JSON goes back to it.
    serde_json::to_string_pretty(result).unwrap_or_default() + "\n"
}

/// `endpoints list`: the daemon's rows; in the text form each with the
/// alias the client resolves for its id.
async fn endpoint_list(
    port: &IpcAdmin,
    paths: &ProfilePaths,
    json: bool,
) -> Result<String, TransportError> {
    let list = port.endpoints_result().await?;
    if json {
        // The method's result as the daemon sent it, which the
        // `ipc/endpoint-list` schema describes: the alias is the
        // configuration's, shown in the text form alone.
        return Ok(to_json(&list));
    }
    // The daemon's rows carry ids alone; the alias is the
    // configuration's, read here and never sent.
    let known = aliases(paths)
        .inspect_err(|why| eprintln!("transportctl: note: the aliases are unknown: {why}"))
        .ok();
    let alias_of = |id: &EndpointId| -> Option<Option<&str>> {
        known
            .as_ref()
            .and_then(|known| known.get(id))
            .map(|alias| alias.as_ref().map(EndpointAlias::as_str))
    };
    Ok({
        let shown: Vec<&str> = list
            .endpoints
            .iter()
            .map(|row| match alias_of(&row.id) {
                Some(Some(alias)) => alias,
                Some(None) => NO_ALIAS,
                None => UNKNOWN_ALIAS,
            })
            .collect();
        let width = shown.iter().map(|a| a.len()).max().unwrap_or(0);
        let mut out = String::new();
        for (row, alias) in list.endpoints.iter().zip(shown) {
            let _ = writeln!(
                out,
                "{}  {alias:<width$}  {}{}{}",
                row.id.as_str(),
                if row.enabled { "enabled" } else { "disabled" },
                if row.default { ", default" } else { "" },
                row.lease.as_ref().map_or_else(String::new, |lease| format!(
                    ", leased by {}",
                    lease.client_kind
                )),
            );
        }
        out
    })
}

async fn call(
    port: &IpcAdmin,
    paths: &ProfilePaths,
    action: Admin,
    json: bool,
) -> Result<String, TransportError> {
    Ok(match action {
        Admin::Status => {
            let status = port.status_result().await?;
            if json {
                to_json(&status)
            } else {
                let mut out = String::new();
                let _ = writeln!(out, "peer         {}", status.peer.as_str());
                let _ = writeln!(out, "health       {:?}", status.health);
                let _ = writeln!(
                    out,
                    "connections  {} data, {} admin",
                    status.ipc.data_connections, status.ipc.admin_connections
                );
                let _ = writeln!(out, "leases       {}", status.ipc.active_leases);
                let c = &status.connectivity;
                let _ = writeln!(
                    out,
                    "inbound      direct {:?}, relay {:?} ({}/{} reservations)",
                    c.direct_inbound,
                    c.relay_inbound,
                    c.active_relay_reservations,
                    c.target_relay_reservations
                );
                out
            }
        }
        Admin::EndpointsList => endpoint_list(port, paths, json).await?,
        Admin::Shutdown(grace) => {
            // No --grace: none sent, the daemon's default.
            port.request_shutdown(grace).await?;
            "shutdown requested\n".to_owned()
        }
        Admin::TrustList(None) => {
            let pages = port.trust_pages().await?;
            let mut out = String::new();
            if json {
                // ONE `ipc/trust-list` PAGE PER LINE, as the daemon sent
                // each: the method answers in pages, so a single document
                // would be a shape no schema states.
                for page in &pages {
                    out += &serde_json::to_string(page).unwrap_or_default();
                    out.push('\n');
                }
            } else {
                if let Some(local) = pages.first().and_then(|page| page.local_peer.as_ref()) {
                    let _ = writeln!(out, "local    {}", local.as_str());
                }
                // Each row's source when the negotiated row has one (2.3):
                // configured in config.yaml, or administered and kept in
                // the trust overlay. A 2.1 row says neither.
                for row in pages.iter().flat_map(|page| &page.allowed) {
                    let _ = write!(out, "allowed  {}", row.peer.as_str());
                    match row.source {
                        Some(TrustSource::Configured) => out += "  configured",
                        Some(TrustSource::Administered) => out += "  administered",
                        None => {}
                    }
                    out.push('\n');
                }
                out += "every other peer is denied\n";
            }
            out
        }
        Admin::PeersList => {
            let pages = port.peers_pages().await?;
            let mut out = String::new();
            if json {
                // ONE `ipc/peer-list` PAGE PER LINE, as `trust list` does.
                for page in &pages {
                    out += &serde_json::to_string(page).unwrap_or_default();
                    out.push('\n');
                }
            } else {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
                let left = |until: u64| until.saturating_sub(now).div_ceil(1_000);
                for row in pages.iter().flat_map(|page| &page.peers) {
                    let _ = write!(
                        out,
                        "{}  {}",
                        row.peer.as_str(),
                        if row.connected {
                            "connected"
                        } else {
                            "not connected"
                        }
                    );
                    // When a peer not connected was last up (2.12): a
                    // connected one is up now. Absent from an older
                    // daemon, which says nothing, so neither does this.
                    if !row.connected {
                        match row.last_seen_ms {
                            Some(Some(at)) => {
                                let _ = write!(
                                    out,
                                    "  last seen {}s ago",
                                    now.saturating_sub(at) / 1_000
                                );
                            }
                            Some(None) => out += "  not seen since the daemon started",
                            None => {}
                        }
                    }
                    if let Some(until) = row.backoff_until {
                        let _ = write!(out, "  backoff {}s", left(until));
                    }
                    if let Some(until) = row.quarantined_until {
                        let _ = write!(out, "  quarantined {}s", left(until));
                    }
                    if let Some(outcome) = row.last_outcome {
                        let _ = write!(out, "  last {}", outcome.as_str());
                    }
                    out.push('\n');
                }
            }
            out
        }
        Admin::ListAddresses => {
            if json {
                to_json(&port.addresses_result().await?)
            } else {
                let mut out = String::new();
                for row in port.bootstrap_addresses().await? {
                    let _ = writeln!(
                        out,
                        "{}  {}{}",
                        row.address,
                        match row.source {
                            AddressSource::Authored => "authored",
                            AddressSource::Added => "added",
                        },
                        if row.dialled {
                            ""
                        } else {
                            "  not dialled: the provider is disabled"
                        }
                    );
                }
                out
            }
        }
        // One address per line and nothing else, in the daemon's order:
        // what the show screen's code carries, so the text pipes into
        // another node's `peers add`. Kinds and `truncated` are --json's.
        Admin::ShowAddresses => {
            if json {
                to_json(&port.own_addresses_result().await?)
            } else {
                let mut out = String::new();
                for row in port.own_addresses().await?.addresses {
                    let _ = writeln!(out, "{}", row.address);
                }
                out
            }
        }
        // Answered above, before any printing.
        Admin::AddAddress(_)
        | Admin::RemoveAddress(_)
        | Admin::SetTrust(..)
        | Admin::TrustList(Some(_))
        | Admin::Revoke(_)
        | Admin::SetEnabled(..)
        | Admin::SetDefault(_) => {
            unreachable!("a write is answered in run")
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Neither stand-in an alias column shows can be read as an alias.
    #[test]
    fn the_alias_stand_ins_are_outside_the_alias_grammar() {
        assert!(EndpointAlias::parse(NO_ALIAS).is_err());
        assert!(EndpointAlias::parse(UNKNOWN_ALIAS).is_err());
        assert!(EndpointAlias::parse("unknown").is_ok(), "the control");
    }
}
