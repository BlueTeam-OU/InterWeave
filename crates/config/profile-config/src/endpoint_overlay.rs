// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The endpoint overlay: what `admin.endpoints.set_enabled` and
//! `admin.endpoints.set_default` changed, kept in the state directory
//! beside the trust overlay (ADR-0028 A 2026-10-11).
//!
//! `<state>/endpoint-overlay.json` holds
//! `{"enabled": {<endpoint-id>: <bool>, ...}, "default": <endpoint-id> | null}`
//! and four members that are absent when empty (ADR-0067 decision 6),
//! each `<endpoint-id>` a DERIVED id (`EndpointId::parse_derived`).
//! `enabled` and `default` are deltas against `config.yaml`'s
//! `endpoints`: `enabled` is always present and holds only the endpoints
//! whose state differs from the configuration; `default` is present only
//! when the default differs from the configured one -- an endpoint id, or
//! `null` for "no default" where `config.yaml` names one. Absent, the
//! configured default applies.
//!
//! The four members:
//!
//! - `inbound` and `outbound`, each `{<endpoint-id>: [<peer-id>, ...]}`:
//!   an endpoint's OWN trusted set in each direction, which replaces the
//!   operator's list at that endpoint and enters the admission set
//!   ([`EndpointSet`]). An id present with `[]` is an own and empty set;
//!   an id absent has no own set -- two states, kept apart through load,
//!   normalise and write. An id stands in both members or neither
//!   ([`EndpointOverlayError::SetInOneMember`]); the two lists may
//!   differ. At most [`PeerTrustPolicy::MAX_ALLOWED_PEERS`] a list and
//!   [`MAX_ENDPOINT_SET_ROWS`] rows over every list.
//! - `added`: endpoint entries added through the administrative path,
//!   each as a `config.yaml` entry (`key`, `alias`, `enabled`,
//!   `advertise`, `allowed_client_kinds`) and never with `inbound` or
//!   `outbound`: an added entry's trust is its overlay set alone, so its
//!   id stands in both of those members, `[]` for no trusted peer
//!   ([`EndpointOverlayError::AddedEntryWithoutSet`]); an absent id is
//!   "no own set" for an AUTHORED endpoint only. The entry's own filters
//!   are [`ADDED_TRUST`], which admits nobody, so a runtime that holds
//!   the endpoint without its set fails closed in both directions.
//!   Authored and added entries together are at most
//!   [`MAX_ENDPOINTS`](crate::MAX_ENDPOINTS), and an added id or alias
//!   that an authored or another added entry holds refuses the load.
//! - `advertise`, `{<endpoint-id>: <bool>}`: a delta over an AUTHORED
//!   entry's `advertise`, as `enabled` is over its `enabled`.
//!
//! Like the trust overlay it is AUTHORISATION, not a cache: a missing
//! file is the empty overlay, but a present one that cannot be trusted
//! -- unreadable, unparseable, someone else's, readable or writable by
//! anyone else -- stops the start. Skipping it would re-enable every
//! endpoint the operator disabled, and deleting it to "reset" does the
//! same, which is why nothing here ever removes it.
//!
//! `config.yaml` is never written. The overlay is NORMALISED at load
//! against the configuration the daemon starts with, so an operator who
//! edits `config.yaml` between restarts is not undone by a stale entry,
//! and THE EFFECTIVE DEFAULT IS ENABLED OR THERE IS NONE: normalisation
//! never undoes a disable, only a default
//! ([`EndpointOverlay::load_within`]). A reader that must write nothing
//! takes the same normalisation in memory
//! ([`EndpointOverlay::read_within`]).
//!
//! # A release before Stage 22's overlay
//!
//! Releases before Stage 22 keyed the file by endpoint LABEL
//! (`{"enabled": {"claude": false}}`). A member that is not a derived id
//! but is in the alias grammar is a LABEL MEMBER, and the loader maps
//! nothing: a label equal to the alias of a configured entry is REFUSED
//! ([`EndpointOverlayError::LabelMember`]) naming the command that ends
//! it, `transportctl endpoint adopt` or `endpoint init`, which move the
//! members to the id ([`EndpointOverlay::carry_label_within`]); one equal
//! to no configured alias is the retired endpoint's and is dropped as an
//! unconfigured id is ([`Normalised::LabelWithoutAliasDropped`]). Dropping
//! it silently when an entry DID carry the alias would lose the
//! operator's disable.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use interweave_transport_api::{EndpointId, TransportIdentity};
pub use interweave_trust_api::MAX_ENDPOINT_SET_ROWS;
use interweave_trust_api::{EndpointSet, PeerTrustPolicy};
use serde::ser::{SerializeMap as _, SerializeSeq as _};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::private_read::{self, PrivateReadError};
use crate::{
    ClientKind, EndpointAlias, EndpointConfig, EndpointsConfig, MAX_ENDPOINTS, PersistError,
    ProfilePaths, TrustBoundary, persist,
};

/// The overlay's file name in the profile's state directory.
pub const ENDPOINT_OVERLAY_FILE: &str = "endpoint-overlay.json";

/// The most an endpoint overlay may hold on disk: 1.5 MiB, over the
/// largest file a normalised overlay can be, as `write_within` writes it
/// (pretty-printed, two-space indent). That file names at most
/// [`MAX_ENDPOINTS`](crate::MAX_ENDPOINTS) endpoints, each id 52 bytes
/// ([`EndpointId::MAX_BYTES`] is the ceiling), and is, per piece:
///
/// - the set rows, [`MAX_ENDPOINT_SET_ROWS`] (16384) of the longest
///   `PeerId`, 52 bytes: 6 indent + 54 quoted + comma and newline, 62
///   bytes a row, 1,015,808;
/// - each of 64 ids heading a list in both members, 62 bytes to open
///   and 7 to close it, 128 times: 8,832;
/// - the `enabled` map, 64 ids at 67 bytes: 4,288;
/// - 64 added entries, the dominant authored-or-added case (an
///   `advertise` delta is 67 bytes, an added entry far more): `key` 73,
///   `alias` at its 64 bytes 83, `enabled` and `advertise` 49, and at
///   most 16 `allowed_client_kinds` of 64 characters, which as JSON
///   are at worst control characters of six bytes each (394 bytes and
///   a comma and newline a line, 6,336), about 6,594 an entry,
///   422,016;
///
/// 1,451,000 bytes with the member heads. The bound leaves about 8% over
/// that (`the_largest_legal_overlay_fits_its_bound_and_the_bound_is_not_loose`
/// builds that file and holds the bound to it from both sides); anything
/// past it is refused before it is parsed.
pub const MAX_ENDPOINT_OVERLAY_BYTES: u64 = 1536 * 1024;

/// One member's own sets by endpoint.
type EndpointSets = BTreeMap<EndpointId, EndpointSet>;

/// The overlay's members, normalised or not;
/// [`EndpointOverlay::load_within`] hands out only a normalised one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct EndpointOverlay {
    /// The endpoints whose enabled state differs from `config.yaml`'s
    /// (or, for an added endpoint, from its entry's).
    enabled: BTreeMap<EndpointId, bool>,
    /// `None`: absent, the configured default applies. `Some(None)`:
    /// `null`, no default. `Some(Some(id))`: that endpoint.
    #[expect(
        clippy::option_option,
        reason = "the member's three states are serde's: absent, null, an id"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<Option<EndpointId>>,
    /// Each own set's inbound list. INVARIANT: the same ids as
    /// `outbound`, which every constructor and mutator keeps and
    /// `an_id_in_one_member_alone_is_refused` pins at the file. Presence
    /// is the state: an id with an empty set is own and empty.
    #[serde(
        skip_serializing_if = "BTreeMap::is_empty",
        serialize_with = "sets_out"
    )]
    inbound: EndpointSets,
    /// Each own set's outbound list; see `inbound`.
    #[serde(
        skip_serializing_if = "BTreeMap::is_empty",
        serialize_with = "sets_out"
    )]
    outbound: EndpointSets,
    /// The entries added through the administrative path, by id, written
    /// as a list. Never carries trust of its own: an added entry's is its
    /// set above, which a loaded overlay always holds for it, and its
    /// filters are [`ADDED_TRUST`] whether it was loaded or added.
    #[serde(
        skip_serializing_if = "BTreeMap::is_empty",
        serialize_with = "added_out"
    )]
    added: BTreeMap<EndpointId, EndpointConfig>,
    /// `advertise` deltas over AUTHORED entries.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    advertise: BTreeMap<EndpointId, bool>,
}

