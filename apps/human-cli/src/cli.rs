// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `interweave-human`'s command line: no `clap`, a small tested parser,
//! as `transportctl`'s. Flags go anywhere; `--` ends them, so a message
//! starting with `-` is still a message; no usage error repeats a value
//! it was given, since a message typed in the wrong place would otherwise
//! be copied to whatever logs stderr.

use std::time::Duration;

use interweave_human_store::at_rest::DiscardScope;
use interweave_human_ui_model::ConversationKey;
use interweave_transport_api::{ChannelId, EndpointId, TransportIdentity};

/// The usage text, every verb and the exit codes.
pub(crate) const USAGE: &str = "\
usage:
  interweave-human --version
  interweave-human --profile <name> status [--json]
  interweave-human --profile <name> conversations [--json]
  interweave-human --profile <name> messages <conversation> [--json]
  interweave-human --profile <name> read <conversation> [--keep <id>]... [--json]
  interweave-human --profile <name> send <conversation> <text>|- [--json]
  interweave-human --profile <name> cancel|retry|unkeep <id> [--json]
  interweave-human --profile <name> listen [--json]
  interweave-human --profile <name> trust list [--json]
  interweave-human --profile <name> trust allow|revoke <peer>
  interweave-human --profile <name> identities list [--json]
  interweave-human --profile <name> identities enable|disable <endpoint>
  interweave-human --profile <name> identities default <endpoint>|--none
  interweave-human --profile <name> store health [--json]
  interweave-human --profile <name> store discard
                   [--confirm everything|unfinished-plaintext] [--json]
  interweave-human --profile <name> card show [--json]
  interweave-human --profile <name> card set <name> [<surname>] [--photo <file>|--no-photo]
  interweave-human --profile <name> card share <conversation> [--json]
  interweave-human --profile <name> proposals [--json]
  interweave-human --profile <name> proposals read <conversation>
                   [--accept <id>]... [--decline <id>]... [--json]
  interweave-human --profile <name> contacts [--json]
  interweave-human --profile <name> contacts show <contact> [--json]
  interweave-human --profile <name> contacts rename <contact> <name>
  interweave-human --profile <name> contacts photo <contact> <file>|--none
  interweave-human --profile <name> contacts delete <contact>
  interweave-human --profile <name> peers [--json]
  interweave-human --profile <name> peers add|remove <address>
  interweave-human --profile <name> peers show [--json]

  --wait <ms>   how long to wait for the session and for an answer
                (default 5000)

A <conversation> is <peer> (its default endpoint), <peer>/<endpoint>, or
#<channel>. An <id> is a message's as messages and read print it: p<row>
pending, u<row> unread, k<row> kept; a proposal's is its card's u<row>,
as proposals prints it. A <contact> is its id, as contacts prints it. `send ... -` reads the text from
stdin. `--` ends the flags. `store discard` without --confirm lists what
it would remove and removes nothing; --confirm names what it listed.

Exit codes:
  0  done
  1  refused: by this tool, by the store or by the daemon
  2  a usage error
  3  no daemon answered for the profile
  4  another client holds this profile's message store
  5  the message store cannot be used: it needs recovery, is not
     private to this user, cannot be opened now, or (store health)
     cannot hold new messages now
  6  a message is still on its way when --wait ends: it stays pending
     in the message store, and its id is printed";

