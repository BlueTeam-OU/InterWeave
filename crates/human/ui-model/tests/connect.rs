// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Onboarding's Connect (`design/2026-10-22-add-a-peer.md` §5.1) through
//! the model's public surface: when a start opens, settles or defers it,
//! the line its visit's acts leave, and where focus returns. Isolation
//! only: the runtime's answers are given here as the facade would hand
//! them over.

#![allow(clippy::expect_used, clippy::panic)]

use interweave_human_client_api::{
    AddressAnswer, AddressChangeFailure, AddressList, AddressOrigin, AddressOutcome,
    AddressProblem, AddressRefused, PeerAddress, ShowAddresses, TrustList, TrustProblem, TrustRow,
};
use interweave_human_ui_model::{
    AddResult, ConnectFocus, ConnectGate, ConnectInput, ConnectLine, Intent, PeersFocus,
    PeersInput, TrustChange, UiModel,
};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::TransportIdentity;

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("a peer")
}

fn at(p: &TransportIdentity) -> String {
    format!("/ip4/192.168.1.20/tcp/4001/p2p/{}", p.as_str())
}

fn added(p: &TransportIdentity) -> PeerAddress {
    PeerAddress {
        address: at(p),
        peer: Some(p.clone()),
        origin: AddressOrigin::AddedHere,
        dialled: true,
    }
}

fn list(rows: &[PeerAddress]) -> AddressList {
    AddressList {
        addresses: rows.to_vec(),
    }
}

fn trust_list(me: &TransportIdentity, trusted: &[TransportIdentity]) -> TrustList {
    TrustList {
        local_peer: Some(me.clone()),
        allowed: trusted.iter().cloned().map(TrustRow::added_here).collect(),
    }
}

/// The start's two reads, as the root asks them once a session is ready.
fn start_reads(m: &mut UiModel) {
    assert_eq!(m.read_addresses(), Some(Intent::ReadAddresses));
    assert_eq!(m.trust_settings_mut().opened(), Some(Intent::ReadTrust));
}

/// An owed start whose two reads found both lists empty: Connect open.
fn connect_open(me: &TransportIdentity) -> UiModel {
    let mut m = UiModel::new();
    m.connect_owed();
    start_reads(&mut m);
    m.trust_settings_mut().read(Ok(trust_list(me, &[])));
    m.addresses_read(Ok(list(&[])));
    assert_eq!(m.connect_turn(), Some(ConnectGate::Open));
    m
}

#[allow(clippy::unnecessary_wraps)]
fn answer(
    outcome: AddressOutcome,
    rows: &[PeerAddress],
) -> Result<AddressAnswer, AddressChangeFailure> {
    Ok(AddressAnswer {
        outcome,
        list: Ok(list(rows)),
    })
}

/// Add from Connect, typed, answered added with `rows` the list after.
fn add_from_connect(m: &mut UiModel, them: &TransportIdentity, rows: &[PeerAddress]) {
    assert!(m.connect_input(ConnectInput::Add).is_empty());
    assert!(m.peers().add_open());
    let _ = m.peers_input(PeersInput::FieldChanged(at(them)));
    assert_eq!(
        m.peers_input(PeersInput::Add),
        vec![Intent::AddAddresses(vec![at(them)])]
    );
    m.addresses_added(
        &[at(them)],
        answer(
            AddressOutcome::Added {
                peer: Some(them.clone()),
            },
            rows,
        ),
    );
}

fn line(m: &UiModel) -> Option<ConnectLine> {
    m.connect().expect("Connect open").line()
}

fn connect_focus(m: &UiModel) -> Option<ConnectFocus> {
    m.connect().expect("Connect open").focus().0
}

