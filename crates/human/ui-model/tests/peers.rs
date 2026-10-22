// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Peers by address (`design/2026-10-22-add-a-peer.md`, ADR-0062) through
//! the model's public surface: what each input asks of the runtime, and
//! what each answer leaves on show. Isolation only: the runtime's
//! answers are given here as the facade would hand them over.

#![allow(clippy::expect_used, clippy::panic)]

use interweave_human_client_api::{
    AddressAnswer, AddressChangeFailure, AddressList, AddressOrigin, AddressOutcome,
    AddressProblem, AddressRefused, PeerAddress, TrustList, TrustProblem, TrustRow,
    TrustSetFailure,
};
use interweave_human_ui_model::{
    AddNote, AddResult, Barred, BlockProblem, BlockState, BlockView, CODE_LINES, Intent,
    LineSelection, ListProblem, PeersFocus, PeersInput, PeersSnack, Refusal, RowState, ScanVerdict,
    TYPED_LINES, TrustChange, TrustLine, UiModel,
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

fn input(m: &mut UiModel, i: PeersInput) -> Vec<Intent> {
    m.peers_input(i)
}

/// A model with the trust list and the address list read.
fn opened(me: &TransportIdentity, trusted: &[TransportIdentity], rows: &[PeerAddress]) -> UiModel {
    let mut m = UiModel::new();
    let asked = input(&mut m, PeersInput::Opened);
    assert_eq!(asked, vec![Intent::ReadTrust, Intent::ReadAddresses]);
    m.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(me.clone()),
        allowed: trusted.iter().cloned().map(TrustRow::added_here).collect(),
    }));
    m.addresses_read(Ok(list(rows)));
    m
}

/// The facade's answer shape, `Ok` by construction here.
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

fn focus(m: &UiModel) -> Option<PeersFocus> {
    m.peers().focus().0.cloned()
}

/// ADR-0062 decision 8: the notice keys on two facts, the address list
/// empty and the trust set empty, each read; either gaining an entry
/// hides it, and an unread or unreadable list claims nothing.
#[test]
fn the_no_reach_notice_shows_on_two_empty_lists_and_nothing_else() {
    let (me, them) = (peer(), peer());
    let mut m = UiModel::new();
    assert!(!m.no_reach(), "nothing read: no claim");

    let m2 = opened(&me, &[], &[]);
    assert!(m2.no_reach(), "both read and empty");
    assert!(
        !opened(&me, std::slice::from_ref(&them), &[]).no_reach(),
        "a trusted peer, no address"
    );
    assert!(
        !opened(&me, &[], &[added(&them)]).no_reach(),
        "an address, no trust"
    );

    let _ = input(&mut m, PeersInput::Opened);
    m.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(me),
        allowed: Vec::new(),
    }));
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert!(!m.no_reach(), "an unreadable list is unknown, not empty");
    assert_eq!(m.peers().list_problem(), Some(ListProblem::Unavailable));
}

/// A list a change may have altered since it was read is no fact for
/// the notice, though its rows stay on show as last read: a lost add
/// whose read back fails, and a Trust made whose list could not be
/// read again, each leave it hidden until a read succeeds.
#[test]
fn the_no_reach_notice_claims_nothing_on_a_list_a_change_may_have_altered() {
    let (me, them) = (peer(), peer());

    let mut m = opened(&me, &[], &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let _ = input(&mut m, PeersInput::FieldChanged(at(&them)));
    let _ = input(&mut m, PeersInput::Add);
    m.addresses_added(
        &[at(&them)],
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        )),
    );
    assert!(!m.no_reach(), "the add may have been made");
    assert_eq!(m.peers_turn(), Some(Intent::ReadAddresses));
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert!(!m.no_reach(), "its read back failed: still unknown");
    m.addresses_read(Ok(list(&[])));
    assert!(m.no_reach(), "a read that succeeds is the fact again");

    let mut t = opened(&me, &[], &[]);
    t.trust_settings_mut().set(
        TrustChange {
            peer: them.clone(),
            allowed: true,
        },
        Err(TrustSetFailure::MadeNotReadBack(TrustProblem::Unavailable)),
    );
    assert!(!t.no_reach(), "trust made, its list not read back");
    assert_eq!(
        t.trust_settings_mut().take_reread(),
        Some(Intent::ReadTrust)
    );
    t.trust_settings_mut().read(Err(TrustProblem::Unavailable));
    assert!(!t.no_reach(), "the read again failed: still unknown");
    t.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(me),
        allowed: vec![TrustRow::added_here(them)],
    }));
    assert!(!t.no_reach(), "read: a trusted peer");
}