/// What the command line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    /// Print the usage and exit 0.
    Help,
    /// Print the version and exit 0.
    Version,
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
    /// The store's health, and its key and files at rest.
    StoreHealth,
    /// Discard and recreate a store that needs recovery (ADR-0060
    /// decision 6): without the scope the person confirms, it lists what
    /// it would remove and removes nothing
    /// (`store::tests::a_discard_is_refused_without_the_scope_it_lists_and_removes_nothing`).
    StoreDiscard(Option<DiscardScope>),
    /// The identity in front's own card and key.
    CardShow,
    /// Set the own card: name, surname, and what happens to the photo.
    CardSet {
        /// The name.
        name: String,
        /// The surname, if any.
        surname: Option<String>,
        /// The photo.
        photo: PhotoArg,
    },
    /// Share the own card in a direct conversation.
    CardShare(ConversationKey),
    /// Every card proposal held, showing none of them read.
    Proposals,
    /// One conversation's proposals, shown and so read, and the named ones
    /// accepted or declined in the same run.
    ProposalsRead {
        /// The conversation.
        conversation: ConversationKey,
        /// The proposals to accept, by id.
        accept: Vec<String>,
        /// The proposals to decline, by id.
        decline: Vec<String>,
    },
    /// Every contact.
    Contacts,
    /// One contact, with its routes.
    ContactShow(String),
    /// The person's own name for a contact.
    ContactRename(String, String),
    /// The person's own photo for a contact, or its removal.
    ContactPhoto(String, PhotoArg),
    /// Delete a contact, its routes and their keys.
    ContactDelete(String),
    /// The bootstrap addresses: authored and added (ADR-0062).
    PeersList,
    /// This node's own addresses, as another node's `peers add` takes
    /// them (ADR-0062 decision 4).
    PeersShow,
    /// Add a peer by address, as the person typed it.
    PeerAdd(String),
    /// Remove an added address.
    PeerRemove(String),
}

/// What a photo argument asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PhotoArg {
    /// Leave it.
    Keep,
    /// Remove it.
    Remove,
    /// The image in this file.
    File(std::path::PathBuf),
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
    photo: Option<String>,
    no_photo: bool,
    accept: Vec<String>,
    decline: Vec<String>,
    confirm: Option<String>,
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

/// `--version` alone, or a usage error; `None` when the line asks no
/// version. Judged over every flag before a `--`, ahead of anything else,
/// so neither `--help` beside it nor a flag taking it as a value decides
/// (transport-daemon's rule, #386 review). After `--` it is a word, as a
/// message's text may be.
fn version_request(args: &[String]) -> Option<Result<Command, String>> {
    args.iter()
        .take_while(|a| *a != "--")
        .any(|a| a == "--version")
        .then(|| match args {
            [_] => Ok(Command::Version),
            _ => Err("--version takes no other argument".to_owned()),
        })
}