/// §5.1: Connect opens only when both reads succeed on two empty lists;
/// either list with an entry settles it, a failed read defers it to the
/// next start, nothing is said while a read is out, and each is said
/// once.
#[test]
fn a_start_opens_settles_or_defers_connect_on_its_two_reads() {
    let (me, them) = (peer(), peer());

    let mut not_owed = UiModel::new();
    start_reads(&mut not_owed);
    not_owed.trust_settings_mut().read(Ok(trust_list(&me, &[])));
    not_owed.addresses_read(Ok(list(&[])));
    assert_eq!(not_owed.connect_turn(), None, "not owed: never opens");
    assert!(not_owed.connect().is_none());

    let mut m = connect_open(&me);
    assert_eq!(m.connect_turn(), None, "said once");
    assert_eq!(connect_focus(&m), Some(ConnectFocus::Title));
    assert_eq!(line(&m), None, "nothing done yet: no line");

    let mut pending = UiModel::new();
    pending.connect_owed();
    start_reads(&mut pending);
    pending.trust_settings_mut().read(Ok(trust_list(&me, &[])));
    assert_eq!(pending.connect_turn(), None, "the list still unread");
    pending.addresses_read(Ok(list(&[])));
    assert_eq!(pending.connect_turn(), Some(ConnectGate::Open));

    let mut has_address = UiModel::new();
    has_address.connect_owed();
    start_reads(&mut has_address);
    has_address
        .trust_settings_mut()
        .read(Ok(trust_list(&me, &[])));
    has_address.addresses_read(Ok(list(&[added(&them)])));
    assert_eq!(has_address.connect_turn(), Some(ConnectGate::Settled));
    assert_eq!(has_address.connect_turn(), None);
    assert!(has_address.connect().is_none());

    let mut has_trust = UiModel::new();
    has_trust.connect_owed();
    start_reads(&mut has_trust);
    has_trust
        .trust_settings_mut()
        .read(Ok(trust_list(&me, std::slice::from_ref(&them))));
    has_trust.addresses_read(Ok(list(&[])));
    assert_eq!(has_trust.connect_turn(), Some(ConnectGate::Settled));

    let mut list_failed = UiModel::new();
    list_failed.connect_owed();
    start_reads(&mut list_failed);
    list_failed
        .trust_settings_mut()
        .read(Ok(trust_list(&me, &[])));
    list_failed.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(list_failed.connect_turn(), Some(ConnectGate::Deferred));
    assert_eq!(list_failed.connect_turn(), None);
    assert!(list_failed.connect().is_none());
    assert!(!list_failed.no_reach(), "and no notice either (§5.6)");

    let mut trust_failed = UiModel::new();
    trust_failed.connect_owed();
    start_reads(&mut trust_failed);
    trust_failed
        .trust_settings_mut()
        .read(Err(TrustProblem::Unavailable));
    trust_failed.addresses_read(Ok(list(&[])));
    assert_eq!(trust_failed.connect_turn(), Some(ConnectGate::Deferred));
}

/// Not now, Continue and Back are one input: Connect goes, and the next
/// turn settles it, once.
#[test]
fn leaving_connect_settles_it() {
    let me = peer();
    let mut m = connect_open(&me);
    assert!(m.connect_input(ConnectInput::Leave).is_empty());
    assert!(m.connect().is_none());
    assert_eq!(m.connect_turn(), Some(ConnectGate::Settled));
    assert_eq!(m.connect_turn(), None);
    assert!(m.no_reach(), "Chats then shows the no-reach notice");
}

/// An add of an untrusted peer, step two's Not now: the line says the
/// peer is not trusted yet and offers Trust, focus back on Add. Trust
/// reopens step two; once it is made the line says trusted and focus,
/// with Trust gone, lands on Continue.
#[test]
fn an_add_left_untrusted_offers_trust_until_it_is_made() {
    let (me, them) = (peer(), peer());
    let mut m = connect_open(&me);
    add_from_connect(&mut m, &them, &[added(&them)]);
    assert_eq!(
        m.peers().focus().0,
        Some(PeersFocus::Block(them.clone())),
        "step two"
    );
    assert_eq!(line(&m), None, "nothing said until the page closes");
    let _ = m.peers_input(PeersInput::NotNow(them.clone()));
    assert!(!m.peers().add_open());
    assert_eq!(line(&m), Some(ConnectLine::NeedsTrust));
    assert!(ConnectLine::NeedsTrust.offers_trust());
    assert_eq!(connect_focus(&m), Some(ConnectFocus::Add));

    assert!(m.connect_input(ConnectInput::Trust).is_empty());
    assert!(m.peers().add_open(), "step two again");
    assert_eq!(m.peers().focus().0, Some(PeersFocus::Block(them.clone())));
    let change = TrustChange {
        peer: them.clone(),
        allowed: true,
    };
    assert_eq!(
        m.peers_input(PeersInput::Trust(them.clone())),
        vec![Intent::SetTrust(change.clone())]
    );
    m.trust_settings_mut()
        .set(change, Ok(trust_list(&me, std::slice::from_ref(&them))));
    assert_eq!(m.peers_turn(), None);
    assert!(!m.peers().add_open());
    assert_eq!(line(&m), Some(ConnectLine::Trusted));
    assert_eq!(connect_focus(&m), Some(ConnectFocus::Continue));
    assert!(
        m.connect_input(ConnectInput::Trust).is_empty() && !m.peers().add_open(),
        "nothing left to trust: Trust opens nothing"
    );
}

