// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The facade's address methods (ADR-0062) over a scripted admin port:
//! what each runtime answer becomes for the view, and which authority a
//! call holds. Isolation only: the real method's rules are the runtime's
//! and are proved beside it (`crates/transport/composition/tests/
//! bootstrap_overlay.rs`), not here.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::BTreeSet;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use interweave_human_store::{HumanStore, StoreOptions};
use interweave_human_transport_client::{
    AddressAnswer, AddressChangeFailure, AddressList, AddressOrigin, AddressOutcome,
    AddressProblem, AddressRefused, ClientConfig, LineAdded, PeerAddress, ShowAddresses,
    TransportClient,
};
use interweave_local_client_api::{
    AddressChange, AddressRefusal, AddressSource, AddressesChange, AdminBinding, AdminCapability,
    AdminPort, AdminStatus, BootstrapAddress, EndpointAdminView, Generation, LineChange,
    LocalAdminPort, OwnAddress, OwnAddressKind, OwnAddresses, TrustAdminView,
};
use interweave_local_client_fake::{FakeConfig, FakeNetwork, FakeNode};
use interweave_profile_identity::ProfileIdentity;
use interweave_transport_api::{EndpointId, TransportError, TransportIdentity};

fn peer() -> TransportIdentity {
    ProfileIdentity::generate()
        .transport_identity()
        .expect("peer id")
}

/// What the scripted runtime holds and will answer next.
#[derive(Default)]
struct Script {
    rows: Vec<BootstrapAddress>,
    /// The next change's answer; `None` adds or removes as asked.
    change: Option<Result<AddressChange, TransportError>>,
    /// The next list read fails with this.
    list_fails: Option<TransportError>,
    /// The binding refuses to open a port with this.
    open_fails: Option<TransportError>,
    /// Every capability set a port was opened with.
    asked: Vec<BTreeSet<AdminCapability>>,
    /// Every address a change was asked for.
    sent: Vec<String>,
    /// The next add of several's answer; `None` answers each line as
    /// a one-line add would, all or none.
    adds: Option<Result<AddressesChange, TransportError>>,
    /// Every list an add of several was asked for.
    sent_lists: Vec<Vec<String>>,
    /// The own-address read's answer; `None` is unsupported.
    own: Option<Result<OwnAddresses, TransportError>>,
}

#[derive(Clone, Default)]
struct Book(Arc<Mutex<Script>>);

struct BookPort {
    book: Book,
    port: LocalAdminPort,
}

impl AdminBinding for Book {
    type Admin = BookPort;

    fn admin(
        &self,
        capabilities: BTreeSet<AdminCapability>,
    ) -> impl Future<Output = Result<BookPort, TransportError>> + Send {
        std::future::ready(self.open(capabilities))
    }
}

impl Book {
    fn open(&self, capabilities: BTreeSet<AdminCapability>) -> Result<BookPort, TransportError> {
        let mut script = self.0.lock().unwrap();
        script.asked.push(capabilities.clone());
        if let Some(error) = script.open_fails {
            return Err(error);
        }
        Ok(BookPort {
            book: self.clone(),
            port: LocalAdminPort::new(
                Generation::parse("book-admin-port-0").unwrap(),
                capabilities,
            ),
        })
    }
}

impl BookPort {
    fn list(&self) -> Result<Vec<BootstrapAddress>, TransportError> {
        let mut script = self.book.0.lock().unwrap();
        match script.list_fails.take() {
            Some(error) => Err(error),
            None => Ok(script.rows.clone()),
        }
    }

    fn change(&self, address: String, add: bool) -> Result<AddressChange, TransportError> {
        let mut script = self.book.0.lock().unwrap();
        script.sent.push(address.clone());
        if let Some(answer) = script.change.take() {
            return answer;
        }
        let at = script.rows.iter().position(|row| row.address == address);
        Ok(match (add, at) {
            (true, Some(_)) | (false, None) => AddressChange::Unchanged,
            (true, None) => {
                script.rows.push(BootstrapAddress {
                    address,
                    source: AddressSource::Added,
                    dialled: true,
                });
                AddressChange::Added
            }
            (false, Some(at)) => {
                script.rows.remove(at);
                AddressChange::Removed
            }
        })
    }
}

