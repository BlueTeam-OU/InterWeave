// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The endpoint overlay: what `admin.endpoints.set_enabled` and
//! `admin.endpoints.set_default` changed, kept in the state directory
//! beside the trust overlay (ADR-0028 A 2026-10-11).
//!
//! `<state>/endpoint-overlay.json` holds exactly
//! `{"enabled": {<endpoint-id>: <bool>, ...}, "default": <endpoint-id> | null}`,
//! each `<endpoint-id>` a DERIVED id (`EndpointId::parse_derived`),
//! both deltas against `config.yaml`'s `endpoints`: `enabled` is always
//! present and holds only the endpoints whose state differs from the
//! configuration; `default` is present only when the default differs
//! from the configured one -- an endpoint id, or `null` for "no default"
//! where `config.yaml` names one. Absent, the configured default applies.
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

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use interweave_transport_api::EndpointId;
use serde::{Deserialize, Deserializer, Serialize};

use crate::private_read::{self, PrivateReadError};
use crate::{EndpointAlias, EndpointsConfig, PersistError, ProfilePaths, TrustBoundary, persist};

/// The overlay's file name in the profile's state directory.
pub const ENDPOINT_OVERLAY_FILE: &str = "endpoint-overlay.json";

/// The most an endpoint overlay may hold on disk. A normalised overlay
/// names only configured endpoints -- at most
/// [`MAX_ENDPOINTS`](crate::MAX_ENDPOINTS), each id at most
/// [`EndpointId::MAX_BYTES`] -- and its largest pretty-printed form is
/// under 7 KiB (`the_largest_overlay_fits_its_bound`); anything past
/// this is refused before it is parsed.
pub const MAX_ENDPOINT_OVERLAY_BYTES: u64 = 16 * 1024;

/// The overlay's two deltas, normalised or not;
/// [`EndpointOverlay::load_within`] hands out only a normalised one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct EndpointOverlay {
    /// The endpoints whose enabled state differs from `config.yaml`'s.
    enabled: BTreeMap<EndpointId, bool>,
    /// `None`: absent, the configured default applies. `Some(None)`:
    /// `null`, no default. `Some(Some(id))`: that endpoint.
    #[expect(
        clippy::option_option,
        reason = "the member's three states are serde's: absent, null, an id"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<Option<EndpointId>>,
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
}

/// Which member of the overlay a label was found in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayMember {
    /// A key of `enabled`.
    Enabled,
    /// The `default` value.
    Default,
}

impl core::fmt::Display for OverlayMember {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Enabled => "enabled",
            Self::Default => "default",
        })
    }
}

/// What one member's text is, by the DERIVED grammar and never the
/// transitional label path of `EndpointId::parse`.
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

/// `enabled`'s entries, an endpoint named twice refused: a map keeps the
/// last of a key named twice, which would read the operator's first
/// state of it as never written (ADR-0028 A 2026-10-11).
fn unique_entries<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<String, bool>, D::Error> {
    struct Entries;
    impl<'de> serde::de::Visitor<'de> for Entries {
        type Value = BTreeMap<String, bool>;
        fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("an object of endpoint ids to booleans")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut out = BTreeMap::new();
            while let Some((id, on)) = map.next_entry::<String, bool>()? {
                if out.contains_key(&id) {
                    return Err(serde::de::Error::custom(format!(
                        "endpoint {id} named twice"
                    )));
                }
                out.insert(id, on);
            }
            Ok(out)
        }
    }
    d.deserialize_map(Entries)
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

/// `config.yaml`'s endpoints as the overlay reads them.
fn configured_enabled(config: &EndpointsConfig) -> BTreeMap<EndpointId, bool> {
    config
        .entries
        .iter()
        .map(|e| (e.id.clone(), e.enabled))
        .collect()
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
        let aliased = |label: &EndpointAlias| {
            config
                .entries
                .iter()
                .any(|e| e.alias.as_ref() == Some(label))
        };
        let mut changes = Vec::new();
        let mut overlay = Self::default();
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
        let configured = configured_enabled(config);
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
        let mut enabled = configured_enabled(config);
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
    /// constructed from. `config.yaml` itself is never written.
    #[must_use]
    pub fn apply(&self, config: &EndpointsConfig) -> EndpointsConfig {
        let effective = self.effective(config);
        let mut out = config.clone();
        for entry in &mut out.entries {
            if let Some(on) = effective.enabled.get(&entry.id) {
                entry.enabled = *on;
            }
        }
        out.default_direct_endpoint = effective.default;
        out
    }

    /// The overlay recording `effective`, a state reachable from
    /// `config`: each delta present exactly when it differs.
    fn recording(config: &EndpointsConfig, effective: &EffectiveEndpoints) -> Self {
        let configured = configured_enabled(config);
        Self {
            enabled: effective
                .enabled
                .iter()
                .filter(|(id, on)| configured.get(*id) != Some(*on))
                .map(|(id, on)| (id.clone(), *on))
                .collect(),
            default: (effective.default != config.default_direct_endpoint)
                .then(|| effective.default.clone()),
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
        let next = Self::recording(config, &effective);
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
        let next = Self::recording(config, &effective);
        Ok((next != *self).then_some(next))
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
