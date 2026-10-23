// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The bootstrap-peers overlay: the addresses a person added through
//! `admin.peers.add_address` (or the embedded host's function), kept in
//! the state directory beside the trust and endpoint overlays (ADR-0062,
//! ADR-0028 A 2026-10-21 (ii)).
//!
//! `<state>/bootstrap-overlay.json` holds exactly `{"peers": [...]}`, each
//! a `multiaddr-with-peer-id` in the configuration's grammar
//! ([`crate::split_peer_multiaddr`]), direct or a circuit route. The
//! static-bootstrap provider emits them after `config.yaml`'s authored
//! entries, with the same provenance: a reachability hint, no trust.
//!
//! Like its siblings it is ADMINISTERED STATE, not a cache: a missing
//! file is the empty overlay, but a present one that cannot be trusted
//! -- unreadable, unparseable, someone else's, readable or writable by
//! anyone else, an entry outside the grammar, a composed count past the
//! provider's bound -- stops the start, named, never skipped or
//! truncated. `config.yaml` is never written; the overlay is NORMALISED
//! at load against the authored entries, so a duplicate of one leaves.
//!
//! WHAT IS NOT HERE: the special-use floor (ADR-0052 rule 2) and the
//! refusal under an authored `enabled: false` provider. Both are the
//! add method's, which holds the profile and the class predicate; this
//! module is the file and its two moves.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::private_read::{self, PrivateReadError};
use interweave_discovery_api::MAX_ADDRESS_BYTES;

use crate::{
    MAX_STATIC_BOOTSTRAP_PEERS, MAX_STATIC_PEER_BYTES, PersistError, ProfilePaths, TrustBoundary,
    persist, split_peer_multiaddr,
};

/// The overlay's file name in the profile's state directory.
pub const BOOTSTRAP_OVERLAY_FILE: &str = "bootstrap-overlay.json";

/// The most an overlay may hold on disk: the provider's bound of entries
/// at their byte ceiling, pretty-printed, with room to spare. Anything
/// larger is refused before it is parsed.
pub const MAX_BOOTSTRAP_OVERLAY_BYTES: u64 = 64 * 1024;

const _: () = assert!(
    (MAX_STATIC_BOOTSTRAP_PEERS * (MAX_STATIC_PEER_BYTES + 16)) as u64
        <= MAX_BOOTSTRAP_OVERLAY_BYTES
);

/// The overlay: the added entries, in the order they were added.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapOverlay {
    peers: Vec<String>,
}

/// Why an add or a removal is refused, by name: the closed set the
/// `admin.peers` answer carries (`peers-address-result`'s `refused`),
/// less the two the method itself decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveRefusal {
    /// Not a `multiaddr-with-peer-id` in the configuration's grammar.
    Grammar(&'static str),
    /// Authored and added entries would pass the provider's bound.
    Bound,
    /// The address is `config.yaml`'s: the overlay cannot undo it.
    Authored,
}

/// Why a bootstrap overlay stops the start, or why a change could not be
/// kept.
#[derive(Debug)]
pub enum BootstrapOverlayError {
    /// The file is present and could not be read.
    Read(std::io::Error),
    /// The file is not a regular file owned by this uid and readable by
    /// nobody else, or is a symbolic link.
    NotPrivate {
        /// What is wrong with it.
        detail: String,
    },
    /// The file is larger than [`MAX_BOOTSTRAP_OVERLAY_BYTES`].
    TooLarge,
    /// The file is not the overlay's shape.
    Parse(String),
    /// An entry is not in the configuration's grammar.
    InvalidEntry {
        /// Its position in `peers`, from zero.
        index: usize,
        /// What the grammar refused.
        reason: &'static str,
    },
    /// Authored and added entries together pass the provider's bound.
    ComposedTooLarge {
        /// How many there are.
        got: usize,
    },
    /// The overlay could not be written: at load, the normalisation
    /// rewrite; at a change, the change itself.
    Write(PersistError),
}