/// A typed line, trimmed, is one add; its untrusted peer gets a block;
/// Trust on the block is the confirmed change, and once trust lists the
/// peer the page closes with the trusted snackbar, focus on its opener.
#[test]
fn a_typed_address_is_added_then_its_peer_trusted_from_its_block() {
    let (me, them) = (peer(), peer());
    let mut m = opened(&me, &[], &[]);
    assert!(input(&mut m, PeersInput::OpenAdd).is_empty());
    assert_eq!(focus(&m), Some(PeersFocus::Field { select: None }));
    let _ = input(
        &mut m,
        PeersInput::FieldChanged(format!(" \t{}  \r\n\r\n", at(&them))),
    );
    assert_eq!(
        input(&mut m, PeersInput::Add),
        vec![Intent::AddAddresses(vec![at(&them)])]
    );
    assert!(
        input(&mut m, PeersInput::Add).is_empty(),
        "one add at a time"
    );

    m.addresses_added(
        &[at(&them)],
        answer(
            AddressOutcome::Added {
                peer: Some(them.clone()),
            },
            &[added(&them)],
        ),
    );
    assert_eq!(
        m.peers().add_result(),
        Some(&AddResult::Added {
            lines: 1,
            peers: vec![them.clone()]
        })
    );
    assert_eq!(m.peers().field(), "", "the added line leaves the field");
    assert_eq!(focus(&m), Some(PeersFocus::Block(them.clone())));
    assert_eq!(
        m.peers().rows(m.trust_settings())[0].state,
        RowState::NeedsTrust,
        "an address grants no trust"
    );

    let change = TrustChange {
        peer: them.clone(),
        allowed: true,
    };
    assert_eq!(
        input(&mut m, PeersInput::Trust(them.clone())),
        vec![Intent::SetTrust(change.clone())]
    );
    assert_eq!(
        m.peers().blocks(m.trust_settings())[0].state,
        BlockState::Trusting
    );
    m.trust_settings_mut().set(
        change,
        Ok(TrustList {
            local_peer: Some(me),
            allowed: vec![TrustRow::added_here(them.clone())],
        }),
    );
    assert_eq!(m.peers_turn(), None);
    assert!(
        !m.peers().add_open(),
        "every block decided: the page closes"
    );
    assert_eq!(m.peers().snack().0, Some(&PeersSnack::PeerTrusted));
    assert_eq!(focus(&m), Some(PeersFocus::Opener));
    assert_eq!(
        m.peers().rows(m.trust_settings())[0].state,
        RowState::Trusted
    );
}

