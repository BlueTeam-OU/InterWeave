// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `interweave-human`: the command-line human client (ADR-0059 decision
//! 9, plan section 29 step 4's command half). The same facade, store and
//! model as the desktop client, over the daemon's data socket, with no
//! display: a person at a shell, or a script.
//!
//! Exit codes: 0 done; 1 refused, by this tool, the store or the daemon;
//! 2 a usage error; 3 no daemon answered; 4 another client holds the
//! profile's message store; 5 the message store cannot be used; 6 a
//! message is still on its way when `--wait` ends. `--help` prints the
//! same.

#![cfg(unix)]
#![forbid(unsafe_code)]

mod admin;
mod cards;
#[cfg(test)]
mod cards_tests;
mod cli;
mod identity;
mod output;
mod session;
mod startup;
mod store;
mod verbs;
#[cfg(test)]
mod verbs_tests;

use std::io::{Read as _, Write as _};
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use interweave_human_app_core::FacadeSide;
use interweave_human_store::key_custody::FilesystemCustody;
use interweave_human_store::{HumanStore, StoreError};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_ipc_client::{IpcBinding, SocketPaths};
use interweave_profile_config::ProfileLock;
use interweave_transport_api::{EndpointId, MAX_PAYLOAD_BYTES};

/// The workspace version with the commit as build metadata (ADR-0065
/// decision 6; `tools/stamp/commit_stamp.rs`).
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "+", env!("INTERWEAVE_COMMIT"));

/// Why a verb did not complete, each with its exit code.
#[derive(Debug)]
pub(crate) enum Failure {
    /// Refused: by this tool, by the store or by the daemon. Exit 1.
    Refused(String),
    /// No daemon answered for the profile. Exit 3.
    Unreachable(String),
    /// Another client holds the profile's message store. Exit 4.
    Locked(String),
    /// The message store cannot be used. Exit 5.
    Store(String),
    /// A message is still on its way when the wait ended: it stays
    /// pending in the store. Exit 6.
    Pending(String),
}

impl Failure {
    const fn code(&self) -> u8 {
        match self {
            Self::Refused(_) => 1,
            Self::Unreachable(_) => 3,
            Self::Locked(_) => 4,
            Self::Store(_) => 5,
            Self::Pending(_) => 6,
        }
    }

    pub(crate) fn message(&self) -> &str {
        match self {
            Self::Refused(m)
            | Self::Unreachable(m)
            | Self::Locked(m)
            | Self::Store(m)
            | Self::Pending(m) => m,
        }
    }
}

/// What a verb prints on stdout: all of it when it completes, and what a
/// script still needs when it does not -- the id of a message that did
/// not finish.
struct Outcome {
    out: String,
    result: Result<(), Failure>,
}

impl From<Failure> for Outcome {
    fn from(failure: Failure) -> Self {
        Self {
            out: String::new(),
            result: Err(failure),
        }
    }
}

/// Unix milliseconds, for what the store and the envelope record.
fn wall_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// The facade side over the IPC binding, as the desktop builds it
/// (`apps/human-desktop/src/ipc.rs`): building opens nothing, the first
/// turn opens the session.
fn facade(
    profile: &startup::Profile,
    store: HumanStore,
) -> Result<FacadeSide<IpcBinding, IpcBinding>, StoreError> {
    let binding = IpcBinding::new(
        SocketPaths {
            data: profile.data_socket.clone(),
            admin: profile.admin_socket.clone(),
        },
        startup::CLIENT_KIND,
    );
    let config = ClientConfig {
        client_kind: startup::CLIENT_KIND.to_owned(),
        endpoints: profile.endpoints.clone(),
        channels: profile.channels.clone(),
        max_payload_bytes: MAX_PAYLOAD_BYTES,
    };
    let client = TransportClient::new(
        binding.clone(),
        binding,
        store,
        config,
        Box::new(wall_ms),
        0,
    )?;
    let mut facade = FacadeSide::new(client, wall_ms);
    // The identity's own key, by the desktop's rule: a file this build
    // refuses is said by its kind and left as it is, and the client runs
    // with no key, so nothing is signed and no card is shared.
    if let Some(endpoint) = profile.endpoints.first() {
        facade.set_key_source(interweave_human_app_core::KeySource::new(
            profile.paths.clone(),
            endpoint.clone(),
            Box::new(interweave_human_store::identity_key::FilesystemCustody),
        ));
        if let Some(problem) = facade.key_problem() {
            eprintln!(
                "interweave-human: the identity's key is not usable: {}",
                cards::key_problem_name(problem)
            );
        }
    }
    Ok(facade)
}

/// What stops `listen`: an interrupt or a terminate. Registered when it is
/// made, not when first polled, so a signal that arrives while the
/// session is still opening ends `listen` as cleanly as a later one.
fn stopped() -> std::io::Result<impl std::future::Future<Output = ()>> {
    use tokio::signal::unix::{SignalKind, signal};
    let mut term = signal(SignalKind::terminate())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    Ok(async move {
        tokio::select! {
            _ = term.recv() => {}
            _ = interrupt.recv() => {}
        }
    })
}

