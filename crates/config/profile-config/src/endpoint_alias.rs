// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! An endpoint's local alias, and the one resolver every client picks an
//! endpoint through (ADR-0067 decision 3, ENDPOINTS.md section `EndpointId`
//! and section Configured endpoint set, A 2026-11-04 and A 2026-11-19).
//!
//! What a person or an operator types to pick an endpoint -- `human`,
//! `claude`, `fabric-control` -- is an ALIAS of its configuration entry:
//! it never reaches the wire, is unique only within the profile and
//! proves nothing. So it lives here, in the configuration, and not in
//! `transport-api`, whose types are the wire's.
//!
//! An alias and an `EndpointId` cannot be confused: an alias begins with
//! a lowercase letter and a derived id with `12D3KooW`.

use interweave_transport_api::EndpointId;
use serde::{Deserialize, Serialize};

use crate::EndpointsConfig;
use crate::endpoint_overlay::EndpointOverlay;

/// An endpoint's local alias: `^[a-z][a-z0-9._-]{0,63}$`, the retired
/// label grammar.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct EndpointAlias(String);

/// Why a text is not an [`EndpointAlias`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasError {
    /// Empty.
    Empty,
    /// Longer than [`EndpointAlias::MAX_BYTES`].
    TooLong {
        /// Its length in bytes.
        got: usize,
    },
    /// The first byte is not a lowercase ASCII letter.
    IllegalLeadingByte {
        /// The byte.
        byte: u8,
    },
    /// A byte outside `a-z0-9._-`.
    IllegalByte {
        /// Where.
        index: usize,
        /// The byte.
        byte: u8,
    },
}

impl core::fmt::Display for AliasError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Empty => f.write_str("an alias is not empty"),
            Self::TooLong { got } => write!(
                f,
                "an alias is at most {} bytes, got {got}",
                EndpointAlias::MAX_BYTES
            ),
            Self::IllegalLeadingByte { byte } => write!(
                f,
                "an alias begins with a lowercase letter, not byte 0x{byte:02x}"
            ),
            Self::IllegalByte { index, byte } => write!(
                f,
                "byte 0x{byte:02x} at {index} is outside an alias's a-z0-9._-"
            ),
        }
    }
}

impl core::error::Error for AliasError {}

impl EndpointAlias {
    /// The longest alias, in bytes.
    pub const MAX_BYTES: usize = 64;

    /// Parse an alias.
    ///
    /// # Errors
    /// An [`AliasError`] naming the first rule the text breaks.
    pub fn parse(value: impl Into<String>) -> Result<Self, AliasError> {
        let value = value.into();
        let bytes = value.as_bytes();
        let Some(&first) = bytes.first() else {
            return Err(AliasError::Empty);
        };
        if bytes.len() > Self::MAX_BYTES {
            return Err(AliasError::TooLong { got: bytes.len() });
        }
        if !first.is_ascii_lowercase() {
            return Err(AliasError::IllegalLeadingByte { byte: first });
        }
        for (index, &byte) in bytes.iter().enumerate() {
            let ok = byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-');
            if !ok {
                return Err(AliasError::IllegalByte { index, byte });
            }
        }
        Ok(Self(value))
    }

    /// The alias's text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl core::fmt::Display for EndpointAlias {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for EndpointAlias {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        Self::parse(text.as_str())
            .map_err(|e| serde::de::Error::custom(format!("AliasInvalid: alias {text:?}: {e}")))
    }
}

/// What a client or a configuration names an endpoint by: its id or its
/// alias. The two grammars are disjoint, so a text is at most one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndpointSelector {
    /// A derived `EndpointId`.
    Id(EndpointId),
    /// A local alias.
    Alias(EndpointAlias),
}

impl EndpointSelector {
    /// The selector `text` is: a derived id when it parses as one, else
    /// an alias; `None` when it is neither.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        if let Ok(id) = EndpointId::parse_derived(text) {
            return Some(Self::Id(id));
        }
        EndpointAlias::parse(text).ok().map(Self::Alias)
    }

    /// The id this selector names among `endpoints`' entries: an id that
    /// an entry holds, or the id of the entry whose alias it is.
    #[must_use]
    pub fn resolve_in(&self, endpoints: &EndpointsConfig) -> Option<EndpointId> {
        endpoints
            .entries
            .iter()
            .find(|e| match self {
                Self::Id(id) => &e.id == id,
                Self::Alias(alias) => e.alias.as_ref() == Some(alias),
            })
            .map(|e| e.id.clone())
    }
}