fn sets_out<S: Serializer>(sets: &EndpointSets, s: S) -> Result<S::Ok, S::Error> {
    let mut map = s.serialize_map(Some(sets.len()))?;
    for (id, set) in sets {
        map.serialize_entry(id, &set.peers().iter().collect::<Vec<_>>())?;
    }
    map.end()
}

/// An added entry as it is written: a `config.yaml` entry without the
/// trust members, which an added entry never carries.
#[derive(Serialize)]
struct AddedOut<'a> {
    key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    alias: Option<&'a EndpointAlias>,
    enabled: bool,
    advertise: bool,
    #[serde(skip_serializing_if = "<[ClientKind]>::is_empty")]
    allowed_client_kinds: &'a [ClientKind],
}

fn added_out<S: Serializer>(
    added: &BTreeMap<EndpointId, EndpointConfig>,
    s: S,
) -> Result<S::Ok, S::Error> {
    let mut seq = s.serialize_seq(Some(added.len()))?;
    for e in added.values() {
        seq.serialize_element(&AddedOut {
            key: e.key_file_name(),
            alias: e.alias.as_ref(),
            enabled: e.enabled,
            advertise: e.advertise,
            allowed_client_kinds: &e.allowed_client_kinds,
        })?;
    }
    seq.end()
}

/// The file as it is written, members still TEXT: what a derived id and a
/// label both are until [`EndpointOverlay::resolve`] judges them against
/// the configuration. Read and written by the carry untouched.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    #[serde(deserialize_with = "unique_entries")]
    enabled: BTreeMap<String, bool>,
    #[expect(
        clippy::option_option,
        reason = "the member's three states are serde's: absent, null, a text"
    )]
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    default: Option<Option<String>>,
    // The four members below are read and written back untouched by the
    // carry, which judges none of them.
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "unique_entries"
    )]
    inbound: BTreeMap<String, Vec<String>>,
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "unique_entries"
    )]
    outbound: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    added: Vec<serde_json::Value>,
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "unique_entries"
    )]
    advertise: BTreeMap<String, bool>,
}

/// Which member of the overlay a label was found in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayMember {
    /// A key of `enabled`.
    Enabled,
    /// The `default` value.
    Default,
    /// A key of `inbound`.
    Inbound,
    /// A key of `outbound`.
    Outbound,
}

impl core::fmt::Display for OverlayMember {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Enabled => "enabled",
            Self::Default => "default",
            Self::Inbound => "inbound",
            Self::Outbound => "outbound",
        })
    }
}

/// What one member's text is, by the DERIVED grammar
/// (`EndpointId::parse_derived`).
enum Member {
    Id(EndpointId),
    /// Not a derived id, but in the alias grammar.
    Label(EndpointAlias),
}

/// A text that is neither grammar is a parse refusal.
fn classify(text: &str) -> Result<Member, EndpointOverlayError> {
    if let Ok(id) = EndpointId::parse_derived(text) {
        return Ok(Member::Id(id));
    }
    EndpointAlias::parse(text).map(Member::Label).map_err(|_| {
        EndpointOverlayError::Parse(format!(
            "{text:?} is neither a derived endpoint id nor an endpoint label"
        ))
    })
}

/// A `default` member that is present, as `null` or an id: absence is
/// `#[serde(default)]`'s, so the two never read alike.
#[expect(
    clippy::option_option,
    reason = "the member's three states are serde's: absent, null, a text"
)]
fn present<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(d).map(Some)
}

/// A member's entries (`enabled`, `inbound`, `outbound`, `advertise`),
/// an endpoint named twice refused: a map keeps the last of a key named
/// twice, which would read the operator's first state of it as never
/// written (ADR-0028 A 2026-10-11).
fn unique_entries<'de, D, V>(d: D) -> Result<BTreeMap<String, V>, D::Error>
where
    D: Deserializer<'de>,
    V: Deserialize<'de>,
{
    struct Entries<V>(core::marker::PhantomData<V>);
    impl<'de, V: Deserialize<'de>> serde::de::Visitor<'de> for Entries<V> {
        type Value = BTreeMap<String, V>;
        fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("an object keyed by endpoint id")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut out = BTreeMap::new();
            while let Some((id, value)) = map.next_entry::<String, V>()? {
                if out.contains_key(&id) {
                    return Err(serde::de::Error::custom(format!(
                        "endpoint {id} named twice"
                    )));
                }
                out.insert(id, value);
            }
            Ok(out)
        }
    }
    d.deserialize_map(Entries(core::marker::PhantomData))
}

/// The endpoints' state once the overlay is composed over the
/// configuration: what the runtime starts with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveEndpoints {
    /// Every configured endpoint and whether it is enabled.
    pub enabled: BTreeMap<EndpointId, bool>,
    /// The default, enabled whenever there is one.
    pub default: Option<EndpointId>,
}

/// What [`EndpointOverlay::carry_label_within`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carried {
    /// No file, or no member for the label: nothing was written.
    Nothing,
    /// The label's members now name the id.
    Moved,
}

/// What normalising an overlay at load changed, one entry each: what the
/// caller warns about, naming the entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Normalised {
    /// An `enabled` entry naming an endpoint `config.yaml` no longer has.
    UnknownEndpointDropped(EndpointId),
    /// A label member (an `enabled` key or the `default`) equal to the
    /// alias of no configured entry: the retired endpoint's.
    LabelWithoutAliasDropped {
        /// The label.
        label: String,
        /// Where it stood.
        member: OverlayMember,
    },
    /// An `enabled` entry equal to `config.yaml`'s state.
    EnabledEqualsConfig(EndpointId),
    /// A `default` naming an endpoint `config.yaml` no longer has.
    UnknownDefaultDropped(EndpointId),
    /// A `default` equal to the configured default (`None`: both none).
    DefaultEqualsConfig(Option<EndpointId>),
    /// A `default` naming a disabled endpoint: dropped, the configured
    /// default applies.
    DisabledDefaultDropped(EndpointId),
    /// The configured default is disabled by the overlay: the overlay
    /// now says no default (`null`).
    ConfiguredDefaultCleared(EndpointId),
    /// An own set (both members together) for an endpoint neither
    /// `config.yaml` nor `added` has.
    UnknownSetDropped(EndpointId),
    /// An `advertise` delta for an endpoint that is not an authored one:
    /// unknown, or an added entry, which carries its own.
    UnknownAdvertiseDropped(EndpointId),
    /// An `advertise` delta equal to `config.yaml`'s value.
    AdvertiseEqualsConfig(EndpointId),
}

impl core::fmt::Display for Normalised {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnknownEndpointDropped(id) => write!(
                f,
                "dropped enabled.{}: config.yaml configures no such endpoint",
                id.as_str()
            ),
            Self::LabelWithoutAliasDropped { label, member } => write!(
                f,
                "dropped {member} label {label}: a label, and no configured endpoint has that alias"
            ),
            Self::EnabledEqualsConfig(id) => write!(
                f,
                "dropped enabled.{}: config.yaml already says so",
                id.as_str()
            ),
            Self::UnknownDefaultDropped(id) => write!(
                f,
                "dropped default {}: config.yaml configures no such endpoint",
                id.as_str()
            ),
            Self::DefaultEqualsConfig(Some(id)) => write!(
                f,
                "dropped default {}: config.yaml already names it",
                id.as_str()
            ),
            Self::DefaultEqualsConfig(None) => {
                write!(f, "dropped default null: config.yaml names no default")
            }
            Self::DisabledDefaultDropped(id) => write!(
                f,
                "dropped default {}: the endpoint is disabled; the configured default applies",
                id.as_str()
            ),
            Self::ConfiguredDefaultCleared(id) => write!(
                f,
                "set default null: the configured default {} is disabled",
                id.as_str()
            ),
            Self::UnknownSetDropped(id) => write!(
                f,
                "dropped the inbound and outbound sets of {}: no such endpoint is configured",
                id.as_str()
            ),
            Self::UnknownAdvertiseDropped(id) => write!(
                f,
                "dropped advertise.{}: config.yaml has no such entry",
                id.as_str()
            ),
            Self::AdvertiseEqualsConfig(id) => write!(
                f,
                "dropped advertise.{}: config.yaml already says so",
                id.as_str()
            ),
        }
    }
}