impl core::fmt::Display for BootstrapOverlayError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Read(e) => write!(f, "the bootstrap overlay cannot be read: {e}"),
            Self::NotPrivate { detail } => {
                write!(f, "the bootstrap overlay is not private: {detail}")
            }
            Self::TooLarge => write!(
                f,
                "the bootstrap overlay is larger than {MAX_BOOTSTRAP_OVERLAY_BYTES} bytes"
            ),
            Self::Parse(e) => write!(f, "the bootstrap overlay does not parse: {e}"),
            Self::InvalidEntry { index, reason } => write!(
                f,
                "the bootstrap overlay's entry {index} is not a multiaddr-with-peer-id: {reason}"
            ),
            Self::ComposedTooLarge { got } => write!(
                f,
                "config.yaml's static-bootstrap entries and the bootstrap overlay's together hold \
                 {got}, more than {MAX_STATIC_BOOTSTRAP_PEERS}"
            ),
            Self::Write(e) => write!(f, "the bootstrap overlay cannot be written: {e}"),
        }
    }
}

impl core::error::Error for BootstrapOverlayError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Read(e) => Some(e),
            Self::Write(e) => Some(e),
            _ => None,
        }
    }
}

impl From<PrivateReadError> for BootstrapOverlayError {
    fn from(e: PrivateReadError) -> Self {
        match e {
            PrivateReadError::Read(e) => Self::Read(e),
            PrivateReadError::NotPrivate { detail } => Self::NotPrivate { detail },
            PrivateReadError::TooLarge => Self::TooLarge,
        }
    }
}

impl BootstrapOverlayError {
    /// Whether the failed write had already put the new overlay in place
    /// (the rename landed, syncing the directory failed): a caller that
    /// promises "a failed write changes nothing" puts the previous one
    /// back after this.
    #[must_use]
    pub const fn installed(&self) -> bool {
        matches!(self, Self::Write(PersistError::Unsynced(_)))
    }
}

/// The configuration's rule for one static-bootstrap entry, whole: the
/// grammar, the entry and its address half each within their bounds,
/// and a host this build can dial -- what `config.yaml`'s validator
/// applies, so an entry it would refuse is refused here by name, before
/// anything is written, rather than at wiring where it names nothing
/// (#367 review F3, `an_entry_the_configuration_refuses_is_refused_here`).
/// Public because the show side (`admin.peers.own_addresses`) lists only
/// what an add would take, by this same rule.
///
/// # Errors
/// A short static reason naming the rule the entry fails.
pub fn entry_grammar(entry: &str) -> Result<(), &'static str> {
    if entry.len() > MAX_STATIC_PEER_BYTES {
        return Err("the entry is longer than a static-bootstrap entry may be");
    }
    let (address, _) = split_peer_multiaddr(entry)?;
    if address.len() > MAX_ADDRESS_BYTES {
        return Err("the address is longer than a candidate address may be");
    }
    if crate::host_this_build_cannot_dial(address).is_some() {
        return Err("the address names a host this build cannot dial");
    }
    Ok(())
}

impl BootstrapOverlay {
    /// The overlay's path for `paths`' profile.
    #[must_use]
    pub fn path_for(paths: &ProfilePaths) -> PathBuf {
        paths.state_dir().join(BOOTSTRAP_OVERLAY_FILE)
    }

    /// The added entries, in the order they were added.
    #[must_use]
    pub fn peers(&self) -> &[String] {
        &self.peers
    }

    /// The provider's entries: `authored` first, then the added ones.
    #[must_use]
    pub fn composed(&self, authored: &[String]) -> Vec<String> {
        authored.iter().chain(&self.peers).cloned().collect()
    }