/// On a page opened from Connect an add answered unchanged reads as
/// checked, never "Nothing was added" (§5.3), counts as added, and the
/// page closes with no snackbar; from Peers it stays unchanged.
#[test]
fn an_unchanged_add_from_connect_reads_checked_and_counts() {
    let (me, them) = (peer(), peer());
    let mut m = UiModel::new();
    m.connect_owed();
    start_reads(&mut m);
    m.trust_settings_mut().read(Ok(trust_list(&me, &[])));
    m.addresses_read(Ok(list(&[])));
    assert_eq!(m.connect_turn(), Some(ConnectGate::Open));
    let _ = m.connect_input(ConnectInput::Add);
    let _ = m.peers_input(PeersInput::FieldChanged(at(&them)));
    let _ = m.peers_input(PeersInput::Add);
    m.addresses_added(
        &[at(&them)],
        answer(AddressOutcome::Unchanged, &[added(&them)]),
    );
    assert_eq!(
        m.peers().add_result(),
        Some(&AddResult::Checked { lines: 1 })
    );
    let _ = m.peers_input(PeersInput::Close);
    assert_eq!(m.peers().snack().0, None, "Connect's line says it");
    assert_eq!(line(&m), Some(ConnectLine::NeedsTrust), "counted as added");

    // Control: the same answer on a page Peers opened.
    let mut p = UiModel::new();
    let _ = p.peers_input(PeersInput::Opened);
    p.trust_settings_mut().read(Ok(trust_list(&me, &[])));
    p.addresses_read(Ok(list(&[added(&them)])));
    let _ = p.peers_input(PeersInput::OpenAdd);
    let _ = p.peers_input(PeersInput::FieldChanged(at(&them)));
    let _ = p.peers_input(PeersInput::Add);
    p.addresses_added(
        &[at(&them)],
        answer(AddressOutcome::Unchanged, &[added(&them)]),
    );
    assert_eq!(
        p.peers().add_result(),
        Some(&AddResult::Unchanged { lines: 1 })
    );
}

/// Show that drew this device's addresses leaves `ConnectDoneShown`;
/// one that drew none leaves no line; an add wins over Show.
#[test]
fn show_alone_says_shown_and_an_add_wins_over_it() {
    let (me, them) = (peer(), peer());
    let mut m = connect_open(&me);
    assert_eq!(
        m.connect_input(ConnectInput::Show),
        vec![Intent::ReadOwnAddresses]
    );
    m.own_addresses_read(Ok(ShowAddresses {
        peer: me.clone(),
        addresses: vec![],
        truncated: false,
    }));
    let _ = m.peers_input(PeersInput::CloseShow);
    assert_eq!(line(&m), None, "no address drawn: no line");
    assert_eq!(connect_focus(&m), Some(ConnectFocus::Show));

    let _ = m.connect_input(ConnectInput::Show);
    m.own_addresses_read(Ok(ShowAddresses {
        peer: me.clone(),
        addresses: vec![at(&me)],
        truncated: false,
    }));
    let _ = m.peers_input(PeersInput::CloseShow);
    assert_eq!(line(&m), Some(ConnectLine::Shown));

    // An add of a peer already trusted: trusted, and it wins.
    let mut t = UiModel::new();
    t.connect_owed();
    start_reads(&mut t);
    t.trust_settings_mut().read(Ok(trust_list(&me, &[])));
    t.addresses_read(Ok(list(&[])));
    let _ = t.connect_turn();
    let _ = t.connect_input(ConnectInput::Show);
    t.own_addresses_read(Ok(ShowAddresses {
        peer: me.clone(),
        addresses: vec![at(&me)],
        truncated: false,
    }));
    let _ = t.peers_input(PeersInput::CloseShow);
    add_from_connect(&mut t, &them, &[added(&them)]);
    let _ = t.peers_input(PeersInput::NotNow(them));
    assert_eq!(line(&t), Some(ConnectLine::NeedsTrust));
}