impl BookPort {
    fn add_lines(&self, lines: Vec<String>) -> Result<AddressesChange, TransportError> {
        let mut script = self.book.0.lock().unwrap();
        script.sent_lists.push(lines.clone());
        if let Some(answer) = script.adds.take() {
            return answer;
        }
        let mut rows = Vec::new();
        for line in lines {
            if script.rows.iter().any(|row| row.address == line) {
                rows.push(LineChange::Unchanged);
            } else {
                script.rows.push(BootstrapAddress {
                    address: line,
                    source: AddressSource::Added,
                    dialled: true,
                });
                rows.push(LineChange::Added);
            }
        }
        Ok(AddressesChange::Added(rows))
    }

    fn own(&self) -> Result<OwnAddresses, TransportError> {
        self.book
            .0
            .lock()
            .unwrap()
            .own
            .clone()
            .unwrap_or(Err(TransportError::ProtocolUnsupported))
    }
}

impl AdminPort for BookPort {
    fn port(&self) -> &LocalAdminPort {
        &self.port
    }
    fn status(&self) -> impl Future<Output = Result<AdminStatus, TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn leases(
        &self,
    ) -> impl Future<Output = Result<Vec<EndpointAdminView>, TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn revoke_endpoint(
        &self,
        _: EndpointId,
    ) -> impl Future<Output = Result<(), TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn set_endpoint_enabled(
        &self,
        _: EndpointId,
        _: bool,
    ) -> impl Future<Output = Result<Option<Generation>, TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn set_default_endpoint(
        &self,
        _: Option<EndpointId>,
    ) -> impl Future<Output = Result<(), TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn trust(&self) -> impl Future<Output = Result<TrustAdminView, TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn add_bootstrap_address(
        &self,
        address: String,
    ) -> impl Future<Output = Result<AddressChange, TransportError>> + Send {
        std::future::ready(self.change(address, true))
    }
    fn add_bootstrap_addresses(
        &self,
        addresses: Vec<String>,
    ) -> impl Future<Output = Result<AddressesChange, TransportError>> + Send {
        std::future::ready(self.add_lines(addresses))
    }
    fn own_addresses(&self) -> impl Future<Output = Result<OwnAddresses, TransportError>> + Send {
        std::future::ready(self.own())
    }
    fn remove_bootstrap_address(
        &self,
        address: String,
    ) -> impl Future<Output = Result<AddressChange, TransportError>> + Send {
        std::future::ready(self.change(address, false))
    }
    fn bootstrap_addresses(
        &self,
    ) -> impl Future<Output = Result<Vec<BootstrapAddress>, TransportError>> + Send {
        std::future::ready(self.list())
    }
    fn set_trust(
        &self,
        _: TransportIdentity,
        _: bool,
    ) -> impl Future<Output = Result<(), TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
    fn shutdown(&self, _: Duration) -> impl Future<Output = Result<(), TransportError>> + Send {
        std::future::ready(Err(TransportError::ProtocolUnsupported))
    }
}

fn node() -> FakeNode {
    let config = || FakeConfig {
        peer: peer(),
        endpoints: Vec::new(),
        default_endpoint: None,
        queue_bound: 16,
    };
    FakeNetwork::pair(config(), config()).0
}

fn facade<A: AdminBinding>(admin: A) -> TransportClient<FakeNode, A> {
    TransportClient::new(
        node(),
        admin,
        HumanStore::open_in_memory(StoreOptions::default()).expect("store"),
        ClientConfig {
            client_kind: "human-client".to_owned(),
            endpoints: Vec::new(),
            channels: Vec::new(),
            max_payload_bytes: 49_152,
        },
        Box::new(|| 1_786_600_000_000),
        0,
    )
    .expect("an empty store")
}

