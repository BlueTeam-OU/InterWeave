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
    m.peers().focus().0
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
        let barred = matches!(note, AddNote::Barred(_));
        // Not permitted or incompatible: the line under Add; else Add,
        // to add again (§8).
        let want = if barred {
            PeersFocus::AddNote
        } else {
            PeersFocus::AddButton
        };
        assert_eq!(focus(&m), Some(want), "{problem:?}");
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
    assert_eq!(
        focus(&m),
        Some(PeersFocus::Row(at(&b))),
        "the row shown before it: b, newest, is shown first"
    );

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

    let _ = input(&mut m, PeersInput::TrustRow(at(&a)));
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
        focus(&unread),
        Some(PeersFocus::SnackAction),
        "on the snackbar's Try again"
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

/// A peer already trusted gets no block: with none, the page closes on
/// the add with the added snackbar. (This profile's own address never
/// reaches an add: `this_devices_own_address_is_refused_before_sending`.)
#[test]
fn an_add_for_a_trusted_peer_closes_at_once() {
    let (me, t, u) = (peer(), peer(), peer());
    let mut m = opened(&me, &[t.clone(), u.clone()], &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let lines = vec![at(&t), at(&u)];
    // Several lines in one add come from a scan (TYPED_LINES).
    let _ = input(&mut m, PeersInput::Scanned(lines.join("\n")));
    m.addresses_added(
        &lines,
        answer(
            AddressOutcome::Added { peer: None },
            &[added(&t), added(&u)],
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

/// A Trust the runtime answered as made, though its trust list could
/// not be read back (`MadeNotReadBack`), is known and trusted (§5.3):
/// never shown as not saved, and the one-block page closes on it, with
/// no read back to wait for.
#[test]
fn a_trust_made_but_not_read_back_is_trusted() {
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
    let _ = input(&mut m, PeersInput::Trust(them.clone()));
    m.trust_settings_mut().set(
        TrustChange {
            peer: them,
            allowed: true,
        },
        Err(TrustSetFailure::MadeNotReadBack(TrustProblem::Unavailable)),
    );
    let _ = m.peers_turn();
    assert!(!m.peers().add_open(), "decided: the page closes");
    assert_eq!(m.peers().snack().0, Some(&PeersSnack::PeerTrusted));
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
        input(&mut m, PeersInput::TrustRow(at(&p))),
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
    let _ = input(&mut m, PeersInput::TrustRow(at(&p)));
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

/// A Try again on a failed Undo that fails again goes as the first did,
/// to the row after the removed one (section 8), not the first row shown.
#[test]
fn a_second_failed_undo_keeps_its_focus() {
    let (me, a, b, c) = (peer(), peer(), peer(), peer());
    // Shown C, B, A.
    let mut m = opened(&me, &[], &[added(&a), added(&b), added(&c)]);
    let _ = input(&mut m, PeersInput::Remove(at(&b)));
    let after = [added(&a), added(&c)];
    m.address_removed(&at(&b), answer(AddressOutcome::Removed, &after));
    assert_eq!(focus(&m), Some(PeersFocus::Row(at(&a))));
    let failed = || Err(AddressChangeFailure::NotMade(AddressProblem::Unavailable));
    let _ = input(&mut m, PeersInput::Undo);
    m.addresses_added(&[at(&b)], failed());
    assert_eq!(
        focus(&m),
        Some(PeersFocus::Row(at(&a))),
        "the first failure"
    );
    assert_eq!(
        input(&mut m, PeersInput::TryAgain),
        vec![Intent::AddAddresses(vec![at(&b)])]
    );
    m.addresses_added(&[at(&b)], failed());
    assert_eq!(
        focus(&m),
        Some(PeersFocus::Row(at(&a))),
        "failed again: as the first"
    );
}

/// An Undo whose answer is lost and whose read back fails is unknown:
/// its snackbar offers Try again, and focus goes there (section 8).
#[test]
fn an_unknown_undo_focuses_its_try_again() {
    let (me, a) = (peer(), peer());
    let mut m = opened(&me, &[], &[added(&a)]);
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(&at(&a), answer(AddressOutcome::Removed, &[]));
    let _ = input(&mut m, PeersInput::Undo);
    m.addresses_added(
        &[at(&a)],
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        )),
    );
    assert_eq!(m.peers_turn(), Some(Intent::ReadAddresses));
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::UndoUnknown { address: at(&a) })
    );
    assert_eq!(focus(&m), Some(PeersFocus::SnackAction));
}

/// A block a trust read shows trusted is decided as its own Trust would
/// be: a row's trust step opened with trust unread closes once its read
/// lists the peer, focus on that row; and an add's block made before the
/// opening read answered settles the same way (§5.3).
#[test]
fn a_block_a_read_shows_trusted_settles_its_step() {
    let (me, p) = (peer(), peer());
    let listing = |m: &mut UiModel| {
        m.trust_settings_mut().read(Ok(TrustList {
            local_peer: Some(me.clone()),
            allowed: vec![TrustRow::added_here(p.clone())],
        }));
        let _ = m.peers_turn();
    };

    let mut m = UiModel::new();
    let _ = input(&mut m, PeersInput::Opened);
    m.trust_settings_mut().read(Err(TrustProblem::Unavailable));
    m.addresses_read(Ok(list(&[added(&p)])));
    assert_eq!(
        input(&mut m, PeersInput::TrustRow(at(&p))),
        vec![Intent::ReadTrust]
    );
    listing(&mut m);
    assert!(!m.peers().add_open(), "decided by the read: closed");
    assert_eq!(focus(&m), Some(PeersFocus::Row(at(&p))));

    // The add answered before the opening trust read.
    let mut m = UiModel::new();
    let _ = input(&mut m, PeersInput::Opened);
    m.addresses_read(Ok(list(&[])));
    let _ = input(&mut m, PeersInput::OpenAdd);
    let _ = input(&mut m, PeersInput::FieldChanged(at(&p)));
    let _ = input(&mut m, PeersInput::Add);
    m.addresses_added(
        &[at(&p)],
        answer(
            AddressOutcome::Added {
                peer: Some(p.clone()),
            },
            &[added(&p)],
        ),
    );
    assert_eq!(m.peers().blocks(m.trust_settings()).len(), 1);
    listing(&mut m);
    assert!(!m.peers().add_open(), "the opening read decided it");
}

/// Two rows name one peer: the trust step a row's Trust opens returns
/// focus to that row, not the other -- here the newest, shown first and
/// last in the runtime's order (§8).
#[test]
fn a_rows_trust_returns_focus_to_that_row() {
    let (me, p) = (peer(), peer());
    let mut second = added(&p);
    second.address = format!("/ip4/192.168.1.21/tcp/4001/p2p/{}", p.as_str());
    let mut m = opened(&me, &[], &[added(&p), second.clone()]);
    let rows = m.peers().rows(m.trust_settings());
    assert_eq!(rows[0].address, second.address, "newest first");
    let _ = input(&mut m, PeersInput::TrustRow(second.address.clone()));
    let _ = input(&mut m, PeersInput::NotNow(p.clone()));
    assert_eq!(
        focus(&m),
        Some(PeersFocus::Row(second.address)),
        "the row shown first, whose Trust it was"
    );
}

/// An Undo answered as added whose list could not be read back focuses
/// the restored row once a read lists it -- not before, when no such row
/// is drawn.
#[test]
fn an_undo_restored_unread_focuses_its_row_once_read() {
    let (me, a, b) = (peer(), peer(), peer());
    let mut m = opened(&me, &[], &[added(&a), added(&b)]);
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(&at(&a), answer(AddressOutcome::Removed, &[added(&b)]));
    let _ = input(&mut m, PeersInput::Undo);
    let before = m.peers().focus().1;
    m.addresses_added(
        &[at(&a)],
        Ok(AddressAnswer {
            outcome: AddressOutcome::Added {
                peer: Some(a.clone()),
            },
            list: Err(AddressProblem::Unavailable),
        }),
    );
    assert_eq!(m.peers().focus().1, before, "no row to focus yet");
    assert_eq!(m.peers_turn(), Some(Intent::ReadAddresses));
    m.addresses_read(Ok(list(&[added(&a), added(&b)])));
    assert_eq!(focus(&m), Some(PeersFocus::Row(at(&a))));
}

/// A trust step opened while the trust line's Try again is on its way
/// keeps its focus when that read answers: the title is the page's.
#[test]
fn a_trust_retry_answered_under_a_trust_step_keeps_its_focus() {
    let (me, p) = (peer(), peer());
    let mut m = UiModel::new();
    let _ = input(&mut m, PeersInput::Opened);
    m.trust_settings_mut().read(Err(TrustProblem::Unavailable));
    m.addresses_read(Ok(list(&[added(&p)])));
    assert_eq!(
        input(&mut m, PeersInput::TrustReadAgain),
        vec![Intent::ReadTrust]
    );
    let _ = input(&mut m, PeersInput::TrustRow(at(&p)));
    m.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(me),
        allowed: Vec::new(),
    }));
    let _ = m.peers_turn();
    assert_eq!(focus(&m), Some(PeersFocus::Block(p)));
}

