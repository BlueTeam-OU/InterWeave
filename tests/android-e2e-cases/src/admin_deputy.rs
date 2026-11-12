// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! TM-42's Android half (threat-model.md, "Android remote message
//! triggers admin action"): a remote message cannot act as the app's
//! `LocalAdminPort`. The case runs on the app's own client, meaning the
//! facade, store and hub the app's service composed. The desktop sends
//! the app's `human` endpoint every admin request a remote could hope to
//! smuggle in ([`admin_shaped`]): raw JSON requests that would trust a
//! stranger, revoke the endpoint, add the stranger's address and shut the
//! runtime down, and one `HumanChatV2` message whose text is the trust
//! request ([`admin_text`]).
//!
//! What the case asserts:
//! - Every payload was handed to the facade as its bytes. This is the
//!   control showing the would-be deputy was given them.
//! - Read afterwards through this side's own admin port, the trust list,
//!   the endpoint leases and the bootstrap addresses are as they were
//!   before, the baseline taken once the facade holds its `human` lease,
//!   so a revoke shows as that lease gone or under a new epoch. The stranger is neither trusted nor addressed, and the
//!   runtime still answers.
//! - The positive control: the same reads see a change made through the
//!   admin port itself (the stranger trusted, then untrusted again). So
//!   "unchanged" is a comparison that can fail.
//!
//! The case ends on the desktop's [`crate::closing`] message, so that on
//! a device its process outlives the desktop's acceptance of every send
//! ([`crate::close_exchange`]).
//!
//! What this does NOT prove: anything about code that already runs in
//! the app's process (threat-model.md's residual), or a message the
//! facade drained after the reads.

use std::time::{Duration, Instant};

use interweave_human_chat_protocol::HumanChatV2;
use interweave_local_client_api::{
    AdminBinding, AdminCapability, AdminPort as _, DataSessionBinding, TrustChange,
};
use interweave_transport_api::TransportIdentity;
use serde_json::{Map, Value};

use crate::{AppClient, DEFAULT_DEADLINE, closing, keys};

/// The serial of the desktop's close: the case ends when it is handed.
pub const CLOSE_SERIAL: u8 = 0x2a;

/// Every admin request a remote sends as a raw direct payload, in the
/// local IPC's own JSON shape (`LOCAL-IPC.md`): trust `stranger`, revoke
/// the `human` endpoint, add an address for `stranger`, and shut down.
#[must_use]
pub fn admin_shaped(stranger: &TransportIdentity) -> Vec<Vec<u8>> {
    let peer = stranger.as_str();
    [
        format!(
            r#"{{"type":"request","id":"a","method":"admin.trust.set","params":{{"peer":"{peer}","allowed":true}}}}"#
        ),
        r#"{"type":"request","id":"b","method":"admin.endpoints.revoke","params":{"endpoint":"human"}}"#
            .to_owned(),
        format!(
            r#"{{"type":"request","id":"c","method":"admin.peers.add_address","params":{{"peer":"{peer}","address":"/ip4/192.0.2.7/tcp/4001/p2p/{peer}"}}}}"#
        ),
        r#"{"type":"request","id":"d","method":"admin.shutdown"}"#.to_owned(),
    ]
    .into_iter()
    .map(String::into_bytes)
    .collect()
}

/// The text of the one chat message the desktop sends: the trust request
/// again, as a person's message.
#[must_use]
pub fn admin_text(stranger: &TransportIdentity) -> String {
    let raw = admin_shaped(stranger).swap_remove(0);
    String::from_utf8(raw).unwrap_or_default()
}

/// The arguments of [`crate::cases::ADMIN_SHAPED`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminShapedArgs {
    /// The desktop peer the requests come from, trusted by this side.
    pub desktop: TransportIdentity,
    /// The peer the requests try to trust and address: trusted by no one.
    pub stranger: TransportIdentity,
    /// How long the desktop may take to send everything.
    pub deadline: Duration,
}

impl AdminShapedArgs {
    /// The JSON a host passes.
    #[must_use]
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "desktop": self.desktop.as_str(),
            "stranger": self.stranger.as_str(),
            "deadline_ms": u64::try_from(self.deadline.as_millis()).unwrap_or(u64::MAX),
        })
    }

    /// Parse what [`to_json`](Self::to_json) wrote.
    ///
    /// # Errors
    /// A missing or malformed field, by name.
    pub fn from_json(args: &Value) -> Result<Self, String> {
        let peer = |name: &str| {
            args.get(name)
                .and_then(Value::as_str)
                .and_then(|s| TransportIdentity::parse(s).ok())
                .ok_or_else(|| format!("argument {name:?} is not a PeerId"))
        };
        let deadline = match args.get("deadline_ms") {
            None => DEFAULT_DEADLINE,
            Some(v) => Duration::from_millis(
                v.as_u64()
                    .ok_or("argument \"deadline_ms\" is not a count")?,
            ),
        };
        Ok(Self {
            desktop: peer("desktop")?,
            stranger: peer("stranger")?,
            deadline,
        })
    }
}

/// What this side's admin port says of the state a remote request would
/// change: the trust list, every endpoint with its lease, and the
/// bootstrap addresses (without `dialled`, which the desktop's own
/// connection may flip).
#[derive(Debug, PartialEq, Eq)]
struct State {
    trusted: Vec<String>,
    leases: String,
    addresses: Vec<String>,
}