fn run(
    profile: &str,
    verb: &cli::Verb,
    json: bool,
    wait: Duration,
    endpoint: Option<&EndpointId>,
) -> Outcome {
    let until = Instant::now() + wait;
    let mut profile = match startup::resolve(profile) {
        Ok(profile) => profile,
        Err(failure) => return failure.into(),
    };
    // `--endpoint`: one of the profile's human endpoints is leased in place
    // of the first. Never one it does not offer: no endpoint is made here
    // (G2; ENDPOINTS.md, configured only).
    if let Some(endpoint) = endpoint {
        if !profile.endpoints.contains(endpoint) {
            return Failure::Refused(
                "that endpoint is not one a human client may use in this profile".to_owned(),
            )
            .into();
        }
        profile.endpoints = vec![endpoint.clone()];
    }
    // One worker, as the desktop's facade runtime: the IPC client's reader
    // task runs there, so a store call blocking this thread cannot starve
    // the session's keepalive.
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => return Failure::Refused(format!("the runtime: {e}")).into(),
    };
    // Before the lock: from here a signal is `listen`'s to answer.
    let stop = if matches!(verb, cli::Verb::Listen { .. }) {
        let _entered = runtime.enter();
        match stopped() {
            Ok(stop) => Some(stop),
            Err(e) => return Failure::Refused(format!("the signal handlers: {e}")).into(),
        }
    } else {
        None
    };
    let mut out = String::new();
    if matches!(
        verb,
        cli::Verb::IdentitiesList | cli::Verb::SetEnabled(..) | cli::Verb::SetDefault(_)
    ) {
        let result = runtime.block_on(admin::identities(&profile, verb, json, &mut out));
        return Outcome { out, result };
    }
    if matches!(
        verb,
        cli::Verb::PeersList
            | cli::Verb::PeersShow
            | cli::Verb::PeerAdd(_)
            | cli::Verb::PeerAddList(_)
            | cli::Verb::PeerRemove(_)
    ) {
        let result = runtime.block_on(admin::peers(&profile, verb, json, &mut out));
        return Outcome { out, result };
    }
    let lock = match startup::lock(&profile) {
        Ok(lock) => lock,
        Err(failure) => return failure.into(),
    };
    match verb {
        cli::Verb::StoreHealth => return store::health(&profile.paths, &FilesystemCustody, json),
        cli::Verb::StoreDiscard(confirm) => {
            return store::discard(&profile.paths, &FilesystemCustody, *confirm, json);
        }
        _ => {}
    }
    // Another identity than the one in front has no model: its key file
    // is read, under the lock, as the store's files are.
    if let cli::Verb::IdentityShow(Some(endpoint)) = verb
        && profile.endpoints.first() != Some(endpoint)
    {
        let result = if profile.endpoints.contains(endpoint) {
            identity::show_file(&profile.paths, endpoint, json, &mut out)
        } else {
            Err(Failure::Refused(
                "that endpoint is not one a human client may use in this profile".to_owned(),
            ))
        };
        return Outcome { out, result };
    }
    let (_lock, store) = match startup::open_store(&profile) {
        Ok(store) => (lock, store),
        Err(failure) => return failure.into(),
    };
    let name = profile.paths.profile().to_owned();
    let paths = profile.paths.clone();
    let result = runtime.block_on(async {
        let facade = facade(&profile, store).map_err(|e| {
            Failure::Store(format!(
                "the message store's pending messages could not be read: {e}"
            ))
        })?;
        let mut session = session::Session::new(facade, move || ProfileLock::is_held(&paths).ok())?;
        let result = if let Some(stop) = stop {
            let (input, consume) = match verb {
                cli::Verb::Listen { input, consume } => (*input, *consume),
                _ => (false, false),
            };
            let mode = verbs::ListenMode {
                json,
                consume,
                input: input.then(stdin_lines),
            };
            verbs::listen(&mut session, mode, &mut std::io::stdout(), stop).await
        } else {
            let r = verbs::Run {
                profile: &name,
                json,
                until,
            };
            verbs::run(&mut session, verb, &r, &mut out).await
        };
        session.close().await;
        result
    });
    Outcome { out, result }
}

/// `listen --input`'s lines, read from stdin on a thread of its own so a
/// blocked read never holds the session's turns (G1), and a full channel
/// holds the read rather than the session.
fn stdin_lines() -> tokio::sync::mpsc::Receiver<verbs::InputLine> {
    let (send, lines) = tokio::sync::mpsc::channel(verbs::INPUT_QUEUE);
    std::thread::spawn(move || verbs::read_input(std::io::stdin().lock(), &send));
    lines
}

