// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `identities`: the profile's human endpoints as the daemon holds them,
//! over the admin socket -- a separate connection and a separate authority
//! from the data session (ADR-0040) -- as `transportctl`'s endpoint
//! commands reach them (`apps/transportctl/src/admin.rs`). No model holds
//! an endpoint's administration, so these are the daemon's methods,
//! narrowed to the endpoints a human client may lease: any other endpoint
//! is `transportctl`'s to administer, and refused here.

use std::fmt::Write as _;

use interweave_ipc_client::{IpcBinding, SocketPaths};
use interweave_local_client_api::{
    AddressChange, AddressSource, AdminBinding as _, AdminCapability, AdminPort as _, OwnAddresses,
};
use interweave_profile_config::ProfileLock;
use interweave_transport_api::{EndpointId, TransportError};
use serde_json::json;

use crate::Failure;
use crate::cli::Verb;
use crate::output;
use crate::startup::{CLIENT_KIND, Profile};

fn unreachable(profile: &Profile) -> Failure {
    let name = profile.paths.profile();
    match ProfileLock::is_held(&profile.paths) {
        Ok(false) => Failure::Unreachable(format!("no daemon is running for profile {name:?}")),
        Ok(true) => Failure::Unreachable(format!(
            "a daemon holds profile {name:?}'s lock, but its admin socket does not answer"
        )),
        Err(e) => Failure::Unreachable(format!(
            "the admin socket does not answer, and the lock could not be probed: {e}"
        )),
    }
}

fn refused(e: TransportError) -> Failure {
    Failure::Refused(format!("the daemon refused: {e:?}"))
}

/// Refuse a change to an endpoint that is not one of `ours`, before any
/// socket: it is `transportctl`'s to administer.
fn narrowed(ours: &[EndpointId], verb: &Verb) -> Result<(), Failure> {
    match verb {
        Verb::SetEnabled(endpoint, _) | Verb::SetDefault(Some(endpoint))
            if !ours.contains(endpoint) =>
        {
            Err(Failure::Refused(
                "that endpoint is not one a human client may use in this profile".to_owned(),
            ))
        }
        _ => Ok(()),
    }
}

/// Whether `identities default --none` may clear `current`, the
/// profile's default now: only when it is one of `ours`, or there is none.
fn may_clear(current: Option<&EndpointId>, ours: &[EndpointId]) -> Result<(), Failure> {
    match current {
        Some(id) if !ours.contains(id) => Err(Failure::Refused(
            "the profile's default endpoint is not one a human client uses: \
             clear it with transportctl"
                .to_owned(),
        )),
        _ => Ok(()),
    }
}

/// What an address change prints, or why it was refused. A refusal is
/// named by its rule's token; the daemon's detail is for a log, and is
/// not shown (`an_address_refusal_names_its_rule_and_not_the_detail`).
fn change_line(change: &AddressChange) -> Result<String, Failure> {
    match change {
        AddressChange::Added => Ok("added\n".to_owned()),
        AddressChange::Removed => Ok("removed\n".to_owned()),
        AddressChange::Unchanged => Ok("unchanged\n".to_owned()),
        AddressChange::Refused { reason, .. } => Err(Failure::Refused(format!(
            "the daemon refused the address: {}",
            reason.as_str()
        ))),
    }
}

/// What `peers show` prints: one address per line in the daemon's order
/// and nothing else, so the text pipes into another node's `peers add`;
/// or one JSON object with the peer, each address's kind and whether the
/// daemon dropped rows past its bound
/// (`own_addresses_print_in_the_daemons_order_and_json_says_what_was_dropped`).
fn own_lines(own: &OwnAddresses, json: bool) -> String {
    if json {
        return output::json_line(&json!({
            "peer": own.peer.as_str(),
            "addresses": own.addresses.iter().map(|row| json!({
                "address": row.address,
                "kind": row.kind.as_str(),
            })).collect::<Vec<_>>(),
            "truncated": own.truncated,
        }));
    }
    let mut out = String::new();
    for row in &own.addresses {
        let _ = writeln!(out, "{}", output::one_line(&row.address));
    }
    out
}

