// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Stage 6 exit gate, the routing half: same-PeerId multi-endpoint
//! delivery (ADR-0030 Model B).
//!
//! The network scenarios land with the backend. What is here at the
//! opening commit is the one claim already true: the registry this
//! stage routes through resolves an explicit destination and a default
//! deterministically, and collapses every failure to the same coarse
//! wire code. That is the invariant the network tests will later assert
//! from the other side of a socket; asserting it here first means a
//! regression in the registry is caught as a registry defect, not
//! discovered as a mysterious `no_route`.
#![allow(clippy::expect_used, clippy::panic)]

use interweave_transport_api::{DirectRejectReason, EndpointId};
use interweave_transport_runtime::endpoint_registry::ResolveFailure;

/// Every local resolve failure is the SAME wire code. A `const fn`
/// returning one value cannot grow a distinct code by having a variant
/// added, which is the property the exit gate's scenario 6 depends on.
#[test]
fn every_resolve_failure_is_no_route_on_the_wire() {
    for failure in [
        ResolveFailure::EndpointUnknown,
        ResolveFailure::EndpointDisabled,
        ResolveFailure::EndpointOffline,
        ResolveFailure::NoDefaultConfigured,
        ResolveFailure::EndpointPolicyDenied,
    ] {
        assert_eq!(
            failure.to_wire(),
            DirectRejectReason::NoRoute,
            "{failure:?} must not be distinguishable remotely"
        );
    }
}

/// The endpoint id the frame carries is the one the registry keys on: on
/// `/interweave/direct/3.0.0` a derived id (ADR-0067), 52 bytes, which the
/// wire's strict parser and the registry's id both accept, while a label of
/// the retired grammar, still constructible through the transitional
/// `EndpointId::parse`, is refused by the strict one the codec uses.
#[test]
fn endpoint_ids_share_one_grammar_between_wire_and_registry() {
    let id = interweave_test_support::endpoint_keys::endpoint_id("human");
    assert_eq!(
        id.as_str().len(),
        interweave_transport_api::ENDPOINT_FIELD_BYTES
    );
    assert_eq!(EndpointId::parse_derived(id.as_str()), Ok(id.clone()));
    assert_eq!(EndpointId::parse(id.as_str()), Ok(id));
    assert!(EndpointId::parse_derived("human").is_err());
    assert!(EndpointId::parse_derived("a".repeat(52)).is_err());
}

/// ENDPOINT IDS ARE AN OPEN SET, and this stage must not quietly close it.
/// An id is the name of whatever key the endpoint holds (ADR-0067): a
/// profile that adds an endpoint for a model that does not exist yet is
/// making a key, not editing this crate, and no wire byte changes because
/// the frame carries a length-prefixed id rather than a code point.
///
/// The risk is not today's code — it is a future `match` on known ids or
/// aliases somewhere in routing, which would compile, pass every existing
/// test, and silently make unknown endpoints unroutable. This asserts the
/// property directly so that change fails here.
#[test]
fn an_endpoint_no_one_has_heard_of_is_as_valid_as_the_familiar_ones() {
    for name in [
        "human",
        "claude",
        // Models that do not exist yet, and one that never will.
        "gpt-5",
        "gemini",
        "llama-4",
        "some.future.llm",
        "a",
    ] {
        let id = interweave_test_support::endpoint_keys::endpoint_id(name);
        assert!(
            EndpointId::parse_derived(id.as_str()).is_ok(),
            "the key for `{name}` names a legal endpoint"
        );
    }
}
