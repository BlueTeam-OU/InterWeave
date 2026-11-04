// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! TM-45 (`tests/SECURITY-MATRIX.md`): the shipped client's own log is no
//! shadow archive. The desktop client writes one log, its stderr, and
//! posts no platform notification -- it has no notification code; an
//! arrival is told inside its own window -- so its half of the row is
//! that log: no message body, sent or received, and no name or field of
//! a card ever appears in it.
//!
//! Exercised: a send committed before the start and delivered, a text and
//! a signed card received and committed unread, the window drawing them,
//! and a clean close. Not exercised: the failure paths' lines, each a
//! class (`app.rs`), and the daemon's log, which is p2p-network-dev's.

use std::time::Duration;

use crate::common::human;
use crate::harness::{self as app, until_lease};
use crate::world::{self, Peer, envelope, two_daemons, until_rows};

/// Words that appear nowhere but in the content they are planted in, so
/// a match in the log can only be that content.
const SENT: &str = "tm45sentbodyquokka";
const RECEIVED: &str = "tm45receivedbodynarwhal";
const NAME: &str = "Tm45Cardnameaxolotl";
const SURNAME: &str = "Tm45Cardsurnamepangolin";

/// Mutation: `app.rs`'s pump printing a received message's text fails
/// it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_body_name_or_card_field_reaches_the_clients_log() {
    use ed25519_dalek::Signer as _;
    let world = two_daemons().await;
    let sent = envelope(45, &format!("the outbound {SENT}"));
    world::seed_pending(&world.a, &world.b_peer, &human(), &sent);
    let mut client = app::start(&world.a);
    until_lease(&world.a.binding(), true, &client).await;
    let mut peer = Peer::new(&world.b);
    peer.until("B ready", || world.logs(), Peer::is_ready).await;

    let received = envelope(46, &format!("the inbound {RECEIVED}"));
    peer.deliver(&world.a_peer, Some(human()), &received, || world.logs())
        .await;
    let key = ed25519_dalek::SigningKey::from_bytes(&[45; 32]);
    let card = interweave_human_chat_protocol::build_card(
        &format!("{:032x}", 47),
        interweave_human_chat_protocol::CardDraft {
            name: NAME,
            surname: Some(SURNAME),
            photo: None,
        },
        key.verifying_key().to_bytes(),
        interweave_human_chat_protocol::CardRoute {
            source_peer: &world.b_peer,
            source_endpoint: &human(),
        },
        Some(world::wall_ms()),
        |m| key.sign(m).to_bytes(),
    )
    .expect("a card within bounds");
    peer.send_card(&world.a_peer, human(), &card).await;
    peer.until(
        "the pending send reaching B",
        || format!("{}\n{}", client.log(), world.logs()),
        |p| p.got(&sent.app_message_id),
    )
    .await;
    until_rows(&world.a, "unread_inbound", 2, || client.log()).await;
    // A few turns of the window's loop over what it was handed.
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(client.terminate().success(), "{}", client.log());

    let log = client.log();
    // The control: the log was written, and read, through all of it.
    assert!(log.contains("human-desktop: session"), "a live log: {log}");
    for word in [SENT, RECEIVED, NAME, SURNAME] {
        assert!(!log.contains(word), "{word:?} in the client's log:\n{log}");
    }
}
