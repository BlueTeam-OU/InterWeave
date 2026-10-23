// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The daemon's command line (plan §16 (11): no `clap`, a small tested
//! parser).
//!
//! `transport-daemon --profile <name> [--create-identity]`. The profile is
//! required: nothing in the architecture names a default profile, and a
//! daemon guessing one would serve an identity nobody chose.
//! `transport-daemon --version` alone prints the version (ADR-0065).

/// What the command line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    /// Run the daemon for a profile.
    Run(Args),
    /// Print the usage and exit 0.
    Help,
    /// Print the version and exit 0.
    Version,
}

/// A run's arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Args {
    /// The profile to serve.
    pub(crate) profile: String,
    /// Create the identity key if -- and only if -- none exists yet
    /// (plan §16 (12)); without it a missing key is fatal.
    pub(crate) create_identity: bool,
}

/// The usage text.
pub(crate) const USAGE: &str =
    "usage: transport-daemon --profile <name> [--create-identity] | --version";

/// The workspace version with the commit as build metadata (ADR-0065
/// decision 6; `tools/stamp/commit_stamp.rs`).
pub(crate) const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "+", env!("INTERWEAVE_COMMIT"));

/// Parse the arguments after the program name.
///
/// # Errors
/// A message naming what is wrong: exit code 2.
pub(crate) fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Command, String> {
    let mut profile = None;
    let mut create_identity = false;
    let mut args = args.into_iter().peekable();
    // Alone, or not a version request: a start line that also asks for the
    // version is a mistake, not a run that prints and then starts.
    if args.peek().map(String::as_str) == Some("--version") {
        args.next();
        return match args.next() {
            None => Ok(Command::Version),
            Some(other) => Err(format!("--version takes no other argument, not {other:?}")),
        };
    }
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(Command::Help),
            "--create-identity" => create_identity = true,
            "--profile" => {
                let name = args.next().ok_or("--profile needs a name")?;
                if profile.replace(name).is_some() {
                    return Err("--profile given twice".to_owned());
                }
            }
            other => return Err(format!("unexpected argument {other:?}")),
        }
    }
    let profile = profile.ok_or("--profile <name> is required")?;
    Ok(Command::Run(Args {
        profile,
        create_identity,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(args: &[&str]) -> Result<Command, String> {
        parse(args.iter().map(|a| (*a).to_owned()))
    }

    #[test]
    fn a_profile_and_the_create_flag_parse() {
        assert_eq!(
            parsed(&["--profile", "work"]),
            Ok(Command::Run(Args {
                profile: "work".into(),
                create_identity: false
            }))
        );
        assert_eq!(
            parsed(&["--create-identity", "--profile", "work"]),
            Ok(Command::Run(Args {
                profile: "work".into(),
                create_identity: true
            }))
        );
        assert_eq!(parsed(&["--help"]), Ok(Command::Help));
        assert_eq!(parsed(&["--version"]), Ok(Command::Version));
    }

    #[test]
    fn a_missing_or_repeated_profile_and_anything_unknown_are_usage_errors() {
        assert!(parsed(&[]).is_err(), "no default profile is guessed");
        assert!(parsed(&["--profile"]).is_err());
        assert!(parsed(&["--profile", "a", "--profile", "b"]).is_err());
        assert!(parsed(&["--profile", "a", "--verbose"]).is_err());
        assert!(parsed(&["--version", "--profile", "a"]).is_err());
        assert!(parsed(&["--profile", "a", "--version"]).is_err());
    }
}