/// An empty field, and a field of more lines than one add carries, send
/// nothing: each says why, the long one selecting from the line past the
/// bound.
#[test]
fn an_empty_or_too_long_field_sends_nothing() {
    let me = peer();
    let mut m = opened(&me, &[], &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let _ = input(&mut m, PeersInput::FieldChanged(" \n\t\r\n".to_owned()));
    assert!(input(&mut m, PeersInput::Add).is_empty());
    assert_eq!(m.peers().add_page_note(), Some(&AddNote::AddressNeeded));
    let _ = input(&mut m, PeersInput::FieldChanged("x".to_owned()));
    assert_eq!(
        m.peers().add_page_note(),
        None,
        "goes once the field is not empty"
    );

    // Typed lines past TYPED_LINES, never split into single writes.
    let many: Vec<String> = (0..=TYPED_LINES).map(|_| at(&peer())).collect();
    let _ = input(&mut m, PeersInput::FieldChanged(many.join("\n")));
    assert!(input(&mut m, PeersInput::Add).is_empty());
    assert_eq!(
        m.peers().add_page_note(),
        Some(&AddNote::TooManyLines {
            lines: TYPED_LINES + 1
        })
    );
    assert_eq!(
        focus(&m),
        Some(PeersFocus::Field {
            select: Some(LineSelection::From(TYPED_LINES))
        })
    );
    assert!(!m.peers().busy(), "nothing was sent");
}

/// The client's two scan rules refuse before anything is sent, and any
/// one line refuses the code; a code that is no address list keeps the
/// viewfinder open; a scan with an address in the field is never taken.
#[test]
fn a_scan_is_taken_whole_or_refused_whole_before_it_is_sent() {
    let me = peer();
    let mut m = opened(&me, &[], &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);

    let _ = input(&mut m, PeersInput::FieldChanged(at(&peer())));
    let _ = input(&mut m, PeersInput::ScanPressed);
    assert!(!m.peers().scan_open(), "an address in the field");
    assert_eq!(m.peers().add_page_note(), Some(&AddNote::ScanAfterField));
    let _ = input(&mut m, PeersInput::FieldChanged(String::new()));
    assert_eq!(m.peers().add_page_note(), None);
    let _ = input(&mut m, PeersInput::ScanPressed);
    assert!(m.peers().scan_open());

    assert_eq!(
        m.scanned("https://example.org"),
        (ScanVerdict::NotAddress, None)
    );
    assert!(m.peers().scan_open(), "the viewfinder stays open");

    let dns = format!("/dns4/peer.example.net/tcp/4001/p2p/{}", peer().as_str());
    let code = format!("{}\n{dns}", at(&peer()));
    assert_eq!(m.scanned(&code), (ScanVerdict::Taken, None));
    assert_eq!(
        m.peers().add_page_note(),
        Some(&AddNote::Refused {
            line: dns.clone(),
            why: Refusal::DnsFromScan
        })
    );
    assert_eq!(m.peers().field(), "", "a scan never enters the field");

    let long: Vec<String> = (0..=CODE_LINES).map(|_| at(&peer())).collect();
    assert_eq!(m.scanned(&long.join("\n")), (ScanVerdict::Taken, None));
    assert!(matches!(
        m.peers().add_page_note(),
        Some(AddNote::Refused {
            why: Refusal::TooMany,
            ..
        })
    ));

    let good = vec![at(&peer()), at(&peer())];
    assert_eq!(
        m.scanned(&good.join("\r\n")),
        (ScanVerdict::Taken, Some(Intent::AddAddresses(good)))
    );

    // The same DNS line typed is the person's own: sent.
    let mut typed = opened(&me, &[], &[]);
    let _ = input(&mut typed, PeersInput::OpenAdd);
    let _ = input(&mut typed, PeersInput::FieldChanged(dns.clone()));
    assert_eq!(
        input(&mut typed, PeersInput::Add),
        vec![Intent::AddAddresses(vec![dns])]
    );
}

/// The runtime's refusals are rendered by name, nothing stored; a
/// removal's refusal answering an add is a failure, never a reason.
#[test]
fn each_runtime_refusal_is_named_and_stores_nothing() {
    let me = peer();
    for (refused, expected) in [
        (AddressRefused::Grammar, Some(Refusal::Grammar)),
        (AddressRefused::SpecialUse, Some(Refusal::SpecialUse)),
        (AddressRefused::Bound, Some(Refusal::Limit)),
        (AddressRefused::ListOff, Some(Refusal::ListOff)),
        (AddressRefused::Configured, None),
    ] {
        let mut m = opened(&me, &[], &[]);
        let _ = input(&mut m, PeersInput::OpenAdd);
        let line = "/ip4/127.0.0.1/tcp/1/p2p/x".to_owned();
        let _ = input(&mut m, PeersInput::FieldChanged(line.clone()));
        let _ = input(&mut m, PeersInput::Add);
        m.addresses_added(
            std::slice::from_ref(&line),
            answer(AddressOutcome::Refused(refused), &[]),
        );
        match expected {
            Some(why) => {
                assert_eq!(
                    m.peers().add_page_note(),
                    Some(&AddNote::Refused {
                        line: line.clone(),
                        why
                    }),
                    "{refused:?}"
                );
                assert_eq!(
                    focus(&m),
                    Some(PeersFocus::Field {
                        select: Some(LineSelection::Line(line.clone()))
                    })
                );
            }
            None => assert_eq!(m.peers().add_page_note(), Some(&AddNote::Failed)),
        }
        assert_eq!(m.peers().add_result(), None);
        assert_eq!(m.peers().field(), line, "the field is kept");
    }
}

/// §5.7's outcomes for a failed add: not written, not permitted,
/// incompatible below 2.7, and a multi-line add a 2.7/2.8 runtime does
/// not take.
#[test]
fn a_failed_add_says_what_is_true() {
    let me = peer();
    let one = vec![at(&peer())];
    let two = vec![at(&peer()), at(&peer())];
    let cases = [
        (&one, AddressProblem::Internal, AddNote::Failed),
        (&one, AddressProblem::Unavailable, AddNote::Failed),
        (
            &one,
            AddressProblem::NotPermitted,
            AddNote::Barred(Barred::NotPermitted),
        ),
        (
            &one,
            AddressProblem::Incompatible,
            AddNote::Barred(Barred::Incompatible),
        ),
        (
            &two,
            AddressProblem::Incompatible,
            AddNote::SeveralIncompatible { lines: 2 },
        ),
    ];
    for (lines, problem, note) in cases {
        let mut m = opened(&me, &[], &[]);
        let _ = input(&mut m, PeersInput::OpenAdd);
        // Several lines reach the runtime only from a scan: typed, the
        // field takes TYPED_LINES.
        let sent = if lines.len() > TYPED_LINES {
            input(&mut m, PeersInput::Scanned(lines.join("\n")))
        } else {
            let _ = input(&mut m, PeersInput::FieldChanged(lines.join("\n")));
            input(&mut m, PeersInput::Add)
        };
        assert_eq!(sent, vec![Intent::AddAddresses(lines.clone())]);
        m.addresses_added(lines, Err(AddressChangeFailure::NotMade(problem)));
        assert_eq!(m.peers().add_page_note(), Some(&note), "{problem:?}");
        assert_eq!(focus(&m), Some(PeersFocus::AddButton));
        let barred = matches!(note, AddNote::Barred(_));
        assert_eq!(
            m.peers().list_problem().is_some(),
            barred,
            "only a barred change bars the list"
        );
        assert_eq!(m.peers_turn(), None, "nothing to read back");
    }
}

/// A lost answer reads the list back before anything is said: every line
/// listed is accepted as checked, any line missing is an add not made,
/// and a failed read is unknown.
#[test]
fn a_lost_add_answer_is_decided_on_the_list_read_back() {
    let (me, them) = (peer(), peer());
    let line = at(&them);
    let lost = || {
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        ))
    };
    let start = || {
        let mut m = opened(&me, &[], &[]);
        let _ = input(&mut m, PeersInput::OpenAdd);
        let _ = input(&mut m, PeersInput::FieldChanged(line.clone()));
        let _ = input(&mut m, PeersInput::Add);
        m.addresses_added(std::slice::from_ref(&line), lost());
        assert_eq!(m.peers().add_page_note(), Some(&AddNote::Checking));
        assert_eq!(m.peers_turn(), Some(Intent::ReadAddresses));
        m
    };

    let mut listed = start();
    listed.addresses_read(Ok(list(&[added(&them)])));
    assert_eq!(
        listed.peers().add_result(),
        Some(&AddResult::Checked { lines: 1 })
    );
    assert_eq!(focus(&listed), Some(PeersFocus::Block(them.clone())));

    let mut missing = start();
    missing.addresses_read(Ok(list(&[])));
    assert_eq!(missing.peers().add_page_note(), Some(&AddNote::Failed));
    assert_eq!(missing.peers().field(), line, "the field is kept");

    // An add is all or none: a read back listing one of two lines shows
    // the other was there before and this add was not made.
    let other = peer();
    let two = vec![at(&other), line.clone()];
    let mut partly = opened(&me, &[], &[added(&other)]);
    let _ = input(&mut partly, PeersInput::OpenAdd);
    // Several lines in one add come from a scan (TYPED_LINES).
    let _ = input(&mut partly, PeersInput::Scanned(two.join("\n")));
    partly.addresses_added(&two, lost());
    let _ = partly.peers_turn();
    partly.addresses_read(Ok(list(&[added(&other)])));
    assert_eq!(partly.peers().add_page_note(), Some(&AddNote::Failed));
    assert_eq!(partly.peers().add_result(), None, "nothing shown as added");

    let mut unread = start();
    unread.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(
        unread.peers().add_page_note(),
        Some(&AddNote::Unknown { lines: 1 })
    );
    assert_eq!(focus(&unread), Some(PeersFocus::AddButton));
}

