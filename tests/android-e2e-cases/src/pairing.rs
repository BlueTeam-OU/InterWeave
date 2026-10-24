// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Plan §29 (e)'s two device rows, the Android half of each, over the
//! admin port and the data session of the runtime now serving:
//!
//! - [`crate::cases::ADDED_ADDRESS`]: an address added for a trusted peer
//!   through the binding (`admin.peers.add_address`'s embedded function)
//!   is dialled, and after a restart it is still listed as added and
//!   dialled again, with no static-bootstrap entry authored for the peer.
//!   [`crate::cases::AWAIT_PEER`] is the far end's half: it holds no
//!   address for the dialler, so the connection it sees is the dial.
//! - [`crate::cases::PAIR`]: one step of two fresh profiles pairing --
//!   each holding the other's address and trusting nobody, then one
//!   trusting the other, then both -- and whether a message crosses.
//!
//! What either case does NOT show alone is which side dialled, or that
//! nothing but the overlay produced the address: those are the host's to
//! arrange (the far end holds no address for this side; for the overlay
//! alone, the profile enables no peer cache, as `added_address`'s row
//! does and `pair`'s does not) and to say in the run's file.

use std::collections::BTreeSet;
use std::time::Duration;

use interweave_local_client_api::{
    AddressChange, AddressSource, AdminBinding, AdminCapability, AdminPort, DataSessionBinding,
    DataSessionPort as _, PeerGateView,
};
use interweave_transport_api::{DirectDestination, MessageId, TransportIdentity};
use serde_json::{Map, Value};

use crate::{DEFAULT_DEADLINE, Told, human, keys, lease_request, payload, send_until_routed};

/// How often a case reads the peer row or tries a send while it waits.
const POLL: Duration = Duration::from_millis(250);

fn ms(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

fn peer_arg(args: &Value, name: &str) -> Result<TransportIdentity, String> {
    args.get(name)
        .and_then(Value::as_str)
        .and_then(|s| TransportIdentity::parse(s).ok())
        .ok_or_else(|| format!("argument \"{name}\" is not a PeerId"))
}

fn duration_arg(args: &Value, name: &str, default: Option<Duration>) -> Result<Duration, String> {
    match (args.get(name), default) {
        (None, Some(d)) => Ok(d),
        (None, None) => Err(format!("argument \"{name}\" is missing")),
        (Some(v), _) => v
            .as_u64()
            .map(Duration::from_millis)
            .ok_or_else(|| format!("argument \"{name}\" is not a count")),
    }
}

fn bool_arg(args: &Value, name: &str) -> Result<bool, String> {
    args.get(name)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("argument \"{name}\" is not a boolean"))
}

/// Whether `address` is a `multiaddr-with-peer-id` reaching `peer`: its
/// last component names it.
fn reaches(address: &str, peer: &TransportIdentity) -> bool {
    address
        .rsplit_once("/p2p/")
        .is_some_and(|(_, id)| id == peer.as_str())
}

/// The arguments of [`crate::cases::ADDED_ADDRESS`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedAddressArgs {
    /// The trusted peer the address reaches.
    pub peer: TransportIdentity,
    /// The address, a `multiaddr-with-peer-id` ending in `peer`.
    pub address: String,
    /// Add it (the first start), or find it already added (a later one).
    pub add: bool,
    /// Before an add, how long the peer must stay unconnected: the
    /// control that nothing else dials it.
    pub before: Duration,
    /// How long the connection may take once the address is held.
    pub deadline: Duration,
    /// How long the connection must then stay up, so the far end's case,
    /// which reads its row now and then, sees it before this side ends.
    pub hold: Duration,
}