/// Each return from a page opened from Connect that did something §5.1
/// counts is said once, its words unchanged included, and closes with no
/// snackbar; a return that did nothing counted is not said, an earlier
/// line still showing included.
#[test]
fn a_return_is_said_only_when_its_page_did_something_counted() {
    let (me, them, other) = (peer(), peer(), peer());
    let mut m = connect_open(&me);
    let said = |m: &UiModel| m.connect().expect("Connect open").said();
    let show = |m: &mut UiModel, addresses: Vec<String>| {
        let _ = m.connect_input(ConnectInput::Show);
        m.own_addresses_read(Ok(ShowAddresses {
            peer: me.clone(),
            addresses,
            truncated: false,
        }));
        let _ = m.peers_input(PeersInput::CloseShow);
    };
    let start = said(&m);

    show(&mut m, vec![]);
    assert_eq!(said(&m), start, "a Show that drew nothing");
    let _ = m.connect_input(ConnectInput::Add);
    let _ = m.peers_input(PeersInput::Close);
    assert_eq!(said(&m), start, "Back from Add with nothing added");
    let _ = m.connect_input(ConnectInput::Add);
    let _ = m.peers_input(PeersInput::FieldChanged(at(&other)));
    let _ = m.peers_input(PeersInput::Add);
    m.addresses_added(
        &[at(&other)],
        answer(AddressOutcome::Refused(AddressRefused::Grammar), &[]),
    );
    let _ = m.peers_input(PeersInput::Close);
    assert_eq!(said(&m), start, "a refusal");
    assert_eq!(line(&m), None);

    show(&mut m, vec![at(&me)]);
    assert_eq!(said(&m), start + 1, "a Show that drew this device's");
    show(&mut m, vec![at(&me)]);
    assert_eq!(said(&m), start + 2, "again, the same line");
    assert_eq!(line(&m), Some(ConnectLine::Shown));

    add_from_connect(&mut m, &them, &[added(&them)]);
    let _ = m.peers_input(PeersInput::NotNow(them.clone()));
    assert_eq!(said(&m), start + 3, "an add, step two Not now");
    assert_eq!(m.peers().snack().0, None, "no snackbar from Connect");

    assert!(m.connect_input(ConnectInput::Trust).is_empty());
    assert!(m.peers().add_open(), "step two reopened");
    let _ = m.peers_input(PeersInput::Close);
    assert_eq!(said(&m), start + 4, "Back on step two is Done");
    assert_eq!(m.peers().snack().0, None);

    let _ = m.connect_input(ConnectInput::Add);
    let _ = m.peers_input(PeersInput::FieldChanged(at(&other)));
    let _ = m.peers_input(PeersInput::Add);
    let _ = m.peers_input(PeersInput::Close);
    assert_eq!(said(&m), start + 4, "an add whose answer is unknown");
    assert_eq!(line(&m), Some(ConnectLine::NeedsTrust), "the earlier line");
}

/// Back while the add is on its way: the page closes at once, and the add
/// adds nothing to the line (§5.3).
#[test]
fn back_on_an_add_on_its_way_adds_nothing_to_the_line() {
    let (me, them) = (peer(), peer());
    let mut m = connect_open(&me);
    let _ = m.connect_input(ConnectInput::Add);
    let _ = m.peers_input(PeersInput::FieldChanged(at(&them)));
    let _ = m.peers_input(PeersInput::Add);
    let _ = m.peers_input(PeersInput::Close);
    assert!(!m.peers().add_open());
    assert_eq!(connect_focus(&m), Some(ConnectFocus::Add));
    m.addresses_added(
        &[at(&them)],
        answer(
            AddressOutcome::Added {
                peer: Some(them.clone()),
            },
            &[added(&them)],
        ),
    );
    let _ = m.peers_turn();
    assert_eq!(line(&m), None);
}