    /// Read the overlay at `path`, normalise it against `authored`,
    /// rewrite it when normalising changed it, and check the composed
    /// count. An absent file is the empty overlay and is not created.
    ///
    /// # Errors
    /// Every [`BootstrapOverlayError`], each a reason not to start.
    pub fn load_within(
        path: &Path,
        authored: &[String],
        boundary: &TrustBoundary,
    ) -> Result<Self, BootstrapOverlayError> {
        let Some(text) =
            private_read::read_private_within(path, boundary, MAX_BOOTSTRAP_OVERLAY_BYTES)?
        else {
            return Ok(Self::default());
        };
        let mut overlay: Self =
            private_read::object_only(&text).map_err(BootstrapOverlayError::Parse)?;
        for (index, entry) in overlay.peers.iter().enumerate() {
            entry_grammar(entry)
                .map_err(|reason| BootstrapOverlayError::InvalidEntry { index, reason })?;
        }
        if overlay.normalise(authored) {
            overlay.write_within(path, boundary)?;
        }
        let got = authored.len() + overlay.peers.len();
        if got > MAX_STATIC_BOOTSTRAP_PEERS {
            return Err(BootstrapOverlayError::ComposedTooLarge { got });
        }
        Ok(overlay)
    }

    /// Drop an entry `config.yaml` already authors, and a second copy of
    /// one the overlay holds (the first kept). Whether that changed
    /// anything.
    fn normalise(&mut self, authored: &[String]) -> bool {
        let before = self.peers.len();
        let mut seen: Vec<&String> = authored.iter().collect();
        let mut kept = Vec::with_capacity(before);
        for entry in &self.peers {
            if !seen.contains(&entry) {
                seen.push(entry);
                kept.push(entry.clone());
            }
        }
        self.peers = kept;
        before != self.peers.len()
    }

    /// The overlay after adding `address`, or `None` when it is already
    /// there (authored or added).
    ///
    /// # Errors
    /// [`MoveRefusal::Grammar`] outside the configuration's grammar,
    /// [`MoveRefusal::Bound`] at the provider's bound.
    pub fn add(&self, authored: &[String], address: &str) -> Result<Option<Self>, MoveRefusal> {
        entry_grammar(address).map_err(MoveRefusal::Grammar)?;
        if authored
            .iter()
            .chain(&self.peers)
            .any(|entry| entry == address)
        {
            return Ok(None);
        }
        if authored.len() + self.peers.len() >= MAX_STATIC_BOOTSTRAP_PEERS {
            return Err(MoveRefusal::Bound);
        }
        let mut next = self.clone();
        next.peers.push(address.to_owned());
        Ok(Some(next))
    }

    /// The overlay after removing `address`, or `None` when it does not
    /// hold it.
    ///
    /// # Errors
    /// [`MoveRefusal::Authored`] for an entry `config.yaml` authors.
    pub fn remove(&self, authored: &[String], address: &str) -> Result<Option<Self>, MoveRefusal> {
        if authored.iter().any(|entry| entry == address) {
            return Err(MoveRefusal::Authored);
        }
        let mut next = self.clone();
        next.peers.retain(|entry| entry != address);
        Ok((next.peers.len() != self.peers.len()).then_some(next))
    }

    /// Write the overlay whole: a temporary file in the same directory,
    /// owner-only from creation, renamed over `path`.
    ///
    /// # Errors
    /// [`BootstrapOverlayError::Write`]. A failure before the rename
    /// leaves the previous file as it was; one after it --
    /// [`BootstrapOverlayError::installed`] -- leaves THIS overlay in
    /// place, its name perhaps not durable.
    pub fn write_within(
        &self,
        path: &Path,
        boundary: &TrustBoundary,
    ) -> Result<(), BootstrapOverlayError> {
        let text = serde_json::to_vec_pretty(self).map_err(|e| {
            BootstrapOverlayError::Write(PersistError::Io(std::io::Error::other(e)))
        })?;
        persist::write_private_atomic_within(path, &text, boundary)
            .map_err(BootstrapOverlayError::Write)
    }
}
