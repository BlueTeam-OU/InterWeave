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
  interweave-human --profile <name> send <conversation> <text>|- [--no-pending]
                   [--endpoint <endpoint>] [--json]
  interweave-human --profile <name> cancel|retry|unkeep <id> [--json]
  interweave-human --profile <name> listen [--input] [--consume] [--events <list>] [--conversation <c>]
                   [--endpoint <endpoint>] [--json]
  interweave-human --profile <name> trust list [--json]
  interweave-human --profile <name> trust allow|revoke <peer>
  interweave-human --profile <name> identities list [--json]
  interweave-human --profile <name> identities enable|disable <endpoint>
  interweave-human --profile <name> identities default <endpoint>|--none
  interweave-human --profile <name> identity show [<endpoint>] [--json]
  interweave-human --profile <name> identity new-key [--confirm key-change] [--json]
  interweave-human --profile <name> store health [--json]
  interweave-human --profile <name> store discard
                   [--confirm everything|unfinished-plaintext] [--json]
  interweave-human --profile <name> card show [--json]
  interweave-human --profile <name> card set <name> [<surname>] [--photo <file>|--no-photo]
  interweave-human --profile <name> card share <conversation> [--json]
  interweave-human --profile <name> proposals [--json]
  interweave-human --profile <name> proposals read <conversation>
                   [--accept <id>]... [--accept-key-change <id>]...
                   [--decline <id>]... [--json]
  interweave-human --profile <name> contacts [--json]
  interweave-human --profile <name> contacts show <contact> [--json]
  interweave-human --profile <name> contacts rename <contact> <name>
  interweave-human --profile <name> contacts photo <contact> <file>|--none
  interweave-human --profile <name> contacts delete <contact>
  interweave-human --profile <name> contacts route delete <contact> <peer>/<endpoint>
  interweave-human --profile <name> peers [--json]
  interweave-human --profile <name> peers add|remove <address>
  interweave-human --profile <name> peers add -|--file <file>
  interweave-human --profile <name> peers show [--json]
  interweave-human --profile <name> peers state [--json]

  --wait <ms>   how long to wait for the session and for an answer
                (default 5000)

A <conversation> is <peer> (its default endpoint), <peer>/<endpoint>, or
#<channel>. An <id> is a message's as messages and read print it: p<row>
pending, u<row> unread, k<row> kept; a proposal's is its card's u<row>,
as proposals prints it. A <contact> is its id, as contacts prints it. `send ... -` reads the text from
stdin. `--` ends the flags. `store discard` without --confirm lists what
it would remove and removes nothing; --confirm names what it listed.
`identity new-key` without --confirm key-change says what a new key would
do and changes nothing.

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
        /// The one of the profile's human endpoints to lease, in place of
        /// the first (`--endpoint`, G2).
        endpoint: Option<EndpointId>,
    },
}

/// The events `listen` names, in the order the README lists them:
/// `--events` takes a comma-separated list of these. `input-refused` is
/// not among them: it answers an `--input` line, which is answered once
/// whatever the filter.
pub(crate) const LISTEN_EVENTS: [&str; 8] = [
    "message",
    "proposal",
    "outbound",
    "session",
    "connectivity",
    "path",
    "disconnected",
    "failed",
];

/// What `listen` prints of what it hears (G7): the events named, and of
/// those the ones of one conversation. What it does not print it does not
/// consume either: a message filtered out stays unread, a proposal
/// undeclined.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ListenFilter {
    /// The events printed, every one when `None`.
    pub(crate) events: Option<Vec<&'static str>>,
    /// The conversation whose events are printed: one naming another
    /// conversation, or another peer, is not. `<peer>` alone is every
    /// conversation with that peer.
    pub(crate) conversation: Option<ConversationKey>,
}

impl ListenFilter {
    /// Whether an `event` of `about` -- the conversation it names, or for
    /// a peer's event that peer's, `None` for this node's own -- is
    /// printed.
    pub(crate) fn passes(&self, event: &str, about: Option<&ConversationKey>) -> bool {
        let named = self.events.as_ref().is_none_or(|e| e.contains(&event));
        let ours = match (&self.conversation, about) {
            (Some(want), Some(have)) => same_conversation(want, have),
            _ => true,
        };
        named && ours
    }
}

