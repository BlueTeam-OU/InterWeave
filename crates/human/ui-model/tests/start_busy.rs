// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `IdStartBusy` as a session notice: a service start refused with the
//! profile in use after add-a-peer §5.1's bound
//! (design/2026-10-19-android-identity.md §6, A start refused with the
//! profile in use after the bound; §11d Q12), through the model's public
//! surface; and `Refused` in its place for a start that compared with
//! C4's `PeerId` (§6, An answer after the bound). Where it shows on screen, its focus and its announcement are
//! the view's (ui-slint's `tests/views.rs`); when it is said, the
//! platform's (android-platform's `ViewSide`).

#![allow(clippy::expect_used)]

use interweave_human_client_api::{ClientEvent, SessionState};
use interweave_human_ui_model::{Intent, SessionNotice, StartBusy, UiModel};

/// A model as a refused start leaves it: no facade, re-opening, and the
/// root's word that nothing runs.
fn refused() -> UiModel {
    let mut model = UiModel::new();
    model.client_event(ClientEvent::Session(SessionState::Reconnecting {
        attempt: 1,
        next_at: 0,
    }));
    model.daemon_seen(Some(false));
    model
}

/// It shows in the place of the notice any other refused start shows --
/// never `NoDaemon`, whose words say the app connects by itself, untrue
/// here (§11d Q12) -- and below the locked notice, which takes its place
/// when Try again's prompt is cancelled (§6, Its results), and below the
/// network-denied one, whose cause the person can act on first.
///
/// Mutations: the busy check moved after the session's match (`NoDaemon`
/// shows), or before the lock's (the locked notice never takes its place),
/// fails it.
#[test]
fn the_busy_start_notice_shows_below_the_lock_and_network_denied() {
    let mut model = refused();
    assert_eq!(
        model.session_notice(),
        Some(SessionNotice::NoDaemon),
        "control: any other refused start"
    );
    model.start_busy(Some(StartBusy::Shown));
    assert_eq!(
        model.session_notice(),
        Some(SessionNotice::StartBusy(StartBusy::Shown)),
        "in NoDaemon's place"
    );
    model.network_access(false);
    assert_eq!(model.session_notice(), Some(SessionNotice::NetworkDenied));
    model.network_access(true);
    model.authentication_needed(true);
    assert_eq!(
        model.session_notice(),
        Some(SessionNotice::AuthenticationNeeded),
        "the locked notice in its place"
    );
    model.authentication_needed(false);
    model.start_busy(None);
    assert_eq!(
        model.session_notice(),
        Some(SessionNotice::NoDaemon),
        "gone with the root's word"
    );
}

/// Try again makes one start: offered while the notice shows with no
/// start for it, refused from its press until the start answers -- a
/// second start would itself be refused with `LockHeld` (§6, Try again)
/// -- and offered again when it answers `LockHeld` again. Its resolution
/// stays Try again throughout, so the view still draws it, disabled.
///
/// Mutations: `offers` answering true for `Starting` (a second start
/// while the first runs), or `retry_start_asked` leaving `Shown` (Try
/// again never disabled), fails it.
#[test]
fn a_busy_start_is_retried_once_until_it_answers() {
    let mut model = refused();
    assert!(
        !model.offers(&Intent::RetryStart),
        "control: no notice, nothing to retry"
    );
    model.retry_start_asked();
    assert_eq!(
        model.session_notice(),
        Some(SessionNotice::NoDaemon),
        "a press with no notice shows none"
    );

    model.start_busy(Some(StartBusy::Shown));
    assert!(model.offers(&Intent::RetryStart), "Try again, enabled");
    model.retry_start_asked();
    assert_eq!(
        model.session_notice(),
        Some(SessionNotice::StartBusy(StartBusy::Starting)),
        "the notice stays while the start runs"
    );
    assert!(
        !model.offers(&Intent::RetryStart),
        "a second press starts nothing"
    );
    assert_eq!(
        SessionNotice::StartBusy(StartBusy::Starting).resolution(),
        Some(Intent::RetryStart),
        "still drawn, disabled"
    );

    // Answered `LockHeld` again.
    model.start_busy(Some(StartBusy::Shown));
    assert!(model.offers(&Intent::RetryStart), "enabled again");

    // A return's start: the root says it runs, and Try again waits too.
    model.start_busy(Some(StartBusy::Starting));
    assert!(!model.offers(&Intent::RetryStart));
}

/// A start that compared with C4's `PeerId` and was refused after the
/// bound -- another `PeerId`, or `Unavailable` -- shows `Refused` where
/// `IdStartBusy` shows, not `NoDaemon`, and below the locked and the
/// network-denied notices as `IdStartBusy` is (identity note §6, An
/// answer after the bound).
///
/// Mutations: the refused check left out of `session_notice` (`NoDaemon`
/// shows), or placed before the lock's, fails it.
#[test]
fn a_refused_compared_start_shows_where_the_busy_notice_does() {
    let mut model = refused();
    assert_eq!(model.session_notice(), Some(SessionNotice::NoDaemon));
    model.start_refused(Some(StartBusy::Shown));
    assert_eq!(
        model.session_notice(),
        Some(SessionNotice::StartRefused(StartBusy::Shown)),
        "in NoDaemon's place"
    );
    model.network_access(false);
    assert_eq!(model.session_notice(), Some(SessionNotice::NetworkDenied));
    model.network_access(true);
    model.authentication_needed(true);
    assert_eq!(
        model.session_notice(),
        Some(SessionNotice::AuthenticationNeeded),
        "the locked notice in its place"
    );
    model.authentication_needed(false);
    model.start_refused(None);
    assert_eq!(
        model.session_notice(),
        Some(SessionNotice::NoDaemon),
        "gone with the root's word"
    );
}

/// Its Try again makes one start, as `IdStartBusy`'s does: offered while
/// the notice shows with no start for it, not from its press until the
/// start answers, and offered again when that start is refused the same
/// way (§6, The comparison outlasts the bound).
///
/// Mutations: `offers` not reading the refused notice (Try again not
/// offered), or `retry_start_asked` leaving it `Shown`, fails it.
#[test]
fn a_refused_compared_start_is_retried_once_until_it_answers() {
    let mut model = refused();
    model.start_refused(Some(StartBusy::Shown));
    assert!(model.offers(&Intent::RetryStart), "Try again, enabled");
    model.retry_start_asked();
    assert_eq!(
        model.session_notice(),
        Some(SessionNotice::StartRefused(StartBusy::Starting)),
        "the notice stays while the start runs"
    );
    assert!(!model.offers(&Intent::RetryStart), "a second press");
    assert_eq!(
        SessionNotice::StartRefused(StartBusy::Starting).resolution(),
        Some(Intent::RetryStart),
        "still drawn, disabled"
    );
    model.start_refused(Some(StartBusy::Shown));
    assert!(model.offers(&Intent::RetryStart), "refused again: enabled");
}