/// Remove runs at once with Undo; Undo adds the address again; a row the
/// configuration turned off has no Undo; a configured row is never
/// removed.
#[test]
fn a_removal_offers_undo_unless_the_list_is_off() {
    let (me, a, b) = (peer(), peer(), peer());
    let mut m = opened(&me, std::slice::from_ref(&a), &[added(&a), added(&b)]);

    assert_eq!(
        input(&mut m, PeersInput::Remove(at(&a))),
        vec![Intent::RemoveAddress(at(&a))]
    );
    m.address_removed(&at(&a), answer(AddressOutcome::Removed, &[added(&b)]));
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::Removed {
            address: at(&a),
            trusted: true,
            undo: true
        })
    );
    assert_eq!(focus(&m), Some(PeersFocus::Row(at(&b))), "the next row");

    assert_eq!(
        input(&mut m, PeersInput::Undo),
        vec![Intent::AddAddresses(vec![at(&a)])]
    );
    m.addresses_added(
        &[at(&a)],
        answer(
            AddressOutcome::Added {
                peer: Some(a.clone()),
            },
            &[added(&a), added(&b)],
        ),
    );
    assert_eq!(m.peers().snack().0, None);
    assert_eq!(focus(&m), Some(PeersFocus::Row(at(&a))));
    assert!(!m.peers().add_open(), "an Undo opens no add page");

    let off = PeerAddress {
        dialled: false,
        ..added(&b)
    };
    let mut m = opened(&me, &[], &[off]);
    let _ = input(&mut m, PeersInput::Remove(at(&b)));
    m.address_removed(&at(&b), answer(AddressOutcome::Removed, &[]));
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::Removed {
            address: at(&b),
            trusted: false,
            undo: false
        })
    );
    assert!(input(&mut m, PeersInput::Undo).is_empty());
    assert_eq!(focus(&m), Some(PeersFocus::AddAddress), "no row left");

    let configured = PeerAddress {
        origin: AddressOrigin::Configured,
        ..added(&a)
    };
    let mut m = opened(&me, &[], &[configured]);
    assert!(input(&mut m, PeersInput::Remove(at(&a))).is_empty());
}

/// A row's Trust opens the trust step for its peer alone: the page is
/// open with one block, and step one -- the field and Add -- is not on
/// show, so nothing drawn is a control a press would not reach. Add an
/// address opens step one.
#[test]
fn a_rows_trust_opens_the_trust_step_without_step_one() {
    let (me, a) = (peer(), peer());
    let mut m = opened(&me, &[], &[added(&a)]);
    assert!(!m.peers().step_one_shown(), "no page");

    let _ = input(&mut m, PeersInput::Trust(a.clone()));
    assert!(m.peers().add_open());
    assert_eq!(m.peers().blocks(m.trust_settings()).len(), 1);
    assert!(!m.peers().step_one_shown(), "the trust step alone");

    let mut m = opened(&me, &[], &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    assert!(m.peers().step_one_shown());
    let _ = input(&mut m, PeersInput::FieldChanged(at(&a)));
    let _ = input(&mut m, PeersInput::Add);
    m.addresses_added(
        &[at(&a)],
        answer(
            AddressOutcome::Added {
                peer: Some(a.clone()),
            },
            &[added(&a)],
        ),
    );
    assert!(!m.peers().step_one_shown(), "step one done: its result");
}

/// Focus after a removal walks the rows as Peers shows them -- added
/// newest first, then the configuration's -- never the runtime's order:
/// the next row shown, else the previous; and an Undo refused goes as
/// after the Remove it undid (§8).
#[test]
fn focus_after_a_removal_follows_the_rows_as_shown() {
    let (me, a, b, c, cfg) = (peer(), peer(), peer(), peer(), peer());
    let configured = PeerAddress {
        origin: AddressOrigin::Configured,
        ..added(&cfg)
    };
    // The runtime's order: the configuration's first, then A, B, C as
    // added. Shown: C, B, A, then the configured row.
    let rows = [configured.clone(), added(&a), added(&b), added(&c)];
    let mut m = opened(&me, &[], &rows);
    let shown: Vec<String> = m
        .peers()
        .rows(m.trust_settings())
        .into_iter()
        .map(|r| r.address)
        .collect();
    assert_eq!(shown, vec![at(&c), at(&b), at(&a), at(&cfg)]);

    let _ = input(&mut m, PeersInput::Remove(at(&b)));
    let after = [configured.clone(), added(&a), added(&c)];
    m.address_removed(&at(&b), answer(AddressOutcome::Removed, &after));
    assert_eq!(
        focus(&m),
        Some(PeersFocus::Row(at(&a))),
        "B's next row shown"
    );

    let _ = input(&mut m, PeersInput::Undo);
    m.addresses_added(
        &[at(&b)],
        answer(AddressOutcome::Refused(AddressRefused::Bound), &after),
    );
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::UndoRefused(Refusal::Limit))
    );
    assert_eq!(
        focus(&m),
        Some(PeersFocus::Row(at(&a))),
        "as after the Remove"
    );

    let _ = input(&mut m, PeersInput::SnackClosed);
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    let after = [configured, added(&c)];
    m.address_removed(&at(&a), answer(AddressOutcome::Removed, &after));
    assert_eq!(
        focus(&m),
        Some(PeersFocus::Row(at(&cfg))),
        "the oldest added row's next row shown is the configured one"
    );
}

/// An Undo the runtime refuses leaves the address removed with that
/// refusal's own words; one whose call fails offers Try again, which adds
/// it again.
#[test]
fn a_refused_or_failed_undo_leaves_the_address_removed() {
    let (me, a) = (peer(), peer());
    let removed = |m: &mut UiModel| {
        let _ = input(m, PeersInput::Remove(at(&a)));
        m.address_removed(&at(&a), answer(AddressOutcome::Removed, &[]));
        let _ = input(m, PeersInput::Undo);
    };

    let mut m = opened(&me, &[], &[added(&a)]);
    removed(&mut m);
    m.addresses_added(
        &[at(&a)],
        answer(AddressOutcome::Refused(AddressRefused::Bound), &[]),
    );
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::UndoRefused(Refusal::Limit))
    );
    assert!(input(&mut m, PeersInput::TryAgain).is_empty());

    let mut m = opened(&me, &[], &[added(&a)]);
    removed(&mut m);
    m.addresses_added(
        &[at(&a)],
        Err(AddressChangeFailure::NotMade(AddressProblem::Unavailable)),
    );
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::UndoFailed { address: at(&a) })
    );
    assert_eq!(
        input(&mut m, PeersInput::TryAgain),
        vec![Intent::AddAddresses(vec![at(&a)])]
    );
}