/// A step-two Trust still awaited when Done closes the page leaves its
/// peer's trust not known: `ConnectDoneTrustUnknown`, never
/// `ConnectDoneNeedsTrust` (acceptance 10). A peer known untrusted from
/// an earlier add of the visit wins over it (§5.1's precedence).
#[test]
fn a_trust_awaited_at_done_is_unknown_and_needs_trust_wins() {
    let (me, them, other) = (peer(), peer(), peer());
    let mut m = connect_open(&me);
    add_from_connect(&mut m, &them, &[added(&them)]);
    let _ = m.peers_input(PeersInput::Trust(them.clone()));
    let _ = m.peers_input(PeersInput::Close);
    assert!(!m.peers().add_open());
    assert_eq!(line(&m), Some(ConnectLine::TrustUnknown));
    assert!(ConnectLine::TrustUnknown.offers_trust());

    let mut w = connect_open(&me);
    add_from_connect(&mut w, &other, &[added(&other)]);
    let _ = w.peers_input(PeersInput::NotNow(other.clone()));
    assert_eq!(line(&w), Some(ConnectLine::NeedsTrust));
    add_from_connect(&mut w, &them, &[added(&other), added(&them)]);
    let _ = w.peers_input(PeersInput::Trust(them.clone()));
    let _ = w.peers_input(PeersInput::Close);
    assert_eq!(line(&w), Some(ConnectLine::NeedsTrust), "still to do wins");
    assert_eq!(
        w.connect().expect("open").to_trust(),
        vec![other, them],
        "Trust offers both, in the order added"
    );
}

/// A Trust whose answer was lost as its page closed leaves the line not
/// known; the answer landing later moves the line to what it says --
/// trusted, Trust gone -- at the next turn, and a press of the line's
/// Trust before that turn opens nothing on a peer the list now allows.
/// A refusal read back the same way says not trusted.
#[test]
fn a_late_trust_answer_moves_connects_line() {
    let (me, them) = (peer(), peer());
    let change = TrustChange {
        peer: them.clone(),
        allowed: true,
    };
    let lost = |m: &mut UiModel| {
        add_from_connect(m, &them, &[added(&them)]);
        assert_eq!(
            m.peers_input(PeersInput::Trust(them.clone())),
            vec![Intent::SetTrust(change.clone())]
        );
        let _ = m.peers_input(PeersInput::Close);
        assert_eq!(line(m), Some(ConnectLine::TrustUnknown));
        assert_eq!(m.connect_turn(), None);
        assert_eq!(line(m), Some(ConnectLine::TrustUnknown), "still lost");
    };

    let mut m = connect_open(&me);
    lost(&mut m);
    m.trust_settings_mut().set(
        change.clone(),
        Ok(trust_list(&me, std::slice::from_ref(&them))),
    );
    assert_eq!(m.connect_turn(), None);
    assert_eq!(line(&m), Some(ConnectLine::Trusted), "the answer read");
    assert!(m.connect().expect("open").to_trust().is_empty());

    let mut p = connect_open(&me);
    lost(&mut p);
    p.trust_settings_mut().set(
        change.clone(),
        Ok(trust_list(&me, std::slice::from_ref(&them))),
    );
    assert!(p.connect_input(ConnectInput::Trust).is_empty());
    assert!(!p.peers().add_open(), "nothing to reopen");
    assert_eq!(line(&p), Some(ConnectLine::Trusted), "read at the press");

    let mut n = connect_open(&me);
    lost(&mut n);
    n.trust_settings_mut().set(change, Ok(trust_list(&me, &[])));
    assert_eq!(n.connect_turn(), None);
    assert_eq!(line(&n), Some(ConnectLine::NeedsTrust), "read not trusted");
}

/// A press reaching Connect while a page it opened is up opens nothing.
#[test]
fn connect_opens_one_page_at_a_time() {
    let me = peer();
    let mut m = connect_open(&me);
    let _ = m.connect_input(ConnectInput::Add);
    assert!(m.connect_input(ConnectInput::Show).is_empty());
    assert!(m.peers().show().is_none());
    assert!(m.connect_input(ConnectInput::Leave).is_empty());
    assert!(m.connect().is_some(), "Leave too waits for the page");
}