impl core::fmt::Display for EndpointSelector {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Id(id) => f.write_str(id.as_str()),
            Self::Alias(alias) => f.write_str(alias.as_str()),
        }
    }
}

/// Why [`resolve_endpoint`] named no endpoint. Each is a refused start,
/// named for what the client was given -- never a hello sent with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveEndpointError {
    /// The text is neither a derived `EndpointId` nor an alias.
    NotASelector(String),
    /// No configured endpoint carries this alias.
    UnknownAlias(EndpointAlias),
    /// No configured endpoint has this id.
    UnknownId(EndpointId),
}

impl core::fmt::Display for ResolveEndpointError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotASelector(text) => {
                write!(f, "endpoint {text:?} is neither an EndpointId nor an alias")
            }
            Self::UnknownAlias(alias) => {
                write!(f, "no configured endpoint has the alias '{alias}'")
            }
            Self::UnknownId(id) => {
                write!(f, "no configured endpoint has the id '{}'", id.as_str())
            }
        }
    }
}

impl core::error::Error for ResolveEndpointError {}

/// The configured `EndpointId` a client's `selector` -- an alias or an
/// id, as `--endpoint` takes it -- names, over the COMPOSED
/// configuration: `config.yaml`'s entries with `overlay` applied
/// ([`EndpointOverlay::apply`]).
///
/// This is the one resolver every client goes through (ENDPOINTS.md
/// section Configured endpoint set, A 2026-11-19); `ipc/hello` takes the
/// id it returns, never the alias. An endpoint resolves whether or not it
/// is enabled: whether it may be leased is the daemon's to answer.
///
/// The overlay is applied rather than ignored so that the endpoints it
/// ADDS resolve too: `EndpointOverlay::apply` puts the overlay's `added`
/// entries into the composed `entries` this searches, so an added alias
/// resolves as an authored one does
/// (`an_added_alias_resolves_and_an_unknown_one_is_refused`).
///
/// # Errors
/// [`ResolveEndpointError`]: a text that is neither grammar, an alias no
/// entry carries, an id no entry has.
pub fn resolve_endpoint(
    config: &EndpointsConfig,
    overlay: &EndpointOverlay,
    selector: &str,
) -> Result<EndpointId, ResolveEndpointError> {
    let Some(parsed) = EndpointSelector::parse(selector) else {
        return Err(ResolveEndpointError::NotASelector(selector.to_owned()));
    };
    let composed = overlay.apply(config);
    parsed.resolve_in(&composed).ok_or(match parsed {
        EndpointSelector::Id(id) => ResolveEndpointError::UnknownId(id),
        EndpointSelector::Alias(alias) => ResolveEndpointError::UnknownAlias(alias),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_alias_grammar_is_the_retired_labels() {
        for ok in ["human", "a", "fabric-control", "x.y_z-0", &"a".repeat(64)] {
            assert!(EndpointAlias::parse(ok).is_ok(), "{ok}");
        }
        assert_eq!(EndpointAlias::parse(""), Err(AliasError::Empty));
        assert_eq!(
            EndpointAlias::parse("a".repeat(65)),
            Err(AliasError::TooLong { got: 65 })
        );
        assert_eq!(
            EndpointAlias::parse("Human"),
            Err(AliasError::IllegalLeadingByte { byte: b'H' })
        );
        assert_eq!(
            EndpointAlias::parse("0human"),
            Err(AliasError::IllegalLeadingByte { byte: b'0' })
        );
        assert_eq!(
            EndpointAlias::parse("hu man"),
            Err(AliasError::IllegalByte {
                index: 2,
                byte: b' '
            })
        );
        assert_eq!(
            EndpointAlias::parse("hu/man"),
            Err(AliasError::IllegalByte {
                index: 2,
                byte: b'/'
            })
        );
    }

    #[test]
    fn a_selector_is_an_id_or_an_alias_never_both() {
        let id = EndpointId::from_ed25519_public(&[1; 32]);
        assert_eq!(
            EndpointSelector::parse(id.as_str()),
            Some(EndpointSelector::Id(id))
        );
        assert_eq!(
            EndpointSelector::parse("human"),
            Some(EndpointSelector::Alias(
                EndpointAlias::parse("human").expect("alias")
            ))
        );
        assert_eq!(EndpointSelector::parse("Not An Id!"), None);
    }
}