/// Why an endpoint overlay stops the start, or why a set could not be
/// kept or made.
#[derive(Debug)]
pub enum EndpointOverlayError {
    /// The file is present and could not be read.
    Read(std::io::Error),
    /// The file, or its directory, is not private to this uid, or the
    /// file is a symbolic link or not a regular file.
    NotPrivate {
        /// What is wrong with it.
        detail: String,
    },
    /// The file is larger than [`MAX_ENDPOINT_OVERLAY_BYTES`].
    TooLarge,
    /// The file is not the overlay's shape.
    Parse(String),
    /// A label member equal to the alias of a configured entry: the
    /// operator's choice for that endpoint was written under its label
    /// by a release before Stage 22, and dropping it would lose it. The
    /// loader maps nothing; the command in the text moves it.
    LabelMember {
        /// The label.
        label: String,
        /// Where it stands.
        member: OverlayMember,
    },
    /// The carry found `enabled` naming both the label and the id, with
    /// different states: which is the operator's is not for a tool to
    /// pick.
    CarryConflict {
        /// The label.
        label: String,
        /// The id the label was to move to.
        id: EndpointId,
    },
    /// An endpoint id stands in one of `inbound` and `outbound` and not
    /// the other: an own set is both lists or none (ADR-0067 decision 6).
    SetInOneMember {
        /// The endpoint.
        endpoint: EndpointId,
        /// The member it stands in.
        only_in: OverlayMember,
    },
    /// A peer is named twice in one list. Refused rather than collapsed:
    /// a file that says so has been edited by something that is not this
    /// crate, which writes each peer once.
    DuplicatePeer {
        /// The member.
        member: OverlayMember,
        /// The endpoint.
        endpoint: EndpointId,
        /// The peer named twice.
        peer: TransportIdentity,
    },
    /// One list holds more than [`PeerTrustPolicy::MAX_ALLOWED_PEERS`].
    SetTooLarge {
        /// The member.
        member: OverlayMember,
        /// The endpoint.
        endpoint: EndpointId,
        /// Peers in the list.
        got: usize,
    },
    /// All the lists together hold more than [`MAX_ENDPOINT_SET_ROWS`]
    /// rows, both members counted.
    SetRowsExceeded {
        /// Rows the file, or the write, would hold.
        got: usize,
    },
    /// An added entry carries `inbound` or `outbound`: its trust is its
    /// overlay set alone.
    AddedEntryCarriesTrust {
        /// The member it carries.
        member: OverlayMember,
    },
    /// An added entry's id stands in neither `inbound` nor `outbound`.
    /// An added endpoint's trust is its overlay set alone, `[]` at the
    /// add; the file states it rather than leave the endpoint to its
    /// filter, [`ADDED_TRUST`], which admits nobody.
    AddedEntryWithoutSet {
        /// The added endpoint.
        endpoint: EndpointId,
    },
    /// An added entry's id is an authored entry's, or another added
    /// entry's.
    AddedIdCollides(EndpointId),
    /// An added entry's alias is an authored entry's, or another added
    /// entry's: an alias is unique within the profile.
    AddedAliasCollides(EndpointAlias),
    /// Authored and added entries together exceed
    /// [`MAX_ENDPOINTS`](crate::MAX_ENDPOINTS).
    TooManyEndpoints {
        /// Endpoints the configuration and the overlay hold together.
        got: usize,
    },
    /// A set named an endpoint `config.yaml` does not configure.
    Unknown,
    /// A default was set to a disabled endpoint.
    Disabled,
    /// The overlay could not be written: at load, the normalisation
    /// rewrite; at a set, the set itself.
    Write(PersistError),
}

impl core::fmt::Display for EndpointOverlayError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Read(e) => write!(f, "the endpoint overlay cannot be read: {e}"),
            Self::NotPrivate { detail } => {
                write!(f, "the endpoint overlay is not private: {detail}")
            }
            Self::TooLarge => write!(
                f,
                "the endpoint overlay is larger than {MAX_ENDPOINT_OVERLAY_BYTES} bytes"
            ),
            Self::Parse(e) => write!(f, "the endpoint overlay does not parse: {e}"),
            Self::LabelMember { label, member } => write!(
                f,
                "the endpoint overlay names an endpoint by its label '{label}' in `{member}`, \
                 and config.yaml has an entry with that alias; a release before Stage 22 wrote \
                 it, and the overlay is read by id. Run `transportctl endpoint adopt --profile \
                 <profile> --alias {label}` (or `transportctl endpoint init --profile <profile> \
                 --alias {label}` for an endpoint that had no sealed key), which moves it"
            ),
            Self::CarryConflict { label, id } => write!(
                f,
                "the endpoint overlay's `enabled` names both '{label}' and {} with different \
                 states; remove the one that is not meant by hand, then run the command again",
                id.as_str()
            ),
            Self::SetInOneMember { endpoint, only_in } => write!(
                f,
                "the endpoint overlay names {} in `{only_in}` and not in the other of \
                 `inbound` and `outbound`: an endpoint's own set is both lists or none",
                endpoint.as_str()
            ),
            Self::DuplicatePeer {
                member,
                endpoint,
                peer,
            } => write!(
                f,
                "the endpoint overlay names peer {} twice in `{member}` of {}",
                peer.as_str(),
                endpoint.as_str()
            ),
            Self::SetTooLarge {
                member,
                endpoint,
                got,
            } => write!(
                f,
                "the endpoint overlay's `{member}` of {} holds {got} peers; the ceiling is {}",
                endpoint.as_str(),
                PeerTrustPolicy::MAX_ALLOWED_PEERS
            ),
            Self::SetRowsExceeded { got } => write!(
                f,
                "the endpoint overlay's own sets hold {got} rows together; the ceiling is \
                 {MAX_ENDPOINT_SET_ROWS}"
            ),
            Self::AddedEntryCarriesTrust { member } => write!(
                f,
                "an endpoint the endpoint overlay adds carries `{member}`: an added \
                 endpoint's trust is its own set in the overlay alone"
            ),
            Self::AddedEntryWithoutSet { endpoint } => write!(
                f,
                "the endpoint overlay adds endpoint {}, which has no own trusted set: an \
                 added endpoint's trust is its set alone; give it `[]` in both `inbound` \
                 and `outbound`",
                endpoint.as_str()
            ),
            Self::AddedIdCollides(id) => write!(
                f,
                "the endpoint overlay adds endpoint {}, which is already configured or added",
                id.as_str()
            ),
            Self::AddedAliasCollides(alias) => write!(
                f,
                "the endpoint overlay adds an endpoint with alias {alias}, which another \
                 entry already carries"
            ),
            Self::TooManyEndpoints { got } => write!(
                f,
                "config.yaml and the endpoint overlay hold {got} endpoints together; \
                 the maximum is {MAX_ENDPOINTS}"
            ),
            Self::Unknown => write!(f, "no such endpoint is configured"),
            Self::Disabled => write!(f, "the endpoint is disabled"),
            Self::Write(e) => write!(f, "the endpoint overlay cannot be written: {e}"),
        }
    }
}

impl core::error::Error for EndpointOverlayError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Read(e) => Some(e),
            Self::Write(e) => Some(e),
            _ => None,
        }
    }
}

impl From<PrivateReadError> for EndpointOverlayError {
    fn from(e: PrivateReadError) -> Self {
        match e {
            PrivateReadError::Read(e) => Self::Read(e),
            PrivateReadError::NotPrivate { detail } => Self::NotPrivate { detail },
            PrivateReadError::TooLarge => Self::TooLarge,
        }
    }
}

impl EndpointOverlayError {
    /// Whether the failed write had already put the new overlay in place:
    /// the rename landed and syncing the directory failed. A caller that
    /// promises "a failed write changes nothing" puts the previous
    /// overlay back after this one.
    #[must_use]
    pub const fn installed(&self) -> bool {
        matches!(self, Self::Write(PersistError::Unsynced(_)))
    }
}

