// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `interweave-human`'s command line: no `clap`, a small tested parser,
//! as `transportctl`'s. Flags go anywhere; `--` ends them, so a message
//! starting with `-` is still a message; no usage error repeats a value
//! it was given, since a message typed in the wrong place would otherwise
//! be copied to whatever logs stderr.

use std::time::Duration;

use interweave_human_ui_model::ConversationKey;
use interweave_transport_api::{ChannelId, EndpointId, TransportIdentity};

/// The usage text, every verb and the exit codes.
pub(crate) const USAGE: &str = "\
usage:
  interweave-human --profile <name> status [--json]
  interweave-human --profile <name> conversations [--json]
  interweave-human --profile <name> messages <conversation> [--json]
  interweave-human --profile <name> read <conversation> [--keep <id>]... [--json]
  interweave-human --profile <name> send <conversation> <text>|-
  interweave-human --profile <name> cancel|retry|unkeep <id>
  interweave-human --profile <name> listen [--json]
  interweave-human --profile <name> trust list [--json]
  interweave-human --profile <name> trust allow|revoke <peer>
  interweave-human --profile <name> identities list [--json]
  interweave-human --profile <name> identities enable|disable <endpoint>
  interweave-human --profile <name> identities default <endpoint>|--none
  interweave-human --profile <name> store health [--json]

  --wait <ms>   how long to wait for the session and for an answer
                (default 5000)

A <conversation> is <peer> (its default endpoint), <peer>/<endpoint>, or
#<channel>. An <id> is a message's as messages and read print it: p<row>
pending, u<row> unread, k<row> kept. `send ... -` reads the text from
stdin. `--` ends the flags.

Exit codes:
  0  done
  1  refused: by this tool, by the store or by the daemon
  2  a usage error
  3  no daemon answered for the profile
  4  another client holds this profile's message store
  5  the message store cannot be used: it needs recovery, is not
     private to this user, or cannot be opened now";

/// What the command line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    /// Print the usage and exit 0.
    Help,
    /// A verb against one profile.
    Run {
        /// The profile.
        profile: String,
        /// The verb.
        verb: Verb,
        /// Print JSON, one object per line.
        json: bool,
        /// How long to wait for the session and for an answer.
        wait: Duration,
    },
}

/// One verb.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Verb {
    /// The session, the connectivity and the store, as the client sees them.
    Status,
    /// Every conversation, newest first.
    Conversations,
    /// One conversation's messages, read or not, marking nothing read.
    Messages(ConversationKey),
    /// One conversation's messages, its unread ones marked read, and the
    /// named ones kept while their read copy is held (RETENTION.md).
    Read {
        /// The conversation.
        conversation: ConversationKey,
        /// The messages to keep, by id.
        keep: Vec<String>,
    },
    /// Send a text message.
    Send {
        /// The conversation.
        conversation: ConversationKey,
        /// The text, or `None` to read it from stdin.
        text: Option<String>,
    },
    /// Cancel a pending message.
    Cancel(String),
    /// Retry a pending message that needs attention.
    Retry(String),
    /// Release a kept message.
    Unkeep(String),
    /// Hold the session open, printing what happens until a signal.
    Listen,
    /// The daemon's trust list.
    TrustList,
    /// Allow a peer, or revoke it.
    SetTrust(TransportIdentity, bool),
    /// The identities this client may hold: the profile's human
    /// endpoints, as the daemon reports them.
    IdentitiesList,
    /// Enable or disable one.
    SetEnabled(EndpointId, bool),
    /// Set the default one, or clear it.
    SetDefault(Option<EndpointId>),
    /// The store's health.
    StoreHealth,
}

/// The default `--wait`.
pub(crate) const DEFAULT_WAIT: Duration = Duration::from_millis(5_000);

/// The flags, collected; each verb takes what it accepts and refuses the
/// rest.
#[derive(Default)]
struct Flags {
    profile: Option<String>,
    json: bool,
    none: bool,
    wait: Option<String>,
    keep: Vec<String>,
}

/// Parse a conversation's text: `<peer>`, `<peer>/<endpoint>` or
/// `#<channel>`.
///
/// # Errors
/// What is wrong with it, without repeating it.
pub(crate) fn conversation(text: &str) -> Result<ConversationKey, String> {
    if let Some(channel) = text.strip_prefix('#') {
        return ChannelId::parse(channel)
            .map(ConversationKey::Channel)
            .map_err(|e| format!("the channel: {e}"));
    }
    let (peer, endpoint) = match text.split_once('/') {
        Some((peer, endpoint)) => (peer, Some(endpoint)),
        None => (text, None),
    };
    Ok(ConversationKey::Direct {
        peer: TransportIdentity::parse(peer).map_err(|e| format!("the peer: {e}"))?,
        endpoint: endpoint
            .map(|e| EndpointId::parse(e).map_err(|e| format!("the endpoint: {e}")))
            .transpose()?,
    })
}