impl AddedAddressArgs {
    /// The JSON a host passes: `peer`, `address`, `add`, `before_ms`,
    /// `deadline_ms` and `hold_ms` (all three optional).
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut out = Map::new();
        out.insert("peer".to_owned(), self.peer.as_str().into());
        out.insert("address".to_owned(), self.address.clone().into());
        out.insert("add".to_owned(), self.add.into());
        out.insert("before_ms".to_owned(), ms(self.before).into());
        out.insert("deadline_ms".to_owned(), ms(self.deadline).into());
        out.insert("hold_ms".to_owned(), ms(self.hold).into());
        Value::Object(out)
    }

    /// Parse what [`to_json`](Self::to_json) wrote.
    ///
    /// # Errors
    /// A missing or malformed field, by name, or an address that does not
    /// reach `peer`.
    pub fn from_json(args: &Value) -> Result<Self, String> {
        let peer = peer_arg(args, "peer")?;
        let address = args
            .get("address")
            .and_then(Value::as_str)
            .ok_or("argument \"address\" is missing")?
            .to_owned();
        if !reaches(&address, &peer) {
            return Err(format!(
                "argument \"address\" ({address}) does not end in /p2p/{}",
                peer.as_str()
            ));
        }
        Ok(Self {
            peer,
            address,
            add: bool_arg(args, "add")?,
            before: duration_arg(args, "before_ms", Some(Duration::ZERO))?,
            deadline: duration_arg(args, "deadline_ms", Some(DEFAULT_DEADLINE))?,
            hold: duration_arg(args, "hold_ms", Some(Duration::ZERO))?,
        })
    }
}

/// The peer's row, which exists only for an allowlisted peer.
async fn row_of(
    port: &impl AdminPort,
    peer: &TransportIdentity,
) -> Result<Option<PeerGateView>, String> {
    Ok(port
        .peers()
        .await
        .map_err(|e| format!("the peer rows were refused: {e:?}"))?
        .into_iter()
        .find(|row| &row.peer == peer))
}

fn outcome_label(row: Option<&PeerGateView>) -> String {
    row.and_then(|r| r.last_outcome)
        .map_or("none", |o| o.as_str())
        .to_owned()
}