/// An added entry's `inbound` and `outbound` filter: the empty static
/// subset, never the inheritance an absent member reads as in
/// `config.yaml` (ADR-0067 decision 6: an added endpoint admits nobody
/// until a set names a peer). The runtime falls back to an entry's filter
/// whenever it holds the endpoint without its own set -- after a removal
/// left the overlay ahead of it, a later trust write publishes sources
/// without that set -- and an inheriting filter there would admit every
/// operator-listed peer (the composition crate's
/// `tests/endpoint_set_ahead.rs`). Every construction of an added entry,
/// the add's and the load's, sets both members to it
/// (`an_add_writes_the_entry_and_its_two_empty_sets`,
/// `a_loaded_added_entrys_filters_admit_nobody`).
pub const ADDED_TRUST: interweave_trust_api::EndpointTrustPolicy =
    interweave_trust_api::EndpointTrustPolicy::StaticSubset {
        allowed_peers: BTreeSet::new(),
    };

/// What an added entry is judged against when it is not an authored
/// one's: the member's name for the error that names it.
const TRUST_MEMBERS: [(&str, OverlayMember); 2] = [
    ("inbound", OverlayMember::Inbound),
    ("outbound", OverlayMember::Outbound),
];

/// What a refused set write or seed reports when the seed itself is too
/// long: the same per-list error a file's list would meet.
fn bounded_set(
    member: OverlayMember,
    endpoint: &EndpointId,
    peers: impl IntoIterator<Item = TransportIdentity>,
) -> Result<EndpointSet, EndpointOverlayError> {
    let peers: BTreeSet<TransportIdentity> = peers.into_iter().collect();
    let got = peers.len();
    EndpointSet::new(peers).map_err(|_| EndpointOverlayError::SetTooLarge {
        member,
        endpoint: endpoint.clone(),
        got,
    })
}

/// What [`EndpointOverlay::seed_sets`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seeded {
    /// The overlay already carries a set for the endpoint, an empty one
    /// included: nothing changed.
    AlreadyPresent,
    /// The overlay with the endpoint's set created from the seed.
    Created(EndpointOverlay),
}

/// Why an administrative add is refused (`admin.endpoints.add`,
/// `LOCAL-IPC.md` §The endpoint set's methods), in the order
/// [`EndpointOverlay::add`] checks them: [`Self::ALL`]'s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddRefusal {
    /// The `key` value is not `<EndpointId>.key` under the full grammar
    /// check: the load's `EndpointIdNotDerived`.
    IdNotDerived,
    /// The alias is outside the alias grammar: the load's `AliasInvalid`.
    AliasInvalid,
    /// The key file is absent or unreadable. The load keeps such an entry
    /// in the key-missing state; the add creates no endpoint without its
    /// key.
    KeyMissing,
    /// The key file's header derives another id than the name's stem.
    KeyMismatch,
    /// The id is an authored entry's, or an added one's with another
    /// alias or other kinds.
    Exists,
    /// Another entry, authored or added, holds the alias.
    AliasTaken,
    /// Authored and added entries would pass
    /// [`MAX_ENDPOINTS`](crate::MAX_ENDPOINTS).
    Bound,
}

impl AddRefusal {
    /// Every reason, in the order they are checked.
    pub const ALL: [Self; 7] = [
        Self::IdNotDerived,
        Self::AliasInvalid,
        Self::KeyMissing,
        Self::KeyMismatch,
        Self::Exists,
        Self::AliasTaken,
        Self::Bound,
    ];
}

/// What [`EndpointOverlay::add`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Added {
    /// The overlay with the entry added and its two sets present and
    /// empty.
    Added {
        /// The overlay to write.
        overlay: EndpointOverlay,
        /// The derived id.
        endpoint: EndpointId,
    },
    /// The `added` member already holds the id with the same alias and
    /// kinds: nothing to write (a retry after a lost answer).
    Unchanged(EndpointId),
    /// Refused, nothing to write.
    Refused {
        /// Why.
        reason: AddRefusal,
        /// The id the key's name derives: `None` only for
        /// [`AddRefusal::IdNotDerived`], which is refused because there is
        /// none (`the_add_refusals_are_checked_in_order`).
        endpoint: Option<EndpointId>,
        /// For a log.
        detail: String,
    },
}

/// Why an administrative remove is refused, in the order
/// [`EndpointOverlay::remove`] checks them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveRefusal {
    /// A `config.yaml` entry, which no program rewrites.
    Authored,
    /// The effective default: the client moves it first.
    Default,
}

/// What [`EndpointOverlay::remove`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Removed {
    /// The overlay without the entry and every member keyed by its id.
    Removed(EndpointOverlay),
    /// Refused, nothing to write.
    Refused(RemoveRefusal),
}

/// Why an alias rename is refused, in the order
/// [`EndpointOverlay::set_alias`] checks them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasRefusal {
    /// An authored entry, whose alias is the file's.
    Authored,
    /// Outside the alias grammar.
    AliasInvalid,
    /// Another entry, authored or added, holds it.
    AliasTaken,
}

/// What [`EndpointOverlay::set_alias`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Renamed {
    /// The overlay with the added entry's alias rewritten.
    Renamed(EndpointOverlay),
    /// The entry already has that alias: nothing to write.
    Unchanged,
    /// Refused, nothing to write.
    Refused {
        /// Why.
        reason: AliasRefusal,
        /// For a log.
        detail: String,
    },
}

impl EndpointOverlay {
    /// The overlay's path for `paths`' profile.
    #[must_use]
    pub fn path_for(paths: &ProfilePaths) -> PathBuf {
        paths.state_dir().join(ENDPOINT_OVERLAY_FILE)
    }

    /// Read the overlay at `path`, normalise it against `config`, rewrite
    /// it when normalising changed anything, and return it with what
    /// normalising changed. An absent file is the empty overlay and is
    /// not created. The state directory's walk stops at `boundary`.
    ///
    /// Normalising, in order: a label member (not a derived id) equal to
    /// a configured entry's alias refuses the load
    /// ([`EndpointOverlayError::LabelMember`]) and one equal to none
    /// leaves; an `enabled` entry naming an endpoint
    /// `config` lacks, or equal to its state, leaves; a `default` naming
    /// an endpoint `config` lacks, or equal to the configured default,
    /// leaves. Then THE EFFECTIVE DEFAULT IS ENABLED OR THERE IS NONE,
    /// and only the default moves to make it so: a `default` naming a
    /// disabled endpoint leaves (the configured default applies), and a
    /// configured default the overlay disables becomes `default: null`.
    ///
    /// # Errors
    /// Every [`EndpointOverlayError`] but `Unknown`, `Disabled` and
    /// `CarryConflict`, none
    /// of them recoverable: each is a reason not to start (ADR-0028 A
    /// 2026-10-11), a failed rewrite included -- the stale entry left on
    /// disk could undo the operator's next `config.yaml` edit.
    pub fn load_within(
        path: &Path,
        config: &EndpointsConfig,
        boundary: &TrustBoundary,
    ) -> Result<(Self, Vec<Normalised>), EndpointOverlayError> {
        let (overlay, changes) = Self::read_within(path, config, boundary)?;
        if !changes.is_empty() {
            overlay.write_within(path, boundary)?;
        }
        Ok((overlay, changes))
    }

    /// [`Self::load_within`] READ-ONLY: the same read under the same
    /// rules and the same normalisation, in memory, and the file NEVER
    /// written back, whatever normalising changed
    /// (`tests/endpoint_overlay.rs`,
    /// `read_within_normalises_in_memory_and_writes_nothing`). For a
    /// client that resolves an alias and writes nothing in the profile;
    /// what it reads is what the next start's `load_within` composes.
    ///
    /// # Errors
    /// As [`Self::load_within`], less [`EndpointOverlayError::Write`].
    pub fn read_within(
        path: &Path,
        config: &EndpointsConfig,
        boundary: &TrustBoundary,
    ) -> Result<(Self, Vec<Normalised>), EndpointOverlayError> {
        let Some(text) =
            private_read::read_private_within(path, boundary, MAX_ENDPOINT_OVERLAY_BYTES)?
        else {
            return Ok((Self::default(), Vec::new()));
        };
        let raw: Raw = private_read::object_only(&text).map_err(EndpointOverlayError::Parse)?;
        let (mut overlay, mut changes) = Self::resolve(raw, config)?;
        changes.extend(overlay.normalise(config));
        Ok((overlay, changes))
    }

