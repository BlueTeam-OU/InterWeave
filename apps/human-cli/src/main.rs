// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `interweave-human`: the command-line human client (ADR-0059 decision
//! 9, plan section 29 step 4's command half). The same facade, store and
//! model as the desktop client, over the daemon's data socket, with no
//! display: a person at a shell, or a script.
//!
//! Exit codes: 0 done; 1 refused, by this tool, the store or the daemon;
//! 2 a usage error; 3 no daemon answered; 4 another client holds the
//! profile's message store; 5 the message store cannot be used. `--help`
//! prints the same.

#![cfg(unix)]
#![forbid(unsafe_code)]

mod cli;
mod startup;

use std::io::Write as _;
use std::process::ExitCode;

/// Why a verb did not complete, each with its exit code.
#[derive(Debug)]
pub(crate) enum Failure {
    /// Refused: by this tool, by the store or by the daemon. Exit 1.
    Refused(String),
    /// No daemon answered for the profile. Exit 3.
    #[expect(
        dead_code,
        reason = "the session verbs, the next commits on this branch, construct it"
    )]
    Unreachable(String),
    /// Another client holds the profile's message store. Exit 4.
    Locked(String),
    /// The message store cannot be used. Exit 5.
    Store(String),
}

impl Failure {
    const fn code(&self) -> u8 {
        match self {
            Self::Refused(_) => 1,
            Self::Unreachable(_) => 3,
            Self::Locked(_) => 4,
            Self::Store(_) => 5,
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::Refused(m) | Self::Unreachable(m) | Self::Locked(m) | Self::Store(m) => m,
        }
    }
}

/// What a verb prints on stdout when it completes.
type Done = Result<String, Failure>;

fn run(profile: &str, _verb: &cli::Verb, _json: bool) -> Done {
    let profile = startup::resolve(profile)?;
    let _lock = startup::lock(&profile)?;
    let _store = startup::open_store(&profile)?;
    Err(Failure::Refused("this verb is not built yet".to_owned()))
}

fn main() -> ExitCode {
    let command = match cli::parse(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("interweave-human: {message}\n{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };
    let outcome = match command {
        cli::Command::Help => {
            println!("{}", cli::USAGE);
            return ExitCode::SUCCESS;
        }
        cli::Command::Run {
            profile,
            verb,
            json,
            wait: _,
        } => run(&profile, &verb, json),
    };
    match outcome {
        Ok(out) => {
            let mut stdout = std::io::stdout().lock();
            if stdout
                .write_all(out.as_bytes())
                .and_then(|()| stdout.flush())
                .is_err()
            {
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Err(failure) => {
            eprintln!("interweave-human: {}", failure.message());
            ExitCode::from(failure.code())
        }
    }
}