fn direct(p: &TransportIdentity) -> String {
    format!("/ip4/192.168.1.20/tcp/4001/p2p/{}", p.as_str())
}

/// The list as the runtime holds it, each row's origin and whether it is
/// dialled kept, its peer read from the address -- a circuit route's
/// destination, never its relay -- over a port holding `admin.peers`
/// alone.
#[tokio::test]
async fn the_list_is_read_in_the_runtimes_order_with_its_peers() {
    let (configured, relay, far) = (peer(), peer(), peer());
    let circuit = format!(
        "/ip4/203.0.113.7/tcp/4001/p2p/{}/p2p-circuit/p2p/{}",
        relay.as_str(),
        far.as_str()
    );
    let book = Book::default();
    book.0.lock().unwrap().rows = vec![
        BootstrapAddress {
            address: direct(&configured),
            source: AddressSource::Authored,
            dialled: false,
        },
        BootstrapAddress {
            address: circuit.clone(),
            source: AddressSource::Added,
            dialled: true,
        },
        BootstrapAddress {
            address: "/ip4/192.168.1.20/tcp/4001/p2p/not-a-peer".to_owned(),
            source: AddressSource::Added,
            dialled: true,
        },
    ];
    let client = facade(book.clone());

    let list = client.addresses().await.expect("the list");

    assert_eq!(
        list.addresses,
        vec![
            PeerAddress {
                address: direct(&configured),
                peer: Some(configured),
                origin: AddressOrigin::Configured,
                dialled: false,
            },
            PeerAddress {
                address: circuit,
                peer: Some(far),
                origin: AddressOrigin::AddedHere,
                dialled: true,
            },
            PeerAddress {
                address: "/ip4/192.168.1.20/tcp/4001/p2p/not-a-peer".to_owned(),
                peer: None,
                origin: AddressOrigin::AddedHere,
                dialled: true,
            },
        ]
    );
    assert_eq!(
        book.0.lock().unwrap().asked,
        vec![BTreeSet::from([AdminCapability::Peers])]
    );
}

/// An add is sent as given, answers the peer it reaches and the list read
/// back; a second add of it is unchanged, and a removal removes it.
#[tokio::test]
async fn an_add_answers_its_peer_and_the_list_after_it() {
    let p = peer();
    let book = Book::default();
    let client = facade(book.clone());

    let added = client.add_address(direct(&p)).await.expect("an answer");
    let row = PeerAddress {
        address: direct(&p),
        peer: Some(p.clone()),
        origin: AddressOrigin::AddedHere,
        dialled: true,
    };
    assert_eq!(
        added,
        AddressAnswer {
            outcome: AddressOutcome::Added {
                peer: Some(p.clone())
            },
            list: Ok(AddressList {
                addresses: vec![row.clone()]
            }),
        }
    );

    let again = client.add_address(direct(&p)).await.expect("an answer");
    assert_eq!(again.outcome, AddressOutcome::Unchanged);
    assert_eq!(
        again.list,
        Ok(AddressList {
            addresses: vec![row]
        })
    );

    let removed = client.remove_address(direct(&p)).await.expect("an answer");
    assert_eq!(removed.outcome, AddressOutcome::Removed);
    assert_eq!(removed.list, Ok(AddressList::default()));
    assert_eq!(book.0.lock().unwrap().sent, vec![direct(&p); 3]);
}

/// Every refusal the method names is an answer naming its rule, with the
/// list read back unchanged -- never an error.
#[tokio::test]
async fn each_refusal_is_answered_by_its_rule() {
    let cases = [
        (AddressRefusal::Grammar, AddressRefused::Grammar),
        (AddressRefusal::SpecialUse, AddressRefused::SpecialUse),
        (AddressRefusal::Bound, AddressRefused::Bound),
        (AddressRefusal::ProviderDisabled, AddressRefused::ListOff),
        (AddressRefusal::Authored, AddressRefused::Configured),
        (AddressRefusal::OwnPeer, AddressRefused::OwnPeer),
    ];
    for (reason, expected) in cases {
        let book = Book::default();
        book.0.lock().unwrap().change = Some(Ok(AddressChange::Refused {
            reason,
            detail: "for a log".to_owned(),
        }));
        let answer = facade(book)
            .add_address("/ip4/127.0.0.1/tcp/1/p2p/x".to_owned())
            .await
            .expect("a refusal is an answer");
        assert_eq!(
            answer,
            AddressAnswer {
                outcome: AddressOutcome::Refused(expected),
                list: Ok(AddressList::default()),
            },
            "{reason:?}"
        );
    }
}