/// A removal not made keeps the row and offers Try again, focus on the
/// row's menu; a lost answer is decided on the list read back.
#[test]
fn a_failed_or_lost_removal_is_said_truly() {
    let (me, a) = (peer(), peer());
    let mut m = opened(&me, &[], &[added(&a)]);
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(
        &at(&a),
        Err(AddressChangeFailure::NotMade(AddressProblem::Unavailable)),
    );
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::RemoveFailed { address: at(&a) })
    );
    assert_eq!(focus(&m), Some(PeersFocus::RowMenu(at(&a))));
    assert_eq!(
        input(&mut m, PeersInput::TryAgain),
        vec![Intent::RemoveAddress(at(&a))]
    );

    let lost = || {
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        ))
    };
    let mut still = opened(&me, &[], &[added(&a)]);
    let _ = input(&mut still, PeersInput::Remove(at(&a)));
    still.address_removed(&at(&a), lost());
    assert_eq!(still.peers_turn(), Some(Intent::ReadAddresses));
    still.addresses_read(Ok(list(&[added(&a)])));
    assert_eq!(
        still.peers().snack().0,
        Some(&PeersSnack::RemoveFailed { address: at(&a) })
    );

    let mut gone = opened(&me, &[], &[added(&a)]);
    let _ = input(&mut gone, PeersInput::Remove(at(&a)));
    gone.address_removed(&at(&a), lost());
    let _ = gone.peers_turn();
    gone.addresses_read(Ok(list(&[])));
    assert!(matches!(
        gone.peers().snack().0,
        Some(PeersSnack::Removed { undo: true, .. })
    ));

    let mut unread = opened(&me, &[], &[added(&a)]);
    let _ = input(&mut unread, PeersInput::Remove(at(&a)));
    unread.address_removed(&at(&a), lost());
    let _ = unread.peers_turn();
    unread.addresses_read(Err(AddressProblem::Internal));
    assert_eq!(
        unread.peers().snack().0,
        Some(&PeersSnack::RemoveUnknown { address: at(&a) })
    );
    assert_eq!(
        unread.peers().list(),
        Some(&list(&[added(&a)])),
        "the row stays as last read"
    );
    assert_eq!(unread.peers().list_problem(), None);
}

/// Rows read newest added first, then the configuration's, each in its
/// state against the trust list.
#[test]
fn rows_are_newest_added_first_then_configured_in_their_states() {
    let (me, t, u, c) = (peer(), peer(), peer(), peer());
    let configured = PeerAddress {
        origin: AddressOrigin::Configured,
        dialled: false,
        ..added(&c)
    };
    let m = opened(
        &me,
        std::slice::from_ref(&t),
        &[configured, added(&t), added(&u)],
    );
    let rows: Vec<(String, RowState)> = m
        .peers()
        .rows(m.trust_settings())
        .into_iter()
        .map(|r| (r.address, r.state))
        .collect();
    assert_eq!(
        rows,
        vec![
            (at(&u), RowState::NeedsTrust),
            (at(&t), RowState::Trusted),
            (at(&c), RowState::ProviderOff),
        ]
    );
}

