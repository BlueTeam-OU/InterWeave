// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! One admitted request, one port call (plan §16 (1), (5)).
//!
//! Nothing is decided here that the port does not decide: the capability
//! was judged at admission, and every refusal past it -- a missing lease,
//! an unjoined channel, a peer out of trust -- is the binding's answer,
//! carried back as the response's code. This is the whole of what makes
//! IPC a serialization of the port rather than a second behaviour model.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use interweave_ipc_protocol::{
    AddressList, AddressResult, AddressesResult, AdminStatusResult, DirectoryResult, EmptyResult,
    EndpointAddResult, EndpointList, EndpointRemoveResult, OwnAddressList, PeerList, Request,
    RequestId, ResponseFrame, SendResult, SessionLossesResult, SetAliasResult, SetEnabledResult,
    TrustList, TrustSeedResult, TrustSetResult,
};
use interweave_local_client_api::{
    AddressChange, AdminPort, DataSessionPort, EndpointSetKind, TrustSource,
};
use interweave_transport_api::{BroadcastMessageV1, DirectDestination, TransportError};

use crate::counters::Counters;

/// Answer a data-socket request through its session.
pub(crate) async fn data<S: DataSessionPort>(
    session: &S,
    id: RequestId,
    request: Request,
) -> ResponseFrame {
    let empty = |id, outcome: Result<(), TransportError>| match outcome {
        Ok(()) => ResponseFrame::success(id, &EmptyResult {}),
        Err(code) => ResponseFrame::failure(id, code),
    };
    match request {
        Request::ChannelJoin(p) => empty(id, session.join(p.channel).await),
        Request::ChannelLeave(p) => empty(id, session.leave(p.channel).await),
        Request::BroadcastPublish(p) => {
            let message = BroadcastMessageV1 {
                message_id: p.message_id,
                // Diagnostic only (the field's own contract): no admission
                // path reads it.
                sent_at_ms: wall_ms(),
                payload: p.payload,
            };
            empty(id, session.broadcast(p.channel, message).await)
        }
        Request::DirectSend(p) => {
            // Absent asks the remote for its configured default, never
            // fan-out (ADR-0030).
            let destination = match p.endpoint {
                Some(endpoint) => DirectDestination::to_endpoint(p.peer, endpoint),
                None => DirectDestination::to_default(p.peer),
            };
            match session
                .send_direct(destination, p.message_id, p.payload)
                .await
            {
                Ok(resolved_endpoint) => {
                    ResponseFrame::success(id, &SendResult { resolved_endpoint })
                }
                Err(code) => ResponseFrame::failure(id, code),
            }
        }
        Request::EndpointsQuery(p) => match session.query_endpoints(p.peer).await {
            Ok(directory) => ResponseFrame::success(id, &DirectoryResult::from(directory)),
            Err(code) => ResponseFrame::failure(id, code),
        },
        Request::SessionLosses => match session.losses().await {
            Ok(losses) => ResponseFrame::success(id, &SessionLossesResult::from(losses)),
            Err(code) => ResponseFrame::failure(id, code),
        },
        // Admission refuses the admin domain on the data socket before
        // dispatch; reaching here would be admission's bug, answered as
        // the refusal it should have been.
        Request::AdminStatus
        | Request::AdminEndpointsList
        | Request::AdminEndpointsRevoke(_)
        | Request::AdminEndpointsSetEnabled(_)
        | Request::AdminEndpointsSetDefault(_)
        | Request::AdminEndpointsAdd(_)
        | Request::AdminEndpointsRemove(_)
        | Request::AdminEndpointsSetAdvertise(_)
        | Request::AdminEndpointsSetAlias(_)
        | Request::AdminShutdown(_)
        | Request::AdminTrustList(_)
        | Request::AdminTrustSet(_)
        | Request::AdminTrustSeed(_)
        | Request::AdminPeersList(_)
        | Request::AdminPeersAddAddress(_)
        | Request::AdminPeersRemoveAddress(_)
        | Request::AdminPeersListAddresses
        | Request::AdminPeersOwnAddresses
        | Request::AdminPeersAddAddresses(_) => {
            ResponseFrame::failure(id, TransportError::CapabilityDenied)
        }
    }
}