    /// Judge each member's text by the derived grammar: an id stays, a
    /// label equal to a configured entry's alias refuses the load, one
    /// equal to none leaves (the first step of the normalisation).
    fn resolve(
        raw: Raw,
        config: &EndpointsConfig,
    ) -> Result<(Self, Vec<Normalised>), EndpointOverlayError> {
        let mut changes = Vec::new();
        let added = Self::resolve_added(raw.added, config)?;
        let (inbound, outbound) = Self::resolve_sets(raw.inbound, raw.outbound)?;
        // ADR-0067 decision 6's "an absent id is no own set" is the
        // authored endpoint's case: an added endpoint's trust is its set
        // alone, so the file must state it, `[]` for nobody. Its filter
        // (`ADDED_TRUST`) admits nobody either, so the refusal keeps the
        // file exact rather than closing a widening. Both members hold
        // the same ids by now, so one is asked.
        if let Some(endpoint) = added.keys().find(|id| !inbound.contains_key(*id)) {
            return Err(EndpointOverlayError::AddedEntryWithoutSet {
                endpoint: endpoint.clone(),
            });
        }
        let mut overlay = Self {
            inbound,
            outbound,
            added,
            ..Self::default()
        };
        for (text, on) in raw.advertise {
            let Member::Id(id) = classify(&text)? else {
                return Err(EndpointOverlayError::Parse(format!(
                    "advertise.{text} is not a derived endpoint id"
                )));
            };
            overlay.advertise.insert(id, on);
        }
        // An added entry's alias is an alias of the profile like an
        // authored one's, so a pre-Stage-22 label naming either is the
        // operator's choice to keep.
        let aliased = |label: &EndpointAlias| {
            config
                .entries
                .iter()
                .chain(overlay.added.values())
                .any(|e| e.alias.as_ref() == Some(label))
        };
        let mut label_member = |label: EndpointAlias, member| {
            if aliased(&label) {
                return Err(EndpointOverlayError::LabelMember {
                    label: label.to_string(),
                    member,
                });
            }
            changes.push(Normalised::LabelWithoutAliasDropped {
                label: label.to_string(),
                member,
            });
            Ok(())
        };
        for (text, on) in raw.enabled {
            match classify(&text)? {
                Member::Id(id) => {
                    overlay.enabled.insert(id, on);
                }
                Member::Label(label) => label_member(label, OverlayMember::Enabled)?,
            }
        }
        overlay.default = match raw.default {
            None => None,
            Some(None) => Some(None),
            Some(Some(text)) => match classify(&text)? {
                Member::Id(id) => Some(Some(id)),
                Member::Label(label) => {
                    label_member(label, OverlayMember::Default)?;
                    None
                }
            },
        };
        Ok((overlay, changes))
    }

    /// The `added` member judged: each entry as `config.yaml` would read
    /// it (the one entry reader, so the id derives from the key name as
    /// it does for an authored entry), refused when it carries a trust
    /// member, repeats an id or alias, or takes the profile past
    /// [`MAX_ENDPOINTS`].
    fn resolve_added(
        raw: Vec<serde_json::Value>,
        config: &EndpointsConfig,
    ) -> Result<BTreeMap<EndpointId, EndpointConfig>, EndpointOverlayError> {
        // Refused, never truncated, and before any entry is parsed: a
        // file naming thousands of entries costs one length check.
        let total = config.entries.len().saturating_add(raw.len());
        if total > MAX_ENDPOINTS {
            return Err(EndpointOverlayError::TooManyEndpoints { got: total });
        }
        let mut added: BTreeMap<EndpointId, EndpointConfig> = BTreeMap::new();
        for value in raw {
            if let Some(object) = value.as_object() {
                for (name, member) in TRUST_MEMBERS {
                    if object.contains_key(name) {
                        return Err(EndpointOverlayError::AddedEntryCarriesTrust { member });
                    }
                }
            }
            let mut entry: EndpointConfig = serde_json::from_value(value)
                .map_err(|e| EndpointOverlayError::Parse(format!("an added endpoint: {e}")))?;
            // The entry reader reads an absent member as inheritance, which
            // an added endpoint never has.
            entry.inbound = ADDED_TRUST;
            entry.outbound = ADDED_TRUST;
            if config.entries.iter().any(|e| e.id == entry.id) || added.contains_key(&entry.id) {
                return Err(EndpointOverlayError::AddedIdCollides(entry.id));
            }
            if let Some(alias) = &entry.alias
                && config
                    .entries
                    .iter()
                    .chain(added.values())
                    .any(|e| e.alias.as_ref() == Some(alias))
            {
                return Err(EndpointOverlayError::AddedAliasCollides(alias.clone()));
            }
            added.insert(entry.id.clone(), entry);
        }
        Ok(added)
    }

    /// The `inbound` and `outbound` members judged: derived ids, canonical
    /// peers, each peer once, each list and all rows within their bounds,
    /// and the same ids in both.
    fn resolve_sets(
        inbound: BTreeMap<String, Vec<String>>,
        outbound: BTreeMap<String, Vec<String>>,
    ) -> Result<(EndpointSets, EndpointSets), EndpointOverlayError> {
        // Counted from the raw lengths before a peer is parsed.
        let rows: usize = inbound
            .values()
            .chain(outbound.values())
            .map(Vec::len)
            .fold(0, usize::saturating_add);
        if rows > MAX_ENDPOINT_SET_ROWS {
            return Err(EndpointOverlayError::SetRowsExceeded { got: rows });
        }
        let judge = |raw: BTreeMap<String, Vec<String>>, member: OverlayMember| {
            let mut out = EndpointSets::new();
            for (text, peers) in raw {
                let endpoint = EndpointId::parse_derived(&text).map_err(|_| {
                    EndpointOverlayError::Parse(format!(
                        "{member}.{text} is not a derived endpoint id"
                    ))
                })?;
                if peers.len() > PeerTrustPolicy::MAX_ALLOWED_PEERS {
                    return Err(EndpointOverlayError::SetTooLarge {
                        member,
                        endpoint,
                        got: peers.len(),
                    });
                }
                let mut seen = BTreeSet::new();
                for text in peers {
                    let peer = TransportIdentity::parse(text).map_err(|e| {
                        EndpointOverlayError::Parse(format!("{member}.{}: {e}", endpoint.as_str()))
                    })?;
                    if let Some(again) = seen.replace(peer) {
                        return Err(EndpointOverlayError::DuplicatePeer {
                            member,
                            endpoint,
                            peer: again,
                        });
                    }
                }
                let set = bounded_set(member, &endpoint, seen)?;
                out.insert(endpoint, set);
            }
            Ok(out)
        };
        let inbound = judge(inbound, OverlayMember::Inbound)?;
        let outbound = judge(outbound, OverlayMember::Outbound)?;
        for (this, other, only_in) in [
            (&inbound, &outbound, OverlayMember::Inbound),
            (&outbound, &inbound, OverlayMember::Outbound),
        ] {
            if let Some(endpoint) = this.keys().find(|id| !other.contains_key(*id)) {
                return Err(EndpointOverlayError::SetInOneMember {
                    endpoint: endpoint.clone(),
                    only_in,
                });
            }
        }
        Ok((inbound, outbound))
    }

