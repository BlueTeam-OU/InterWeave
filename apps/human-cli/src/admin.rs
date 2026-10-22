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
use interweave_local_client_api::{AdminBinding as _, AdminCapability, AdminPort as _};
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
                            output::printable(&kind)
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