/// Answer an admin-socket request through its port, in the one shape
/// each result has on major 3 (`LOCAL-IPC.md` §Version negotiation and
/// phases: "One shape per document on 3.0").
pub(crate) async fn admin<A: AdminPort>(
    port: &A,
    counters: &Counters,
    default_grace: Duration,
    id: RequestId,
    request: Request,
) -> ResponseFrame {
    let empty = |id, outcome: Result<(), TransportError>| match outcome {
        Ok(()) => ResponseFrame::success(id, &EmptyResult {}),
        Err(code) => ResponseFrame::failure(id, code),
    };
    match request {
        Request::AdminStatus => match port.status().await {
            Ok(status) => {
                let result = AdminStatusResult::new(status, counters.snapshot());
                ResponseFrame::success(id, &result)
            }
            Err(code) => ResponseFrame::failure(id, code),
        },
        Request::AdminEndpointsList => match port.leases().await {
            // A ROW THAT DOES NOT PERSIST IS REFUSED, as an unpersisted
            // trust row is: only a store-less runtime reports one, and it
            // is never served over IPC -- one that is served is refused
            // here with `Internal`, never served a row the contract (the
            // 2.5 row, the one 3.0 serves) does not describe for it
            // (`an_unpersisted_endpoint_row_is_refused`).
            Ok(views) if views.iter().any(|row| !row.persisted) => {
                ResponseFrame::failure(id, TransportError::Internal)
            }
            Ok(views) => match EndpointList::from_views(views) {
                Ok(list) => ResponseFrame::success(id, &list),
                Err(code) => ResponseFrame::failure(id, code),
            },
            Err(code) => ResponseFrame::failure(id, code),
        },
        Request::AdminEndpointsRevoke(p) => empty(id, port.revoke_endpoint(p.endpoint).await),
        Request::AdminEndpointsSetEnabled(p) => {
            match port.set_endpoint_enabled(p.endpoint, p.enabled).await {
                Ok(revoked_epoch) => {
                    ResponseFrame::success(id, &SetEnabledResult { revoked_epoch })
                }
                Err(code) => ResponseFrame::failure(id, code),
            }
        }
        Request::AdminEndpointsSetDefault(p) => {
            empty(id, port.set_default_endpoint(p.endpoint).await)
        }
        // The endpoint set's methods (3.0): a refusal is the port's ANSWER,
        // carried as a result naming its rule, and a failure of the port
        // itself is the error (`an_endpoint_set_refusal_is_a_result_naming_its_rule`).
        Request::AdminEndpointsAdd(p) => match port
            .add_endpoint(p.key, p.alias, p.allowed_client_kinds)
            .await
        {
            Ok(change) => ResponseFrame::success(id, &EndpointAddResult(change)),
            Err(code) => ResponseFrame::failure(id, code),
        },
        Request::AdminEndpointsRemove(p) => match port.remove_endpoint(p.endpoint).await {
            Ok(change) => ResponseFrame::success(id, &EndpointRemoveResult(change)),
            Err(code) => ResponseFrame::failure(id, code),
        },
        Request::AdminEndpointsSetAdvertise(p) => empty(
            id,
            port.set_endpoint_advertise(p.endpoint, p.advertise).await,
        ),
        Request::AdminEndpointsSetAlias(p) => {
            match port.set_endpoint_alias(p.endpoint, p.alias).await {
                Ok(change) => ResponseFrame::success(id, &SetAliasResult(change)),
                Err(code) => ResponseFrame::failure(id, code),
            }
        }
        Request::AdminShutdown(p) => {
            let grace = p
                .grace_ms
                .map_or(default_grace, |ms| Duration::from_millis(u64::from(ms)));
            empty(id, port.shutdown(grace).await)
        }
        // One page of the policy the port answers whole: the cursor is a
        // position in its order, so nothing is held between pages.
        //
        // The endpoint form (IPC 3.0, ADR-0067 decision 6) is the same page
        // over one direction of the endpoint's set. `TrustListParams`
        // decodes `endpoint` and `direction` together, so one without the
        // other is not a request that reaches here; if one did, it is the
        // params error it would have been.
        Request::AdminTrustList(p) => match (p.endpoint, p.direction) {
            (Some(endpoint), Some(direction)) => {
                match port.endpoint_trust(endpoint, direction).await {
                    // A row that does not persist is refused as the operator
                    // form refuses it, and so is an `own` set's row that is
                    // not `administered`: after the seed the configuration
                    // no longer decides an own set, so a row that says
                    // otherwise is the port's defect, never served as the
                    // contract's row
                    // (`an_endpoint_list_refuses_a_row_its_set_cannot_carry`).
                    Ok(view)
                        if view.allowed.iter().any(|row| {
                            !row.persisted
                                || (view.set == EndpointSetKind::Own
                                    && row.source != TrustSource::Administered)
                        }) =>
                    {
                        ResponseFrame::failure(id, TransportError::Internal)
                    }
                    Ok(view) => ResponseFrame::success(
                        id,
                        &TrustList::page_endpoint(view, p.after.as_ref()),
                    ),
                    Err(code) => ResponseFrame::failure(id, code),
                }
            }
            (None, None) => match port.trust().await {
                // A ROW THAT DOES NOT PERSIST IS REFUSED, never served in
                // the 2.1 shape the contract (the 2.3 row, the one 3.0
                // serves) forbids: only a store-less runtime reports one,
                // and it is never served over IPC (LOCAL-CLIENT.md §7 item
                // 11) -- one that is served is refused here with
                // `Internal`, never served the row
                // (`an_unpersisted_trust_row_is_refused`).
                Ok(view) if view.allowed.iter().any(|row| !row.persisted) => {
                    ResponseFrame::failure(id, TransportError::Internal)
                }
                Ok(view) => ResponseFrame::success(id, &TrustList::page(view, p.after.as_ref())),
                Err(code) => ResponseFrame::failure(id, code),
            },
            _ => ResponseFrame::failure(id, TransportError::InvalidArgument),
        },
        // The set's own result, a refusal -- superseded among them -- by
        // name: the 2.10 shape, the one 3.0 serves.
        Request::AdminTrustSet(p) => {
            let change = match p.endpoint {
                Some(endpoint) => port.set_endpoint_trust(endpoint, p.peer, p.allowed).await,
                None => port.set_trust(p.peer, p.allowed).await,
            };
            match change {
                Ok(change) => ResponseFrame::success(id, &TrustSetResult::from(change)),
                Err(code) => ResponseFrame::failure(id, code),
            }
        }
        Request::AdminTrustSeed(p) => match port.seed_endpoint_trust(p.endpoint).await {
            Ok(seed) => ResponseFrame::success(id, &TrustSeedResult::from(seed)),
            Err(code) => ResponseFrame::failure(id, code),
        },
        // The dial gate's rows, paged as the trust list is: the cursor is
        // a position in their order, so nothing is held between pages.
        Request::AdminPeersList(p) => match port.peers().await {
            Ok(rows) => ResponseFrame::success(id, &PeerList::page(rows, p.after.as_ref())),
            Err(code) => ResponseFrame::failure(id, code),
        },
        // A refusal is the port's ANSWER, carried as a result naming its
        // rule; only a failure of the port itself is an error (ADR-0062).
        Request::AdminPeersAddAddress(p) => {
            address_answer(id, port.add_bootstrap_address(p.address).await)
        }
        Request::AdminPeersRemoveAddress(p) => {
            address_answer(id, port.remove_bootstrap_address(p.address).await)
        }
        Request::AdminPeersListAddresses => match port.bootstrap_addresses().await {
            Ok(rows) => match AddressList::from_views(rows) {
                Ok(list) => ResponseFrame::success(id, &list),
                Err(code) => ResponseFrame::failure(id, code),
            },
            Err(code) => ResponseFrame::failure(id, code),
        },
        Request::AdminPeersAddAddresses(p) => {
            let lines = p.addresses.len();
            match port.add_bootstrap_addresses(p.addresses).await {
                Ok(change) => match AddressesResult::from_view(change, lines) {
                    Ok(result) => ResponseFrame::success(id, &result),
                    Err(code) => ResponseFrame::failure(id, code),
                },
                Err(code) => ResponseFrame::failure(id, code),
            }
        }
        Request::AdminPeersOwnAddresses => match port.own_addresses().await {
            Ok(own) => match OwnAddressList::from_view(own) {
                Ok(list) => ResponseFrame::success(id, &list),
                Err(code) => ResponseFrame::failure(id, code),
            },
            Err(code) => ResponseFrame::failure(id, code),
        },
        // The data domain on the admin socket: admission's to refuse, as
        // above.
        Request::ChannelJoin(_)
        | Request::ChannelLeave(_)
        | Request::BroadcastPublish(_)
        | Request::DirectSend(_)
        | Request::EndpointsQuery(_)
        | Request::SessionLosses => ResponseFrame::failure(id, TransportError::CapabilityDenied),
    }
}