/// Whether `have` is the conversation `want` names: a direct one with no
/// endpoint names every endpoint of its peer, and so does a peer's event
/// (`have` with no endpoint) every conversation with it.
fn same_conversation(want: &ConversationKey, have: &ConversationKey) -> bool {
    match (want, have) {
        (
            ConversationKey::Direct {
                peer: p,
                endpoint: e,
            },
            ConversationKey::Direct {
                peer: q,
                endpoint: f,
            },
        ) => p == q && (e.is_none() || f.is_none() || e == f),
        (ConversationKey::Channel(a), ConversationKey::Channel(b)) => a == b,
        _ => false,
    }
}

/// `--events`' list: each a name `listen` prints, none twice.
fn listen_events(list: &str) -> Result<Vec<&'static str>, String> {
    let mut events = Vec::new();
    for word in list.split(',').map(str::trim) {
        let Some(event) = LISTEN_EVENTS.iter().find(|e| **e == word) else {
            return Err(format!(
                "--events: {word:?} is not an event listen prints ({})",
                LISTEN_EVENTS.join(", ")
            ));
        };
        if !events.contains(event) {
            events.push(*event);
        }
    }
    Ok(events)
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
        /// Whether a send still on its way when `--wait` ends stays pending
        /// for a later run to send; `--no-pending` cancels it instead (G4).
        pending: bool,
    },
    /// Cancel a pending message.
    Cancel(String),
    /// Retry a pending message that needs attention.
    Retry(String),
    /// Release a kept message.
    Unkeep(String),
    /// Hold the session open, printing what happens until a signal.
    Listen {
        /// Sends read as JSON lines on stdin, over the lease listen holds
        /// (`--input`, G1).
        input: bool,
        /// Each message marked read, not kept, once its line is written
        /// and flushed (`--consume`, G3).
        consume: bool,
        /// The events printed (`--events`, `--conversation`, G7).
        filter: ListenFilter,
    },
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
    /// An identity's fingerprint: the one in front, or the named human
    /// endpoint's.
    IdentityShow(Option<EndpointId>),
    /// A new key for the identity in front, whose key cannot be used:
    /// made only when `confirm` (the person's `--confirm key-change`),
    /// else what it would do is said and nothing changes.
    IdentityNewKey {
        /// The key change confirmed.
        confirm: bool,
    },
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
        /// The proposals to accept, by id: none of them a key change.
        accept: Vec<String>,
        /// The key changes to accept, by id: each must be one (ADR-0063
        /// decision 4: a changed key is accepted only by the person's
        /// act, named for it).
        accept_key_change: Vec<String>,
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
    /// Delete one route of a contact and its key; never the last.
    RouteDelete {
        /// The contact.
        contact: String,
        /// The route's `PeerId`.
        peer: TransportIdentity,
        /// The route's `EndpointId`.
        endpoint: EndpointId,
    },
    /// The bootstrap addresses: authored and added (ADR-0062).
    PeersList,
    /// This node's own addresses, as another node's `peers add` takes
    /// them (ADR-0062 decision 4).
    PeersShow,
    /// Each trusted peer's state as the daemon's dial gate holds it:
    /// whether it is connected, and when it was last seen.
    PeersState,
    /// Add a peer by address, as the person typed it.
    PeerAdd(String),
    /// Add several addresses, all or none, one per line.
    PeerAddList(ListSource),
    /// Remove an added address.
    PeerRemove(String),
}