    /// Move a pre-Stage-22 overlay's members for `label` to `id`, in the
    /// file at `path`, and say whether there was anything to move
    /// (`transportctl endpoint adopt` and `init`, and
    /// `provision_identity` for the Android first identity).
    ///
    /// The file is read RAW -- by the same private read, size bound and
    /// object-only check as [`Self::read_within`], members as text, no
    /// configuration, no normalisation -- so what stands in it besides
    /// the label is written back as it was. `enabled.<label>` becomes
    /// `enabled.<id>`, and a `default` of `<label>` becomes `<id>`; an
    /// `enabled` naming both with the SAME state drops the label
    /// (a second call finds nothing), with DIFFERENT
    /// states it is refused ([`EndpointOverlayError::CarryConflict`]).
    /// Written whole, as [`Self::write_within`] writes. `config.yaml` is
    /// never read or written. An absent file, or one with no member for
    /// `label`, is [`Carried::Nothing`] and is not written.
    ///
    /// # Errors
    /// The read's, [`EndpointOverlayError::CarryConflict`], and
    /// [`EndpointOverlayError::Write`] (nothing changed unless
    /// [`EndpointOverlayError::installed`]).
    pub fn carry_label_within(
        path: &Path,
        boundary: &TrustBoundary,
        label: &EndpointAlias,
        id: &EndpointId,
    ) -> Result<Carried, EndpointOverlayError> {
        let Some(text) =
            private_read::read_private_within(path, boundary, MAX_ENDPOINT_OVERLAY_BYTES)?
        else {
            return Ok(Carried::Nothing);
        };
        let mut raw: Raw = private_read::object_only(&text).map_err(EndpointOverlayError::Parse)?;
        let mut moved = false;
        if let Some(on) = raw.enabled.remove(label.as_str()) {
            match raw.enabled.get(id.as_str()) {
                Some(&there) if there != on => {
                    return Err(EndpointOverlayError::CarryConflict {
                        label: label.to_string(),
                        id: id.clone(),
                    });
                }
                Some(_) => {}
                None => {
                    raw.enabled.insert(id.as_str().to_owned(), on);
                }
            }
            moved = true;
        }
        if raw.default == Some(Some(label.to_string())) {
            raw.default = Some(Some(id.as_str().to_owned()));
            moved = true;
        }
        if !moved {
            return Ok(Carried::Nothing);
        }
        let out = serde_json::to_vec_pretty(&raw)
            .map_err(|e| EndpointOverlayError::Write(PersistError::Io(std::io::Error::other(e))))?;
        persist::write_private_atomic_within(path, &out, boundary)
            .map_err(EndpointOverlayError::Write)?;
        Ok(Carried::Moved)
    }

    /// Drop what the configuration already says, then move the default
    /// until it is enabled or none. What changed, in order.
    fn normalise(&mut self, config: &EndpointsConfig) -> Vec<Normalised> {
        let configured = self.configured_enabled(config);
        let configured_default = config.default_direct_endpoint.clone();
        let mut changes = Vec::new();
        self.enabled.retain(|id, enabled| match configured.get(id) {
            None => {
                changes.push(Normalised::UnknownEndpointDropped(id.clone()));
                false
            }
            Some(c) if c == enabled => {
                changes.push(Normalised::EnabledEqualsConfig(id.clone()));
                false
            }
            Some(_) => true,
        });
        // An own set follows its endpoint: dropped from BOTH members
        // together, so the two never stand on different ids.
        let unknown_sets: Vec<EndpointId> = self
            .inbound
            .keys()
            .filter(|id| !configured.contains_key(*id))
            .cloned()
            .collect();
        for id in unknown_sets {
            self.inbound.remove(&id);
            self.outbound.remove(&id);
            changes.push(Normalised::UnknownSetDropped(id));
        }
        // `advertise` is a delta over AUTHORED entries only: an added
        // entry carries its own.
        let authored: BTreeMap<&EndpointId, bool> = config
            .entries
            .iter()
            .map(|e| (&e.id, e.advertise))
            .collect();
        self.advertise
            .retain(|id, advertise| match authored.get(id) {
                None => {
                    changes.push(Normalised::UnknownAdvertiseDropped(id.clone()));
                    false
                }
                Some(c) if c == advertise => {
                    changes.push(Normalised::AdvertiseEqualsConfig(id.clone()));
                    false
                }
                Some(_) => true,
            });
        match &self.default {
            Some(Some(id)) if !configured.contains_key(id) => {
                changes.push(Normalised::UnknownDefaultDropped(id.clone()));
                self.default = None;
            }
            Some(d) if *d == configured_default => {
                changes.push(Normalised::DefaultEqualsConfig(d.clone()));
                self.default = None;
            }
            _ => {}
        }
        // The predicate. An overlay default naming a disabled endpoint
        // leaves first, so the configured default is then judged the
        // same way: at most two passes.
        if let Some(Some(id)) = &self.default
            && !self.is_enabled(&configured, id)
        {
            changes.push(Normalised::DisabledDefaultDropped(id.clone()));
            self.default = None;
        }
        if self.default.is_none()
            && let Some(id) = &configured_default
            && !self.is_enabled(&configured, id)
        {
            changes.push(Normalised::ConfiguredDefaultCleared(id.clone()));
            self.default = Some(None);
        }
        changes
    }

    /// `config.yaml`'s endpoints as the overlay reads them, the added
    /// entries among them with the state they were added with.
    fn configured_enabled(&self, config: &EndpointsConfig) -> BTreeMap<EndpointId, bool> {
        config
            .entries
            .iter()
            .chain(self.added.values())
            .map(|e| (e.id.clone(), e.enabled))
            .collect()
    }

    /// Whether `id` is enabled once this overlay is composed over
    /// `configured`; an endpoint not configured is not.
    fn is_enabled(&self, configured: &BTreeMap<EndpointId, bool>, id: &EndpointId) -> bool {
        self.enabled
            .get(id)
            .or_else(|| configured.get(id))
            .copied()
            .unwrap_or(false)
    }

    /// The endpoints' state with this overlay composed over `config`.
    #[must_use]
    pub fn effective(&self, config: &EndpointsConfig) -> EffectiveEndpoints {
        let mut enabled = self.configured_enabled(config);
        for (id, on) in &self.enabled {
            if let Some(slot) = enabled.get_mut(id) {
                *slot = *on;
            }
        }
        let default = match &self.default {
            Some(d) => d.clone(),
            None => config.default_direct_endpoint.clone(),
        };
        EffectiveEndpoints { enabled, default }
    }

    /// `config` with this overlay composed over it: what the runtime is
    /// constructed from. The authored entries come first, with the
    /// `enabled` and `advertise` deltas applied, then the added entries
    /// (by id) with their `enabled` as the overlay now has it and their
    /// own `advertise`. `config.yaml` itself is never written.
    #[must_use]
    pub fn apply(&self, config: &EndpointsConfig) -> EndpointsConfig {
        let effective = self.effective(config);
        let mut out = config.clone();
        for entry in &mut out.entries {
            if let Some(advertise) = self.advertise.get(&entry.id) {
                entry.advertise = *advertise;
            }
        }
        out.entries.extend(self.added.values().cloned());
        for entry in &mut out.entries {
            if let Some(on) = effective.enabled.get(&entry.id) {
                entry.enabled = *on;
            }
        }
        out.default_direct_endpoint = effective.default;
        out
    }

    /// This overlay recording `effective`, a state reachable from
    /// `config`: each of the two deltas present exactly when it differs,
    /// every other member as it was.
    fn recording(&self, config: &EndpointsConfig, effective: &EffectiveEndpoints) -> Self {
        let configured = self.configured_enabled(config);
        Self {
            enabled: effective
                .enabled
                .iter()
                .filter(|(id, on)| configured.get(*id) != Some(*on))
                .map(|(id, on)| (id.clone(), *on))
                .collect(),
            default: (effective.default != config.default_direct_endpoint)
                .then(|| effective.default.clone()),
            ..self.clone()
        }
    }

    /// The overlay after `admin.endpoints.set_enabled(endpoint, enabled)`,
    /// or `None` when the set changes nothing. Disabling the effective
    /// default clears it (the owner, 2026-09-28), so that one move
    /// carries both deltas; enabling it again restores nothing.
    ///
    /// # Errors
    /// [`EndpointOverlayError::Unknown`] for an endpoint `config` does
    /// not configure.
    pub fn set_enabled(
        &self,
        config: &EndpointsConfig,
        endpoint: &EndpointId,
        enabled: bool,
    ) -> Result<Option<Self>, EndpointOverlayError> {
        let mut effective = self.effective(config);
        let Some(slot) = effective.enabled.get_mut(endpoint) else {
            return Err(EndpointOverlayError::Unknown);
        };
        *slot = enabled;
        if !enabled && effective.default.as_ref() == Some(endpoint) {
            effective.default = None;
        }
        let next = self.recording(config, &effective);
        Ok((next != *self).then_some(next))
    }