/// A refusal is not printed: it can carry a peer's identity, as
/// `send_until_routed` says.
async fn state(port: &impl interweave_local_client_api::AdminPort) -> Result<State, String> {
    let trust = port
        .trust()
        .await
        .map_err(|_| "the trust list was refused".to_owned())?;
    let leases = port
        .leases()
        .await
        .map_err(|_| "the endpoint list was refused".to_owned())?;
    let addresses = port
        .bootstrap_addresses()
        .await
        .map_err(|_| "the bootstrap address list was refused".to_owned())?;
    Ok(State {
        trusted: trust.peers().map(|p| p.as_str().to_owned()).collect(),
        leases: format!("{leases:?}"),
        addresses: addresses
            .into_iter()
            .map(|a| format!("{} {:?}", a.address, a.source))
            .collect(),
    })
}

/// The case: see the module note.
pub(crate) fn admin_shaped_case<B: DataSessionBinding + AdminBinding>(
    binding: &B,
    client: &mut dyn AppClient,
    args: &AdminShapedArgs,
) -> Result<Map<String, Value>, String> {
    let port = crate::block_on(
        binding.admin(
            [
                AdminCapability::Trust,
                AdminCapability::Endpoints,
                AdminCapability::Peers,
            ]
            .into(),
        ),
    )?
    .map_err(|_| "this side's admin port was refused".to_owned())?;
    // The baseline once the facade holds the `human` lease it serves on:
    // a revoke would end that lease, or show it again under a new epoch.
    let until = Instant::now() + args.deadline;
    let before = loop {
        client.updates(Duration::from_millis(100))?;
        let leased = crate::block_on(port.leases())?
            .map_err(|_| "the endpoint list was refused".to_owned())?
            .iter()
            .any(|e| e.endpoint.as_str() == "human" && e.lease.is_some());
        if leased {
            break crate::block_on(state(&port))??;
        }
        if Instant::now() >= until {
            return Err("the app's facade never held the human lease".into());
        }
    };
    let stranger = args.stranger.as_str().to_owned();
    if !before.trusted.contains(&args.desktop.as_str().to_owned()) {
        return Err(format!(
            "the control: this side trusts the desktop, and its list is {:?}",
            before.trusted
        ));
    }
    if before.trusted.contains(&stranger) || before.addresses.iter().any(|a| a.contains(&stranger))
    {
        return Err("the stranger is already trusted or addressed before anything was sent".into());
    }

    // Every payload handed to the facade, the close last. The chat
    // message is found by its parsed text: in its envelope the request's
    // quotes are escaped, so its bytes are not the request's.
    let raw = admin_shaped(&args.stranger);
    let text = admin_text(&args.stranger);
    let close = closing(CLOSE_SERIAL).1.into_bytes();
    loop {
        // Running the hub's loop is what lets the facade drain; an error
        // here is the service stopping, which is a failure in itself.
        client.updates(Duration::from_millis(100))?;
        let handed = client.handed();
        let has = |bytes: &[u8]| handed.iter().any(|h| h.bytes == bytes);
        let chat = handed.iter().any(|h| {
            std::str::from_utf8(&h.bytes)
                .ok()
                .and_then(|s| HumanChatV2::parse(s).ok())
                .is_some_and(|m| m.text == text)
        });
        let missing = raw.iter().filter(|w| !has(w)).count();
        if missing == 0 && chat && has(&close) {
            break;
        }
        if Instant::now() >= until {
            return Err(format!(
                "not handed to the facade within the deadline: {missing} of {} raw requests{}{}",
                raw.len(),
                if chat { "" } else { ", the chat message" },
                if has(&close) { "" } else { ", the close" },
            ));
        }
    }

    let after = crate::block_on(state(&port))??;
    if after != before {
        return Err(format!(
            "a remote message changed this side's state:\nbefore {before:?}\nafter  {after:?}"
        ));
    }
    // The client still runs: a shutdown request did not reach the service.
    client.updates(Duration::ZERO)?;

    // The positive control: the same reads see a change made through the
    // admin port, which is then put back.
    let set = |allowed: bool| {
        crate::block_on(port.set_trust(args.stranger.clone(), allowed))
            .and_then(|r| r.map_err(|_| "the control's trust change was refused".to_owned()))
    };
    if !matches!(set(true)?, TrustChange::Set) {
        return Err("the control's trust change was refused".into());
    }
    let changed = crate::block_on(state(&port))??;
    let undone = set(false);
    if !changed.trusted.contains(&stranger) {
        return Err(format!(
            "the control: a trust change through the admin port is not seen by the same read: {changed:?}"
        ));
    }
    if !matches!(undone?, TrustChange::Set) {
        return Err("the control's trust change back was refused".into());
    }
    if crate::block_on(state(&port))?? != before {
        return Err("the control's trust change was not put back".into());
    }
    let mut out = Map::new();
    out.insert(keys::HANDED.to_owned(), (raw.len() + 1).to_string().into());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stranger() -> TransportIdentity {
        TransportIdentity::parse("12D3KooWD3eckifWpRn9wQpMG9R9hX3sD158z7EqHWmweQAJU5SA")
            .expect("a PeerId")
    }

    #[test]
    fn the_arguments_round_trip() {
        let args = AdminShapedArgs {
            desktop: stranger(),
            stranger: stranger(),
            deadline: Duration::from_secs(7),
        };
        assert_eq!(AdminShapedArgs::from_json(&args.to_json()), Ok(args));
    }

    #[test]
    fn every_admin_shaped_payload_is_json_naming_an_admin_method() {
        let all = admin_shaped(&stranger());
        assert_eq!(all.len(), 4);
        for raw in &all {
            let v: Value = serde_json::from_slice(raw).expect("JSON");
            assert!(
                v["method"]
                    .as_str()
                    .is_some_and(|m| m.starts_with("admin.")),
                "{v}"
            );
        }
        assert_eq!(admin_text(&stranger()).into_bytes(), all[0]);
    }
}