/// Whether a failed change was made is said truly: a port that never
/// opened, a method the runtime does not have, or the method's failed
/// write of the list made nothing; a failure
/// after the request may have left says so; a list that cannot be read
/// back after an answered change keeps what the change did.
#[tokio::test]
async fn a_failure_says_whether_the_change_was_made() {
    let p = peer();

    let refused = Book::default();
    refused.0.lock().unwrap().open_fails = Some(TransportError::CapabilityDenied);
    assert_eq!(
        facade(refused).add_address(direct(&p)).await,
        Err(AddressChangeFailure::NotMade(AddressProblem::NotPermitted))
    );

    let old = Book::default();
    old.0.lock().unwrap().change = Some(Err(TransportError::ProtocolUnsupported));
    assert_eq!(
        facade(old).add_address(direct(&p)).await,
        Err(AddressChangeFailure::NotMade(AddressProblem::Incompatible))
    );

    let unwritten = Book::default();
    unwritten.0.lock().unwrap().change = Some(Err(TransportError::Internal));
    assert_eq!(
        facade(unwritten).add_address(direct(&p)).await,
        Err(AddressChangeFailure::NotMade(AddressProblem::Internal)),
        "the method's failed write changed nothing"
    );

    let lost = Book::default();
    lost.0.lock().unwrap().change = Some(Err(TransportError::Timeout));
    assert_eq!(
        facade(lost).remove_address(direct(&p)).await,
        Err(AddressChangeFailure::Unconfirmed(
            AddressProblem::Unavailable
        ))
    );

    let unread = Book::default();
    unread.0.lock().unwrap().list_fails = Some(TransportError::BackendUnavailable);
    let client = facade(unread.clone());
    assert_eq!(
        client.add_address(direct(&p)).await,
        Ok(AddressAnswer {
            outcome: AddressOutcome::Added { peer: Some(p) },
            list: Err(AddressProblem::Unavailable),
        })
    );
    assert_eq!(unread.0.lock().unwrap().rows.len(), 1, "the add was made");
}

/// A runtime whose port predates the methods answers them unsupported,
/// which the person is told as a mixed install. (The shared fake node
/// answers them since IPC 2.9, so the predating port is this book's.)
#[tokio::test]
async fn a_runtime_without_the_methods_is_incompatible() {
    let old = Book::default();
    old.0.lock().unwrap().list_fails = Some(TransportError::ProtocolUnsupported);
    let client = facade(old);
    assert_eq!(client.addresses().await, Err(AddressProblem::Incompatible));
}

/// One line is the one-address add any runtime with the methods serves;
/// several are one call carrying them all, never one call per line, and
/// only the lines the runtime answered added are marked so.
#[tokio::test]
async fn an_add_of_several_is_one_call_and_one_line_is_the_one_address_add() {
    let (old, a, b) = (peer(), peer(), peer());
    let book = Book::default();
    book.0.lock().unwrap().rows = vec![BootstrapAddress {
        address: direct(&old),
        source: AddressSource::Added,
        dialled: true,
    }];
    let client = facade(book.clone());

    let one = client.add_addresses(vec![direct(&a)]).await.expect("added");
    assert_eq!(
        one.outcome,
        AddressOutcome::Added {
            peer: Some(a.clone())
        }
    );
    let lines = vec![direct(&b), direct(&old)];
    let several = client.add_addresses(lines.clone()).await.expect("added");
    assert_eq!(
        several.outcome,
        AddressOutcome::AddedLines(vec![
            LineAdded {
                added: true,
                peer: Some(b.clone())
            },
            LineAdded {
                added: false,
                peer: Some(old.clone())
            },
        ])
    );
    let again = client.add_addresses(lines.clone()).await.expect("answered");
    assert_eq!(again.outcome, AddressOutcome::Unchanged, "every line there");
    let script = book.0.lock().unwrap();
    assert_eq!(
        script.sent,
        vec![direct(&a)],
        "one line, the one-address add"
    );
    assert_eq!(
        script.sent_lists,
        vec![lines.clone(), lines],
        "several, one call each"
    );
    assert_eq!(
        several.list.expect("read back").addresses.len(),
        3,
        "the list is read back"
    );
}