/// At the bound, or with the list off, Add an address opens nothing and
/// says why under itself.
#[test]
fn add_an_address_opens_nothing_at_the_bound_or_with_the_list_off() {
    let me = peer();
    let full: Vec<PeerAddress> = (0..64).map(|_| added(&peer())).collect();
    let mut m = opened(&me, &[], &full);
    let _ = input(&mut m, PeersInput::OpenAdd);
    assert!(!m.peers().add_open());
    assert_eq!(m.peers().add_note(), Some(Refusal::Limit));
    assert_eq!(focus(&m), Some(PeersFocus::AddAddressNote));

    let off = PeerAddress {
        dialled: false,
        ..added(&peer())
    };
    let mut m = opened(&me, &[], &[off]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    assert_eq!(m.peers().add_note(), Some(Refusal::ListOff));
}

/// Two untrusted peers in one add are two blocks; Not now on one moves
/// focus to the other; deciding both closes the page with the added
/// snackbar, since not every block was trusted. A trust write that fails
/// shows in its block, which stays undecided.
#[test]
fn several_peers_are_several_blocks_each_decided_on_its_own() {
    let (me, a, b) = (peer(), peer(), peer());
    let mut m = opened(&me, &[], &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let lines = vec![at(&a), at(&b)];
    // Several lines in one add come from a scan (TYPED_LINES).
    let _ = input(&mut m, PeersInput::Scanned(lines.join("\n")));
    m.addresses_added(
        &lines,
        answer(
            AddressOutcome::Added { peer: None },
            &[added(&a), added(&b)],
        ),
    );
    assert_eq!(
        m.peers().add_result(),
        Some(&AddResult::Added {
            lines: 2,
            peers: vec![a.clone(), b.clone()]
        })
    );

    let change = TrustChange {
        peer: a.clone(),
        allowed: true,
    };
    let _ = input(&mut m, PeersInput::Trust(a.clone()));
    m.trust_settings_mut().set(
        change,
        Err(TrustSetFailure::NotMade(TrustProblem::Unavailable)),
    );
    let _ = m.peers_turn();
    assert_eq!(
        m.peers().blocks(m.trust_settings())[0],
        BlockView {
            peer: a.clone(),
            state: BlockState::Undecided,
            problem: Some(BlockProblem::Trust(TrustProblem::Unavailable)),
        }
    );
    assert_eq!(focus(&m), Some(PeersFocus::BlockProblem(a.clone())));

    let _ = input(&mut m, PeersInput::NotNow(a.clone()));
    assert_eq!(focus(&m), Some(PeersFocus::Block(b.clone())));
    let _ = input(&mut m, PeersInput::NotNow(b));
    assert!(!m.peers().add_open());
    assert_eq!(m.peers().snack().0, Some(&PeersSnack::Added));
}

/// A peer already trusted, or this profile's own, gets no block: with
/// none, the page closes on the add with the added snackbar.
#[test]
fn an_add_for_a_trusted_peer_closes_at_once() {
    let (me, t) = (peer(), peer());
    let mut m = opened(&me, std::slice::from_ref(&t), &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let lines = vec![at(&t), at(&me)];
    // Several lines in one add come from a scan (TYPED_LINES).
    let _ = input(&mut m, PeersInput::Scanned(lines.join("\n")));
    m.addresses_added(
        &lines,
        answer(
            AddressOutcome::Added { peer: None },
            &[added(&t), added(&me)],
        ),
    );
    assert!(!m.peers().add_open());
    assert_eq!(m.peers().snack().0, Some(&PeersSnack::Added));
    assert_eq!(focus(&m), Some(PeersFocus::Opener));
}

/// Back while an add is on its way closes nothing, so the answer has a
/// page to land on; a scanned line the runtime refuses is never selected
/// in the field, which it never entered: focus goes to the note.
#[test]
fn an_add_on_its_way_keeps_its_page_and_a_refused_scan_focuses_its_note() {
    let me = peer();
    let mut m = opened(&me, &[], &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let _ = input(&mut m, PeersInput::ScanPressed);
    let line = at(&peer());
    let (_, sent) = m.scanned(&line);
    assert_eq!(sent, Some(Intent::AddAddresses(vec![line.clone()])));
    let _ = input(&mut m, PeersInput::Close);
    assert!(m.peers().add_open(), "the add is on its way");
    m.addresses_added(
        std::slice::from_ref(&line),
        answer(AddressOutcome::Refused(AddressRefused::SpecialUse), &[]),
    );
    assert_eq!(focus(&m), Some(PeersFocus::Note));
    assert_eq!(m.peers().field(), "");
    let _ = input(&mut m, PeersInput::Close);
    assert!(!m.peers().add_open(), "step one closes once answered");
    assert_eq!(focus(&m), Some(PeersFocus::Opener));
}

/// A removal answered not permitted puts Peers in its unreadable state,
/// focus on that line, with no snackbar; Try again that then reads the
/// list focuses as when Peers opens.
#[test]
fn a_barred_removal_makes_peers_unreadable_and_try_again_reopens_it() {
    let (me, a) = (peer(), peer());
    let mut m = opened(&me, &[], &[added(&a)]);
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(
        &at(&a),
        Err(AddressChangeFailure::NotMade(AddressProblem::NotPermitted)),
    );
    assert_eq!(
        m.peers().list_problem(),
        Some(ListProblem::Barred(Barred::NotPermitted))
    );
    assert_eq!(m.peers().list(), None, "nothing read is shown");
    assert_eq!(m.peers().snack().0, None);
    assert_eq!(focus(&m), Some(PeersFocus::ListProblem));

    assert_eq!(
        input(&mut m, PeersInput::ReadAgain),
        vec![Intent::ReadAddresses]
    );
    m.addresses_read(Ok(list(&[added(&a)])));
    assert_eq!(m.peers().list_problem(), None);
    assert_eq!(focus(&m), Some(PeersFocus::Title));
}

/// A removal whose answer and read back were both lost is read again
/// once its snackbar leaves.
#[test]
fn an_unknown_removal_is_read_again_when_its_snackbar_leaves() {
    let (me, a) = (peer(), peer());
    let mut m = opened(&me, &[], &[added(&a)]);
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(
        &at(&a),
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        )),
    );
    assert_eq!(m.peers_turn(), Some(Intent::ReadAddresses));
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::RemoveUnknown { address: at(&a) })
    );
    assert_eq!(m.peers_turn(), None, "not while the snackbar is up");
    let _ = input(&mut m, PeersInput::SnackClosed);
    assert_eq!(m.peers_turn(), Some(Intent::ReadAddresses));
    // That read failing too: Peers unreadable, focus on why.
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(m.peers().list_problem(), Some(ListProblem::Unavailable));
    assert_eq!(focus(&m), Some(PeersFocus::ListProblem));
}

/// A Trust whose answer was lost is checked on the trust list read
/// back: not listed, the block says the trust was not made and stays
/// undecided.
#[test]
fn a_lost_trust_answer_read_back_unlisted_is_not_made() {
    let (me, them) = (peer(), peer());
    let mut m = opened(&me, &[], &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let _ = input(&mut m, PeersInput::FieldChanged(at(&them)));
    let _ = input(&mut m, PeersInput::Add);
    m.addresses_added(
        &[at(&them)],
        answer(
            AddressOutcome::Added {
                peer: Some(them.clone()),
            },
            &[added(&them)],
        ),
    );
    let change = TrustChange {
        peer: them.clone(),
        allowed: true,
    };
    let _ = input(&mut m, PeersInput::Trust(them.clone()));
    m.trust_settings_mut().set(
        change,
        Err(TrustSetFailure::Unconfirmed(TrustProblem::Unavailable)),
    );
    let _ = m.peers_turn();
    assert_eq!(
        m.peers().blocks(m.trust_settings())[0].state,
        BlockState::Checking
    );
    assert_eq!(
        m.trust_settings_mut().take_reread(),
        Some(Intent::ReadTrust)
    );
    m.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(me),
        allowed: Vec::new(),
    }));
    let _ = m.peers_turn();
    assert_eq!(
        m.peers().blocks(m.trust_settings())[0],
        BlockView {
            peer: them.clone(),
            state: BlockState::Undecided,
            problem: Some(BlockProblem::NotMade),
        }
    );
    assert_eq!(focus(&m), Some(PeersFocus::BlockProblem(them)));
}

