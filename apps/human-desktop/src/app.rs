// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The window's side of the root: the model side over the view, fed by
//! the facade thread. One `pump` applies what the facade sent, renders,
//! takes what the person did -- draining the view's queue, which its bound
//! depends on -- and hands the commands over.

use interweave_human_app_core::{
    AddAct, AddTold, CardSaved, ModelSide, Opener, Problem, Surface, Update,
};
use interweave_human_transport_client::{AddOutcome, ClientEvent, SessionState};
use interweave_human_ui_model::{IdentityIntent, UiModel, ViewEvent, WindowEvent};
use interweave_human_ui_slint::View;
use interweave_transport_api::EndpointId;

use crate::facade_thread::{FacadeFault, FacadeThread, FromFacade};

/// The view as the root drives it.
pub struct SlintSurface(pub View);

impl Surface for SlintSurface {
    fn render(&mut self, model: &UiModel) {
        self.0.render(model);
    }

    fn take_events(&mut self, model: &UiModel) -> Vec<ViewEvent> {
        self.0.take_events(model)
    }

    fn take_window(&mut self) -> Vec<WindowEvent> {
        self.0.take_window_events()
    }

    fn take_identity_intents(&mut self) -> Vec<IdentityIntent> {
        self.0.take_identity_intents()
    }
}

/// Opens an activated link with the desktop's handler: `xdg-open`, one
/// argument, no shell, so nothing in the link is ever interpreted. The
/// link is content: a failure is reported by its kind alone, never with
/// the link.
pub struct DesktopOpener {
    program: std::path::PathBuf,
    report: fn(&str),
    spawned: fn(u32),
}

impl DesktopOpener {
    /// The desktop's handler, failures to stderr.
    #[must_use]
    pub fn new() -> Self {
        Self::with(std::path::PathBuf::from("xdg-open"), |line| {
            eprintln!("human-desktop: {line}");
        })
    }

    /// `program` in place of `xdg-open`, failures to `report`: how a test
    /// sees what is run and what is said.
    #[must_use]
    pub const fn with(program: std::path::PathBuf, report: fn(&str)) -> Self {
        Self {
            program,
            report,
            spawned: |_| {},
        }
    }

    /// Told each handler's process id as it starts: how a test checks the
    /// process is reaped.
    #[must_use]
    pub const fn on_spawn(mut self, spawned: fn(u32)) -> Self {
        self.spawned = spawned;
        self
    }
}

impl Default for DesktopOpener {
    fn default() -> Self {
        Self::new()
    }
}

impl Opener for DesktopOpener {
    fn open(&mut self, destination: &str) {
        match std::process::Command::new(&self.program)
            .arg(destination)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            // Reaped on a thread of its own, so an opened link leaves no
            // defunct process behind for the window's lifetime.
            Ok(mut child) => {
                (self.spawned)(child.id());
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
            }
            Err(e) => (self.report)(&format!("a link could not be opened: {}", e.kind())),
        }
    }

    fn ask_network_access(&mut self) {
        // The desktop root does not report access withheld, so the model
        // raises no such ask here; if one came, there is no platform
        // prompt to show, and the report says so.
        (self.report)("network access was asked for: the desktop has no prompt for it");
    }

    fn authenticate(&mut self) {
        // The desktop's daemon holds the key, with no presence policy, so
        // the root never reports authentication needed; if an ask came,
        // there is no prompt here, and the report says so.
        (self.report)("authentication was asked for: the desktop has no prompt for it");
    }
}

/// The window's side of the root.
pub struct App<S: Surface, O: Opener> {
    side: ModelSide<S, O>,
    facade: Option<FacadeThread>,
    listing_failed: bool,
    /// When the root was made, and whether it has turned yet: the session
    /// log says how long after start each state came.
    started: std::time::Instant,
    turned: bool,
    /// Where each log line goes, without its `human-desktop: ` prefix:
    /// stderr unless a test reads it ([`Self::with_log`]). The shipped
    /// binary's stderr is read as the client's log by
    /// `tests/desktop-e2e/tests/human_app/content_free_logs.rs`.
    log: fn(&str),
}