/// Carry out a `peers` verb (ADR-0062), on the admin socket's
/// `admin.peers` capability: add, remove and list from IPC 2.7, show from
/// 2.8. An address is sent as typed: the daemon's method judges it,
/// against the configuration's grammar and the special-use floor.
///
/// # Errors
/// [`Failure::Unreachable`] when no daemon answers, [`Failure::Refused`]
/// when it refuses or speaks too old a minor.
pub(crate) async fn peers(
    profile: &Profile,
    verb: &Verb,
    json: bool,
    out: &mut String,
) -> Result<(), Failure> {
    let binding = IpcBinding::new(
        SocketPaths {
            data: profile.data_socket.clone(),
            admin: profile.admin_socket.clone(),
        },
        CLIENT_KIND,
    );
    let port = match binding.admin([AdminCapability::Peers].into()).await {
        Ok(port) => port,
        Err(TransportError::BackendUnavailable) => return Err(unreachable(profile)),
        Err(e) => return Err(refused(e)),
    };
    if !port.port().holds(AdminCapability::Peers) {
        return Err(Failure::Refused(
            "the daemon does not grant the peers capability (IPC 2.7): is it older?".to_owned(),
        ));
    }
    match verb {
        Verb::PeerAdd(address) => {
            let change = port
                .add_bootstrap_address(address.clone())
                .await
                .map_err(refused)?;
            out.push_str(&change_line(&change)?);
        }
        Verb::PeerRemove(address) => {
            let change = port
                .remove_bootstrap_address(address.clone())
                .await
                .map_err(refused)?;
            out.push_str(&change_line(&change)?);
        }
        Verb::PeersShow => match port.own_addresses().await {
            Ok(own) => out.push_str(&own_lines(&own, json)),
            Err(TransportError::ProtocolUnsupported) => {
                return Err(Failure::Refused(
                    "the daemon does not report its own addresses (IPC 2.8): is it older?"
                        .to_owned(),
                ));
            }
            Err(e) => return Err(refused(e)),
        },
        Verb::PeersList => {
            for row in port.bootstrap_addresses().await.map_err(refused)? {
                let source = match row.source {
                    AddressSource::Authored => "authored",
                    AddressSource::Added => "added",
                };
                if json {
                    out.push_str(&output::json_line(&json!({
                        "address": row.address,
                        "source": source,
                        "dialled": row.dialled,
                    })));
                } else {
                    let _ = writeln!(
                        out,
                        "{}  {source}{}",
                        output::one_line(&row.address),
                        if row.dialled {
                            ""
                        } else {
                            ", not dialled: the provider is off"
                        }
                    );
                }
            }
        }
        _ => return Err(Failure::Refused("not a peers verb".to_owned())),
    }
    Ok(())
}