    /// The overlay after `admin.endpoints.set_default(endpoint)`, or
    /// `None` when the set changes nothing. A default must be able to
    /// receive, as `config.yaml`'s must.
    ///
    /// # Errors
    /// [`EndpointOverlayError::Unknown`] or
    /// [`EndpointOverlayError::Disabled`] for an endpoint that could not
    /// receive.
    pub fn set_default(
        &self,
        config: &EndpointsConfig,
        endpoint: Option<&EndpointId>,
    ) -> Result<Option<Self>, EndpointOverlayError> {
        let mut effective = self.effective(config);
        if let Some(id) = endpoint {
            match effective.enabled.get(id) {
                None => return Err(EndpointOverlayError::Unknown),
                Some(false) => return Err(EndpointOverlayError::Disabled),
                Some(true) => {}
            }
        }
        effective.default = endpoint.cloned();
        let next = self.recording(config, &effective);
        Ok((next != *self).then_some(next))
    }

    /// The endpoint's own sets, inbound then outbound: `None` when the
    /// overlay carries none for it -- the endpoint then trusts what its
    /// configuration authored -- and a pair, empty ones included, when it
    /// does.
    #[must_use]
    pub fn own_sets(&self, endpoint: &EndpointId) -> Option<(&EndpointSet, &EndpointSet)> {
        Some((self.inbound.get(endpoint)?, self.outbound.get(endpoint)?))
    }

    /// Every own set of every endpoint, both directions: what
    /// [`PeerTrustPolicy::admission`] unites with the operator's list.
    pub fn every_set(&self) -> impl Iterator<Item = &EndpointSet> {
        self.inbound.values().chain(self.outbound.values())
    }

    /// The rows all own sets hold together, both directions counted:
    /// what [`MAX_ENDPOINT_SET_ROWS`] bounds.
    #[must_use]
    pub fn set_rows(&self) -> usize {
        self.every_set().map(EndpointSet::len).sum()
    }

    /// The entries added through the administrative path, by id, as
    /// loaded: their key state is [`EndpointKeyState::NotRead`] until
    /// [`Self::read_added_keys`].
    pub fn added(&self) -> impl Iterator<Item = &EndpointConfig> {
        self.added.values()
    }

    /// Read each added entry's key header as the loader reads an authored
    /// entry's ([`ProfileConfig::read_endpoint_keys`](crate::ProfileConfig::read_endpoint_keys)),
    /// which [`Self::apply`] then carries into the composed entries.
    /// Returns the refusals, an entry whose header does not derive its id
    /// being left `NotRead`.
    #[must_use]
    pub fn read_added_keys(&mut self, paths: &ProfilePaths) -> Vec<crate::ConfigError> {
        self.added
            .values_mut()
            .filter_map(|entry| crate::load::read_entry_key(paths, entry))
            .collect()
    }

    /// Refuse an overlay whose own sets hold more than
    /// [`MAX_ENDPOINT_SET_ROWS`] rows.
    fn within_rows(self) -> Result<Self, EndpointOverlayError> {
        match self.set_rows() {
            got if got > MAX_ENDPOINT_SET_ROWS => {
                Err(EndpointOverlayError::SetRowsExceeded { got })
            }
            _ => Ok(self),
        }
    }

    /// This overlay with `endpoint`'s own sets created from the seeds,
    /// when it has none; the flag says whether they were created.
    fn with_sets(
        &self,
        config: &EndpointsConfig,
        endpoint: &EndpointId,
        seed_inbound: impl IntoIterator<Item = TransportIdentity>,
        seed_outbound: impl IntoIterator<Item = TransportIdentity>,
    ) -> Result<(Self, bool), EndpointOverlayError> {
        if !self.configured_enabled(config).contains_key(endpoint) {
            return Err(EndpointOverlayError::Unknown);
        }
        if self.inbound.contains_key(endpoint) {
            return Ok((self.clone(), false));
        }
        let mut next = self.clone();
        next.inbound.insert(
            endpoint.clone(),
            bounded_set(OverlayMember::Inbound, endpoint, seed_inbound)?,
        );
        next.outbound.insert(
            endpoint.clone(),
            bounded_set(OverlayMember::Outbound, endpoint, seed_outbound)?,
        );
        Ok((next.within_rows()?, true))
    }

    /// The overlay after giving `endpoint` its own sets from the seeds
    /// the caller computed, one per direction; [`Seeded::AlreadyPresent`]
    /// -- nothing changed -- when the overlay already carries a set for
    /// it, an empty one included (own and empty is a set, and a seed
    /// never refills it).
    ///
    /// # Errors
    /// [`EndpointOverlayError::Unknown`] for an endpoint `config` and the
    /// overlay do not configure; [`EndpointOverlayError::SetTooLarge`]
    /// for a seed past the per-list bound;
    /// [`EndpointOverlayError::SetRowsExceeded`] for a result past
    /// [`MAX_ENDPOINT_SET_ROWS`], nothing changed.
    pub fn seed_sets(
        &self,
        config: &EndpointsConfig,
        endpoint: &EndpointId,
        seed_inbound: impl IntoIterator<Item = TransportIdentity>,
        seed_outbound: impl IntoIterator<Item = TransportIdentity>,
    ) -> Result<Seeded, EndpointOverlayError> {
        match self.with_sets(config, endpoint, seed_inbound, seed_outbound)? {
            (next, true) => Ok(Seeded::Created(next)),
            (_, false) => Ok(Seeded::AlreadyPresent),
        }
    }

    /// The overlay after setting (`trusted`) or removing a peer in BOTH
    /// of `endpoint`'s sets, or `None` when that changes nothing. An
    /// endpoint with no own set is given one first, from the seeds the
    /// caller computed ([`Self::seed_sets`]), and that creation is itself
    /// a change: the result is `Some` even when the peer was already in,
    /// or not in, the seeds.
    ///
    /// # Errors
    /// As [`Self::seed_sets`], and [`EndpointOverlayError::SetTooLarge`]
    /// for a peer that would take a list past its bound.
    pub fn set_peer(
        &self,
        config: &EndpointsConfig,
        endpoint: &EndpointId,
        seed_inbound: impl IntoIterator<Item = TransportIdentity>,
        seed_outbound: impl IntoIterator<Item = TransportIdentity>,
        peer: &TransportIdentity,
        trusted: bool,
    ) -> Result<Option<Self>, EndpointOverlayError> {
        let (mut next, created) = self.with_sets(config, endpoint, seed_inbound, seed_outbound)?;
        for (member, sets) in [
            (OverlayMember::Inbound, &mut next.inbound),
            (OverlayMember::Outbound, &mut next.outbound),
        ] {
            let Some(set) = sets.get_mut(endpoint) else {
                continue;
            };
            let mut peers = set.peers().clone();
            let changed = if trusted {
                peers.insert(peer.clone())
            } else {
                peers.remove(peer)
            };
            if changed {
                *set = bounded_set(member, endpoint, peers)?;
            }
        }
        let next = next.within_rows()?;
        Ok((created || next != *self).then_some(next))
    }

    /// Whether `alias` is held by an entry, authored or added, other than
    /// `except`'s.
    fn alias_held(
        &self,
        config: &EndpointsConfig,
        alias: &EndpointAlias,
        except: Option<&EndpointId>,
    ) -> Option<EndpointId> {
        config
            .entries
            .iter()
            .chain(self.added.values())
            .find(|e| Some(&e.id) != except && e.alias.as_ref() == Some(alias))
            .map(|e| e.id.clone())
    }