impl<S: Surface, O: Opener> App<S, O> {
    /// The root over `surface`, fed by `facade`.
    pub fn new(surface: S, opener: O, facade: FacadeThread) -> Self {
        Self {
            side: ModelSide::new(surface, opener),
            facade: Some(facade),
            listing_failed: false,
            started: std::time::Instant::now(),
            turned: false,
            log: |line| eprintln!("human-desktop: {line}"),
        }
    }

    /// Every log line to `log` instead of stderr: how a test reads what
    /// the client says about a failure.
    #[must_use]
    pub const fn with_log(mut self, log: fn(&str)) -> Self {
        self.log = log;
        self
    }

    /// The model side.
    #[must_use]
    pub const fn side(&self) -> &ModelSide<S, O> {
        &self.side
    }

    /// The model side, to drive the surface's platform half.
    pub const fn side_mut(&mut self) -> &mut ModelSide<S, O> {
        &mut self.side
    }

    /// Whether the store could not be listed at start.
    #[must_use]
    pub const fn listing_failed(&self) -> bool {
        self.listing_failed
    }

    /// One turn: apply what the facade sent, render, take the person's
    /// actions until the view's queue is empty, and hand the commands to
    /// the facade. Problems are logged as classes, never content.
    pub fn pump(&mut self) {
        let Some(facade) = &self.facade else {
            return;
        };
        let log = self.log;
        if !self.turned {
            self.turned = true;
            log(&format!("first turn at {} ms", self.elapsed_ms()));
        }
        let messages = facade.take();
        // Named before the updates are applied: the facade sets what it
        // holds before it sends anything.
        if self.side.front().is_none()
            && let Some(held) = facade.held()
        {
            self.side.hold(&held.held, &held.front);
        }
        for message in messages {
            match message {
                FromFacade::Update(identity, update) => {
                    if let Update::Client(ClientEvent::Session(state)) = &*update {
                        log(&format!(
                            "session {} at {} ms",
                            session_class(state),
                            self.elapsed_ms()
                        ));
                    }
                    // Each identity's update to its own model; only the
                    // front's is drawn.
                    self.side.apply_as(&identity, *update);
                }
                FromFacade::Daemon(present) => self.side.daemon_seen(present),
                FromFacade::Held(held) => {
                    if let Err(problem) = self.side.held_changed(&held) {
                        log(&format!("the identities held were not taken: {problem:?}"));
                    }
                }
                // Before the updates and the held list that name them: the
                // facade thread sends them in that order.
                FromFacade::HeldMore(more) => {
                    if !self.side.hold_more(&more) {
                        log("the identities held later were not taken");
                    }
                }
                // No screen shows it yet: the class goes to the log, never
                // the identity, its alias or the card's name.
                FromFacade::Add(told) => log(&format!("add: {}", add_class(&told))),
                FromFacade::ListingFailed => {
                    self.listing_failed = true;
                    log("the message store could not be listed");
                }
                FromFacade::FacadeFailed(FacadeFault::Store) => {
                    log("the message store's pending rows could not be read");
                }
                FromFacade::FacadeFailed(FacadeFault::OverBound) => {
                    log("more identities are held than a client holds");
                }
                FromFacade::Closed => {}
            }
        }
        // The person's acts on the identities before the turn: a switch
        // drains and sends the old front's acts as its own ahead of its
        // crossing, and the turn below then draws the new front's model
        // and asks as it, with no second wake needed to show the switch.
        // Try again names its identity, whichever is in front.
        let mut stopped = false;
        self.side.take_identity_acts(|crossing| {
            if !stopped && !facade.cross(crossing) {
                log("the transport side has stopped");
                stopped = true;
            }
        });
        // The front's, by name once the facade said what it holds: before
        // that the one model is the front's, whichever it is.
        let front = self.side.front().cloned();
        for command in self.side.turn() {
            if stopped {
                break;
            }
            if !facade.command(front.clone(), command) {
                log("the transport side has stopped");
                stopped = true;
            }
        }
        self.log_problems();
    }