/// Parse the arguments after the program name.
///
/// # Errors
/// A message naming what is wrong: exit code 2.
pub(crate) fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Command, String> {
    let args: Vec<String> = args.into_iter().collect();
    if let Some(version) = version_request(&args) {
        return version;
    }
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
            "--accept" => flags.accept.push(value("--accept")?),
            "--decline" => flags.decline.push(value("--decline")?),
            "--photo" => {
                let v = value("--photo")?;
                if flags.photo.replace(v).is_some() {
                    return Err("--photo given twice".to_owned());
                }
            }
            "--no-photo" => flags.no_photo = true,
            "--confirm" => {
                let v = value("--confirm")?;
                if flags.confirm.replace(v).is_some() {
                    return Err("--confirm given twice".to_owned());
                }
            }
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
            true,
        ),
        ["send", c, text] => (
            Verb::Send {
                conversation: conversation(c)?,
                text: Some((*text).to_owned()),
            },
            true,
        ),
        ["cancel", id] => (Verb::Cancel((*id).to_owned()), true),
        ["retry", id] => (Verb::Retry((*id).to_owned()), true),
        ["unkeep", id] => (Verb::Unkeep((*id).to_owned()), true),
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
        ["store", "discard"] => {
            let scope = match flags.confirm.take().as_deref() {
                None => None,
                Some("everything") => Some(DiscardScope::Everything),
                Some("unfinished-plaintext") => Some(DiscardScope::UnfinishedPlaintext),
                Some(_) => {
                    return Err("--confirm takes everything or unfinished-plaintext".to_owned());
                }
            };
            (Verb::StoreDiscard(scope), true)
        }
        ["card", "show"] => (Verb::CardShow, true),
        ["card", "set", name, rest @ ..] if rest.len() <= 1 => {
            let photo = match (flags.photo.take(), std::mem::take(&mut flags.no_photo)) {
                (Some(_), true) => {
                    return Err("--photo and --no-photo together".to_owned());
                }
                (Some(file), false) => PhotoArg::File(file.into()),
                (None, true) => PhotoArg::Remove,
                (None, false) => PhotoArg::Keep,
            };
            (
                Verb::CardSet {
                    name: (*name).to_owned(),
                    surname: rest.first().map(|s| (*s).to_owned()),
                    photo,
                },
                false,
            )
        }
        ["card", "share", c] => (Verb::CardShare(conversation(c)?), true),
        ["proposals"] => (Verb::Proposals, true),
        ["proposals", "read", c] => (
            Verb::ProposalsRead {
                conversation: conversation(c)?,
                accept: std::mem::take(&mut flags.accept),
                decline: std::mem::take(&mut flags.decline),
            },
            true,
        ),
        ["contacts"] => (Verb::Contacts, true),
        ["contacts", "show", id] => (Verb::ContactShow((*id).to_owned()), true),
        ["contacts", "rename", id, name] => (
            Verb::ContactRename((*id).to_owned(), (*name).to_owned()),
            false,
        ),
        ["contacts", "photo", id] if std::mem::take(&mut flags.none) => (
            Verb::ContactPhoto((*id).to_owned(), PhotoArg::Remove),
            false,
        ),
        ["peers"] => (Verb::PeersList, true),
        ["peers", "show"] => (Verb::PeersShow, true),
        ["peers", "add", address] => (Verb::PeerAdd((*address).to_owned()), false),
        ["peers", "remove", address] => (Verb::PeerRemove((*address).to_owned()), false),
        ["contacts", "delete", id] => (Verb::ContactDelete((*id).to_owned()), false),
        ["contacts", "photo", id, file] => (
            Verb::ContactPhoto((*id).to_owned(), PhotoArg::File((*file).into())),
            false,
        ),
        ["send", ..] => return Err("send takes a conversation and one text, or -".to_owned()),
        _ => return Err("an unknown command (see --help)".to_owned()),
    };
    let json = std::mem::take(&mut flags.json);
    if json && !json_allowed {
        return Err(
            "--json applies to the verbs that read, and to send, cancel, retry, unkeep, card share and store discard"
                .to_owned(),
        );
    }
    if !flags.keep.is_empty() {
        return Err("--keep applies to read only".to_owned());
    }
    if flags.none {
        return Err("--none applies to identities default and contacts photo only".to_owned());
    }
    if !flags.accept.is_empty() || !flags.decline.is_empty() {
        return Err("--accept and --decline apply to proposals read only".to_owned());
    }
    if flags.confirm.is_some() {
        return Err("--confirm applies to store discard only".to_owned());
    }
    if flags.photo.is_some() || flags.no_photo {
        return Err("--photo and --no-photo apply to card set only".to_owned());
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
            &["--profile", "p", "send", &c, "hello", "--json"],
            &["--profile", "p", "cancel", "p3", "--json"],
            &["--profile", "p", "retry", "p3", "--json"],
            &["--profile", "p", "unkeep", "k3", "--json"],
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
            &["--profile", "p", "store", "discard", "--json"],
            &["--profile", "p", "peers", "show", "--json"],
            &[
                "--profile",
                "p",
                "store",
                "discard",
                "--confirm",
                "everything",
            ],
            &["--profile", "p", "card", "show", "--json"],
            &[
                "--profile",
                "p",
                "card",
                "set",
                "Ada",
                "Lovelace",
                "--photo",
                "a.jpg",
            ],
            &["--profile", "p", "card", "set", "Ada", "--no-photo"],
            &["--profile", "p", "card", "share", &c, "--json"],
            &["--profile", "p", "proposals", "--json"],
            &[
                "--profile",
                "p",
                "proposals",
                "read",
                &c,
                "--accept",
                "u3",
                "--decline",
                "u4",
            ],
            &["--profile", "p", "contacts", "--json"],
            &["--profile", "p", "contacts", "show", "c1", "--json"],
            &["--profile", "p", "contacts", "rename", "c1", "Ada"],
            &["--profile", "p", "contacts", "photo", "c1", "a.jpg"],
            &["--profile", "p", "contacts", "photo", "c1", "--none"],
            &["--profile", "p", "contacts", "delete", "c1"],
            &["--profile", "p", "peers", "--json"],
            &[
                "--profile",
                "p",
                "peers",
                "add",
                "/ip4/192.0.2.1/tcp/4001/p2p/x",
            ],
            &[
                "--profile",
                "p",
                "peers",
                "remove",
                "/ip4/192.0.2.1/tcp/4001/p2p/x",
            ],
        ];
        for args in cases {
            assert!(matches!(run(args), Ok(Command::Run { .. })), "{args:?}");
            let json = args.contains(&"--json");
            assert!(
                matches!(run(args), Ok(Command::Run { json: j, .. }) if j == json),
                "{args:?}: --json read"
            );
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
            "card show",
            "card set",
            "card share",
            "proposals read",
            "contacts show",
            "contacts rename",
            "contacts photo",
            "contacts delete",
            "peers add|remove",
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
            vec!["--profile", "p", "trust", "allow", PEER, "--json"],
            vec![
                "--profile",
                "p",
                "card",
                "set",
                "A",
                "--photo",
                "x",
                "--no-photo",
            ],
            vec!["--profile", "p", "card", "set", "A", "B", "C"],
            vec!["--profile", "p", "read", &c, "--accept", "u1"],
            vec!["--profile", "p", "status", "--photo", "x"],
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
    fn json_refused_names_the_verbs_that_take_it() {
        assert_eq!(
            run(&["--profile", "p", "trust", "allow", PEER, "--json"]),
            Err(
                "--json applies to the verbs that read, and to send, cancel, retry, unkeep, card share and store discard"
                    .to_owned()
            )
        );
    }

    #[test]
    fn store_discard_takes_the_scope_it_confirms_and_no_other_verb_takes_confirm() {
        let discard = |extra: &[&str]| {
            let mut args = vec!["--profile", "p", "store", "discard"];
            args.extend_from_slice(extra);
            run(&args)
        };
        assert!(matches!(
            discard(&[]),
            Ok(Command::Run {
                verb: Verb::StoreDiscard(None),
                ..
            })
        ));
        assert!(matches!(
            discard(&["--confirm", "everything"]),
            Ok(Command::Run {
                verb: Verb::StoreDiscard(Some(DiscardScope::Everything)),
                ..
            })
        ));
        assert!(matches!(
            discard(&["--confirm", "unfinished-plaintext"]),
            Ok(Command::Run {
                verb: Verb::StoreDiscard(Some(DiscardScope::UnfinishedPlaintext)),
                ..
            })
        ));
        assert!(discard(&["--confirm", "yes"]).is_err());
        assert!(discard(&["--confirm"]).is_err());
        assert!(discard(&["--confirm", "everything", "--confirm", "everything"]).is_err());
        assert!(
            run(&[
                "--profile",
                "p",
                "store",
                "health",
                "--confirm",
                "everything"
            ])
            .is_err()
        );
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
    fn version_is_alone_or_a_usage_error() {
        assert_eq!(run(&["--version"]), Ok(Command::Version));
        for args in [
            &["--version", "--profile", "p", "status"][..],
            &["--profile", "p", "status", "--version"][..],
            &["--help", "--version"][..],
            &["--version", "--help"][..],
            &["--profile", "--version", "status"][..],
            &["--version", "--version"][..],
        ] {
            assert!(run(args).is_err(), "{args:?}");
        }
        // After `--`, a word: here a message's text.
        assert!(matches!(
            run(&["--profile", "p", "send", "#c", "--", "--version"]),
            Ok(Command::Run {
                verb: Verb::Send { .. },
                ..
            })
        ));
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