/// Where a list of addresses is read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ListSource {
    /// Standard input.
    Stdin,
    /// This file.
    File(std::path::PathBuf),
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
#[allow(
    clippy::struct_excessive_bools,
    reason = "one bool per switch the command line takes"
)]
struct Flags {
    profile: Option<String>,
    json: bool,
    none: bool,
    wait: Option<String>,
    keep: Vec<String>,
    photo: Option<String>,
    no_photo: bool,
    accept: Vec<String>,
    accept_key_change: Vec<String>,
    decline: Vec<String>,
    confirm: Option<String>,
    file: Option<String>,
    input: bool,
    consume: bool,
    no_pending: bool,
    endpoint: Option<String>,
    events: Option<String>,
    on: Option<String>,
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

/// The flags and the words of `args`, or `None` when `--help` is among
/// the flags.
fn flags_and_words(args: Vec<String>) -> Result<Option<(Flags, Vec<String>)>, String> {
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
            "--help" | "-h" => return Ok(None),
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
            "--accept-key-change" => flags.accept_key_change.push(value("--accept-key-change")?),
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
            "--file" => {
                let v = value("--file")?;
                if flags.file.replace(v).is_some() {
                    return Err("--file given twice".to_owned());
                }
            }
            "--json" => flags.json = true,
            "--input" => flags.input = true,
            "--consume" => flags.consume = true,
            "--no-pending" => flags.no_pending = true,
            "--endpoint" => {
                let v = value("--endpoint")?;
                if flags.endpoint.replace(v).is_some() {
                    return Err("--endpoint given twice".to_owned());
                }
            }
            "--events" => {
                let v = value("--events")?;
                if flags.events.replace(v).is_some() {
                    return Err("--events given twice".to_owned());
                }
            }
            "--conversation" => {
                let v = value("--conversation")?;
                if flags.on.replace(v).is_some() {
                    return Err("--conversation given twice".to_owned());
                }
            }
            "--none" => flags.none = true,
            // A lone "-" is send's stdin marker, a word.
            "-" => words.push(arg),
            flag if flag.starts_with('-') => return Err("an unknown flag".to_owned()),
            _ => words.push(arg),
        }
    }
    Ok(Some((flags, words)))
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
    let Some((mut flags, words)) = flags_and_words(args)? else {
        return Ok(Command::Help);
    };
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    if words.is_empty() {
        return Err("a command is required".to_owned());
    }
    let profile = flags.profile.take().ok_or("--profile <name> is required")?;
    let wait = wait_of(flags.wait.take().as_deref())?;
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
                pending: !std::mem::take(&mut flags.no_pending),
            },
            true,
        ),
        ["send", c, text] => (
            Verb::Send {
                conversation: conversation(c)?,
                text: Some((*text).to_owned()),
                pending: !std::mem::take(&mut flags.no_pending),
            },
            true,
        ),
        ["cancel", id] => (Verb::Cancel((*id).to_owned()), true),
        ["retry", id] => (Verb::Retry((*id).to_owned()), true),
        ["unkeep", id] => (Verb::Unkeep((*id).to_owned()), true),
        ["listen"] => (
            Verb::Listen {
                input: std::mem::take(&mut flags.input),
                consume: std::mem::take(&mut flags.consume),
                filter: ListenFilter {
                    events: flags.events.take().map(|l| listen_events(&l)).transpose()?,
                    conversation: flags.on.take().map(|c| conversation(&c)).transpose()?,
                },
            },
            true,
        ),
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
        ["identity", "show"] => (Verb::IdentityShow(None), true),
        ["identity", "show", id] => (Verb::IdentityShow(Some(endpoint(id)?)), true),
        ["identity", "new-key"] => (
            Verb::IdentityNewKey {
                confirm: key_change_confirmed(flags.confirm.take().as_deref())?,
            },
            true,
        ),
        ["store", "health"] => (Verb::StoreHealth, true),
        ["store", "discard"] => (
            Verb::StoreDiscard(discard_scope(flags.confirm.take().as_deref())?),
            true,
        ),
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
                accept_key_change: std::mem::take(&mut flags.accept_key_change),
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
        ["peers", "state"] => (Verb::PeersState, true),
        ["peers", "add"] if flags.file.is_some() => (
            Verb::PeerAddList(ListSource::File(
                flags.file.take().unwrap_or_default().into(),
            )),
            false,
        ),
        ["peers", "add", "-"] => (Verb::PeerAddList(ListSource::Stdin), false),
        ["peers", "add", address] => (Verb::PeerAdd((*address).to_owned()), false),
        ["peers", "remove", address] => (Verb::PeerRemove((*address).to_owned()), false),
        ["contacts", "delete", id] => (Verb::ContactDelete((*id).to_owned()), false),
        ["contacts", "route", "delete", id, route] => {
            let (peer, endpoint) = route_of(route)?;
            (
                Verb::RouteDelete {
                    contact: (*id).to_owned(),
                    peer,
                    endpoint,
                },
                false,
            )
        }
        ["contacts", "photo", id, file] => (
            Verb::ContactPhoto((*id).to_owned(), PhotoArg::File((*file).into())),
            false,
        ),
        ["send", ..] => return Err("send takes a conversation and one text, or -".to_owned()),
        _ => return Err("an unknown command (see --help)".to_owned()),
    };
    let json = std::mem::take(&mut flags.json);
    if matches!(verb, Verb::Listen { input: true, .. }) && !json {
        return Err("--input reads JSON lines and answers in them: it needs --json".to_owned());
    }
    let endpoint = match flags.endpoint.take() {
        Some(id) if matches!(verb, Verb::Listen { .. } | Verb::Send { .. }) => Some(endpoint(&id)?),
        Some(_) => return Err("--endpoint applies to listen and send only".to_owned()),
        None => None,
    };
    if json && !json_allowed {
        return Err(
            "--json applies to the verbs that read, and to send, cancel, retry, unkeep, card share, store discard and identity new-key"
                .to_owned(),
        );
    }
    unused(&flags)?;
    Ok(Command::Run {
        profile,
        verb,
        json,
        wait,
        endpoint,
    })
}