/// The most bytes `send ... -` reads from stdin, derived so it moves with
/// the envelope: a message's text is at most the envelope's decompressed
/// ceiling, the envelope carrying it at a byte a byte at best, and the
/// room is the shell's newline and slack -- 262,144 bytes (256 KiB)
/// today, the figure the README states (#495 review F4).
const MAX_SEND_STDIN_BYTES: usize =
    interweave_human_chat_protocol::MAX_DECOMPRESSED_BYTES + SEND_ROOM_BYTES;

/// The room beside the text in what `send ... -` reads.
const SEND_ROOM_BYTES: usize = 64 << 10;

// The README states this figure: a ceiling that moves fails here until it
// is changed there too.
const _: () = assert!(MAX_SEND_STDIN_BYTES == 262_144);

/// The text `send ... -` reads: stdin, at most [`MAX_SEND_STDIN_BYTES`],
/// less the one trailing newline the shell adds. Read before the profile
/// or the session: a text past the bound opens nothing. Refused with the
/// exit code and why: past the bound or unreadable, refused (1); not
/// UTF-8, a usage error (2), as it was before the bound.
fn stdin_text(reader: impl std::io::Read) -> Result<String, (u8, String)> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_SEND_STDIN_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| (1, format!("the message on stdin could not be read: {e}")))?;
    if bytes.len() > MAX_SEND_STDIN_BYTES {
        return Err((
            1,
            format!(
                "the message on stdin is larger than {} KiB",
                MAX_SEND_STDIN_BYTES / 1024
            ),
        ));
    }
    let mut read = String::from_utf8(bytes)
        .map_err(|_| (2, "the message on stdin is not UTF-8 text".to_owned()))?;
    if read.ends_with('\n') {
        read.pop();
        if read.ends_with('\r') {
            read.pop();
        }
    }
    Ok(read)
}

fn main() -> ExitCode {
    let command = match cli::parse(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(message) => {
            eprintln!(
                "interweave-human: {}\n{}",
                output::one_line(&message),
                cli::USAGE
            );
            return ExitCode::from(2);
        }
    };
    let Outcome { out, result } = match command {
        cli::Command::Help => {
            println!("{}", cli::USAGE);
            return ExitCode::SUCCESS;
        }
        cli::Command::Version => {
            println!("interweave-human {VERSION}");
            return ExitCode::SUCCESS;
        }
        cli::Command::Run {
            profile,
            mut verb,
            json,
            wait,
            endpoint,
        } => {
            if let cli::Verb::Send {
                text: text @ None, ..
            } = &mut verb
            {
                match stdin_text(std::io::stdin().lock()) {
                    Ok(read) => *text = Some(read),
                    Err((code, why)) => {
                        eprintln!("interweave-human: {why}");
                        return ExitCode::from(code);
                    }
                }
            }
            run(&profile, &verb, json, wait, endpoint.as_ref())
        }
    };
    let mut stdout = std::io::stdout().lock();
    let written = stdout
        .write_all(out.as_bytes())
        .and_then(|()| stdout.flush());
    match result {
        Err(failure) => {
            eprintln!("interweave-human: {}", output::one_line(failure.message()));
            ExitCode::from(failure.code())
        }
        Ok(()) if written.is_err() => ExitCode::from(1),
        Ok(()) => ExitCode::SUCCESS,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use std::time::Duration;

    #[tokio::test]
    async fn a_signal_before_listen_first_polls_its_stop_ends_it_cleanly() {
        let stop = super::stopped().expect("handlers");
        // Raised before `stop` is ever polled: had the handlers waited for
        // the first poll, the default disposition would end this process.
        let raised = std::process::Command::new("kill")
            .args(["-TERM", &std::process::id().to_string()])
            .status()
            .expect("kill runs");
        assert!(raised.success());
        tokio::time::timeout(Duration::from_secs(5), stop)
            .await
            .expect("the signal was caught and ends the wait");
    }

    /// `send -`'s bound is on the read, not only on what was read: a
    /// stdin far past it is refused having read exactly the bound and a
    /// byte, counted at the reader (#495 review F5, F7). Mutations: the
    /// read's `take` removed; its limit loosened.
    #[test]
    fn a_stdin_far_past_its_bound_is_refused_having_read_the_bound_and_a_byte() {
        use std::io::Read as _;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU64, Ordering};
        struct Counting<R> {
            inner: R,
            read: Arc<AtomicU64>,
        }
        impl<R: std::io::Read> std::io::Read for Counting<R> {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                let n = self.inner.read(buf)?;
                self.read.fetch_add(n as u64, Ordering::SeqCst);
                Ok(n)
            }
        }
        let read = Arc::new(AtomicU64::new(0));
        let stdin = Counting {
            inner: std::io::repeat(b'a').take(256 << 20),
            read: Arc::clone(&read),
        };
        let answer = super::stdin_text(stdin);
        assert!(matches!(answer, Err((1, _))), "{answer:?}");
        assert_eq!(
            read.load(Ordering::SeqCst),
            super::MAX_SEND_STDIN_BYTES as u64 + 1
        );
    }
}
