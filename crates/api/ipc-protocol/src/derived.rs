// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Every `EndpointId` an IPC document carries is a DERIVED id (ADR-0067
//! decision 7, `LOCAL-IPC.md` §Version negotiation and phases: "Major 3 is
//! the IPC of the derived `EndpointId`").
//!
//! No reader of major 3 tolerates a label, so every IPC field that holds
//! an `EndpointId` reads through these, [`EndpointId::parse_derived`]
//! (also what `EndpointId`'s own `Deserialize` uses, and its only text
//! constructor): a label in a request's params is
//! a params error, answered `InvalidArgument`, and one in a server's frame
//! is the server's violation. `a_label_is_refused_wherever_an_ipc_document_carries_an_endpoint_id`
//! (`tests/schema_agreement.rs`) feeds a label to every such field.

use interweave_transport_api::EndpointId;
use serde::Deserialize;
use serde::de::Error as _;

/// One derived id.
pub(crate) fn endpoint<'de, D: serde::Deserializer<'de>>(d: D) -> Result<EndpointId, D::Error> {
    let text = String::deserialize(d)?;
    EndpointId::parse_derived(text).map_err(D::Error::custom)
}

/// A derived id under a key that may be absent (`#[serde(default)]`), and
/// is never `null` when present.
pub(crate) fn absent_or_endpoint<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<EndpointId>, D::Error> {
    endpoint(d).map(Some)
}

/// A derived id or `null`, under a key that is required.
pub(crate) fn nullable_endpoint<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<EndpointId>, D::Error> {
    Option::<String>::deserialize(d)?
        .map(|text| EndpointId::parse_derived(text).map_err(D::Error::custom))
        .transpose()
}

/// A list of derived ids.
pub(crate) fn endpoints<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<EndpointId>, D::Error> {
    Vec::<String>::deserialize(d)?
        .into_iter()
        .map(|text| EndpointId::parse_derived(text).map_err(D::Error::custom))
        .collect()
}