/// A contact's route: `<peer>/<endpoint>`, as `contacts show` prints it.
fn route_of(text: &str) -> Result<(TransportIdentity, EndpointId), String> {
    match conversation(text)? {
        ConversationKey::Direct {
            peer,
            endpoint: Some(endpoint),
        } => Ok((peer, endpoint)),
        _ => Err("a route is <peer>/<endpoint>, as contacts show prints it".to_owned()),
    }
}

/// `--wait`'s milliseconds, or the default.
fn wait_of(ms: Option<&str>) -> Result<Duration, String> {
    ms.map_or(Ok(DEFAULT_WAIT), |ms| {
        ms.parse::<u64>()
            .map(Duration::from_millis)
            .map_err(|_| "--wait: not a number of milliseconds".to_owned())
    })
}

/// `store discard`'s `--confirm`: the scope it names, if any.
fn discard_scope(confirm: Option<&str>) -> Result<Option<DiscardScope>, String> {
    match confirm {
        None => Ok(None),
        Some("everything") => Ok(Some(DiscardScope::Everything)),
        Some("unfinished-plaintext") => Ok(Some(DiscardScope::UnfinishedPlaintext)),
        Some(_) => Err("--confirm takes everything or unfinished-plaintext".to_owned()),
    }
}

/// `identity new-key`'s `--confirm`: whether it names the key change.
fn key_change_confirmed(confirm: Option<&str>) -> Result<bool, String> {
    match confirm {
        None => Ok(false),
        Some("key-change") => Ok(true),
        Some(_) => Err("identity new-key's --confirm takes key-change".to_owned()),
    }
}