/// A restored row parked for focus is dropped with the list a failed
/// read takes away: the list's Try again that succeeds then goes to
/// Peers' title, as on opening (§8), not to that row.
#[test]
fn a_parked_restored_row_gives_way_to_the_lists_try_again() {
    let (me, a, b) = (peer(), peer(), peer());
    let mut m = opened(&me, &[], &[added(&a), added(&b)]);
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(&at(&a), answer(AddressOutcome::Removed, &[added(&b)]));
    let _ = input(&mut m, PeersInput::Undo);
    m.addresses_added(
        &[at(&a)],
        Ok(AddressAnswer {
            outcome: AddressOutcome::Added {
                peer: Some(a.clone()),
            },
            list: Err(AddressProblem::Unavailable),
        }),
    );
    assert_eq!(m.peers_turn(), Some(Intent::ReadAddresses));
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(focus(&m), Some(PeersFocus::ListProblem));
    assert_eq!(
        input(&mut m, PeersInput::ReadAgain),
        vec![Intent::ReadAddresses]
    );
    m.addresses_read(Ok(list(&[added(&a), added(&b)])));
    assert_eq!(focus(&m), Some(PeersFocus::Title));
}

/// The add page's one closing snackbar says what step one did (§5.3):
/// an add that added nothing closes on its own unchanged line, with no
/// step two or after Not now; an add read back as checked closes on the
/// checked line; never "added" unless it read so.
#[test]
fn the_closing_snackbar_says_what_the_add_did() {
    let (me, t, u) = (peer(), peer(), peer());
    // Every line already there, its peer trusted: no step two.
    let mut m = opened(&me, std::slice::from_ref(&t), &[added(&t)]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let _ = input(&mut m, PeersInput::FieldChanged(at(&t)));
    let _ = input(&mut m, PeersInput::Add);
    m.addresses_added(&[at(&t)], answer(AddressOutcome::Unchanged, &[added(&t)]));
    assert!(!m.peers().add_open());
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::AddedUnchanged { lines: 1 })
    );

    // Already there, its peer not trusted: Not now closes on the same.
    let mut m = opened(&me, &[], &[added(&u)]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let _ = input(&mut m, PeersInput::FieldChanged(at(&u)));
    let _ = input(&mut m, PeersInput::Add);
    m.addresses_added(&[at(&u)], answer(AddressOutcome::Unchanged, &[added(&u)]));
    let _ = input(&mut m, PeersInput::NotNow(u.clone()));
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::AddedUnchanged { lines: 1 })
    );

    // A lost answer read back as listed: checked, not "added".
    let mut m = opened(&me, std::slice::from_ref(&t), &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let _ = input(&mut m, PeersInput::FieldChanged(at(&t)));
    let _ = input(&mut m, PeersInput::Add);
    m.addresses_added(
        &[at(&t)],
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        )),
    );
    assert_eq!(m.peers_turn(), Some(Intent::ReadAddresses));
    m.addresses_read(Ok(list(&[added(&t)])));
    assert!(!m.peers().add_open());
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::AddedChecked { lines: 1 })
    );
}

/// A row naming this device reads as this device's own address, with no
/// trust state and no Trust, whatever the trust list says -- read or
/// unread -- and its Trust opens no step (§5.2, §5.3).
#[test]
fn this_devices_own_row_offers_no_trust() {
    let me = peer();
    let state = |m: &UiModel| m.peers().rows(m.trust_settings())[0].state;
    let mut m = opened(&me, &[], &[added(&me)]);
    assert_eq!(state(&m), RowState::ThisDevice);
    assert!(input(&mut m, PeersInput::TrustRow(at(&me))).is_empty());
    assert!(!m.peers().add_open(), "no trust step for this device");
    let _ = input(&mut m, PeersInput::Opened);
    m.trust_settings_mut().read(Err(TrustProblem::Unavailable));
    assert_eq!(state(&m), RowState::ThisDevice, "trust unread too");
}