/// [`crate::cases::ADDED_ADDRESS`]'s body.
pub(crate) async fn added_address(
    binding: &impl AdminBinding,
    args: &AddedAddressArgs,
) -> Result<Map<String, Value>, String> {
    let port = binding
        .admin(BTreeSet::from([
            AdminCapability::Peers,
            AdminCapability::Status,
        ]))
        .await
        .map_err(|_| "the admin port with the peers capability was refused".to_owned())?;
    let peer = &args.peer;
    let listed = port
        .bootstrap_addresses()
        .await
        .map_err(|e| format!("the bootstrap addresses were refused: {e:?}"))?;
    // The row's claim is the overlay's: an authored entry for the peer
    // would be a second producer of the same dial.
    if let Some(authored) = listed
        .iter()
        .find(|a| a.source == AddressSource::Authored && reaches(&a.address, peer))
    {
        return Err(format!(
            "config.yaml authors {} for the peer: the dial would not be the overlay's alone",
            authored.address
        ));
    }
    let held = listed
        .iter()
        .find(|a| a.address == args.address && a.source == AddressSource::Added);
    let mut out = Map::new();
    if args.add {
        // A phone used before holds the peer's addresses from earlier runs,
        // each on the port that run chose; every one is taken out first,
        // and said, rather than the app's data cleared. The runtime may
        // have dialled them at its start: a run's port is new, so such a
        // dial reaches nothing, and the window below shows it connected
        // nothing.
        let mut removed_first = false;
        for earlier in listed
            .iter()
            .filter(|a| a.source == AddressSource::Added && reaches(&a.address, peer))
        {
            let removed = port
                .remove_bootstrap_address(earlier.address.clone())
                .await
                .map_err(|e| format!("the removal was refused: {e:?}"))?;
            if removed != AddressChange::Removed {
                return Err(format!("the removal of {}: {removed:?}", earlier.address));
            }
            removed_first = true;
        }
        out.insert(
            keys::REMOVED_FIRST.to_owned(),
            removed_first.to_string().into(),
        );
        let before_until = tokio::time::Instant::now() + args.before;
        loop {
            let row = row_of(&port, peer)
                .await?
                .ok_or_else(|| format!("no peer row for {}: is it trusted?", peer.as_str()))?;
            if row.connected {
                return Err(format!(
                    "connected before the add: something other than the add dials it: {row:?}"
                ));
            }
            if tokio::time::Instant::now() >= before_until {
                break;
            }
            tokio::time::sleep(POLL).await;
        }
        let added = port
            .add_bootstrap_address(args.address.clone())
            .await
            .map_err(|e| format!("the add was refused: {e:?}"))?;
        if added != AddressChange::Added {
            return Err(format!("the add of {}: {added:?}", args.address));
        }
    } else {
        match held {
            Some(a) if a.dialled => {}
            Some(a) => return Err(format!("listed and not dialled: {a:?}")),
            None => {
                return Err(format!(
                    "{} is not listed as added after the restart: {listed:?}",
                    args.address
                ));
            }
        }
    }
    let started = tokio::time::Instant::now();
    let until = started + args.deadline;
    loop {
        let row = row_of(&port, peer).await?;
        if row.as_ref().is_some_and(|r| r.connected) {
            break;
        }
        if tokio::time::Instant::now() >= until {
            return Err(format!(
                "not connected to {} within {:?} of holding its address: {row:?}",
                peer.as_str(),
                args.deadline
            ));
        }
        tokio::time::sleep(POLL).await;
    }
    let connected_after = started.elapsed();
    // HELD, not merely seen: on a device the case's process is the
    // runtime, and a case that returned on seeing the connection closed it
    // 51 ms later, before the far end's case read its own row (the second
    // two-phone run, 2026-10-24). The far end reads every 250 ms; the
    // hold also asks the connection to stay up, not to flap.
    let hold_until = tokio::time::Instant::now() + args.hold;
    while tokio::time::Instant::now() < hold_until {
        tokio::time::sleep(POLL).await;
        let row = row_of(&port, peer).await?;
        if !row.as_ref().is_some_and(|r| r.connected) {
            return Err(format!(
                "connected to {} and dropped inside the {:?} hold: {row:?}",
                peer.as_str(),
                args.hold
            ));
        }
    }
    out.insert(
        keys::CONNECTED_AFTER.to_owned(),
        ms(connected_after).to_string().into(),
    );
    Ok(out)
}

/// The arguments of [`crate::cases::AWAIT_PEER`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AwaitPeerArgs {
    /// The trusted peer whose dial is awaited.
    pub peer: TransportIdentity,
    /// How long the dial may take, and then how long the peer may stay
    /// before it leaves.
    pub deadline: Duration,
}

impl AwaitPeerArgs {
    /// The JSON a host passes: `peer` and `deadline_ms` (optional).
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut out = Map::new();
        out.insert("peer".to_owned(), self.peer.as_str().into());
        out.insert("deadline_ms".to_owned(), ms(self.deadline).into());
        Value::Object(out)
    }

    /// Parse what [`to_json`](Self::to_json) wrote.
    ///
    /// # Errors
    /// A missing or malformed field, by name.
    pub fn from_json(args: &Value) -> Result<Self, String> {
        Ok(Self {
            peer: peer_arg(args, "peer")?,
            deadline: duration_arg(args, "deadline_ms", Some(DEFAULT_DEADLINE))?,
        })
    }
}