/// Carry out an `identities` verb.
///
/// # Errors
/// [`Failure::Unreachable`] when no daemon answers, [`Failure::Refused`]
/// when it refuses or the endpoint is not one of this client's.
pub(crate) async fn identities(
    profile: &Profile,
    verb: &Verb,
    json: bool,
    out: &mut String,
) -> Result<(), Failure> {
    narrowed(&profile.endpoints, verb)?;
    let binding = IpcBinding::new(
        SocketPaths {
            data: profile.data_socket.clone(),
            admin: profile.admin_socket.clone(),
        },
        CLIENT_KIND,
    );
    let port = match binding.admin([AdminCapability::Endpoints].into()).await {
        Ok(port) => port,
        Err(TransportError::BackendUnavailable) => return Err(unreachable(profile)),
        Err(e) => return Err(refused(e)),
    };
    if !port.port().holds(AdminCapability::Endpoints) {
        return Err(Failure::Refused(
            "the daemon does not grant the endpoints capability: is it older?".to_owned(),
        ));
    }
    match verb {
        Verb::IdentitiesList => {
            let list = port.endpoints_result().await.map_err(refused)?;
            for endpoint in &profile.endpoints {
                let Some(row) = list.endpoints.iter().find(|row| &row.id == endpoint) else {
                    continue;
                };
                let leased = row.lease.as_ref().map(|lease| lease.client_kind.clone());
                if json {
                    out.push_str(&output::json_line(&json!({
                        "endpoint": row.id.as_str(),
                        "enabled": row.enabled,
                        "default": row.default,
                        "leased_by": leased,
                    })));
                } else {
                    let _ = writeln!(
                        out,
                        "{}  {}{}{}",
                        row.id.as_str(),
                        if row.enabled { "enabled" } else { "disabled" },
                        if row.default { ", default" } else { "" },
                        leased.map_or_else(String::new, |kind| format!(
                            ", leased by {}",
                            output::one_line(&kind)
                        )),
                    );
                }
            }
        }
        Verb::SetEnabled(endpoint, enabled) => {
            let revoked = port
                .set_endpoint_enabled(endpoint.clone(), *enabled)
                .await
                .map_err(refused)?;
            let _ = writeln!(
                out,
                "{} {}{}",
                if *enabled { "enabled" } else { "disabled" },
                endpoint.as_str(),
                if revoked.is_some() {
                    ", its lease revoked"
                } else {
                    ""
                }
            );
        }
        Verb::SetDefault(endpoint) => {
            if endpoint.is_none() {
                // Clearing names no endpoint, so whose default it clears is
                // read first: one that is not a human endpoint is another
                // client's routing, and `transportctl`'s to change.
                let list = port.endpoints_result().await.map_err(refused)?;
                let current = list.endpoints.iter().find(|row| row.default);
                may_clear(current.map(|row| &row.id), &profile.endpoints)?;
            }
            port.set_default_endpoint(endpoint.clone())
                .await
                .map_err(refused)?;
            let _ = match endpoint {
                Some(endpoint) => writeln!(out, "default {}", endpoint.as_str()),
                None => writeln!(out, "no default"),
            };
        }
        _ => {
            return Err(Failure::Refused("not an identities verb".to_owned()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_refusal_names_its_rule_and_not_the_detail() {
        use interweave_local_client_api::AddressRefusal;
        let refused = change_line(&AddressChange::Refused {
            reason: AddressRefusal::SpecialUse,
            detail: "loopback /ip4/127.0.0.1".to_owned(),
        });
        match refused {
            Err(Failure::Refused(message)) => {
                assert!(message.contains("special_use"), "{message}");
                assert!(!message.contains("127.0.0.1"), "{message}");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(change_line(&AddressChange::Added).expect("ok"), "added\n");
        assert_eq!(
            change_line(&AddressChange::Unchanged).expect("ok"),
            "unchanged\n"
        );
    }

    #[test]
    fn own_addresses_print_in_the_daemons_order_and_json_says_what_was_dropped() {
        use interweave_local_client_api::{OwnAddress, OwnAddressKind};
        let peer = interweave_transport_api::TransportIdentity::parse(
            "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN",
        )
        .expect("peer");
        let at = |a: &str, kind| OwnAddress {
            address: format!("{a}/p2p/{}", peer.as_str()),
            kind,
        };
        let own = OwnAddresses {
            peer: peer.clone(),
            addresses: vec![
                at(
                    "/ip4/198.51.100.7/tcp/4001",
                    OwnAddressKind::ConfirmedExternal,
                ),
                at("/ip4/10.0.0.2/tcp/4001", OwnAddressKind::Listen),
            ],
            truncated: true,
        };
        let hostile = OwnAddresses {
            addresses: vec![at(
                "/ip4/1.2.3.4/tcp/1\n/ip4/5.6.7.8/tcp/2",
                OwnAddressKind::Listen,
            )],
            ..own.clone()
        };
        assert_eq!(
            own_lines(&hostile, false).lines().count(),
            1,
            "one address, one line"
        );
        let text = own_lines(&own, false);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "{text}");
        assert!(lines[0].starts_with("/ip4/198.51.100.7/"), "{text}");
        assert!(lines[1].starts_with("/ip4/10.0.0.2/"), "{text}");
        let json: serde_json::Value = serde_json::from_str(&own_lines(&own, true)).expect("json");
        assert_eq!(json["peer"], peer.as_str());
        assert_eq!(json["truncated"], true);
        assert_eq!(json["addresses"][0]["kind"], "confirmed_external");
        assert_eq!(json["addresses"][1]["kind"], "listen");
    }

    #[test]
    fn a_change_to_an_endpoint_not_the_human_clients_is_refused() {
        let id = |s: &str| EndpointId::parse(s).expect("valid");
        let ours = [id("human")];
        for verb in [
            Verb::SetEnabled(id("agent"), false),
            Verb::SetDefault(Some(id("agent"))),
        ] {
            assert!(
                matches!(narrowed(&ours, &verb), Err(Failure::Refused(_))),
                "{verb:?}"
            );
        }
        assert!(matches!(
            may_clear(Some(&id("claude")), &ours),
            Err(Failure::Refused(_))
        ));
        assert!(may_clear(Some(&id("human")), &ours).is_ok());
        assert!(may_clear(None, &ours).is_ok());
        for verb in [
            Verb::SetEnabled(id("human"), false),
            Verb::SetDefault(Some(id("human"))),
            Verb::SetDefault(None),
            Verb::IdentitiesList,
        ] {
            assert!(narrowed(&ours, &verb).is_ok(), "{verb:?}");
        }
    }
}