/// A row whose peer a step-two Trust was just answered as made for reads
/// Trusted though the trust list is not read back yet (§5.2), and the
/// list read back speaks for it from then on.
#[test]
fn a_row_trusted_from_this_page_reads_trusted_before_the_read_back() {
    let (me, them) = (peer(), peer());
    let state = |m: &UiModel| m.peers().rows(m.trust_settings())[0].state;
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
    let _ = input(&mut m, PeersInput::Trust(them.clone()));
    m.trust_settings_mut().set(
        TrustChange {
            peer: them.clone(),
            allowed: true,
        },
        Err(TrustSetFailure::MadeNotReadBack(TrustProblem::Unavailable)),
    );
    let _ = m.peers_turn();
    assert!(!m.peers().add_open());
    assert_eq!(state(&m), RowState::Trusted, "just confirmed here");
    assert_eq!(
        m.trust_settings_mut().take_reread(),
        Some(Intent::ReadTrust)
    );
    // Revoked elsewhere meanwhile: the list read back is the word.
    m.trust_settings_mut().read(Ok(TrustList {
        local_peer: Some(me),
        allowed: Vec::new(),
    }));
    let _ = m.peers_turn();
    assert_eq!(state(&m), RowState::NeedsTrust);
    // And a later failed read leaves it unknown, never Trusted again.
    m.trust_settings_mut().read(Err(TrustProblem::Unavailable));
    let _ = m.peers_turn();
    assert_eq!(state(&m), RowState::TrustUnknown);
}

/// The trust line's Try again answered with a failure no retry mends --
/// not permitted, or incompatible -- leaves no Try again drawn, so focus
/// goes to the line that says why (§8), never to the gone control.
#[test]
fn a_trust_retry_refused_focuses_the_line() {
    let p = peer();
    for refused in [TrustProblem::NotPermitted, TrustProblem::Incompatible] {
        let mut m = UiModel::new();
        let _ = input(&mut m, PeersInput::Opened);
        m.trust_settings_mut().read(Err(TrustProblem::Unavailable));
        m.addresses_read(Ok(list(&[added(&p)])));
        assert_eq!(
            input(&mut m, PeersInput::TrustReadAgain),
            vec![Intent::ReadTrust]
        );
        m.trust_settings_mut().read(Err(refused));
        let _ = m.peers_turn();
        assert_eq!(focus(&m), Some(PeersFocus::TrustLine), "{refused:?}");
    }
}

/// With no trust read ever answered, the session's `PeerId` still names
/// this device's own row: `ThisDevice`, and no trust step from it.
#[test]
fn this_device_is_known_from_the_session_without_trust() {
    let me = peer();
    let mut m = UiModel::new();
    let _ = input(&mut m, PeersInput::Opened);
    m.trust_settings_mut().read(Err(TrustProblem::Unavailable));
    m.this_device(me.clone());
    m.addresses_read(Ok(list(&[added(&me)])));
    assert!(m.trust_settings().list().is_none(), "trust never read");
    assert_eq!(
        m.peers().rows(m.trust_settings())[0].state,
        RowState::ThisDevice
    );
    assert!(input(&mut m, PeersInput::TrustRow(at(&me))).is_empty());
    assert!(!m.peers().add_open());
}

/// Unreadable Peers (§5.2, §8): Try again reads both lists while trust
/// is unread; a Try again that fails -- barred or not -- puts focus on
/// the problem line; a failed opening read leaves it on the title.
#[test]
fn unreadable_peers_try_again_focus_and_reads() {
    let me = peer();
    let mut m = UiModel::new();
    let _ = input(&mut m, PeersInput::Opened);
    m.trust_settings_mut().read(Err(TrustProblem::Unavailable));
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(
        focus(&m),
        Some(PeersFocus::Title),
        "the opening read failing"
    );
    assert_eq!(
        input(&mut m, PeersInput::ReadAgain),
        vec![Intent::ReadAddresses, Intent::ReadTrust]
    );
    m.addresses_read(Err(AddressProblem::NotPermitted));
    assert_eq!(focus(&m), Some(PeersFocus::ListProblem), "barred retry");

    // A kept list whose opening read fails: focus stays on the title.
    let mut m = opened(&me, &[], &[]);
    let _ = input(&mut m, PeersInput::Opened);
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(focus(&m), Some(PeersFocus::Title));
    let _ = input(&mut m, PeersInput::ReadAgain);
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(focus(&m), Some(PeersFocus::ListProblem), "failed retry");
}

/// Step two's focus (§8): Trust pressed goes to the block's waiting
/// line while its answer is awaited; unconfirmed, to its state line; with nothing
/// undecided while another Trust is awaited, to Done; and a Trust made
/// never sends focus to an earlier block's problem line.
#[test]
fn step_two_focus_follows_each_blocks_own_answer() {
    let (me, a, b) = (peer(), peer(), peer());
    let two = vec![at(&a), at(&b)];
    let start = || {
        let mut m = opened(&me, &[], &[]);
        let _ = input(&mut m, PeersInput::OpenAdd);
        let _ = input(&mut m, PeersInput::Scanned(two.join("\n")));
        m.addresses_added(
            &two,
            answer(
                AddressOutcome::Added { peer: None },
                &[added(&a), added(&b)],
            ),
        );
        m
    };
    let change = |p: &TransportIdentity| TrustChange {
        peer: p.clone(),
        allowed: true,
    };

    let mut m = start();
    let _ = input(&mut m, PeersInput::Trust(b.clone()));
    assert_eq!(
        focus(&m),
        Some(PeersFocus::BlockState(b.clone())),
        "awaited: its waiting line"
    );
    assert_eq!(
        m.peers().blocks(m.trust_settings())[1].state,
        BlockState::Trusting
    );
    let _ = input(&mut m, PeersInput::NotNow(a.clone()));
    assert_eq!(
        focus(&m),
        Some(PeersFocus::Done),
        "none undecided, B awaited"
    );
    m.trust_settings_mut().set(
        change(&b),
        Err(TrustSetFailure::Unconfirmed(TrustProblem::Unavailable)),
    );
    let _ = m.peers_turn();
    assert_eq!(
        focus(&m),
        Some(PeersFocus::BlockState(b.clone())),
        "unconfirmed"
    );

    // B's Trust fails, then A's is made: A's next is B's title.
    let mut m = start();
    let _ = input(&mut m, PeersInput::Trust(b.clone()));
    m.trust_settings_mut().set(
        change(&b),
        Err(TrustSetFailure::NotMade(TrustProblem::Unavailable)),
    );
    let _ = m.peers_turn();
    assert_eq!(focus(&m), Some(PeersFocus::BlockProblem(b.clone())));
    let _ = input(&mut m, PeersInput::Trust(a.clone()));
    m.trust_settings_mut().set(
        change(&a),
        Ok(TrustList {
            local_peer: Some(me.clone()),
            allowed: vec![TrustRow::added_here(a.clone())],
        }),
    );
    let _ = m.peers_turn();
    assert_eq!(
        focus(&m),
        Some(PeersFocus::Block(b.clone())),
        "never B's old problem"
    );

    // Not now on a block whose Trust was pressed keeps its last line,
    // and its Trust (§5.3): A still undecided keeps the page open.
    let mut m = start();
    let _ = input(&mut m, PeersInput::Trust(b.clone()));
    m.trust_settings_mut().set(
        change(&b),
        Err(TrustSetFailure::NotMade(TrustProblem::Unavailable)),
    );
    let _ = m.peers_turn();
    let _ = input(&mut m, PeersInput::NotNow(b.clone()));
    let blocks = m.peers().blocks(m.trust_settings());
    assert_eq!(blocks[1].peer, b);
    assert_eq!(blocks[1].state, BlockState::Later);
    assert_eq!(
        blocks[1].problem,
        Some(BlockProblem::Trust(TrustProblem::Unavailable)),
        "its last line stands"
    );
}