/// An address write's answer: the port's change, a refusal by its name.
fn address_answer(id: RequestId, outcome: Result<AddressChange, TransportError>) -> ResponseFrame {
    match outcome {
        Ok(change) => ResponseFrame::success(id, &AddressResult::from(change)),
        Err(code) => ResponseFrame::failure(id, code),
    }
}

fn wall_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::fake::{Fake, PEER};
    use interweave_ipc_protocol::{Frame, Method};
    use interweave_local_client_api::{
        AdminBinding as _, DataCapability, DataSessionBinding as _, SessionRequest,
    };

    fn request(method: Method, params: &serde_json::Value) -> Request {
        let raw = serde_json::value::to_raw_value(params).expect("raw");
        Request::decode(method, Some(&raw)).expect("a well-formed request")
    }

    fn id() -> RequestId {
        RequestId::new("r").expect("id")
    }

    fn body(response: ResponseFrame) -> String {
        Frame::Response(response).to_body()
    }

    async fn leased(
        fake: &Fake,
    ) -> <Fake as interweave_local_client_api::DataSessionBinding>::Session {
        fake.open(
            SessionRequest::new(
                "human-client",
                Some(crate::fake::ep("human")),
                [
                    DataCapability::Commands,
                    DataCapability::Events,
                    DataCapability::EndpointsQuery,
                ],
            )
            .expect("request"),
        )
        .await
        .expect("session")
    }

    /// Each data method is exactly one call of its port method, and the
    /// result is the schema's.
    #[tokio::test]
    async fn every_data_method_is_one_port_call() {
        let fake = Fake::default();
        let session = leased(&fake).await;
        let payload = serde_json::json!({"bytes": "aGk"});
        let mid = "00000000000000000000000000000001";
        let resolved = format!(
            r#""resolved_endpoint":"{}""#,
            crate::fake::ep("remote").as_str()
        );
        let directory = format!(r#""endpoints":["{}"]"#, crate::fake::ep("agent").as_str());
        let cases = [
            (
                Method::ChannelJoin,
                serde_json::json!({"channel": "ops"}),
                "join ops",
                r#""result":{}"#,
            ),
            (
                Method::ChannelLeave,
                serde_json::json!({"channel": "ops"}),
                "leave ops",
                r#""result":{}"#,
            ),
            (
                Method::BroadcastPublish,
                serde_json::json!({"channel": "ops", "message_id": mid, "payload": payload}),
                "broadcast ops",
                r#""result":{}"#,
            ),
            (
                Method::DirectSend,
                serde_json::json!({"peer": PEER, "message_id": mid, "payload": payload}),
                "send DirectDestination",
                resolved.as_str(),
            ),
            (
                Method::EndpointsQuery,
                serde_json::json!({"peer": PEER}),
                "query ",
                directory.as_str(),
            ),
            (
                Method::SessionLosses,
                serde_json::json!({}),
                "losses",
                r#""result":{"queue_overflow":3,"ingress_refused":5}"#,
            ),
        ];
        for (method, params, call, result) in cases {
            let before = fake.script().calls.len();
            let answer = body(data(&session, id(), request(method, &params)).await);
            let calls = fake.script().calls[before..].to_vec();
            assert_eq!(calls.len(), 1, "{}: {calls:?}", method.as_str());
            assert!(calls[0].starts_with(call), "{}: {calls:?}", method.as_str());
            assert!(answer.contains(result), "{}: {answer}", method.as_str());
        }
    }

    /// Absent asks for the remote's default; present names it. The port
    /// sees which, and never a fan-out.
    #[tokio::test]
    async fn an_absent_destination_endpoint_asks_for_the_default() {
        let fake = Fake::default();
        let session = leased(&fake).await;
        let base = serde_json::json!({"peer": PEER, "message_id": "00000000000000000000000000000001",
                                      "payload": {"bytes": ""}});
        let _ = data(&session, id(), request(Method::DirectSend, &base)).await;
        let mut named = base.clone();
        named["endpoint"] = serde_json::json!(crate::fake::ep("agent").as_str());
        let _ = data(&session, id(), request(Method::DirectSend, &named)).await;
        let calls = fake.script().calls.clone();
        let sends: Vec<_> = calls.iter().filter(|c| c.starts_with("send")).collect();
        assert_eq!(sends.len(), 2);
        assert!(sends[0].contains("endpoint: None"), "{}", sends[0]);
        assert!(
            sends[1].contains(crate::fake::ep("agent").as_str()),
            "{}",
            sends[1]
        );
    }

    /// The port's refusal is the answer: here a send from a session with
    /// no lease is the binding's `EndpointNotRegistered`.
    #[tokio::test]
    async fn the_ports_refusal_is_the_responses_code() {
        let fake = Fake::default();
        let session = fake
            .open(SessionRequest::new("diag", None, [DataCapability::Commands]).expect("request"))
            .await
            .expect("session");
        let params = serde_json::json!({"peer": PEER, "message_id": "00000000000000000000000000000001",
                                        "payload": {"bytes": ""}});
        let answer = body(data(&session, id(), request(Method::DirectSend, &params)).await);
        assert!(answer.contains("EndpointNotRegistered"), "{answer}");
    }

    #[tokio::test]
    async fn every_admin_method_is_one_port_call_and_status_carries_the_servers_counters() {
        let fake = Fake::default();
        let port = fake.admin([].into()).await.expect("port");
        let counters = Counters::default();
        let grace = Duration::from_millis(1234);
        let status = body(admin(&port, &counters, grace, id(), Request::AdminStatus).await);
        assert!(status.contains(r#""data_connections":0"#), "{status}");
        let cases = [
            (Method::AdminEndpointsList, serde_json::json!({}), "leases"),
            (
                Method::AdminEndpointsRevoke,
                serde_json::json!({"endpoint": crate::fake::ep("human").as_str()}),
                "revoke human",
            ),
            (
                Method::AdminEndpointsSetEnabled,
                serde_json::json!({"endpoint": crate::fake::ep("human").as_str(), "enabled": false}),
                "set_enabled human false",
            ),
            (
                Method::AdminEndpointsSetDefault,
                serde_json::json!({"endpoint": null}),
                "set_default None",
            ),
            (
                Method::AdminEndpointsAdd,
                serde_json::json!({"key": "k.key", "alias": "work",
                    "allowed_client_kinds": ["human", "cli"]}),
                "add_endpoint k.key work human,cli",
            ),
            (
                Method::AdminEndpointsRemove,
                serde_json::json!({"endpoint": crate::fake::ep("human").as_str()}),
                "remove_endpoint human",
            ),
            (
                Method::AdminEndpointsSetAdvertise,
                serde_json::json!({"endpoint": crate::fake::ep("human").as_str(), "advertise": true}),
                "set_advertise human true",
            ),
            (
                Method::AdminEndpointsSetAlias,
                serde_json::json!({"endpoint": crate::fake::ep("human").as_str(), "alias": "home"}),
                "set_alias human home",
            ),
            (
                Method::AdminShutdown,
                serde_json::json!({}),
                "shutdown 1234",
            ),
            (
                Method::AdminShutdown,
                serde_json::json!({"grace_ms": 7}),
                "shutdown 7",
            ),
            (Method::AdminTrustList, serde_json::json!({}), "trust"),
            (Method::AdminPeersList, serde_json::json!({}), "peers"),
            (
                Method::AdminPeersAddAddress,
                serde_json::json!({"address": "/ip4/192.168.1.20/tcp/4001/p2p/x"}),
                "add_address /ip4/192.168.1.20/tcp/4001/p2p/x",
            ),
            (
                Method::AdminPeersRemoveAddress,
                serde_json::json!({"address": "a"}),
                "remove_address a",
            ),
            (
                Method::AdminPeersListAddresses,
                serde_json::json!({}),
                "list_addresses",
            ),
            (
                Method::AdminPeersOwnAddresses,
                serde_json::json!({}),
                "own_addresses",
            ),
            (
                Method::AdminPeersAddAddresses,
                serde_json::json!({"addresses": ["a", "b"]}),
                "add_addresses a b",
            ),
            (
                Method::AdminTrustSet,
                serde_json::json!({"peer": crate::fake::peer().as_str(), "allowed": false}),
                "set_trust 12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN false",
            ),
        ];
        for (method, params, call) in cases {
            let before = fake.script().calls.len();
            let answer = body(admin(&port, &counters, grace, id(), request(method, &params)).await);
            assert!(
                answer.contains(r#""ok":true"#),
                "{}: {answer}",
                method.as_str()
            );
            assert_eq!(
                fake.script().calls[before..],
                [call.to_owned()],
                "{}",
                method.as_str()
            );
        }
    }

    /// The endpoint dimension (IPC 3.0): the endpoint form of list, set and
    /// seed each reaches its own port method with the endpoint, and the
    /// endpoint-less form still reaches the operator's (the control, in the
    /// same run), never the other.
    #[tokio::test]
    async fn the_endpoint_forms_reach_their_own_port_methods() {
        use interweave_ipc_protocol::TrustSeedResult;
        use interweave_local_client_api::{
            EndpointSetKind, EndpointTrustView, TrustSeed, TrustedPeer,
        };
        let fake = Fake::default();
        fake.script().endpoint_view = Some(EndpointTrustView {
            set: EndpointSetKind::Own,
            allowed: vec![TrustedPeer {
                peer: crate::fake::peer(),
                persisted: true,
                source: TrustSource::Administered,
            }],
        });
        fake.script().seed = Some(TrustSeed::Present);
        let port = fake.admin([].into()).await.expect("port");
        let counters = Counters::default();
        let run = |method, params: serde_json::Value| {
            let request = request(method, &params);
            let (port, counters) = (&port, &counters);
            async move { admin(port, counters, Duration::from_secs(1), id(), request).await }
        };
        let human = crate::fake::ep("human");
        let peer = crate::fake::peer();

        let page = run(
            Method::AdminTrustList,
            serde_json::json!({"endpoint": human.as_str(), "direction": "inbound"}),
        )
        .await
        .outcome::<TrustList>()
        .expect("a page");
        assert_eq!(page.set, Some(EndpointSetKind::Own));
        assert_eq!(page.local_peer, None);
        assert_eq!(page.allowed.len(), 1);
        let _ = run(
            Method::AdminTrustList,
            serde_json::json!({"endpoint": human.as_str(), "direction": "outbound"}),
        )
        .await;
        let set = run(
            Method::AdminTrustSet,
            serde_json::json!({"peer": peer.as_str(), "allowed": true, "endpoint": human.as_str()}),
        )
        .await
        .outcome::<TrustSetResult>()
        .expect("a result");
        assert_eq!(
            interweave_local_client_api::TrustChange::from(set),
            interweave_local_client_api::TrustChange::Set
        );
        let seed = run(
            Method::AdminTrustSeed,
            serde_json::json!({"endpoint": human.as_str()}),
        )
        .await
        .outcome::<TrustSeedResult>()
        .expect("a result");
        assert_eq!(TrustSeed::from(seed), TrustSeed::Present);
        // The control: the endpoint-less forms are the operator's.
        let operator = run(Method::AdminTrustList, serde_json::json!({}))
            .await
            .outcome::<TrustList>()
            .expect("a page");
        assert_eq!(operator.set, None);
        assert!(operator.local_peer.is_some());
        let _ = run(
            Method::AdminTrustSet,
            serde_json::json!({"peer": peer.as_str(), "allowed": false}),
        )
        .await;
        let calls = fake.script().calls.clone();
        assert_eq!(
            calls,
            [
                "endpoint_trust human inbound".to_owned(),
                "endpoint_trust human outbound".to_owned(),
                format!("set_endpoint_trust human {} true", peer.as_str()),
                "seed_endpoint_trust human".to_owned(),
                "trust".to_owned(),
                format!("set_trust {} false", peer.as_str()),
            ]
        );
        // An unknown endpoint is the port's code.
        fake.script().endpoint_view = None;
        let answer = body(
            run(
                Method::AdminTrustList,
                serde_json::json!({"endpoint": human.as_str(), "direction": "inbound"}),
            )
            .await,
        );
        assert!(answer.contains("EndpointUnknown"), "{answer}");
    }

    /// An endpoint answer a set cannot carry is refused `Internal`, as an
    /// unpersisted operator row is: a row not persisted in any set, and
    /// an `own` set's row that is not `administered`. The same row in a
    /// set that may carry it (`authored`, `configured`) is the control.
    #[tokio::test]
    async fn an_endpoint_list_refuses_a_row_its_set_cannot_carry() {
        use interweave_local_client_api::{EndpointSetKind, EndpointTrustView, TrustedPeer};
        let fake = Fake::default();
        let port = fake.admin([].into()).await.expect("port");
        let counters = Counters::default();
        let params = serde_json::json!({
            "endpoint": crate::fake::ep("human").as_str(), "direction": "inbound"
        });
        let row = |persisted, source| TrustedPeer {
            peer: crate::fake::peer(),
            persisted,
            source,
        };
        for (set, persisted, source, served) in [
            (EndpointSetKind::Own, true, TrustSource::Administered, true),
            (
                EndpointSetKind::Authored,
                true,
                TrustSource::Configured,
                true,
            ),
            (
                EndpointSetKind::Inherited,
                true,
                TrustSource::Configured,
                true,
            ),
            (EndpointSetKind::Own, true, TrustSource::Configured, false),
            (
                EndpointSetKind::Own,
                false,
                TrustSource::Administered,
                false,
            ),
            (
                EndpointSetKind::Authored,
                false,
                TrustSource::Configured,
                false,
            ),
        ] {
            fake.script().endpoint_view = Some(EndpointTrustView {
                set,
                allowed: vec![row(persisted, source)],
            });
            let answer = body(
                admin(
                    &port,
                    &counters,
                    Duration::from_secs(1),
                    id(),
                    request(Method::AdminTrustList, &params),
                )
                .await,
            );
            assert_eq!(
                answer.contains(r#""ok":true"#),
                served,
                "{set:?} persisted {persisted} {source:?}: {answer}"
            );
            if !served {
                assert!(answer.contains("Internal"), "{answer}");
            }
        }
    }

    /// A request of the other domain never reaches its port, even if
    /// admission were bypassed.
    #[tokio::test]
    async fn the_other_domains_request_reaches_no_port() {
        let fake = Fake::default();
        let session = leased(&fake).await;
        let port = fake.admin([].into()).await.expect("port");
        let before = fake.script().calls.len();
        let answer = body(data(&session, id(), Request::AdminStatus).await);
        assert!(answer.contains("CapabilityDenied"), "{answer}");
        let join = request(Method::ChannelJoin, &serde_json::json!({"channel": "ops"}));
        let answer = body(admin(&port, &Counters::default(), Duration::ZERO, id(), join).await);
        assert!(answer.contains("CapabilityDenied"), "{answer}");
        assert_eq!(fake.script().calls.len(), before);
    }

    /// The endpoint set's refusals are results naming their rules, never
    /// error frames (LOCAL-IPC.md: the error vocabulary gains no name);
    /// the remove's `removed` is the control, a result with no rule.
    #[tokio::test]
    async fn an_endpoint_set_refusal_is_a_result_naming_its_rule() {
        let fake = Fake::default();
        let port = fake.admin([].into()).await.expect("port");
        let counters = Counters::default();
        let grace = Duration::from_millis(1);
        let answer = |method, params| {
            let request = request(method, &params);
            let port = &port;
            let counters = &counters;
            async move { body(admin(port, counters, grace, id(), request).await) }
        };
        let added = answer(
            Method::AdminEndpointsAdd,
            serde_json::json!({"key": "k.key", "alias": "w", "allowed_client_kinds": []}),
        )
        .await;
        assert!(added.contains(r#""ok":true"#), "{added}");
        assert!(
            added.contains(r#""outcome":"refused""#)
                && added.contains(r#""refused":"key_mismatch""#),
            "{added}"
        );
        let renamed = answer(
            Method::AdminEndpointsSetAlias,
            serde_json::json!({"endpoint": crate::fake::ep("human").as_str(), "alias": "x"}),
        )
        .await;
        assert!(renamed.contains(r#""refused":"alias_taken""#), "{renamed}");
        let removed = answer(
            Method::AdminEndpointsRemove,
            serde_json::json!({"endpoint": crate::fake::ep("human").as_str()}),
        )
        .await;
        assert!(removed.contains(r#""outcome":"removed""#), "{removed}");
        assert!(!removed.contains("refused"), "{removed}");
    }

    /// A refusal is the port's answer, so the response SUCCEEDS and names
    /// the rule; a client renders `refused`, never an error code.
    #[tokio::test]
    async fn an_address_refusal_is_a_result_naming_its_rule() {
        let fake = Fake::default();
        let port = fake.admin([].into()).await.expect("port");
        let answer = body(
            admin(
                &port,
                &Counters::default(),
                Duration::ZERO,
                id(),
                request(
                    Method::AdminPeersRemoveAddress,
                    &serde_json::json!({"address": "a"}),
                ),
            )
            .await,
        );
        assert!(answer.contains(r#""ok":true"#), "{answer}");
        assert!(
            answer.contains(r#""outcome":"refused","refused":"authored""#),
            "{answer}"
        );
    }

    /// `admin.peers.list` answers the port's rows one page at a time, as
    /// the trust list does: one past a page is two requests, the second
    /// named by the first's `next`.
    #[tokio::test]
    async fn the_peer_list_is_paged_through_its_cursor() {
        use interweave_ipc_protocol::{MAX_PEER_PAGE_ROWS, PeerList};
        let fake = Fake::default();
        fake.script().peers = (0..=MAX_PEER_PAGE_ROWS)
            .map(|i| {
                let tail = format!("{i:044}").replace('0', "a");
                interweave_local_client_api::PeerGateView {
                    peer: interweave_transport_api::TransportIdentity::parse(format!(
                        "Qm{}",
                        &tail[..44]
                    ))
                    .expect("peer"),
                    connected: false,
                    last_seen: interweave_local_client_api::PeerLastSeen::At(5),
                    backoff_until: Some(1),
                    quarantined_until: None,
                    route_held_until: Some(7),
                    last_outcome: Some(interweave_local_client_api::PeerOutcome::Denied),
                }
            })
            .collect();
        let port = fake.admin([].into()).await.expect("port");
        let counters = Counters::default();
        let grace = Duration::from_secs(1);
        let (port, counters) = (&port, &counters);
        let page = move |params: serde_json::Value| {
            let request = request(Method::AdminPeersList, &params);
            async move {
                let frame = admin(port, counters, grace, id(), request).await;
                frame.outcome::<PeerList>().expect("a page")
            }
        };
        // The page bound of a row that carries `last_seen_ms`, which every
        // 3.0 row does.
        let per_page = interweave_ipc_protocol::MAX_PEER_PAGE_ROWS_SINCE_LAST_SEEN;
        let first = page(serde_json::json!({})).await;
        assert_eq!(first.peers.len(), per_page);
        let next = first.next.expect("more remain");
        let second = page(serde_json::json!({"after": next.as_str()})).await;
        assert_eq!(second.peers.len(), MAX_PEER_PAGE_ROWS + 1 - per_page);
        assert_eq!(second.next, None, "the last page");
        // The last-seen time and the route hold, both served.
        let row = &page(serde_json::json!({})).await.peers[0];
        assert_eq!(row.last_seen_ms, Some(Some(5)));
        assert_eq!(row.route_held_until, Some(7));
    }

    /// A trust set answers `trust-set-result`, a superseded set the
    /// refusal by name -- never `ok` with nothing said, never `Internal` --
    /// and so does a superseded address write; the applied set is the
    /// control.
    #[tokio::test]
    async fn superseded_is_named() {
        use interweave_ipc_protocol::{AddressResult, TrustSetResult};
        use interweave_local_client_api::{AddressRefusal, TrustChange, TrustRefusal};
        let set = |refused: Option<TrustRefusal>| async move {
            let fake = Fake::default();
            fake.script().trust_refused = refused;
            let port = fake.admin([].into()).await.expect("port");
            admin(
                &port,
                &Counters::default(),
                Duration::from_secs(1),
                id(),
                request(
                    Method::AdminTrustSet,
                    &serde_json::json!({"peer": PEER, "allowed": true}),
                ),
            )
            .await
        };
        let at = |frame: ResponseFrame| frame.outcome::<TrustSetResult>().map(TrustChange::from);
        assert_eq!(at(set(None).await), Ok(TrustChange::Set));
        assert_eq!(
            at(set(Some(TrustRefusal::Superseded)).await),
            Ok(TrustChange::Refused(TrustRefusal::Superseded))
        );

        let fake = Fake::default();
        fake.script().add_refused = Some(AddressRefusal::Superseded);
        let port = fake.admin([].into()).await.expect("port");
        let frame = admin(
            &port,
            &Counters::default(),
            Duration::from_secs(1),
            id(),
            request(
                Method::AdminPeersAddAddress,
                &serde_json::json!({"address": "/ip4/10.0.0.1/tcp/1/p2p/x"}),
            ),
        )
        .await;
        let reason =
            frame
                .outcome::<AddressResult>()
                .map(|result| match AddressChange::from(result) {
                    AddressChange::Refused { reason, .. } => reason,
                    other => panic!("{other:?}"),
                });
        assert_eq!(reason, Ok(AddressRefusal::Superseded));
    }

    /// `admin.status` carries the port's refusal total (the fake counts
    /// 9): the 2.13 block, the one 3.0 serves.
    #[tokio::test]
    async fn the_refused_total_is_served() {
        let fake = Fake::default();
        let port = fake.admin([].into()).await.expect("port");
        let frame = admin(
            &port,
            &Counters::default(),
            Duration::from_secs(1),
            id(),
            Request::AdminStatus,
        )
        .await;
        let wire = body(frame.clone());
        let status = frame.outcome::<AdminStatusResult>().expect("a status");
        assert_eq!(status.ipc.events_refused_total, Some(9), "{wire}");
    }

    /// `own_peer` is answered by its own name, the 2.9 shape 3.0 serves,
    /// never folded into `grammar`; another refusal is the control.
    #[tokio::test]
    async fn own_peer_is_answered_by_name() {
        use interweave_ipc_protocol::AddressResult;
        use interweave_local_client_api::{AddressChange, AddressRefusal};
        for scripted in [AddressRefusal::OwnPeer, AddressRefusal::SpecialUse] {
            let fake = Fake::default();
            fake.script().add_refused = Some(scripted);
            let port = fake.admin([].into()).await.expect("port");
            let frame = admin(
                &port,
                &Counters::default(),
                Duration::from_secs(1),
                id(),
                request(
                    Method::AdminPeersAddAddress,
                    &serde_json::json!({"address": "/ip4/10.0.0.1/tcp/1/p2p/x"}),
                ),
            )
            .await;
            let change = AddressChange::from(frame.outcome::<AddressResult>().expect("a result"));
            assert!(
                matches!(change, AddressChange::Refused { reason, .. } if reason == scripted),
                "{scripted:?}: {change:?}"
            );
        }
    }

    /// A runtime whose trust rows do not persist is refused `Internal`:
    /// the row its view would make (2.1's) is one the contract forbids on
    /// 3.0. Persisted rows are the control.
    #[tokio::test]
    async fn an_unpersisted_trust_row_is_refused() {
        use interweave_ipc_protocol::TrustList;
        for (unpersisted, served) in [(true, false), (false, true)] {
            let fake = Fake::default();
            fake.script().trusted = vec![
                interweave_transport_api::TransportIdentity::parse(
                    "QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N",
                )
                .expect("peer"),
            ];
            fake.script().unpersisted = unpersisted;
            let port = fake.admin([].into()).await.expect("port");
            let frame = admin(
                &port,
                &Counters::default(),
                Duration::from_secs(1),
                id(),
                request(Method::AdminTrustList, &serde_json::json!({})),
            )
            .await;
            let case = format!("unpersisted={unpersisted}");
            match frame.outcome::<TrustList>() {
                Ok(page) => {
                    assert!(served, "{case}: served");
                    assert_eq!(page.allowed.len(), 1, "{case}");
                    assert!(page.allowed[0].persisted, "{case}");
                }
                Err(code) => {
                    assert!(!served, "{case}: refused {code:?}");
                    assert_eq!(code, TransportError::Internal, "{case}");
                }
            }
        }
    }

    /// A runtime whose endpoint rows do not persist is refused `Internal`:
    /// the 2.0 row is one the contract forbids on 3.0. A persisting
    /// runtime is served the 2.5 row, and an empty list is served: no row
    /// claims anything.
    #[tokio::test]
    async fn an_unpersisted_endpoint_row_is_refused() {
        use interweave_ipc_protocol::EndpointList;
        use interweave_local_client_api::EndpointAdminView;
        for (rows, persisted, served) in [(1, false, false), (1, true, true), (0, false, true)] {
            let fake = Fake::default();
            fake.script().endpoints = (0..rows)
                .map(|_| EndpointAdminView {
                    endpoint: crate::fake::ep("human"),
                    enabled: true,
                    default: true,
                    persisted,
                    alias: None,
                    source: interweave_local_client_api::EndpointSource::Authored,
                    advertise: false,
                    lease: None,
                })
                .collect();
            let port = fake.admin([].into()).await.expect("port");
            let frame = admin(
                &port,
                &Counters::default(),
                Duration::from_secs(1),
                id(),
                request(Method::AdminEndpointsList, &serde_json::json!({})),
            )
            .await;
            let case = format!("rows={rows} persisted={persisted}");
            match frame.outcome::<EndpointList>() {
                Ok(list) => {
                    assert!(served, "{case}: served");
                    assert_eq!(list.endpoints.len(), rows, "{case}");
                    if let Some(row) = list.endpoints.first() {
                        assert!(row.persisted, "{case}");
                    }
                }
                Err(code) => {
                    assert!(!served, "{case}: refused {code:?}");
                    assert_eq!(code, TransportError::Internal, "{case}");
                }
            }
        }
    }

    /// `admin.trust.list` answers the port's policy one page at a time:
    /// an allowlist one past a page is two requests, the second named by
    /// the first's `next`, and the local peer rides the first only.
    #[tokio::test]
    async fn the_trust_list_is_paged_through_its_cursor() {
        use interweave_ipc_protocol::{MAX_TRUST_PAGE_ROWS, TrustList};
        let fake = Fake::default();
        fake.script().trusted = (0..=MAX_TRUST_PAGE_ROWS)
            .map(|i| {
                let tail = format!("{i:044}").replace('0', "a");
                interweave_transport_api::TransportIdentity::parse(format!("Qm{}", &tail[..44]))
                    .expect("peer")
            })
            .collect();
        let port = fake.admin([].into()).await.expect("port");
        let counters = Counters::default();
        let grace = Duration::from_secs(1);
        let page = |params: serde_json::Value| {
            let request = request(Method::AdminTrustList, &params);
            async {
                let frame = admin(&port, &counters, grace, id(), request).await;
                frame.outcome::<TrustList>().expect("a page")
            }
        };
        let first = page(serde_json::json!({})).await;
        assert_eq!(first.allowed.len(), MAX_TRUST_PAGE_ROWS);
        assert_eq!(first.local_peer, Some(crate::fake::peer()));
        let next = first.next.expect("one more remains");
        let second = page(serde_json::json!({"after": next.as_str()})).await;
        assert_eq!(second.allowed.len(), 1);
        assert_eq!(second.next, None, "the last page");
        assert_eq!(second.local_peer, None);
    }
}