/// [`crate::cases::AWAIT_PEER`]'s body: this side holds no bootstrap
/// address for the peer -- an added one from an earlier run is removed
/// first, an authored one fails the case -- so a connection that comes up
/// is the peer's dial. It ends once the peer has left again.
pub(crate) async fn await_peer(
    binding: &impl AdminBinding,
    args: &AwaitPeerArgs,
) -> Result<Map<String, Value>, String> {
    let port = binding
        .admin(BTreeSet::from([
            AdminCapability::Peers,
            AdminCapability::Status,
        ]))
        .await
        .map_err(|_| "the admin port with the peers capability was refused".to_owned())?;
    let peer = &args.peer;
    let listed = port
        .bootstrap_addresses()
        .await
        .map_err(|e| format!("the bootstrap addresses were refused: {e:?}"))?;
    if let Some(authored) = listed
        .iter()
        .find(|a| a.source == AddressSource::Authored && reaches(&a.address, peer))
    {
        return Err(format!(
            "config.yaml authors {} for the peer: a connection would not be the peer's dial alone",
            authored.address
        ));
    }
    // A phone that paired before holds the peer's address from that run;
    // it is taken out first, and said, so this side cannot dial.
    let mut removed_first = false;
    for held in listed.iter().filter(|a| reaches(&a.address, peer)) {
        let removed = port
            .remove_bootstrap_address(held.address.clone())
            .await
            .map_err(|e| format!("the removal was refused: {e:?}"))?;
        if removed != AddressChange::Removed {
            return Err(format!("the removal of {}: {removed:?}", held.address));
        }
        removed_first = true;
    }
    let started = tokio::time::Instant::now();
    let until = started + args.deadline;
    loop {
        let row = row_of(&port, peer)
            .await?
            .ok_or_else(|| format!("no peer row for {}: is it trusted?", peer.as_str()))?;
        if row.connected {
            break;
        }
        if tokio::time::Instant::now() >= until {
            return Err(format!(
                "{} did not connect within {:?}: {row:?}",
                peer.as_str(),
                args.deadline
            ));
        }
        tokio::time::sleep(POLL).await;
    }
    let connected_after = started.elapsed();
    // THIS SIDE ENDS ON THE DIALLER'S LEAVING, not on seeing it: on a
    // device the case's process is the runtime, and ending here closed
    // the connection 49 ms after it came up, before the dialler's case
    // read its row (the first two-phone run, 2026-10-24). The host ends
    // the dialler once its case has passed.
    let gone_by = tokio::time::Instant::now() + args.deadline;
    loop {
        let row = row_of(&port, peer).await?;
        if !row.as_ref().is_some_and(|r| r.connected) {
            break;
        }
        if tokio::time::Instant::now() >= gone_by {
            return Err(format!(
                "{} stayed connected {:?} after its dial: the host ends the dialler once it passes",
                peer.as_str(),
                args.deadline
            ));
        }
        tokio::time::sleep(POLL).await;
    }
    let mut out = Map::new();
    out.insert(
        keys::REMOVED_FIRST.to_owned(),
        removed_first.to_string().into(),
    );
    out.insert(
        keys::CONNECTED_AFTER.to_owned(),
        ms(connected_after).to_string().into(),
    );
    Ok(out)
}

/// What one [`crate::cases::PAIR`] step expects of the pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairExpect {
    /// No message to the peer is accepted, nor one from it read, for the
    /// whole window: the control before the trust is mutual.
    Apart(Duration),
    /// A message crosses each way within the deadline. `first` names the
    /// side that sends the close: the other ends on reading it, and the
    /// first ends only once the other has left, so neither process -- its
    /// runtime, on a device -- ends while the other still needs it.
    Crosses {
        /// How long the route may take, and then how long the other side
        /// may take to leave.
        deadline: Duration,
        /// This side sends the close and ends once the other has left;
        /// the other ends on reading the close.
        first: bool,
    },
}

/// The arguments of [`crate::cases::PAIR`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairArgs {
    /// The other phone.
    pub peer: TransportIdentity,
    /// Its address to add first, standing in for the scan; `None` once
    /// added.
    pub address: Option<String>,
    /// Trust the peer in this step, as the person does on the shown
    /// `PeerId`; otherwise it must not be trusted yet when the step
    /// expects the pair apart.
    pub trust: bool,
    /// What the step expects.
    pub expect: PairExpect,
    /// Keeps one run's messages apart from another's.
    pub serial: u8,
}