/// A Try again of an unknown change that writes nothing leaves it
/// unknown (§5.7): the snackbar stays, never "still removed" or "still
/// in the list", and focus stays on its Try again.
#[test]
fn an_unknown_change_stays_unknown_after_a_retry_that_writes_nothing() {
    let (me, a, b) = (peer(), peer(), peer());
    let lost = || {
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        ))
    };
    let not_written = || Err(AddressChangeFailure::NotMade(AddressProblem::Unavailable));

    let mut m = opened(&me, &[], &[added(&a), added(&b)]);
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(&at(&a), lost());
    let _ = m.peers_turn();
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::RemoveUnknown { address: at(&a) })
    );
    let said = m.peers().snack().1;
    let _ = input(&mut m, PeersInput::TryAgain);
    m.address_removed(&at(&a), not_written());
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::RemoveUnknown { address: at(&a) })
    );
    assert_ne!(
        m.peers().snack().1,
        said,
        "said again, for the screen reader"
    );
    assert_eq!(focus(&m), Some(PeersFocus::SnackAction));

    let mut m = opened(&me, &[], &[added(&a), added(&b)]);
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(&at(&a), answer(AddressOutcome::Removed, &[added(&b)]));
    let _ = input(&mut m, PeersInput::Undo);
    m.addresses_added(&[at(&a)], lost());
    let _ = m.peers_turn();
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::UndoUnknown { address: at(&a) })
    );
    let _ = input(&mut m, PeersInput::TryAgain);
    m.addresses_added(&[at(&a)], not_written());
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::UndoUnknown { address: at(&a) })
    );
}

/// A focus target not drawn because Peers became unreadable meanwhile
/// goes to the problem line (§8): here a decided step two whose list a
/// failed re-read took away.
#[test]
fn a_target_gone_with_the_list_focuses_the_problem_line() {
    let (me, t) = (peer(), peer());
    let mut m = opened(&me, &[], &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let _ = input(&mut m, PeersInput::FieldChanged(at(&t)));
    let _ = input(&mut m, PeersInput::Add);
    m.addresses_added(
        &[at(&t)],
        Ok(AddressAnswer {
            outcome: AddressOutcome::Added {
                peer: Some(t.clone()),
            },
            list: Err(AddressProblem::Unavailable),
        }),
    );
    assert_eq!(m.peers_turn(), Some(Intent::ReadAddresses));
    m.addresses_read(Err(AddressProblem::Unavailable));
    let _ = input(&mut m, PeersInput::NotNow(t));
    assert!(!m.peers().add_open());
    assert_eq!(focus(&m), Some(PeersFocus::ListProblem));
}

/// With both the session's and the trust list's `PeerId` known, the
/// session's names this device's row: it is the one a ready session
/// gave, and the list as kept could be from a read before it.
#[test]
fn the_sessions_peer_id_names_this_device_over_the_trust_lists() {
    let (me, other) = (peer(), peer());
    // The trust list as read names `other` as local; the session, `me`.
    let mut m = opened(&other, &[], &[added(&me), added(&other)]);
    m.this_device(me.clone());
    let states: Vec<(String, RowState)> = m
        .peers()
        .rows(m.trust_settings())
        .into_iter()
        .map(|r| (r.address, r.state))
        .collect();
    assert!(
        states.contains(&(at(&me), RowState::ThisDevice)),
        "{states:?}"
    );
    assert!(
        states.contains(&(at(&other), RowState::NeedsTrust)),
        "{states:?}"
    );
}

/// An add left unknown -- its answer lost, its read back failed -- stays
/// unknown when Add again on the same line writes nothing, said again;
/// an add of another line that writes nothing says so of itself (§5.7).
#[test]
fn an_unknown_add_stays_unknown_after_a_retry_that_writes_nothing() {
    let (me, a, b) = (peer(), peer(), peer());
    let not_written = || Err(AddressChangeFailure::NotMade(AddressProblem::Unavailable));
    let mut m = opened(&me, &[], &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let _ = input(&mut m, PeersInput::FieldChanged(at(&a)));
    let _ = input(&mut m, PeersInput::Add);
    m.addresses_added(
        &[at(&a)],
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        )),
    );
    assert_eq!(m.peers_turn(), Some(Intent::ReadAddresses));
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(
        m.peers().add_page_note(),
        Some(&AddNote::Unknown { lines: 1 })
    );
    let said = m.peers().add_note_said().1;

    assert_eq!(
        input(&mut m, PeersInput::Add),
        vec![Intent::AddAddresses(vec![at(&a)])]
    );
    m.addresses_added(&[at(&a)], not_written());
    assert_eq!(
        m.peers().add_page_note(),
        Some(&AddNote::Unknown { lines: 1 }),
        "never AddFailed: the first add may have landed"
    );
    assert_ne!(m.peers().add_note_said().1, said, "said again");
    assert_eq!(focus(&m), Some(PeersFocus::AddButton));

    let _ = input(&mut m, PeersInput::FieldChanged(at(&b)));
    let _ = input(&mut m, PeersInput::Add);
    m.addresses_added(&[at(&b)], not_written());
    assert_eq!(
        m.peers().add_page_note(),
        Some(&AddNote::Failed),
        "of itself"
    );
}