/// A conversation as [`conversation`] reads it back.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the read verbs, the next commits on this branch, print it"
    )
)]
pub(crate) fn conversation_text(key: &ConversationKey) -> String {
    match key {
        ConversationKey::Channel(channel) => format!("#{}", channel.as_str()),
        ConversationKey::Direct {
            peer,
            endpoint: None,
        } => peer.as_str().to_owned(),
        ConversationKey::Direct {
            peer,
            endpoint: Some(endpoint),
        } => format!("{}/{}", peer.as_str(), endpoint.as_str()),
    }
}

/// Parse the arguments after the program name.
///
/// # Errors
/// A message naming what is wrong: exit code 2.
pub(crate) fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Command, String> {
    let mut flags = Flags::default();
    let mut words = Vec::new();
    let mut args = args.into_iter();
    let mut flags_ended = false;
    while let Some(arg) = args.next() {
        if flags_ended {
            words.push(arg);
            continue;
        }
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--" => flags_ended = true,
            "--help" | "-h" => return Ok(Command::Help),
            "--profile" => {
                let v = value("--profile")?;
                if flags.profile.replace(v).is_some() {
                    return Err("--profile given twice".to_owned());
                }
            }
            "--wait" => {
                let v = value("--wait")?;
                if flags.wait.replace(v).is_some() {
                    return Err("--wait given twice".to_owned());
                }
            }
            "--keep" => flags.keep.push(value("--keep")?),
            "--json" => flags.json = true,
            "--none" => flags.none = true,
            // A lone "-" is send's stdin marker, a word.
            "-" => words.push(arg),
            flag if flag.starts_with('-') => return Err("an unknown flag".to_owned()),
            _ => words.push(arg),
        }
    }
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    if words.is_empty() {
        return Err("a command is required".to_owned());
    }
    let profile = flags.profile.take().ok_or("--profile <name> is required")?;
    let wait = flags
        .wait
        .take()
        .map(|ms| {
            ms.parse::<u64>()
                .map(Duration::from_millis)
                .map_err(|_| "--wait: not a number of milliseconds".to_owned())
        })
        .transpose()?
        .unwrap_or(DEFAULT_WAIT);
    let endpoint = |id: &str| EndpointId::parse(id).map_err(|e| format!("the endpoint: {e}"));
    let peer = |id: &str| TransportIdentity::parse(id).map_err(|e| format!("the peer: {e}"));
    let (verb, json_allowed) = match words.as_slice() {
        ["status"] => (Verb::Status, true),
        ["conversations"] => (Verb::Conversations, true),
        ["messages", c] => (Verb::Messages(conversation(c)?), true),
        ["read", c] => (
            Verb::Read {
                conversation: conversation(c)?,
                keep: std::mem::take(&mut flags.keep),
            },
            true,
        ),
        ["send", c, "-"] => (
            Verb::Send {
                conversation: conversation(c)?,
                text: None,
            },
            false,
        ),
        ["send", c, text] => (
            Verb::Send {
                conversation: conversation(c)?,
                text: Some((*text).to_owned()),
            },
            false,
        ),
        ["cancel", id] => (Verb::Cancel((*id).to_owned()), false),
        ["retry", id] => (Verb::Retry((*id).to_owned()), false),
        ["unkeep", id] => (Verb::Unkeep((*id).to_owned()), false),
        ["listen"] => (Verb::Listen, true),
        ["trust", "list"] => (Verb::TrustList, true),
        ["trust", "allow", id] => (Verb::SetTrust(peer(id)?, true), false),
        ["trust", "revoke", id] => (Verb::SetTrust(peer(id)?, false), false),
        ["identities", "list"] => (Verb::IdentitiesList, true),
        ["identities", "enable", id] => (Verb::SetEnabled(endpoint(id)?, true), false),
        ["identities", "disable", id] => (Verb::SetEnabled(endpoint(id)?, false), false),
        ["identities", "default"] if std::mem::take(&mut flags.none) => {
            (Verb::SetDefault(None), false)
        }
        ["identities", "default", id] => (Verb::SetDefault(Some(endpoint(id)?)), false),
        ["store", "health"] => (Verb::StoreHealth, true),
        ["send", ..] => return Err("send takes a conversation and one text, or -".to_owned()),
        _ => return Err("an unknown command (see --help)".to_owned()),
    };
    let json = std::mem::take(&mut flags.json);
    if json && !json_allowed {
        return Err("--json applies to the verbs that read, not to this one".to_owned());
    }
    if !flags.keep.is_empty() {
        return Err("--keep applies to read only".to_owned());
    }
    if flags.none {
        return Err("--none applies to identities default only".to_owned());
    }
    Ok(Command::Run {
        profile,
        verb,
        json,
        wait,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PEER: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";

    fn run(args: &[&str]) -> Result<Command, String> {
        parse(args.iter().map(|s| (*s).to_owned()))
    }

    fn verb(args: &[&str]) -> Verb {
        match run(args) {
            Ok(Command::Run { verb, .. }) => verb,
            other => panic!("{args:?}: {other:?}"),
        }
    }

    #[test]
    fn every_verb_in_the_usage_parses() {
        let c = format!("{PEER}/human");
        let cases: &[&[&str]] = &[
            &["--profile", "p", "status"],
            &["--profile", "p", "conversations", "--json"],
            &["--profile", "p", "messages", &c],
            &["--profile", "p", "read", &c, "--keep", "u7", "--keep", "u8"],
            &["--profile", "p", "send", &c, "hello"],
            &["--profile", "p", "send", &c, "-"],
            &["--profile", "p", "cancel", "p3"],
            &["--profile", "p", "retry", "p3"],
            &["--profile", "p", "unkeep", "k3"],
            &["--profile", "p", "listen", "--json"],
            &["--profile", "p", "trust", "list"],
            &["--profile", "p", "trust", "allow", PEER],
            &["--profile", "p", "trust", "revoke", PEER],
            &["--profile", "p", "identities", "list", "--json"],
            &["--profile", "p", "identities", "enable", "human"],
            &["--profile", "p", "identities", "disable", "human"],
            &["--profile", "p", "identities", "default", "human"],
            &["--profile", "p", "identities", "default", "--none"],
            &["--profile", "p", "store", "health", "--json"],
        ];
        for args in cases {
            assert!(matches!(run(args), Ok(Command::Run { .. })), "{args:?}");
        }
        for word in [
            "status",
            "conversations",
            "messages",
            "read",
            "send",
            "cancel|retry|unkeep",
            "listen",
            "trust list",
            "trust allow|revoke",
            "identities list",
            "identities enable|disable",
            "identities default",
            "store health",
        ] {
            assert!(USAGE.contains(word), "{word} in the usage");
        }
    }

    #[test]
    fn a_conversation_is_a_peer_a_peer_and_endpoint_or_a_channel() {
        for text in [
            PEER.to_owned(),
            format!("{PEER}/human"),
            "#general".to_owned(),
        ] {
            assert_eq!(
                conversation_text(&conversation(&text).expect("parses")),
                text
            );
        }
        assert!(conversation("not a peer").is_err());
        assert!(conversation(&format!("{PEER}/")).is_err());
        assert!(conversation("#").is_err());
    }

    #[test]
    fn a_message_starting_with_a_dash_is_sent_after_the_end_of_the_flags() {
        let c = format!("{PEER}/human");
        assert_eq!(
            verb(&["--profile", "p", "send", &c, "--", "-1 is a number"]),
            Verb::Send {
                conversation: conversation(&c).expect("parses"),
                text: Some("-1 is a number".to_owned()),
            }
        );
        assert_eq!(
            run(&["--profile", "p", "send", &c, "-1 is a number"]),
            Err("an unknown flag".to_owned())
        );
    }

    #[test]
    fn flags_that_do_not_apply_are_usage_errors() {
        let c = format!("{PEER}/human");
        for args in [
            vec!["--profile", "p", "send", &c, "hi", "--json"],
            vec!["--profile", "p", "messages", &c, "--keep", "u1"],
            vec!["--profile", "p", "status", "--none"],
            vec!["--profile", "p", "status", "--wait", "soon"],
            vec!["status"],
            vec!["--profile", "p"],
            vec!["--profile", "p", "--profile", "q", "status"],
            vec!["--profile", "p", "launch"],
        ] {
            assert!(run(&args).is_err(), "{args:?}");
        }
    }

    #[test]
    fn no_usage_error_repeats_a_value_it_was_given() {
        let secret = "correct horse battery staple";
        for args in [
            vec!["--profile", "p", "send", secret],
            vec!["--profile", "p", "messages", secret],
            vec!["--profile", "p", "trust", "allow", secret],
            vec!["--profile", "p", "identities", "enable", secret],
            vec!["--profile", "p", "status", "--wait", secret],
        ] {
            let error = run(&args).expect_err("refused");
            assert!(!error.contains("horse"), "{args:?}: {error}");
        }
    }

    #[test]
    fn help_wins_anywhere_and_wait_defaults() {
        assert_eq!(run(&["status", "--help"]), Ok(Command::Help));
        assert_eq!(
            run(&["--profile", "p", "status"]),
            Ok(Command::Run {
                profile: "p".to_owned(),
                verb: Verb::Status,
                json: false,
                wait: DEFAULT_WAIT,
            })
        );
    }
}