/// The add of several's refusal keeps the line it names, and the list
/// off names none; a runtime below 2.9 sends nothing and the add is not
/// made, while an answer lost after the request left may have been.
#[tokio::test]
async fn an_add_of_several_refused_or_failed_says_which_and_whether_made() {
    let lines = vec![direct(&peer()), direct(&peer())];
    let cases = [
        (
            Ok(AddressesChange::Refused {
                reason: AddressRefusal::Bound,
                line: Some(1),
                detail: "x".to_owned(),
            }),
            Ok(AddressOutcome::RefusedLine {
                why: AddressRefused::Bound,
                line: 1,
            }),
        ),
        (
            Ok(AddressesChange::Refused {
                reason: AddressRefusal::ProviderDisabled,
                line: None,
                detail: "x".to_owned(),
            }),
            Ok(AddressOutcome::Refused(AddressRefused::ListOff)),
        ),
        (
            Err(TransportError::ProtocolUnsupported),
            Err(AddressChangeFailure::NotMade(AddressProblem::Incompatible)),
        ),
        (
            Err(TransportError::Internal),
            Err(AddressChangeFailure::NotMade(AddressProblem::Internal)),
        ),
        (
            Err(TransportError::Timeout),
            Err(AddressChangeFailure::Unconfirmed(
                AddressProblem::Unavailable,
            )),
        ),
    ];
    for (given, want) in cases {
        let book = Book::default();
        book.0.lock().unwrap().adds = Some(given.clone());
        let client = facade(book);
        let got = client.add_addresses(lines.clone()).await.map(|a| a.outcome);
        assert_eq!(got, want, "for {given:?}");
    }
}

/// The show screen's read keeps the runtime's order and what it says of
/// its bound, and says why it failed.
#[tokio::test]
async fn own_addresses_are_read_in_the_runtimes_order() {
    let me = peer();
    let shown = vec![
        format!("/ip4/203.0.113.9/tcp/4001/p2p/{}", me.as_str()),
        direct(&me),
    ];
    let book = Book::default();
    book.0.lock().unwrap().own = Some(Ok(OwnAddresses {
        peer: me.clone(),
        addresses: vec![
            OwnAddress {
                address: shown[0].clone(),
                kind: OwnAddressKind::ConfirmedExternal,
            },
            OwnAddress {
                address: shown[1].clone(),
                kind: OwnAddressKind::Listen,
            },
        ],
        truncated: true,
    }));
    let client = facade(book.clone());
    assert_eq!(
        client.own_addresses().await,
        Ok(ShowAddresses {
            peer: me,
            addresses: shown,
            truncated: true
        })
    );
    assert_eq!(
        book.0.lock().unwrap().asked.last(),
        Some(&BTreeSet::from([AdminCapability::Peers])),
        "admin.peers alone"
    );
    for (given, want) in [
        (
            TransportError::ProtocolUnsupported,
            AddressProblem::Incompatible,
        ),
        (
            TransportError::CapabilityDenied,
            AddressProblem::NotPermitted,
        ),
        (
            TransportError::BackendUnavailable,
            AddressProblem::Unavailable,
        ),
    ] {
        book.0.lock().unwrap().own = Some(Err(given));
        assert_eq!(client.own_addresses().await, Err(want), "for {given:?}");
    }
}