/// A Trust the runtime made whose trust list could not be read back
/// (`MadeNotReadBack`) is decided on the list read again, never shown
/// as not saved: listed, the block is trusted with no problem and the
/// one-block page closes; still unlisted, it is not made.
#[test]
fn a_trust_made_but_not_read_back_is_decided_on_the_list_read_again() {
    for listed in [true, false] {
        let (me, them) = (peer(), peer());
        let mut m = opened(&me, &[], &[]);
        let _ = input(&mut m, PeersInput::OpenAdd);
        let _ = input(&mut m, PeersInput::FieldChanged(at(&them)));
        let _ = input(&mut m, PeersInput::Add);
        m.addresses_added(
            &[at(&them)],
            answer(
                AddressOutcome::Added {
                    peer: Some(them.clone()),
                },
                &[added(&them)],
            ),
        );
        let change = TrustChange {
            peer: them.clone(),
            allowed: true,
        };
        let _ = input(&mut m, PeersInput::Trust(them.clone()));
        m.trust_settings_mut().set(
            change,
            Err(TrustSetFailure::MadeNotReadBack(TrustProblem::Unavailable)),
        );
        assert_eq!(
            m.trust_settings_mut().take_reread(),
            Some(Intent::ReadTrust)
        );
        let _ = m.peers_turn();
        assert_eq!(
            m.peers().blocks(m.trust_settings())[0],
            BlockView {
                peer: them.clone(),
                state: BlockState::Checking,
                problem: None,
            },
            "no verdict before the list is read again"
        );
        let allowed = if listed {
            vec![TrustRow::added_here(them.clone())]
        } else {
            Vec::new()
        };
        m.trust_settings_mut().read(Ok(TrustList {
            local_peer: Some(me),
            allowed,
        }));
        let _ = m.peers_turn();
        if listed {
            assert!(
                !m.peers().add_open(),
                "the one block decided closes the page"
            );
        } else {
            assert_eq!(
                m.peers().blocks(m.trust_settings())[0],
                BlockView {
                    peer: them.clone(),
                    state: BlockState::Undecided,
                    problem: Some(BlockProblem::NotMade),
                }
            );
            assert_eq!(focus(&m), Some(PeersFocus::BlockProblem(them.clone())));
            // A later read listing the peer leaves no problem on a
            // trusted block, and the decided page closes.
            m.trust_settings_mut().read(Ok(TrustList {
                local_peer: None,
                allowed: vec![TrustRow::added_here(them)],
            }));
            let _ = m.peers_turn();
            assert!(!m.peers().add_open(), "trusted at last, the page closes");
        }
    }
}

/// With the address list read and the trust list not the daemon's word,
/// no row claims a trust state and one line says why, with Try again
/// where a retry can mend it (§5.2's trust unread): Try again that
/// succeeds shows the states and puts focus on the title; one that fails
/// keeps the line and focus on Try again. A Remove there claims no
/// trust, and Not now on a block claims none either.
#[test]
fn an_unread_trust_list_is_said_and_no_row_guesses_its_trust() {
    let (me, t) = (peer(), peer());
    let states = |m: &UiModel| -> Vec<RowState> {
        m.peers()
            .rows(m.trust_settings())
            .into_iter()
            .map(|r| r.state)
            .collect()
    };
    let unread = || {
        let mut m = UiModel::new();
        let _ = input(&mut m, PeersInput::Opened);
        m.trust_settings_mut().read(Err(TrustProblem::Unavailable));
        m.addresses_read(Ok(list(&[added(&t)])));
        m
    };

    let mut m = unread();
    assert_eq!(states(&m), vec![RowState::TrustUnknown]);
    assert_eq!(
        m.peers().trust_line(m.trust_settings()),
        Some(TrustLine::Unread { retry: true })
    );
    assert_eq!(
        input(&mut m, PeersInput::TrustReadAgain),
        vec![Intent::ReadTrust]
    );
    m.trust_settings_mut().read(Err(TrustProblem::Unavailable));
    let _ = m.peers_turn();
    assert_eq!(focus(&m), Some(PeersFocus::TrustRetry), "failed: kept");
    let _ = input(&mut m, PeersInput::TrustReadAgain);
    m.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(me.clone()),
        allowed: vec![TrustRow::added_here(t.clone())],
    }));
    let _ = m.peers_turn();
    assert_eq!(focus(&m), Some(PeersFocus::Title), "read: as on opening");
    assert_eq!(m.peers().trust_line(m.trust_settings()), None);
    assert_eq!(states(&m), vec![RowState::Trusted]);

    // Refused: its own line, no Try again.
    let mut m = UiModel::new();
    let _ = input(&mut m, PeersInput::Opened);
    m.trust_settings_mut().read(Err(TrustProblem::NotPermitted));
    m.addresses_read(Ok(list(&[added(&t)])));
    assert_eq!(
        m.peers().trust_line(m.trust_settings()),
        Some(TrustLine::NotPermitted)
    );

    // A Trust made but not read back: the kept list is no fact either.
    let mut m = opened(&me, &[], &[added(&t)]);
    assert_eq!(states(&m), vec![RowState::NeedsTrust]);
    m.trust_settings_mut().set(
        TrustChange {
            peer: t.clone(),
            allowed: true,
        },
        Err(TrustSetFailure::MadeNotReadBack(TrustProblem::Unavailable)),
    );
    assert_eq!(states(&m), vec![RowState::TrustUnknown]);
    assert_eq!(
        m.peers().trust_line(m.trust_settings()),
        Some(TrustLine::Unread { retry: true })
    );

    // A Remove while trust is unread claims no trust, though the list as
    // last read held the peer.
    let mut m = opened(&me, std::slice::from_ref(&t), &[added(&t)]);
    let _ = input(&mut m, PeersInput::Opened);
    m.trust_settings_mut().read(Err(TrustProblem::Unavailable));
    let _ = input(&mut m, PeersInput::Remove(at(&t)));
    m.address_removed(&at(&t), answer(AddressOutcome::Removed, &[]));
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::Removed {
            address: at(&t),
            trusted: false,
            undo: true
        })
    );

    // Not now on a block whose peer's trust is not known: two peers, so
    // the page stays open on the decided block.
    let mut m = unread();
    let _ = input(&mut m, PeersInput::OpenAdd);
    let (u, v) = (peer(), peer());
    let two = vec![at(&u), at(&v)];
    let _ = input(&mut m, PeersInput::Scanned(two.join("\n")));
    m.addresses_added(
        &two,
        answer(
            AddressOutcome::Added { peer: None },
            &[added(&t), added(&u), added(&v)],
        ),
    );
    let _ = input(&mut m, PeersInput::NotNow(u.clone()));
    let blocks = m.peers().blocks(m.trust_settings());
    assert_eq!(blocks[0].peer, u);
    assert_eq!(blocks[0].state, BlockState::LaterUnknown);

    // The address list unreadable too: its line stands alone.
    let mut m = UiModel::new();
    let _ = input(&mut m, PeersInput::Opened);
    m.trust_settings_mut().read(Err(TrustProblem::Unavailable));
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(m.peers().trust_line(m.trust_settings()), None);
}