    /// The overlay after `admin.endpoints.add {key, alias,
    /// allowed_client_kinds}` (`LOCAL-IPC.md` §The endpoint set's
    /// methods). The refusals are checked in [`AddRefusal::ALL`]'s order,
    /// each with nothing to write; the key file's header is read under
    /// `paths` as the load reads an authored entry's
    /// (`crate::load::read_entry_key`). `None` -- a runtime that keeps no
    /// state directory, a test construction -- reads no key file, so every
    /// add that passes the checks before it is `key_missing`: the order
    /// stays this function's alone
    /// (`without_a_state_directory_the_order_is_the_same`). `unchanged` is decided at the
    /// `exists` step, after the key checks, so a retry whose key file has
    /// since gone is `key_missing`. On [`Added::Added`] the entry is in
    /// `added` -- enabled, `advertise: false`, no trust member written,
    /// its filters [`ADDED_TRUST`] -- and its id in `inbound` and
    /// `outbound` with an empty set each;
    /// the default is left where it is. Kinds are compared as a set.
    #[must_use]
    pub fn add(
        &self,
        config: &EndpointsConfig,
        paths: Option<&ProfilePaths>,
        key: &str,
        alias: &str,
        allowed_client_kinds: Vec<ClientKind>,
    ) -> Added {
        let id = match crate::endpoint_key::parse_key_file_name(key) {
            Ok(id) => id,
            Err(e) => {
                return Added::Refused {
                    reason: AddRefusal::IdNotDerived,
                    endpoint: None,
                    detail: format!("key {key:?}: {e}"),
                };
            }
        };
        let refused = |reason, detail: String| Added::Refused {
            reason,
            endpoint: Some(id.clone()),
            detail,
        };
        let alias = match EndpointAlias::parse(alias) {
            Ok(alias) => alias,
            Err(e) => return refused(AddRefusal::AliasInvalid, e.to_string()),
        };
        let mut entry = EndpointConfig {
            id: id.clone(),
            alias: Some(alias.clone()),
            key_state: crate::EndpointKeyState::NotRead,
            enabled: true,
            advertise: false,
            allowed_client_kinds,
            inbound: ADDED_TRUST,
            outbound: ADDED_TRUST,
        };
        let Some(paths) = paths else {
            return refused(
                AddRefusal::KeyMissing,
                "this runtime keeps no state directory: no key file is read".to_owned(),
            );
        };
        let mismatch = crate::load::read_entry_key(paths, &mut entry);
        if let crate::EndpointKeyState::Missing(why) = &entry.key_state {
            let detail = match why {
                crate::KeyMissing::Absent => "the key file is absent".to_owned(),
                crate::KeyMissing::Unreadable(e) => e.to_string(),
            };
            return refused(AddRefusal::KeyMissing, detail);
        }
        if let Some(e) = mismatch {
            return refused(AddRefusal::KeyMismatch, e.to_string());
        }
        if config.entries.iter().any(|e| e.id == id) {
            return refused(
                AddRefusal::Exists,
                "an authored entry holds the id".to_owned(),
            );
        }
        if let Some(held) = self.added.get(&id) {
            let kinds = |k: &[ClientKind]| k.iter().cloned().collect::<BTreeSet<_>>();
            if held.alias == entry.alias
                && kinds(&held.allowed_client_kinds) == kinds(&entry.allowed_client_kinds)
            {
                return Added::Unchanged(id);
            }
            return refused(
                AddRefusal::Exists,
                "an added entry holds the id with another alias or other kinds".to_owned(),
            );
        }
        if let Some(holder) = self.alias_held(config, &alias, None) {
            return refused(
                AddRefusal::AliasTaken,
                format!("endpoint {} holds alias {alias}", holder.as_str()),
            );
        }
        let total = config.entries.len() + self.added.len() + 1;
        if total > MAX_ENDPOINTS {
            return refused(
                AddRefusal::Bound,
                format!("{total} endpoints; the maximum is {MAX_ENDPOINTS}"),
            );
        }
        let mut next = self.clone();
        next.inbound.insert(id.clone(), EndpointSet::default());
        next.outbound.insert(id.clone(), EndpointSet::default());
        next.added.insert(id.clone(), entry);
        Added::Added {
            overlay: next,
            endpoint: id,
        }
    }

    /// The overlay after `admin.endpoints.remove {endpoint}`: an ADDED
    /// entry and, in the same value, every member keyed by its id --
    /// `enabled`, `advertise`, `inbound`, `outbound` -- gone. Refused
    /// `authored` and then `default` (the effective default), in that
    /// order. A `default` member naming the id cannot remain, since that
    /// would be the effective default and is refused.
    ///
    /// # Errors
    /// [`EndpointOverlayError::Unknown`] for an id no entry holds.
    pub fn remove(
        &self,
        config: &EndpointsConfig,
        endpoint: &EndpointId,
    ) -> Result<Removed, EndpointOverlayError> {
        if config.entries.iter().any(|e| &e.id == endpoint) {
            return Ok(Removed::Refused(RemoveRefusal::Authored));
        }
        if !self.added.contains_key(endpoint) {
            return Err(EndpointOverlayError::Unknown);
        }
        if self.effective(config).default.as_ref() == Some(endpoint) {
            return Ok(Removed::Refused(RemoveRefusal::Default));
        }
        let mut next = self.clone();
        next.added.remove(endpoint);
        next.enabled.remove(endpoint);
        next.advertise.remove(endpoint);
        next.inbound.remove(endpoint);
        next.outbound.remove(endpoint);
        Ok(Removed::Removed(next))
    }

    /// The overlay after `admin.endpoints.set_advertise {endpoint,
    /// advertise}`, or `None` when the value is already in effect. An
    /// authored endpoint's value is the `advertise` delta, dropped when it
    /// equals the authored value; an added one's is its entry's member.
    ///
    /// # Errors
    /// [`EndpointOverlayError::Unknown`] for an id no entry holds.
    pub fn set_advertise(
        &self,
        config: &EndpointsConfig,
        endpoint: &EndpointId,
        advertise: bool,
    ) -> Result<Option<Self>, EndpointOverlayError> {
        let mut next = self.clone();
        if let Some(authored) = config.entries.iter().find(|e| &e.id == endpoint) {
            if advertise == authored.advertise {
                next.advertise.remove(endpoint);
            } else {
                next.advertise.insert(endpoint.clone(), advertise);
            }
        } else if let Some(added) = next.added.get_mut(endpoint) {
            added.advertise = advertise;
        } else {
            return Err(EndpointOverlayError::Unknown);
        }
        Ok((next != *self).then_some(next))
    }

    /// The overlay after `admin.endpoints.set_alias {endpoint, alias}`: an
    /// ADDED entry's alias rewritten. Refused `authored`, `alias_invalid`
    /// and `alias_taken`, in that order; `unchanged` for the alias the
    /// entry already has.
    ///
    /// # Errors
    /// [`EndpointOverlayError::Unknown`] for an id no entry holds.
    pub fn set_alias(
        &self,
        config: &EndpointsConfig,
        endpoint: &EndpointId,
        alias: &str,
    ) -> Result<Renamed, EndpointOverlayError> {
        let refused = |reason, detail: String| Ok(Renamed::Refused { reason, detail });
        if config.entries.iter().any(|e| &e.id == endpoint) {
            return refused(
                AliasRefusal::Authored,
                "an authored entry's alias is config.yaml's".to_owned(),
            );
        }
        let Some(entry) = self.added.get(endpoint) else {
            return Err(EndpointOverlayError::Unknown);
        };
        let alias = match EndpointAlias::parse(alias) {
            Ok(alias) => alias,
            Err(e) => return refused(AliasRefusal::AliasInvalid, e.to_string()),
        };
        if entry.alias.as_ref() == Some(&alias) {
            return Ok(Renamed::Unchanged);
        }
        if let Some(holder) = self.alias_held(config, &alias, Some(endpoint)) {
            return refused(
                AliasRefusal::AliasTaken,
                format!("endpoint {} holds alias {alias}", holder.as_str()),
            );
        }
        let mut next = self.clone();
        if let Some(entry) = next.added.get_mut(endpoint) {
            entry.alias = Some(alias);
        }
        Ok(Renamed::Renamed(next))
    }

    /// Whether `endpoint` is an entry the overlay added.
    #[must_use]
    pub fn is_added(&self, endpoint: &EndpointId) -> bool {
        self.added.contains_key(endpoint)
    }

    /// Write the overlay whole: a temporary file in the same directory,
    /// owner-only from creation, renamed over `path`; the state
    /// directory's walk stops at `boundary`.
    ///
    /// # Errors
    /// [`EndpointOverlayError::Write`]. A failure before the rename
    /// leaves the previous file as it was; one after it --
    /// [`EndpointOverlayError::installed`] -- leaves THIS overlay in
    /// place, its name perhaps not durable.
    pub fn write_within(
        &self,
        path: &Path,
        boundary: &TrustBoundary,
    ) -> Result<(), EndpointOverlayError> {
        let text = serde_json::to_vec_pretty(self)
            .map_err(|e| EndpointOverlayError::Write(PersistError::Io(std::io::Error::other(e))))?;
        persist::write_private_atomic_within(path, &text, boundary)
            .map_err(EndpointOverlayError::Write)
    }
}