/// A flag given to a verb that does not take it: a usage error naming
/// the verbs it applies to.
fn unused(flags: &Flags) -> Result<(), String> {
    if !flags.keep.is_empty() {
        return Err("--keep applies to read only".to_owned());
    }
    if flags.none {
        return Err("--none applies to identities default and contacts photo only".to_owned());
    }
    if !flags.accept.is_empty() || !flags.accept_key_change.is_empty() || !flags.decline.is_empty()
    {
        return Err(
            "--accept, --accept-key-change and --decline apply to proposals read only".to_owned(),
        );
    }
    if flags.confirm.is_some() {
        return Err("--confirm applies to store discard and identity new-key only".to_owned());
    }
    if flags.photo.is_some() || flags.no_photo {
        return Err("--photo and --no-photo apply to card set only".to_owned());
    }
    if flags.file.is_some() {
        return Err("--file applies to peers add, in place of its address".to_owned());
    }
    if flags.input || flags.consume {
        return Err("--input and --consume apply to listen only".to_owned());
    }
    if flags.events.is_some() || flags.on.is_some() {
        return Err("--events and --conversation apply to listen only".to_owned());
    }
    if flags.no_pending {
        return Err("--no-pending applies to send only".to_owned());
    }
    Ok(())
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
            &["--profile", "p", "identity", "show"],
            &["--profile", "p", "identity", "show", "second", "--json"],
            &["--profile", "p", "identity", "new-key", "--json"],
            &[
                "--profile",
                "p",
                "identity",
                "new-key",
                "--confirm",
                "key-change",
            ],
            &["--profile", "p", "store", "discard", "--json"],
            &["--profile", "p", "peers", "show", "--json"],
            &["--profile", "p", "peers", "state", "--json"],
            &["--profile", "p", "peers", "state"],
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
                "--accept-key-change",
                "u5",
                "--decline",
                "u4",
            ],
            &["--profile", "p", "contacts", "--json"],
            &["--profile", "p", "contacts", "show", "c1", "--json"],
            &["--profile", "p", "contacts", "rename", "c1", "Ada"],
            &["--profile", "p", "contacts", "photo", "c1", "a.jpg"],
            &["--profile", "p", "contacts", "photo", "c1", "--none"],
            &["--profile", "p", "contacts", "delete", "c1"],
            &["--profile", "p", "contacts", "route", "delete", "c1", &c],
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
            &["--profile", "p", "peers", "add", "-"],
            &["--profile", "p", "peers", "add", "--file", "list.txt"],
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
            "identity show",
            "identity new-key",
            "card show",
            "card set",
            "card share",
            "proposals read",
            "contacts show",
            "contacts rename",
            "contacts photo",
            "contacts delete",
            "contacts route delete",
            "peers add|remove",
            "peers add -|--file",
            "peers state",
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
                pending: true,
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
            vec!["--profile", "p", "read", &c, "--accept-key-change", "u1"],
            vec![
                "--profile",
                "p",
                "proposals",
                "read",
                &c,
                "--accept-key-change",
            ],
            vec!["--profile", "p", "status", "--photo", "x"],
            vec!["--profile", "p", "messages", &c, "--keep", "u1"],
            vec!["--profile", "p", "status", "--none"],
            vec!["--profile", "p", "contacts", "route", "delete", "c1", PEER],
            vec![
                "--profile",
                "p",
                "contacts",
                "route",
                "delete",
                "c1",
                "#general",
            ],
            vec![
                "--profile",
                "p",
                "contacts",
                "route",
                "delete",
                "c1",
                &c,
                "--json",
            ],
            vec![
                "--profile",
                "p",
                "peers",
                "add",
                "/ip4/1.2.3.4",
                "--file",
                "f",
            ],
            vec!["--profile", "p", "peers", "add", "-", "--file", "f"],
            vec!["--profile", "p", "peers", "remove", "--file", "f"],
            vec!["--profile", "p", "peers", "add", "--file", "f", "--json"],
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
                "--json applies to the verbs that read, and to send, cancel, retry, unkeep, card share, store discard and identity new-key"
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
        assert!(discard(&["--confirm", "key-change"]).is_err());
        let new_key = |extra: &[&str]| {
            let mut args = vec!["--profile", "p", "identity", "new-key"];
            args.extend_from_slice(extra);
            run(&args)
        };
        assert!(matches!(
            new_key(&[]),
            Ok(Command::Run {
                verb: Verb::IdentityNewKey { confirm: false },
                ..
            })
        ));
        assert!(matches!(
            new_key(&["--confirm", "key-change"]),
            Ok(Command::Run {
                verb: Verb::IdentityNewKey { confirm: true },
                ..
            })
        ));
        assert!(new_key(&["--confirm", "everything"]).is_err());
        assert!(new_key(&["--confirm", "yes"]).is_err());
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

    /// The carrier's flags (G1-G4): each where it applies and nowhere
    /// else, `--input` only with `--json`, `--endpoint` parsed and carried.
    /// Mutations: a flag accepted on another verb; --input without --json.
    #[test]
    fn the_carriers_flags_go_where_they_apply() {
        let c = format!("{PEER}/human");
        assert!(matches!(
            run(&["--profile", "p", "listen", "--input", "--consume", "--json"]),
            Ok(Command::Run {
                verb: Verb::Listen {
                    input: true,
                    consume: true,
                    ..
                },
                ..
            })
        ));
        assert!(
            run(&["--profile", "p", "listen", "--input"]).is_err(),
            "--input needs --json"
        );
        assert!(matches!(
            run(&["--profile", "p", "send", &c, "x", "--no-pending"]),
            Ok(Command::Run {
                verb: Verb::Send { pending: false, .. },
                ..
            })
        ));
        assert!(matches!(
            run(&["--profile", "p", "listen", "--endpoint", "fabric-control"]),
            Ok(Command::Run {
                endpoint: Some(e),
                ..
            }) if e.as_str() == "fabric-control"
        ));
        for wrong in [
            vec!["--profile", "p", "status", "--input"],
            vec!["--profile", "p", "status", "--consume"],
            vec!["--profile", "p", "listen", "--no-pending"],
            vec!["--profile", "p", "status", "--endpoint", "human"],
        ] {
            assert!(run(&wrong).is_err(), "{wrong:?}");
        }
    }

    /// G7: `listen --events` takes a list of the events listen prints,
    /// each once, and `--conversation` a conversation as `send` does;
    /// neither applies elsewhere, an unknown event is refused, and
    /// `input-refused` is never filtered. Mutation: an unknown event
    /// taken.
    #[test]
    fn listen_filters_name_events_and_a_conversation() {
        let filter = |args: &[&str]| match verb(args) {
            Verb::Listen { filter, .. } => filter,
            other => panic!("{other:?}"),
        };
        let f = filter(&[
            "--profile",
            "p",
            "listen",
            "--events",
            "message, path,message",
            "--conversation",
            PEER,
        ]);
        assert_eq!(f.events, Some(vec!["message", "path"]));
        assert!(matches!(
            &f.conversation,
            Some(ConversationKey::Direct { endpoint: None, .. })
        ));
        assert!(f.passes("message", f.conversation.as_ref()));
        assert!(!f.passes("outbound", None), "not named");
        assert!(f.passes("path", None), "this node's own: no conversation");
        assert_eq!(
            filter(&["--profile", "p", "listen"]),
            ListenFilter::default()
        );
        for wrong in [
            vec!["--profile", "p", "listen", "--events", "messages"],
            vec!["--profile", "p", "listen", "--events", "input-refused"],
            vec!["--profile", "p", "listen", "--events", ""],
            vec!["--profile", "p", "listen", "--conversation", "nope"],
            vec!["--profile", "p", "status", "--events", "message"],
            vec!["--profile", "p", "status", "--conversation", PEER],
        ] {
            assert!(run(&wrong).is_err(), "{wrong:?}");
        }
    }

    /// `<peer>` names every conversation with it, `<peer>/<endpoint>` the
    /// one; a peer's event (no endpoint) is any of its conversations', and
    /// a channel only itself. Mutation: an endpoint-less filter matching
    /// one endpoint only.
    #[test]
    fn a_conversation_filter_names_a_peer_or_one_of_its_endpoints() {
        let direct = |e: Option<&str>| ConversationKey::Direct {
            peer: TransportIdentity::parse(PEER).expect("a PeerId"),
            endpoint: e.map(|e| EndpointId::parse(e).expect("an endpoint")),
        };
        let on = |c: ConversationKey| ListenFilter {
            events: None,
            conversation: Some(c),
        };
        assert!(on(direct(None)).passes("message", Some(&direct(Some("human")))));
        assert!(on(direct(Some("human"))).passes("path", Some(&direct(None))));
        assert!(!on(direct(Some("human"))).passes("message", Some(&direct(Some("other")))));
        let channel = ConversationKey::Channel(ChannelId::parse("news").expect("a channel"));
        assert!(!on(channel.clone()).passes("path", Some(&direct(None))));
        assert!(on(channel.clone()).passes("message", Some(&channel)));
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
                endpoint: None,
            })
        );
    }
}