/// A trust list kept after a failed read is no fact for step two either:
/// an add of a peer it holds still gets its block, and a row's Trust
/// opens an undecided block whose Trust is proposed and whose Not now
/// claims no trust -- never "Trusted" or "Peer trusted" from that list.
#[test]
fn a_kept_trust_list_decides_no_block() {
    let (me, p) = (peer(), peer());
    let stale = || {
        let mut m = opened(&me, std::slice::from_ref(&p), &[added(&p)]);
        let _ = input(&mut m, PeersInput::Opened);
        m.trust_settings_mut().read(Err(TrustProblem::Unavailable));
        assert!(m.trust_settings().list().is_some_and(|l| l.allows(&p)));
        m
    };

    // An add of another address of P: its block, undecided.
    let mut m = stale();
    let other = format!("/ip4/192.168.1.21/tcp/4001/p2p/{}", p.as_str());
    let _ = input(&mut m, PeersInput::OpenAdd);
    let _ = input(&mut m, PeersInput::FieldChanged(other.clone()));
    let _ = input(&mut m, PeersInput::Add);
    let mut second = added(&p);
    second.address = other.clone();
    m.addresses_added(
        std::slice::from_ref(&other),
        answer(
            AddressOutcome::Added {
                peer: Some(p.clone()),
            },
            &[added(&p), second],
        ),
    );
    let blocks = m.peers().blocks(m.trust_settings());
    assert_eq!(blocks.len(), 1, "P's block, though the kept list holds P");
    assert_eq!(blocks[0].state, BlockState::Undecided);

    // A row's Trust: the step reads the set itself, the block undecided,
    // its Trust proposed, Not now unknown.
    let mut m = stale();
    assert_eq!(
        input(&mut m, PeersInput::Trust(p.clone())),
        vec![Intent::ReadTrust]
    );
    assert_eq!(
        m.peers().blocks(m.trust_settings())[0].state,
        BlockState::Undecided
    );
    let asked = input(&mut m, PeersInput::Trust(p.clone()));
    assert_eq!(
        asked,
        vec![Intent::SetTrust(TrustChange {
            peer: p.clone(),
            allowed: true,
        })],
        "the block's Trust goes to the daemon"
    );
    let mut m = stale();
    let _ = input(&mut m, PeersInput::Trust(p.clone()));
    let _ = input(&mut m, PeersInput::NotNow(p.clone()));
    assert_ne!(m.peers().snack().0, Some(&PeersSnack::PeerTrusted));
}

/// What a block knows of its peer is its own: in a step of two blocks,
/// one Trust answered as made stays Trusted when the other's answer is
/// lost and the trust list goes unconfirmed, and Done says so.
#[test]
fn one_blocks_lost_trust_leaves_anothers_known_answer() {
    let (me, a, b) = (peer(), peer(), peer());
    let mut m = opened(&me, &[], &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let two = vec![at(&a), at(&b)];
    let _ = input(&mut m, PeersInput::Scanned(two.join("\n")));
    m.addresses_added(
        &two,
        answer(
            AddressOutcome::Added { peer: None },
            &[added(&a), added(&b)],
        ),
    );
    let _ = input(&mut m, PeersInput::Trust(a.clone()));
    m.trust_settings_mut().set(
        TrustChange {
            peer: a.clone(),
            allowed: true,
        },
        Ok(TrustList {
            local_peer: Some(me),
            allowed: vec![TrustRow::added_here(a.clone())],
        }),
    );
    let _ = m.peers_turn();
    let _ = input(&mut m, PeersInput::Trust(b.clone()));
    m.trust_settings_mut().set(
        TrustChange {
            peer: b.clone(),
            allowed: true,
        },
        Err(TrustSetFailure::Unconfirmed(TrustProblem::Unavailable)),
    );
    let _ = m.peers_turn();
    let blocks = m.peers().blocks(m.trust_settings());
    assert_eq!(blocks[0].state, BlockState::Trusted, "A's answer stands");
    assert_eq!(blocks[1].state, BlockState::Checking);
}