    /// Hand what the person did with the add form or the list of
    /// identities to the facade thread. False, and nothing sent, once the
    /// facade is gone.
    pub fn add(&mut self, act: AddAct) -> bool {
        let Some(facade) = &self.facade else {
            return false;
        };
        let sent = facade.add(act);
        if !sent {
            (self.log)("the transport side has stopped");
        }
        sent
    }

    /// Bring `to` in front: the model side drains the old front's acts and
    /// sends them as its own, then the switch, all to the facade thread in
    /// that order (`ModelSide::switch`). False, and nothing done, when `to`
    /// is in front or not held.
    pub fn switch(&mut self, to: &EndpointId) -> bool {
        let Some(facade) = &self.facade else {
            return false;
        };
        let log = self.log;
        let mut stopped = false;
        let switched = self.side.switch(to, |crossing| {
            if !stopped && !facade.cross(crossing) {
                stopped = true;
                log("the transport side has stopped");
            }
        });
        self.log_problems();
        switched
    }

    fn log_problems(&mut self) {
        let log = self.log;
        for problem in self.side.take_problems() {
            match problem {
                Problem::Command { command, why } => {
                    log(&format!("{} failed: {why:?}", command_name(&command)));
                }
                Problem::UnreadNotListed(why) => {
                    log(&format!("unread messages could not be read again: {why:?}"));
                }
                Problem::CardsNotRead(why) => {
                    log(&format!("cards or contacts could not be read: {why:?}"));
                }
                Problem::ShareNotRecorded(why) => {
                    log(&format!(
                        "a card was accepted and its share could not be recorded yet: {why:?}"
                    ));
                }
            }
        }
    }

    /// Close the session -- the lease is released, the daemon keeps
    /// running -- and wait for the facade thread, at most `wait`.
    pub fn close(&mut self, wait: std::time::Duration) -> bool {
        self.facade.take().is_none_or(|facade| facade.close(wait))
    }
}

/// A command's name for a log line: never its content.
const fn command_name(command: &interweave_human_app_core::Command) -> &'static str {
    use interweave_human_app_core::Command;
    match command {
        Command::Send { .. } => "send",
        Command::MarkRead(_) => "mark read",
        Command::Keep { .. } => "keep",
        Command::Unkeep(_) => "unkeep",
        Command::Retry(_) => "retry",
        Command::Cancel(_) => "cancel",
        Command::Restore(_) => "restore",
        Command::ForgetCancelled(_) => "forget cancelled",
        Command::ForgetCopy(_) => "forget copy",
        Command::ForgetCard(_) => "forget card",
        Command::ForgetContact(_) => "forget contact",
        Command::Reopen => "reopen",
        Command::RecheckStorage => "recheck storage",
        // Named only because the match is whole: the failure line is the
        // one place this is used, and a trust command never reaches it --
        // its outcome is the settings' to say, and the daemon audits every
        // set (LOCAL-CLIENT.md section 5). This client logs nothing of a
        // trust change, the peer least of all.
        Command::ReadTrust => "read trust",
        Command::SetTrust(_) => "set trust",
        // Likewise: an address names where a peer is, and its outcome is
        // Peers' to say.
        Command::ReadAddresses => "read addresses",
        Command::ReadOwnAddresses => "read this device's addresses",
        Command::AddAddresses(_) => "add addresses",
        Command::RemoveAddress(_) => "remove address",
        Command::AcceptCard { .. } => "accept card",
        Command::DeclineCard(_) => "decline card",
        Command::UndoCard(_) => "undo card",
        Command::RetryKey => "retry key",
        Command::Window(interweave_human_app_core::WindowCommand::FetchPage(_)) => "fetch page",
        Command::Window(interweave_human_app_core::WindowCommand::ReadContacts) => "read contacts",
        Command::Window(interweave_human_app_core::WindowCommand::ReadPhoto(_)) => "read photo",
        Command::Window(interweave_human_app_core::WindowCommand::ReadReplyTarget(_)) => {
            "read reply target"
        }
        Command::ShareCard(_) => "share card",
        Command::PrepareOwnPhoto { .. } => "prepare own photo",
        Command::RemoveOwnPhoto => "remove own photo",
        Command::ForgetOwnPhoto => "forget own photo",
        Command::SetOwnCard { .. } => "set own card",
        Command::RenameContact { .. } => "rename contact",
        Command::DeleteContact(_) => "delete contact",
        Command::UndoContact { .. } => "undo contact",
        Command::DeleteRoute { .. } => "delete route",
        Command::SetContactPhoto { .. } => "set contact photo",
    }
}