impl PairArgs {
    /// The JSON a host passes: `peer`, `address` (optional), `trust`,
    /// `serial`, and either `apart_ms`, or `deadline_ms` and `first`.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut out = Map::new();
        out.insert("peer".to_owned(), self.peer.as_str().into());
        if let Some(address) = &self.address {
            out.insert("address".to_owned(), address.clone().into());
        }
        out.insert("trust".to_owned(), self.trust.into());
        out.insert("serial".to_owned(), self.serial.into());
        match self.expect {
            PairExpect::Apart(window) => {
                out.insert("apart_ms".to_owned(), ms(window).into());
            }
            PairExpect::Crosses { deadline, first } => {
                out.insert("deadline_ms".to_owned(), ms(deadline).into());
                out.insert("first".to_owned(), first.into());
            }
        }
        Value::Object(out)
    }

    /// Parse what [`to_json`](Self::to_json) wrote.
    ///
    /// # Errors
    /// A missing or malformed field, by name; both expectations or none.
    pub fn from_json(args: &Value) -> Result<Self, String> {
        let peer = peer_arg(args, "peer")?;
        let address = match args.get("address") {
            None => None,
            Some(v) => {
                let a = v.as_str().ok_or("argument \"address\" is not text")?;
                if !reaches(a, &peer) {
                    return Err(format!(
                        "argument \"address\" ({a}) does not end in /p2p/{}",
                        peer.as_str()
                    ));
                }
                Some(a.to_owned())
            }
        };
        let expect = match (args.get("apart_ms"), args.get("deadline_ms")) {
            (Some(_), None) => PairExpect::Apart(duration_arg(args, "apart_ms", None)?),
            (None, Some(_)) => PairExpect::Crosses {
                deadline: duration_arg(args, "deadline_ms", None)?,
                first: bool_arg(args, "first")?,
            },
            _ => return Err("exactly one of \"apart_ms\" and \"deadline_ms\" is owed".to_owned()),
        };
        let serial = args
            .get("serial")
            .and_then(Value::as_u64)
            .and_then(|s| u8::try_from(s).ok())
            .filter(|s| *s < 0x40)
            .ok_or("argument \"serial\" is not a count below 64")?;
        Ok(Self {
            peer,
            address,
            trust: bool_arg(args, "trust")?,
            expect,
            serial,
        })
    }
}

/// What a side of a pair sends in step `serial`, and its id: the high bit
/// marks the second side, so the two never share one.
#[must_use]
pub fn pair_message(serial: u8, first: bool) -> (MessageId, String) {
    let (bit, side) = if first {
        (0, "first")
    } else {
        (0x80, "second")
    };
    (
        MessageId::from_bytes([serial | bit; 16]),
        format!("pair {serial} from the {side}"),
    )
}

/// The first side's close of step `serial` ([`crate::close_exchange`]'s
/// reason, between two phones).
#[must_use]
pub fn pair_close(serial: u8) -> (MessageId, String) {
    (
        MessageId::from_bytes([serial | 0x40; 16]),
        format!("pair {serial} closes"),
    )
}

fn to(peer: &TransportIdentity) -> DirectDestination {
    DirectDestination {
        peer: peer.clone(),
        endpoint: Some(human()),
    }
}

