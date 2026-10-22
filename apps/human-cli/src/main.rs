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
mod output;
mod session;
mod startup;
mod verbs;
#[cfg(test)]
mod verbs_tests;

use std::io::{Read as _, Write as _};
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use interweave_human_app_core::FacadeSide;
use interweave_human_store::{HumanStore, StorageHealth, StoreError};
use interweave_human_transport_client::{ClientConfig, TransportClient};
use interweave_ipc_client::{IpcBinding, SocketPaths};
use interweave_profile_config::ProfileLock;
use interweave_transport_api::MAX_PAYLOAD_BYTES;
use serde_json::json;

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
    // refuses is said by class and left as it is, and the client runs
    // with no key, so nothing is signed and no card is shared.
    match interweave_human_app_core::identity_key(&profile.paths, &profile.endpoints) {
        Ok(Some(key)) => facade.set_identity_key(key),
        Ok(None) => {}
        Err(e) => eprintln!("interweave-human: the identity's key is not usable: {e}"),
    }
    Ok(facade)
}

/// `store health`: re-checked now, as the window's re-check does, with
/// what the store holds counted. Degraded is exit 5, with the counts
/// printed.
fn store_health(store: &mut HumanStore, json: bool) -> Outcome {
    let unavailable = |e: &StoreError| Failure::Store(format!("the message store: {e}"));
    let healthy = matches!(store.recheck_health(), Ok(StorageHealth::Healthy));
    let counts = (|| {
        Ok::<_, Failure>((
            store.pending_outbound().map_err(|e| unavailable(&e))?.len(),
            store.unread_inbound().map_err(|e| unavailable(&e))?.len(),
            store.kept_inbound().map_err(|e| unavailable(&e))?.len(),
        ))
    })();
    let (pending, unread, kept) = match counts {
        Ok(counts) => counts,
        Err(failure) => return failure.into(),
    };
    let health = if healthy { "healthy" } else { "degraded" };
    let out = if json {
        output::json_line(&json!({
            "health": health, "pending": pending, "unread": unread, "kept": kept,
        }))
    } else {
        format!("health   {health}\npending  {pending}\nunread   {unread}\nkept     {kept}\n")
    };
    Outcome {
        out,
        result: if healthy {
            Ok(())
        } else {
            Err(Failure::Store(
                "the message store cannot hold new messages now".to_owned(),
            ))
        },
    }
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

fn run(profile: &str, verb: &cli::Verb, json: bool, wait: Duration) -> Outcome {
    let until = Instant::now() + wait;
    let profile = match startup::resolve(profile) {
        Ok(profile) => profile,
        Err(failure) => return failure.into(),
    };
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
    let stop = if *verb == cli::Verb::Listen {
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
        cli::Verb::PeersList | cli::Verb::PeerAdd(_) | cli::Verb::PeerRemove(_)
    ) {
        let result = runtime.block_on(admin::peers(&profile, verb, json, &mut out));
        return Outcome { out, result };
    }
    let lock = startup::lock(&profile).and_then(|lock| Ok((lock, startup::open_store(&profile)?)));
    let (_lock, mut store) = match lock {
        Ok(held) => held,
        Err(failure) => return failure.into(),
    };
    if *verb == cli::Verb::StoreHealth {
        return store_health(&mut store, json);
    }
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
            verbs::listen(&mut session, json, &mut std::io::stdout(), stop).await
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

/// The text `send ... -` reads: stdin, less the one trailing newline the
/// shell adds.
fn stdin_text() -> Option<String> {
    let mut read = String::new();
    std::io::stdin().read_to_string(&mut read).ok()?;
    if read.ends_with('\n') {
        read.pop();
        if read.ends_with('\r') {
            read.pop();
        }
    }
    Some(read)
}

fn main() -> ExitCode {
    let command = match cli::parse(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("interweave-human: {message}\n{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };
    let Outcome { out, result } = match command {
        cli::Command::Help => {
            println!("{}", cli::USAGE);
            return ExitCode::SUCCESS;
        }
        cli::Command::Run {
            profile,
            mut verb,
            json,
            wait,
        } => {
            if let cli::Verb::Send {
                text: text @ None, ..
            } = &mut verb
            {
                let Some(read) = stdin_text() else {
                    eprintln!("interweave-human: the message on stdin is not UTF-8 text");
                    return ExitCode::from(2);
                };
                *text = Some(read);
            }
            run(&profile, &verb, json, wait)
        }
    };
    let mut stdout = std::io::stdout().lock();
    let written = stdout
        .write_all(out.as_bytes())
        .and_then(|()| stdout.flush());
    match result {
        Err(failure) => {
            eprintln!("interweave-human: {}", failure.message());
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
}