/// The unknown stays with the change, not the snackbar (§5.7): a Try
/// again whose snackbar closed -- timed out, or swiped away -- before its
/// answer wrote nothing still says the change is unknown, shown anew,
/// focus on its Try again; for an Undo as for a removal.
#[test]
fn an_unknown_change_outlives_its_snackbar_closing_mid_retry() {
    let (me, a, b) = (peer(), peer(), peer());
    let lost = || {
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        ))
    };
    let not_written = || Err(AddressChangeFailure::NotMade(AddressProblem::Unavailable));

    let mut m = opened(&me, &[], &[added(&a), added(&b)]);
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(&at(&a), answer(AddressOutcome::Removed, &[added(&b)]));
    let _ = input(&mut m, PeersInput::Undo);
    m.addresses_added(&[at(&a)], lost());
    let _ = m.peers_turn();
    m.addresses_read(Err(AddressProblem::Unavailable));
    let _ = input(&mut m, PeersInput::TryAgain);
    let _ = input(&mut m, PeersInput::SnackClosed);
    assert_eq!(m.peers().snack().0, None, "closed mid-retry");
    m.addresses_added(&[at(&a)], not_written());
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::UndoUnknown { address: at(&a) }),
        "never UndoFailed"
    );
    assert_eq!(focus(&m), Some(PeersFocus::SnackAction));

    let mut m = opened(&me, &[], &[added(&a), added(&b)]);
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(&at(&a), lost());
    let _ = m.peers_turn();
    m.addresses_read(Err(AddressProblem::Unavailable));
    let _ = input(&mut m, PeersInput::TryAgain);
    let _ = input(&mut m, PeersInput::SnackClosed);
    m.address_removed(&at(&a), not_written());
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::RemoveUnknown { address: at(&a) }),
        "never RemoveFailed"
    );

    // Until a read shows it: the list read back lists the row, so it is
    // known again, and a removal that writes nothing says so.
    let _ = input(&mut m, PeersInput::SnackClosed);
    assert_eq!(m.peers_turn(), Some(Intent::ReadAddresses));
    m.addresses_read(Ok(list(&[added(&a), added(&b)])));
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(&at(&a), not_written());
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::RemoveFailed { address: at(&a) }),
        "known again after the read"
    );
}

/// A block whose Trust was left unknown keeps that line when its Trust
/// again writes nothing, focus on it (§5.7); never the Trust page's
/// "Nothing was changed".
#[test]
fn an_unknown_trust_stays_unknown_after_a_retry_that_writes_nothing() {
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
        change.clone(),
        Err(TrustSetFailure::Unconfirmed(TrustProblem::Unavailable)),
    );
    let _ = m.peers_turn();
    assert_eq!(
        m.trust_settings_mut().take_reread(),
        Some(Intent::ReadTrust)
    );
    m.trust_settings_mut().read(Err(TrustProblem::Unavailable));
    let _ = m.peers_turn();
    assert_eq!(
        m.peers().blocks(m.trust_settings())[0].problem,
        Some(BlockProblem::Unknown)
    );

    let _ = input(&mut m, PeersInput::Trust(them.clone()));
    m.trust_settings_mut().set(
        change,
        Err(TrustSetFailure::NotMade(TrustProblem::Unavailable)),
    );
    let _ = m.peers_turn();
    assert_eq!(
        m.peers().blocks(m.trust_settings())[0].problem,
        Some(BlockProblem::Unknown),
        "still unknown: the first Trust may have landed"
    );
    assert_eq!(focus(&m), Some(PeersFocus::BlockProblem(them.clone())));

    // Refused is an answer, not a no-write: it says so.
    let _ = input(&mut m, PeersInput::Trust(them.clone()));
    m.trust_settings_mut().set(
        TrustChange {
            peer: them.clone(),
            allowed: true,
        },
        Err(TrustSetFailure::NotMade(TrustProblem::Refused)),
    );
    let _ = m.peers_turn();
    assert_eq!(
        m.peers().blocks(m.trust_settings())[0].problem,
        Some(BlockProblem::Trust(TrustProblem::Refused))
    );
}

/// While a Trust on a `PeerId` is pending -- its answer lost, no read that
/// started after it has succeeded -- a Trust on it that writes nothing,
/// wherever it is made, is a retry of an unknown change and reads
/// unknown (§5.3, §5.7). Another peer's not-written Trust says so.
#[test]
fn a_rows_trust_after_a_lost_trust_on_its_peer_is_unknown() {
    let (me, p, q) = (peer(), peer(), peer());
    let mut m = opened(&me, &[], &[added(&p), added(&q)]);
    // The Trust page's change for P was lost, its read back failed.
    m.trust_settings_mut().set(
        TrustChange {
            peer: p.clone(),
            allowed: true,
        },
        Err(TrustSetFailure::Unconfirmed(TrustProblem::Unavailable)),
    );
    let _ = m.trust_settings_mut().take_reread();
    m.trust_settings_mut().read(Err(TrustProblem::Unavailable));
    assert!(
        m.trust_settings().is_pending(&p),
        "pending: lost, not read back"
    );

    let _ = input(&mut m, PeersInput::TrustRow(at(&p)));
    let _ = input(&mut m, PeersInput::Trust(p.clone()));
    m.trust_settings_mut().set(
        TrustChange {
            peer: p.clone(),
            allowed: true,
        },
        Err(TrustSetFailure::NotMade(TrustProblem::Unavailable)),
    );
    let _ = m.peers_turn();
    assert_eq!(
        m.peers().blocks(m.trust_settings())[0].problem,
        Some(BlockProblem::Unknown),
        "still unknown"
    );

    // Q's not-written Trust is no retry: it says so of itself.
    let _ = input(&mut m, PeersInput::NotNow(p.clone()));
    let _ = input(&mut m, PeersInput::TrustRow(at(&q)));
    let _ = input(&mut m, PeersInput::Trust(q.clone()));
    m.trust_settings_mut().set(
        TrustChange {
            peer: q.clone(),
            allowed: true,
        },
        Err(TrustSetFailure::NotMade(TrustProblem::Unavailable)),
    );
    let _ = m.peers_turn();
    assert_eq!(
        m.peers().blocks(m.trust_settings())[0].problem,
        Some(BlockProblem::Trust(TrustProblem::Unavailable))
    );
}