/// [`crate::cases::PAIR`]'s body.
pub(crate) async fn pair<B: DataSessionBinding + AdminBinding>(
    binding: &B,
    args: &PairArgs,
) -> Result<Map<String, Value>, String> {
    let port = binding
        .admin(BTreeSet::from([
            AdminCapability::Peers,
            AdminCapability::Status,
            AdminCapability::Trust,
        ]))
        .await
        .map_err(|_| {
            "the admin port with the peers and trust capabilities was refused".to_owned()
        })?;
    let peer = &args.peer;
    if let Some(address) = &args.address {
        let added = port
            .add_bootstrap_address(address.clone())
            .await
            .map_err(|e| format!("the add was refused: {e:?}"))?;
        if !matches!(added, AddressChange::Added | AddressChange::Unchanged) {
            return Err(format!("the add of {address}: {added:?}"));
        }
    }
    let trusted = || async {
        Ok::<bool, String>(
            port.trust()
                .await
                .map_err(|e| format!("the trust list was refused: {e:?}"))?
                .peers()
                .any(|p| p == peer),
        )
    };
    if args.trust {
        port.set_trust(peer.clone(), true)
            .await
            .map_err(|_| "the trust change was refused".to_owned())?;
    } else if matches!(args.expect, PairExpect::Apart(_)) && trusted().await? {
        // Apart before this side's trust step means not trusted yet; a
        // profile that trusts already is not fresh.
        return Err(format!(
            "{} is trusted before the trust step",
            peer.as_str()
        ));
    }
    let session = binding
        .open(lease_request())
        .await
        .map_err(|_| "the human endpoint's lease was refused".to_owned())?;
    let mut told = Told::default();
    let mut out = Map::new();
    match args.expect {
        PairExpect::Apart(window) => {
            // This side's message, as the first side's would be: whether
            // the other ever accepts it is what the window asks.
            let (id, text) = pair_message(args.serial, true);
            let until = tokio::time::Instant::now() + window;
            let mut tries = 0_u32;
            while tokio::time::Instant::now() < until {
                tries += 1;
                let sent = session.send_direct(to(peer), id, payload(&text)?).await;
                if sent.is_ok() {
                    return Err(format!(
                        "a message was accepted by {} before both trusted (try {tries})",
                        peer.as_str()
                    ));
                }
                told.read(&session, peer, "").await?;
                if let Some((_, bytes)) = told.direct.iter().find(|(from, _)| from == peer) {
                    return Err(format!(
                        "a message from {} arrived before both trusted: {:?}",
                        peer.as_str(),
                        String::from_utf8_lossy(bytes)
                    ));
                }
                tokio::time::sleep(POLL).await;
            }
            out.insert(keys::TRIES.to_owned(), tries.to_string().into());
        }
        PairExpect::Crosses { deadline, first } => {
            let (id, text) = pair_message(args.serial, first);
            send_until_routed(&session, peer, id, &text, deadline).await?;
            let (_, theirs) = pair_message(args.serial, !first);
            told.take_message(&session, peer, &theirs, deadline).await?;
            let (close_id, close) = pair_close(args.serial);
            if first {
                // ITS OUTCOME IS REPORTED, NOT RELIED ON, as
                // `close_exchange`'s: the second side ends on reading the
                // close, and its process is its runtime, so the
                // acceptance can be lost on the closing connection --
                // `TRANSPORT.md`'s "outcome unknown", which the second
                // two-phone run of row 2 met (2026-10-24). That the close
                // arrived is the second side's pass; a close never sent
                // fails there.
                let sent = session
                    .send_direct(to(peer), close_id, payload(&close)?)
                    .await;
                out.insert(
                    keys::CLOSE.to_owned(),
                    sent.map_or_else(|e| format!("{e:?}"), |_| "accepted".to_owned())
                        .into(),
                );
                // The other side may still be owed the acceptance of its
                // own message; it leaves once it has read the close.
                let gone_by = tokio::time::Instant::now() + deadline;
                while row_of(&port, peer).await?.is_some_and(|r| r.connected) {
                    if tokio::time::Instant::now() >= gone_by {
                        return Err(format!(
                            "{} stayed connected {deadline:?} after the close",
                            peer.as_str()
                        ));
                    }
                    tokio::time::sleep(POLL).await;
                }
            } else {
                told.take_message(&session, peer, &close, deadline).await?;
            }
        }
    }
    out.insert(
        keys::TRUSTED.to_owned(),
        trusted().await?.to_string().into(),
    );
    out.insert(
        keys::LAST_OUTCOME.to_owned(),
        outcome_label(row_of(&port, peer).await?.as_ref()).into(),
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn peer() -> TransportIdentity {
        TransportIdentity::parse("12D3KooWD3eckifWpRn9wQpMG9R9hX3sD158z7EqHWmweQAJU5SA")
            .expect("a PeerId")
    }

    fn other_peer() -> TransportIdentity {
        TransportIdentity::parse("12D3KooWQYhTNQdmr3ArTeUHRYzFg94BKyTkoWBDWez9kSCVe2Xo")
            .expect("a PeerId")
    }

    fn at(peer: &TransportIdentity) -> String {
        format!("/ip4/192.168.1.20/tcp/4001/p2p/{}", peer.as_str())
    }

    #[test]
    fn the_arguments_round_trip_and_name_what_is_wrong() {
        let p = peer();
        let added = AddedAddressArgs {
            peer: p.clone(),
            address: at(&p),
            add: true,
            before: Duration::from_secs(3),
            deadline: Duration::from_secs(9),
            hold: Duration::from_secs(5),
        };
        assert_eq!(AddedAddressArgs::from_json(&added.to_json()), Ok(added));
        let waited = AwaitPeerArgs {
            peer: p.clone(),
            deadline: Duration::from_secs(9),
        };
        assert_eq!(AwaitPeerArgs::from_json(&waited.to_json()), Ok(waited));
        for expect in [
            PairExpect::Apart(Duration::from_secs(5)),
            PairExpect::Crosses {
                deadline: Duration::from_secs(9),
                first: false,
            },
        ] {
            let pair = PairArgs {
                peer: p.clone(),
                address: Some(at(&p)),
                trust: true,
                expect,
                serial: 3,
            };
            assert_eq!(PairArgs::from_json(&pair.to_json()), Ok(pair));
        }

        // An address that reaches another peer is refused by name.
        let other = at(&other_peer());
        let mut wrong = AddedAddressArgs {
            peer: p.clone(),
            address: other.clone(),
            add: false,
            before: Duration::ZERO,
            deadline: Duration::ZERO,
            hold: Duration::ZERO,
        }
        .to_json();
        assert!(
            AddedAddressArgs::from_json(&wrong)
                .expect_err("another peer's address")
                .contains("does not end in")
        );
        wrong["address"] = at(&p).into();
        assert!(AddedAddressArgs::from_json(&wrong).is_ok(), "the control");

        // Exactly one expectation, and a serial below the close's bit.
        let mut both = PairArgs {
            peer: p.clone(),
            address: None,
            trust: false,
            expect: PairExpect::Apart(Duration::from_secs(1)),
            serial: 1,
        }
        .to_json();
        both["deadline_ms"] = 1_u64.into();
        assert!(
            PairArgs::from_json(&both)
                .expect_err("both")
                .contains("exactly one")
        );
        let mut high = PairArgs {
            peer: p,
            address: None,
            trust: false,
            expect: PairExpect::Apart(Duration::from_secs(1)),
            serial: 1,
        }
        .to_json();
        high["serial"] = 0x40_u64.into();
        assert!(
            PairArgs::from_json(&high)
                .expect_err("a serial on the close's bit")
                .contains("serial")
        );
    }

    #[test]
    fn a_pairs_messages_and_its_close_never_share_an_id() {
        let mut ids = BTreeSet::new();
        for serial in 0..0x40 {
            assert!(ids.insert(pair_message(serial, true).0.as_bytes().to_vec()));
            assert!(ids.insert(pair_message(serial, false).0.as_bytes().to_vec()));
            assert!(ids.insert(pair_close(serial).0.as_bytes().to_vec()));
        }
    }
}
