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
use std::future::Future;
use std::io::Read as _;

use interweave_ipc_client::{IpcBinding, SocketPaths};
use interweave_local_client_api::{
    AddressChange, AddressSource, AddressesChange, AdminBinding as _, AdminCapability,
    AdminPort as _, MAX_ADDRESSES_PER_ADD, OwnAddresses, PeerGateView, PeerLastSeen,
};
use interweave_profile_config::ProfileLock;
use interweave_transport_api::{EndpointId, TransportError};
use serde_json::json;

use crate::Failure;
use crate::cli::{ListSource, Verb};
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

/// The most bytes a list of addresses is read to: sixteen lines past any
/// address the runtime takes, so a list it could take is never cut, and
/// what is read stays bounded (`a_list_past_its_bounds_is_refused_unsent`).
pub(crate) const MAX_LIST_BYTES: u64 = 64 * 1024;

/// The lines a list add sends, read from `reader`: as the window's add
/// page takes typed lines (`lines_of`: each line trimmed, empty ones
/// dropped).
///
/// # Errors
/// [`Failure::Refused`] when it cannot be read, is not UTF-8 text, is
/// past [`MAX_LIST_BYTES`], holds no address, or more than one add takes.
pub(crate) fn address_list(reader: impl std::io::Read) -> Result<Vec<String>, Failure> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_LIST_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| Failure::Refused(format!("the list of addresses: {e}")))?;
    if bytes.len() as u64 > MAX_LIST_BYTES {
        return Err(Failure::Refused(format!(
            "the list of addresses is larger than {} KiB",
            MAX_LIST_BYTES / 1024
        )));
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| Failure::Refused("the list of addresses is not UTF-8 text".to_owned()))?;
    let lines = interweave_human_ui_model::lines_of(&text);
    if lines.is_empty() {
        return Err(Failure::Refused("the list holds no address".to_owned()));
    }
    if lines.len() > MAX_ADDRESSES_PER_ADD {
        return Err(Failure::Refused(format!(
            "address {} of {}: one add takes at most {MAX_ADDRESSES_PER_ADD}; nothing was sent",
            MAX_ADDRESSES_PER_ADD + 1,
            lines.len()
        )));
    }
    Ok(lines)
}