/// A read ends only the Trusts sent before it started: one on its way
/// when a Trust was lost leaves that Trust pending; one started after
/// ends it (§5.3).
#[test]
fn a_read_ends_only_the_trusts_sent_before_it() {
    let (me, p) = (peer(), peer());
    let mut m = opened(&me, &[], &[added(&p)]);
    let t = m.trust_settings_mut();
    assert_eq!(t.opened(), Some(Intent::ReadTrust), "a read on its way");
    let change = t.propose_allow_of(&p).expect("proposed");
    assert_eq!(
        t.confirm(&change),
        Some(Intent::SetTrust(change.clone())),
        "sent while the read is on its way"
    );
    t.set(
        change,
        Err(TrustSetFailure::Unconfirmed(TrustProblem::Unavailable)),
    );
    let empty = || {
        Ok(TrustList {
            local_peer: None,
            allowed: Vec::new(),
        })
    };
    t.read(empty());
    assert!(
        t.is_pending(&p),
        "that read started before the Trust was lost"
    );
    assert_eq!(t.opened(), Some(Intent::ReadTrust));
    t.read(empty());
    assert!(!t.is_pending(&p), "a read started after it ends it");

    // Lost again, then a Trust answered as made: known, no longer pending.
    let lose = |t: &mut interweave_human_ui_model::TrustSettings, answer| {
        let change = t.propose_allow_of(&p).expect("proposed");
        let _ = t.confirm(&change);
        t.set(change, answer);
    };
    lose(
        t,
        Err(TrustSetFailure::Unconfirmed(TrustProblem::Unavailable)),
    );
    assert!(t.is_pending(&p));
    lose(
        t,
        Err(TrustSetFailure::MadeNotReadBack(TrustProblem::Unavailable)),
    );
    assert!(!t.is_pending(&p), "answered as made");
}

/// The contact names on a row's `PeerId` (§5.2): every contact of the
/// identity in front with a route on it, one name each, in order --
/// case set aside, then the exact name -- and none for another `PeerId`.
#[test]
fn a_rows_contact_names_are_every_contact_on_its_peer_in_order() {
    use interweave_human_ui_model::{ContactView, RouteView};
    use interweave_transport_api::EndpointId;
    let (p, q) = (peer(), peer());
    let contact = |id: &str, name: &str, on: &TransportIdentity, endpoint: &str| ContactView {
        contact_id: id.to_owned(),
        name: name.to_owned(),
        name_from_card: true,
        photo_from_card: None,
        routes: vec![RouteView {
            peer: on.clone(),
            endpoint: EndpointId::parse(endpoint).expect("valid"),
            fingerprint: None,
        }],
    };
    let mut m = UiModel::new();
    m.contacts_listed(vec![
        contact("1", "zoe", &p, "work"),
        contact("2", "Smith, John", &p, "home"),
        contact("3", "Alice", &q, "home"),
        contact("4", "Mum and Dad", &p, "family"),
    ]);
    assert_eq!(
        m.contact_names_on(&p),
        vec![
            "Mum and Dad".to_owned(),
            "Smith, John".to_owned(),
            "zoe".to_owned()
        ]
    );
    assert_eq!(m.contact_names_on(&q), vec!["Alice".to_owned()]);
    assert!(m.contact_names_on(&peer()).is_empty());
}

/// Focus after a removal skips a neighbour the list read back no longer
/// has -- another administrator removed it meanwhile -- for the nearest
/// row still there, else Add an address (§8).
#[test]
fn focus_after_a_removal_skips_rows_removed_elsewhere() {
    let (me, a, b, c) = (peer(), peer(), peer(), peer());
    // Shown C, B, A. B is removed; A went too, elsewhere.
    let mut m = opened(&me, &[], &[added(&a), added(&b), added(&c)]);
    let _ = input(&mut m, PeersInput::Remove(at(&b)));
    m.address_removed(&at(&b), answer(AddressOutcome::Removed, &[added(&c)]));
    assert_eq!(focus(&m), Some(PeersFocus::Row(at(&c))), "A is gone: C");

    let mut m = opened(&me, &[], &[added(&a), added(&b), added(&c)]);
    let _ = input(&mut m, PeersInput::Remove(at(&b)));
    m.address_removed(&at(&b), answer(AddressOutcome::Removed, &[]));
    assert_eq!(focus(&m), Some(PeersFocus::AddAddress), "none left");
}

/// Any list read that succeeds is the runtime's word and ends what was
/// unknown (§5.7, "until a read shows it"): a read back that shows an
/// add not made, or a list that came with another row's answer. A retry
/// that writes nothing after it says not written.
#[test]
fn a_read_that_shows_it_ends_an_unknown_change() {
    let (me, a, b) = (peer(), peer(), peer());
    let lost = || {
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        ))
    };
    let not_written = || Err(AddressChangeFailure::NotMade(AddressProblem::Unavailable));

    // An add: unknown, then lost again with a read back that does not
    // list it, then a retry that writes nothing.
    let mut m = opened(&me, &[], &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let _ = input(&mut m, PeersInput::FieldChanged(at(&a)));
    let _ = input(&mut m, PeersInput::Add);
    m.addresses_added(&[at(&a)], lost());
    let _ = m.peers_turn();
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(
        m.peers().add_page_note(),
        Some(&AddNote::Unknown { lines: 1 })
    );
    let _ = input(&mut m, PeersInput::Add);
    m.addresses_added(&[at(&a)], lost());
    let _ = m.peers_turn();
    m.addresses_read(Ok(list(&[])));
    assert_eq!(m.peers().add_page_note(), Some(&AddNote::Failed));
    let _ = input(&mut m, PeersInput::Add);
    m.addresses_added(&[at(&a)], not_written());
    assert_eq!(
        m.peers().add_page_note(),
        Some(&AddNote::Failed),
        "the read showed it: not unknown"
    );

    // A removal: unknown, then another row's answer brings a list that
    // still has it, then a retry that writes nothing.
    let mut m = opened(&me, &[], &[added(&a), added(&b)]);
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(&at(&a), lost());
    let _ = m.peers_turn();
    m.addresses_read(Err(AddressProblem::Unavailable));
    let _ = input(&mut m, PeersInput::Remove(at(&b)));
    m.address_removed(&at(&b), answer(AddressOutcome::Removed, &[added(&a)]));
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(&at(&a), not_written());
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::RemoveFailed { address: at(&a) }),
        "B's answer listed A: known not removed"
    );
}

/// Focus never goes to a row the list on show no longer has -- an Undo
/// answered added whose list read back lacks the row, another
/// administrator having removed it meanwhile: the row after its place
/// takes it, else Add an address (§8).
#[test]
fn focus_never_goes_to_a_row_the_list_no_longer_has() {
    let (me, a, b) = (peer(), peer(), peer());
    let mut m = opened(&me, &[], &[added(&a), added(&b)]);
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(&at(&a), answer(AddressOutcome::Removed, &[added(&b)]));
    let _ = input(&mut m, PeersInput::Undo);
    m.addresses_added(
        &[at(&a)],
        answer(
            AddressOutcome::Added {
                peer: Some(a.clone()),
            },
            &[added(&b)],
        ),
    );
    assert_eq!(focus(&m), Some(PeersFocus::Row(at(&b))), "A is gone: B");

    let mut m = opened(&me, &[], &[added(&a)]);
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(&at(&a), answer(AddressOutcome::Removed, &[]));
    let _ = input(&mut m, PeersInput::Undo);
    m.addresses_added(
        &[at(&a)],
        answer(
            AddressOutcome::Added {
                peer: Some(a.clone()),
            },
            &[],
        ),
    );
    assert_eq!(focus(&m), Some(PeersFocus::AddAddress), "no row at all");
}