/// What came of an add act or a settle, as the log names it: its class,
/// never the identity, its alias or anything its card says.
const fn add_class(told: &AddTold) -> &'static str {
    match told {
        AddTold::Offered => "create offered",
        AddTold::WaitsSeed => "create waits for the first identity's seed",
        AddTold::NoSeed => "the first identity's seed was not answered",
        AddTold::OutOfBounds => "the card is outside a card's bounds",
        AddTold::NotStarted => "create did not start",
        AddTold::Ended { outcome, card } => match (outcome, card) {
            (AddOutcome::Created { .. }, Some(CardSaved::Saved)) => "created, card saved",
            (AddOutcome::Created { .. }, Some(CardSaved::NotSaved)) => "created, card not saved",
            (AddOutcome::Created { .. }, None) => "created",
            (AddOutcome::NotFinished, _) => "created, not finished",
            (AddOutcome::Failed, _) => "refused, nothing created",
            (AddOutcome::Unknown, _) => "outcome unknown",
            (AddOutcome::Limit, _) => "refused at the limit",
            (AddOutcome::KeyGone, _) => "the form's key is gone",
        },
        AddTold::Held(_) => "an identity is held from now on",
        AddTold::NotHeld(_) => "an identity is not held yet, tried again",
        AddTold::NotHeldUntilRestart(_) => "an identity is not held, not tried again until restart",
    }
}

impl<S: Surface, O: Opener> App<S, O> {
    fn elapsed_ms(&self) -> u128 {
        self.started.elapsed().as_millis()
    }
}

/// A session state as the log names it: its class and attempt, never an
/// endpoint or anything a message carried.
fn session_class(state: &SessionState) -> String {
    match state {
        SessionState::Ready { .. } => "ready".to_owned(),
        SessionState::Reconnecting { attempt, .. } => format!("reconnecting, attempt {attempt}"),
        SessionState::Refused { problem } => format!("refused: {problem:?}"),
        SessionState::StorageDegraded => "storage degraded".to_owned(),
        SessionState::Closed => "closed".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use interweave_human_transport_client::{SessionProblem, SessionState};
    use interweave_test_support::endpoint_keys::endpoint_id;

    use super::session_class;

    /// The log names a state's class and attempt, and no endpoint.
    #[test]
    fn a_session_state_is_logged_as_its_class() {
        let endpoint = endpoint_id("human-secret-label");
        let ready = session_class(&SessionState::Ready {
            endpoint: Some(endpoint),
        });
        assert_eq!(ready, "ready");
        assert_eq!(
            session_class(&SessionState::Reconnecting {
                attempt: 3,
                next_at: 99
            }),
            "reconnecting, attempt 3"
        );
        assert_eq!(
            session_class(&SessionState::Refused {
                problem: SessionProblem::EndpointInUse
            }),
            "refused: EndpointInUse"
        );
        assert_eq!(
            session_class(&SessionState::StorageDegraded),
            "storage degraded"
        );
        // Never written by the shipped desktop
        // (`tests/log_lines.rs::a_closed_session_writes_no_closed_line`):
        // its text is held here alone.
        assert_eq!(session_class(&SessionState::Closed), "closed");
    }
}