/// Add `lines`, all or none, through `add` (`admin.peers.add_addresses`,
/// IPC 2.9): one line printed per address, `added` or `unchanged`, in the
/// order given; a refusal names the address by its number in the list
/// and its rule, the daemon's detail unshown, and nothing was written
/// (`a_list_is_added_whole_or_refused_by_its_line`).
pub(crate) async fn add_list<F, Fut>(lines: Vec<String>, add: F) -> Result<String, Failure>
where
    F: FnOnce(Vec<String>) -> Fut,
    Fut: Future<Output = Result<AddressesChange, TransportError>>,
{
    let count = lines.len();
    match add(lines.clone()).await {
        Ok(AddressesChange::Added(rows)) if rows.len() == count => {
            let mut out = String::new();
            for (row, line) in rows.iter().zip(&lines) {
                let _ = writeln!(out, "{} {}", row.as_str(), output::one_line(line));
            }
            Ok(out)
        }
        Ok(AddressesChange::Added(_)) => Err(Failure::Refused(
            "the daemon answered for another number of addresses than were sent".to_owned(),
        )),
        Ok(AddressesChange::Refused {
            reason,
            line: Some(n),
            ..
        }) => Err(Failure::Refused(format!(
            "the daemon refused address {} of {count} ({}): {}; nothing was added",
            n + 1,
            lines.get(n).map_or("", |l| l.as_str()),
            reason.as_str()
        ))),
        Ok(AddressesChange::Refused { reason, .. }) => Err(Failure::Refused(format!(
            "the daemon refused the list: {}; nothing was added",
            reason.as_str()
        ))),
        Err(TransportError::ProtocolUnsupported) => Err(Failure::Refused(
            "the daemon does not add a list of addresses (IPC 2.9): is it older?".to_owned(),
        )),
        Err(e) => Err(refused(e)),
    }
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
    // A list is read, and judged by this client's own rules, before any
    // socket: a list refused by them is exit 1 whether or not a daemon
    // runs, and no admin connection waits on a slow pipe.
    let list = match verb {
        Verb::PeerAddList(ListSource::Stdin) => Some(address_list(std::io::stdin().lock())?),
        Verb::PeerAddList(ListSource::File(path)) => {
            Some(address_list(std::fs::File::open(path).map_err(|e| {
                Failure::Refused(format!("the list file: {e}"))
            })?)?)
        }
        _ => None,
    };
    let binding = IpcBinding::new(
        SocketPaths {
            data: profile.data_socket.clone(),
            admin: profile.admin_socket.clone(),
        },
        CLIENT_KIND,
    );
    // A trusted peer's state is the dial gate's, read under
    // `admin.status` (`admin.peers.list`, IPC 2.2); the addresses are
    // `admin.peers`'.
    let capability = if matches!(verb, Verb::PeersState) {
        AdminCapability::Status
    } else {
        AdminCapability::Peers
    };
    let port = match binding.admin([capability].into()).await {
        Ok(port) => port,
        Err(TransportError::BackendUnavailable) => return Err(unreachable(profile)),
        Err(e) => return Err(refused(e)),
    };
    if matches!(verb, Verb::PeersState) {
        let rows = match port.peers().await {
            Ok(rows) => rows,
            Err(TransportError::ProtocolUnsupported) => {
                return Err(Failure::Refused(
                    "the daemon does not list its peers (IPC 2.2): is it older?".to_owned(),
                ));
            }
            Err(e) => return Err(refused(e)),
        };
        out.push_str(&peer_states(&rows, json));
        return Ok(());
    }
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
        Verb::PeerAddList(_) => {
            let lines = list.unwrap_or_default();
            out.push_str(&add_list(lines, |l| port.add_bootstrap_addresses(l)).await?);
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

/// Each trusted peer's state, one line per row the daemon's allowlist
/// gives, in its order: whether a connection to it is open, and when one
/// last was. `last_seen_ms` is `null` both when the daemon never saw the
/// peer and when it does not say, and `"last_seen": "unreported"` tells
/// the second apart, so a reader needs no IPC version to know which.
pub(crate) fn peer_states(rows: &[PeerGateView], json: bool) -> String {
    let mut out = String::new();
    for row in rows {
        let last_seen_ms = match row.last_seen {
            PeerLastSeen::At(ms) => Some(ms),
            PeerLastSeen::Never | PeerLastSeen::Unreported => None,
        };
        if json {
            let mut line = json!({
                "peer": row.peer.as_str(),
                "connected": row.connected,
                "last_seen_ms": last_seen_ms,
            });
            if row.last_seen == PeerLastSeen::Unreported {
                line["last_seen"] = json!("unreported");
            }
            out.push_str(&output::json_line(&line));
        } else {
            let state = match (row.connected, row.last_seen) {
                (true, _) => "connected".to_owned(),
                (false, PeerLastSeen::At(ms)) => {
                    format!("not connected, last seen {}", output::utc(ms))
                }
                (false, PeerLastSeen::Never) => "not connected, last seen never".to_owned(),
                (false, PeerLastSeen::Unreported) => {
                    "not connected, last seen not reported by this daemon".to_owned()
                }
            };
            let _ = writeln!(out, "{}  {state}", row.peer.as_str());
        }
    }
    out
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

    fn gate_row(n: u8, connected: bool, last_seen: PeerLastSeen) -> PeerGateView {
        PeerGateView {
            peer: interweave_transport_api::TransportIdentity::parse(match n {
                1 => "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN",
                2 => "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5",
                _ => "12D3KooWNr33yB9DMXYfmRj3A8PXa2zmEKV2BVsdLALW8yQNcxT9",
            })
            .expect("a PeerId"),
            connected,
            last_seen,
            backoff_until: None,
            quarantined_until: None,
            route_held_until: None,
            last_outcome: None,
        }
    }

    /// Each trusted peer is one line: connected or not, and when last seen
    /// -- a time, never, or not reported -- with never and not reported
    /// told apart in the JSON whatever the daemon's IPC version, by the
    /// `last_seen` member the second alone carries. Mutations: the member
    /// added for never too; a connected peer's time printed as its state.
    #[test]
    fn each_trusted_peer_says_whether_it_is_connected_and_when_it_was_last_seen() {
        let rows = [
            gate_row(1, true, PeerLastSeen::At(1_786_600_000_000)),
            gate_row(2, false, PeerLastSeen::Never),
            gate_row(3, false, PeerLastSeen::Unreported),
        ];
        let lines: Vec<serde_json::Value> = peer_states(&rows, true)
            .lines()
            .map(|l| serde_json::from_str(l).expect("a JSON line"))
            .collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0]["connected"], true);
        assert_eq!(lines[0]["last_seen_ms"], 1_786_600_000_000_u64);
        assert!(lines[0].get("last_seen").is_none());
        assert_eq!(lines[1]["last_seen_ms"], serde_json::Value::Null);
        assert!(
            lines[1].get("last_seen").is_none(),
            "never is no unreported"
        );
        assert_eq!(lines[2]["last_seen_ms"], serde_json::Value::Null);
        assert_eq!(lines[2]["last_seen"], "unreported");

        let text = peer_states(&rows, false);
        let text: Vec<&str> = text.lines().collect();
        assert!(text[0].ends_with("  connected"), "{}", text[0]);
        assert!(
            text[1].ends_with("  not connected, last seen never"),
            "{}",
            text[1]
        );
        assert!(text[2].contains("not reported"), "{}", text[2]);
        let seen = peer_states(
            &[gate_row(1, false, PeerLastSeen::At(1_786_600_000_000))],
            false,
        );
        assert!(seen.contains("not connected, last seen 2026-"), "{seen}");
    }

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
    fn a_list_past_its_bounds_is_refused_unsent() {
        let list = |text: &str| address_list(text.as_bytes());
        assert_eq!(
            list(" /ip4/192.0.2.1/tcp/1 \r\n\n\t/ip4/192.0.2.2/tcp/2\n").expect("read"),
            ["/ip4/192.0.2.1/tcp/1", "/ip4/192.0.2.2/tcp/2"],
            "trimmed, empty lines dropped, as the add page takes typed lines"
        );
        let mut sixteen = String::new();
        for n in 1..=16 {
            let _ = writeln!(sixteen, "/ip4/192.0.2.{n}/tcp/1");
        }
        assert_eq!(list(&sixteen).expect("sixteen").len(), 16);
        let seventeen = format!("{sixteen}/ip4/192.0.2.17/tcp/1\n");
        match list(&seventeen) {
            Err(Failure::Refused(why)) => assert!(why.contains("address 17 of 17"), "{why}"),
            other => panic!("{other:?}"),
        }
        for refused in ["", "\n \n\t\n"] {
            assert!(
                matches!(list(refused), Err(Failure::Refused(_))),
                "{refused:?}"
            );
        }
        assert!(matches!(
            address_list(&[0xff_u8, 0xfe][..]),
            Err(Failure::Refused(_))
        ));
        let large = vec![b' '; usize::try_from(MAX_LIST_BYTES).expect("fits") + 1];
        match address_list(&large[..]) {
            Err(Failure::Refused(why)) => assert!(why.contains("KiB"), "{why}"),
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn a_list_is_added_whole_or_refused_by_its_line() {
        use interweave_local_client_api::{AddressRefusal, LineChange};
        let lines = vec![
            "/ip4/192.0.2.1/tcp/1".to_owned(),
            "/ip4/192.0.2.2/tcp/2".to_owned(),
            "/ip4/127.0.0.1/tcp/3".to_owned(),
        ];
        let sent = std::sync::Mutex::new(Vec::new());
        let answer = |change: Result<AddressesChange, TransportError>| {
            let sent = &sent;
            move |l: Vec<String>| {
                sent.lock().expect("lock").push(l);
                std::future::ready(change)
            }
        };
        let out = add_list(
            lines[..2].to_vec(),
            answer(Ok(AddressesChange::Added(vec![
                LineChange::Added,
                LineChange::Unchanged,
            ]))),
        )
        .await
        .expect("added");
        assert_eq!(
            out,
            "added /ip4/192.0.2.1/tcp/1\nunchanged /ip4/192.0.2.2/tcp/2\n"
        );
        assert_eq!(sent.lock().expect("lock")[0], lines[..2], "one call, whole");
        let refused = add_list(
            lines.clone(),
            answer(Ok(AddressesChange::Refused {
                reason: AddressRefusal::SpecialUse,
                line: Some(2),
                detail: "a secret detail".to_owned(),
            })),
        )
        .await;
        match refused {
            Err(Failure::Refused(why)) => {
                assert!(why.contains("address 3 of 3"), "{why}");
                assert!(why.contains("/ip4/127.0.0.1/tcp/3"), "{why}");
                assert!(why.contains("special_use"), "{why}");
                assert!(!why.contains("secret"), "the detail is not shown: {why}");
            }
            other => panic!("{other:?}"),
        }
        let whole = add_list(
            lines.clone(),
            answer(Ok(AddressesChange::Refused {
                reason: AddressRefusal::ProviderDisabled,
                line: None,
                detail: String::new(),
            })),
        )
        .await;
        assert!(
            matches!(&whole, Err(Failure::Refused(why)) if why.contains("provider_disabled")),
            "{whole:?}"
        );
        let short = add_list(
            lines.clone(),
            answer(Ok(AddressesChange::Added(vec![LineChange::Added]))),
        )
        .await;
        assert!(matches!(short, Err(Failure::Refused(_))), "{short:?}");
        let old = add_list(lines, answer(Err(TransportError::ProtocolUnsupported))).await;
        assert!(
            matches!(&old, Err(Failure::Refused(why)) if why.contains("2.9")),
            "{old:?}"
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