/// A read decides an unknown change whichever way it shows it (§5.7): a
/// plain read listing an unknown add's line settles it as checked; one
/// lacking an unknown removal's address settles it as removed. A list
/// that came with another row's answer and shows the change made leaves
/// it unknown -- a retry that writes nothing then still says so, never a
/// not-written text whose claim the list contradicts.
#[test]
fn a_read_decides_an_unknown_change_either_way() {
    let (me, a, b) = (peer(), peer(), peer());
    let lost = || {
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        ))
    };
    let not_written = || Err(AddressChangeFailure::NotMade(AddressProblem::Unavailable));

    // An add: unknown, then a plain read (a session ready again) lists it.
    let mut m = opened(&me, &[], &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let _ = input(&mut m, PeersInput::FieldChanged(at(&a)));
    let _ = input(&mut m, PeersInput::Add);
    m.addresses_added(&[at(&a)], lost());
    let _ = m.peers_turn();
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(m.read_addresses(), Some(Intent::ReadAddresses));
    m.addresses_read(Ok(list(&[added(&a)])));
    assert_eq!(
        m.peers().add_result(),
        Some(&AddResult::Checked { lines: 1 }),
        "the read showed it saved: checked"
    );

    // A removal: unknown, then a plain read lacks it: removed.
    let mut m = opened(&me, &[], &[added(&a), added(&b)]);
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(&at(&a), lost());
    let _ = m.peers_turn();
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(m.read_addresses(), Some(Intent::ReadAddresses));
    m.addresses_read(Ok(list(&[added(&b)])));
    assert!(
        matches!(m.peers().snack().0, Some(PeersSnack::Removed { address, .. }) if *address == at(&a)),
        "{:?}",
        m.peers().snack().0
    );

    // A removal: unknown, its snackbar still up, then an add's answer
    // brings a list lacking it -- shown made, so not ended -- and the
    // snackbar's Try again that writes nothing stays unknown.
    let mut m = opened(&me, &[], &[added(&a), added(&b)]);
    let _ = input(&mut m, PeersInput::Remove(at(&a)));
    m.address_removed(&at(&a), lost());
    let _ = m.peers_turn();
    m.addresses_read(Err(AddressProblem::Unavailable));
    let _ = input(&mut m, PeersInput::OpenAdd);
    let other = format!("/ip4/192.168.1.30/tcp/4001/p2p/{}", b.as_str());
    let _ = input(&mut m, PeersInput::FieldChanged(other.clone()));
    let _ = input(&mut m, PeersInput::Add);
    m.addresses_added(
        &[other],
        answer(
            AddressOutcome::Refused(AddressRefused::Grammar),
            &[added(&b)],
        ),
    );
    let _ = input(&mut m, PeersInput::Close);
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::RemoveUnknown { address: at(&a) })
    );
    assert_eq!(
        input(&mut m, PeersInput::TryAgain),
        vec![Intent::RemoveAddress(at(&a))]
    );
    m.address_removed(&at(&a), not_written());
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::RemoveUnknown { address: at(&a) }),
        "never \"still in the list\": the last list lacked it"
    );
}

/// A row focus is judged against what is drawn when it is read: a read
/// that drops the row after the ask -- another administrator's removal,
/// read before the view draws -- sends focus to the row after its place
/// as last drawn, else the one before, else Add an address, as after a
/// Remove, and says why (`focus_row_gone`, §8). It covers every ask of a
/// row: a row's trust step closing, a failed removal's menu, an Undo.
#[test]
fn a_row_focus_asked_before_a_read_lands_on_what_is_drawn() {
    let (me, a, b, c) = (peer(), peer(), peer(), peer());
    // Added rows show newest first: C, B, A.
    let rows = [added(&a), added(&b), added(&c)];
    // A row's trust step, closed after a read dropped its row: the row
    // after B's place, not the first shown.
    let mut m = opened(&me, &[], &rows);
    let _ = input(&mut m, PeersInput::TrustRow(at(&b)));
    m.addresses_read(Ok(list(&[added(&a), added(&c)])));
    let _ = input(&mut m, PeersInput::Close);
    assert_eq!(focus(&m), Some(PeersFocus::Row(at(&a))), "after B: A");
    assert!(m.peers().focus_row_gone(), "said gone");
    // The last row shown gone: the one before it.
    let mut m = opened(&me, &[], &rows);
    let _ = input(&mut m, PeersInput::TrustRow(at(&a)));
    m.addresses_read(Ok(list(&[added(&b), added(&c)])));
    let _ = input(&mut m, PeersInput::Close);
    assert_eq!(focus(&m), Some(PeersFocus::Row(at(&b))), "before A: B");
    // A row's trust step closed with its row still there says nothing.
    let mut m = opened(&me, &[], &rows);
    let _ = input(&mut m, PeersInput::TrustRow(at(&a)));
    let _ = input(&mut m, PeersInput::Close);
    assert_eq!(focus(&m), Some(PeersFocus::Row(at(&a))));
    assert!(!m.peers().focus_row_gone());

    // A failed removal's menu, its row dropped by a read after the ask,
    // and its Try again failing once the row has gone.
    let mut m = opened(&me, &[], &rows);
    let _ = input(&mut m, PeersInput::Remove(at(&b)));
    m.address_removed(
        &at(&b),
        Err(AddressChangeFailure::NotMade(AddressProblem::Unavailable)),
    );
    assert_eq!(focus(&m), Some(PeersFocus::RowMenu(at(&b))));
    m.addresses_read(Ok(list(&[added(&a), added(&c)])));
    assert_eq!(focus(&m), Some(PeersFocus::Row(at(&a))), "read after");
    let _ = input(&mut m, PeersInput::TryAgain);
    m.address_removed(
        &at(&b),
        Err(AddressChangeFailure::NotMade(AddressProblem::Unavailable)),
    );
    assert_eq!(focus(&m), Some(PeersFocus::Row(at(&a))), "asked after");

    // An Undo whose list read back lacks its row, which the Remove had
    // taken off the list drawn: the row keeps its place from before.
    let mut m = opened(&me, &[], &rows);
    let _ = input(&mut m, PeersInput::Remove(at(&b)));
    m.address_removed(
        &at(&b),
        answer(AddressOutcome::Removed, &[added(&a), added(&c)]),
    );
    let _ = input(&mut m, PeersInput::Undo);
    m.addresses_added(
        &[at(&b)],
        answer(
            AddressOutcome::Added {
                peer: Some(b.clone()),
            },
            &[added(&a), added(&c)],
        ),
    );
    assert_eq!(focus(&m), Some(PeersFocus::Row(at(&a))), "after B: A");
    assert!(m.peers().focus_row_gone());
}

/// A removal left unknown is read again when its snackbar goes; a read
/// that lacks the row decides it as Remove done -- the snackbar with
/// Undo, focus on the row after the removed one, else the one before,
/// else Add an address (§5.7, §8) -- and nothing says a row was gone.
#[test]
fn a_removal_unknown_read_again_without_its_row_is_as_remove_done() {
    let (me, a, b, c) = (peer(), peer(), peer(), peer());
    // Added rows show newest first: C, B, A.
    let mut m = opened(&me, &[], &[added(&a), added(&b), added(&c)]);
    let _ = input(&mut m, PeersInput::Remove(at(&b)));
    m.address_removed(
        &at(&b),
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable,
        )),
    );
    let _ = m.peers_turn();
    m.addresses_read(Err(AddressProblem::Unavailable));
    assert_eq!(
        m.peers().snack().0,
        Some(&PeersSnack::RemoveUnknown { address: at(&b) })
    );
    let _ = input(&mut m, PeersInput::SnackClosed);
    assert_eq!(m.peers_turn(), Some(Intent::ReadAddresses));
    m.addresses_read(Ok(list(&[added(&a), added(&c)])));
    assert!(
        matches!(
            m.peers().snack().0,
            Some(PeersSnack::Removed { address, undo: true, .. }) if *address == at(&b)
        ),
        "{:?}",
        m.peers().snack().0
    );
    assert_eq!(focus(&m), Some(PeersFocus::Row(at(&a))), "after B: A");
    assert!(!m.peers().focus_row_gone());
}

/// A Trust still awaited when its page closes -- Done, Back, Escape, or
/// Peers left -- counts as a Trust whose answer was lost (§5.3): its
/// peer is pending and its rows claim no trust state, the trust line
/// says why. Its late answer still lands: made ends it; unconfirmed keeps it pending; one
/// that wrote nothing has the list read again, which decides it.
#[test]
fn a_trust_awaited_when_its_page_closes_is_pending() {
    let (me, t) = (peer(), peer());
    let change = TrustChange {
        peer: t.clone(),
        allowed: true,
    };
    let closed = || {
        let mut m = opened(&me, &[], &[added(&t)]);
        let _ = input(&mut m, PeersInput::TrustRow(at(&t)));
        assert_eq!(
            input(&mut m, PeersInput::Trust(t.clone())),
            vec![Intent::SetTrust(change.clone())]
        );
        let _ = input(&mut m, PeersInput::Close);
        m
    };
    let state = |m: &UiModel| m.peers().rows(m.trust_settings())[0].state;
    let m = closed();
    assert_eq!(state(&m), RowState::TrustUnknown, "neither claimed");
    assert!(matches!(
        m.peers().trust_line(m.trust_settings()),
        Some(TrustLine::Unread { .. })
    ));
    assert!(m.trust_settings().is_pending(&t));

    let listed = |trusted: bool| TrustList {
        local_peer: Some(me.clone()),
        allowed: if trusted {
            vec![TrustRow::added_here(t.clone())]
        } else {
            Vec::new()
        },
    };
    let mut m = closed();
    m.trust_settings_mut().set(change.clone(), Ok(listed(true)));
    assert_eq!(state(&m), RowState::Trusted, "made");
    assert!(!m.trust_settings().is_pending(&t));

    let mut m = closed();
    m.trust_settings_mut().set(
        change.clone(),
        Err(TrustSetFailure::Unconfirmed(TrustProblem::Unavailable)),
    );
    assert_eq!(state(&m), RowState::TrustUnknown, "still lost");
    assert!(m.trust_settings().is_pending(&t));

    let mut m = closed();
    m.trust_settings_mut().set(
        change.clone(),
        Err(TrustSetFailure::NotMade(TrustProblem::Unavailable)),
    );
    assert_eq!(state(&m), RowState::TrustUnknown, "until read");
    assert_eq!(
        m.trust_settings_mut().take_reread(),
        Some(Intent::ReadTrust),
        "read again"
    );
    m.trust_settings_mut().read(Ok(listed(false)));
    assert_eq!(state(&m), RowState::NeedsTrust, "the read decides");
    assert!(!m.trust_settings().is_pending(&t));
}

/// A line naming this device's own `PeerId` is refused before anything
/// is sent, typed or scanned, and one such line refuses the whole add
/// (§5.3, architect-cto's DECISION 01a166c9); the runtime's `own_peer`
/// says the same. Another device's address is sent as before.
#[test]
fn this_devices_own_address_is_refused_before_sending() {
    use interweave_human_client_api::AddressRefused;
    let (me, other) = (peer(), peer());
    let own = at(&me);
    let refused = |line: &str| {
        Some(AddNote::Refused {
            line: line.to_owned(),
            why: Refusal::OwnAddress,
        })
    };

    let mut m = opened(&me, &[], &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let _ = input(&mut m, PeersInput::FieldChanged(own.clone()));
    assert_eq!(input(&mut m, PeersInput::Add), Vec::new(), "nothing sent");
    assert_eq!(m.peers().add_page_note().cloned(), refused(&own));
    assert_eq!(
        focus(&m),
        Some(PeersFocus::Field {
            select: Some(LineSelection::Line(own.clone()))
        })
    );
    let _ = input(&mut m, PeersInput::FieldChanged(at(&other)));
    assert_eq!(
        input(&mut m, PeersInput::Add),
        vec![Intent::AddAddresses(vec![at(&other)])],
        "the control: another device's address is sent"
    );

    let mut m = opened(&me, &[], &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let code = format!("{}\n{own}", at(&other));
    assert_eq!(m.scanned(&code), (ScanVerdict::Taken, None), "none of it");
    assert_eq!(m.peers().add_page_note().cloned(), refused(&own));
    assert_eq!(focus(&m), Some(PeersFocus::Note));

    // The runtime refusing it: the same words.
    let mut m = opened(&me, &[], &[]);
    let _ = input(&mut m, PeersInput::OpenAdd);
    let _ = input(&mut m, PeersInput::FieldChanged(at(&other)));
    let _ = input(&mut m, PeersInput::Add);
    m.addresses_added(
        &[at(&other)],
        answer(AddressOutcome::Refused(AddressRefused::OwnPeer), &[]),
    );
    assert_eq!(m.peers().add_page_note().cloned(), refused(&at(&other)));
}
