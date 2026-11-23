// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! These types agree with the frozen IPC schemas and fixtures.

// Helpers read files and index JSON outside `#[test]` functions, where
// clippy.toml's allow-*-in-tests does not reach.
#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use interweave_ipc_protocol::{
    AddressList, AddressParams, AddressResult, AddressesParams, AddressesResult, AdminStatusResult,
    AuthorityDomain, Cancel, ChannelParams, ClientInfo, Close, DirectoryResult, EmptyResult,
    EndpointList, EndpointParams, Event, EventType, Frame, GrantedLease, HandshakeOutcome, Hello,
    HelloResponse, HelloTag, IPC_MAJOR, IpcVersion, MAX_BODY_BYTES, MAX_REQUESTED, Method, Nonce,
    OwnAddressList, PeerList, PeerListParams, Ping, PublishParams, QueryParams, Request, RequestId,
    RequestedCapability, ResponseFrame, SendParams, SendResult, ServerCounters, ServerState,
    SessionLossesResult, SetDefaultParams, SetEnabledParams, SetEnabledResult, ShutdownParams,
    Since, TrustList, TrustListParams, TrustSeedParams, TrustSeedResult, TrustSetParams,
    TrustSetResult, UnsupportedMajor, encode_frame, supported,
};
use interweave_local_client_api::{
    AddressChange, AddressRefusal, AddressSource, AddressesChange, AdminCapability, AdminStatus,
    BootstrapAddress, DataCapability, EndpointAdminView, Generation, LeaseRecord, LineChange,
    LocalSessionEvent, OwnAddress, OwnAddressKind, OwnAddresses, PeerGateView, PeerOutcome,
    ReceivedBroadcast, ReceivedDirect, SessionEvent, TrustAdminView, TrustDirection,
};
use interweave_transport_api::{
    ChannelId, ConnectivitySummary, DirectInboundState, EndpointDirectoryV1, EndpointId, Health,
    MediaType, MessageId, PathReadiness, Payload, PreferredPathPolicy, TransportError,
    TransportIdentity,
};
use serde_json::Value;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("crates/api/<crate> is three levels below the root")
        .to_path_buf()
}

fn json_at(relative: &str) -> serde_json::Value {
    let path = root().join(relative);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()))
}

fn schema(name: &str) -> serde_json::Value {
    json_at(&format!("architecture/contracts/schemas/{name}"))
}

fn string_set(value: &serde_json::Value) -> BTreeSet<String> {
    value
        .as_array()
        .expect("array")
        .iter()
        .map(|v| v.as_str().expect("string").to_owned())
        .collect()
}

/// The serialized name of every variant `T`'s serde derive knows, read
/// from the derive itself: `Deserialize` hands its full variant list to
/// `deserialize_enum`, and this deserializer keeps it and fails. A list
/// typed into a test escapes a new variant (#143's supply review); this
/// one cannot, because the compiler writes it.
fn variants<T: serde::de::DeserializeOwned>() -> BTreeSet<String> {
    use serde::de::{Error as _, Visitor};

    struct Probe<'a>(&'a mut Option<&'static [&'static str]>);

    impl<'de> serde::Deserializer<'de> for Probe<'_> {
        type Error = serde::de::value::Error;

        fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
            Err(Self::Error::custom("not an enum"))
        }

        fn deserialize_enum<V: Visitor<'de>>(
            self,
            _: &'static str,
            variants: &'static [&'static str],
            _: V,
        ) -> Result<V::Value, Self::Error> {
            *self.0 = Some(variants);
            Err(Self::Error::custom("probed"))
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct identifier ignored_any
        }
    }

    let mut seen = None;
    let _ = T::deserialize(Probe(&mut seen));
    seen.expect("T derives Deserialize for an enum")
        .iter()
        .map(|&name| name.to_owned())
        .collect()
}

fn names<T: serde::Serialize>(items: impl IntoIterator<Item = T>) -> BTreeSet<String> {
    items
        .into_iter()
        .map(|item| {
            serde_json::to_value(item)
                .expect("ser")
                .as_str()
                .expect("a unit variant serializes as a string")
                .to_owned()
        })
        .collect()
}

#[test]
fn the_requested_capability_vocabulary_matches_the_schema() {
    // `hello.requested_capabilities` references the FULL capability enum,
    // admin included: a client may ask for anything, and refusing is the
    // server's job. That is why this type is separate from the granted
    // DataCapability set rather than an alias for it.
    let declared = string_set(&schema("ipc/capability.schema.json")["enum"]);
    assert_eq!(variants::<RequestedCapability>(), declared);
}

#[test]
fn the_error_vocabulary_matches_the_schema() {
    let declared = string_set(&schema("ipc/error-code.schema.json")["enum"]);
    assert_eq!(variants::<TransportError>(), declared);
}

#[test]
fn the_method_catalogue_matches_the_schema_both_ways() {
    let declared = string_set(&schema("ipc/method.schema.json")["enum"]);
    assert_eq!(variants::<Method>(), declared, "the enum and the schema");
    assert_eq!(names(Method::ALL), declared, "Method::ALL is every variant");
    for method in Method::ALL {
        assert_eq!(
            names([method]),
            BTreeSet::from([method.as_str().to_owned()])
        );
    }
    // `ipc/request` binds exactly the same names, one alternative each.
    let bound: Vec<String> = schema("ipc/request.schema.json")["oneOf"]
        .as_array()
        .expect("oneOf")
        .iter()
        .map(|alt| {
            alt["properties"]["method"]["const"]
                .as_str()
                .expect("const")
                .to_owned()
        })
        .collect();
    assert_eq!(bound.len(), declared.len(), "one alternative per method");
    assert_eq!(bound.into_iter().collect::<BTreeSet<_>>(), declared);
    // And the frame's request class names the catalogue, not a copy of it.
    assert_eq!(
        schema("ipc/frame.schema.json")["$defs"]["request"]["properties"]["method"]["$ref"],
        serde_json::json!("urn:interweave:schemas:ipc:method")
    );
}

#[test]
fn the_event_catalogue_matches_the_schema_both_ways() {
    let bound: Vec<String> = schema("ipc/event.schema.json")["oneOf"]
        .as_array()
        .expect("oneOf")
        .iter()
        .map(|alt| {
            alt["properties"]["event_type"]["const"]
                .as_str()
                .expect("const")
                .to_owned()
        })
        .collect();
    let declared: BTreeSet<String> = bound.iter().cloned().collect();
    assert_eq!(bound.len(), declared.len(), "one alternative per type");
    assert_eq!(variants::<EventType>(), declared, "the enum and the schema");
    assert_eq!(
        names(EventType::ALL),
        declared,
        "EventType::ALL is every variant"
    );
    for kind in EventType::ALL {
        assert_eq!(EventType::parse(kind.as_str()), Some(kind));
    }
    // The envelope's own list of event types is the same set.
    assert_eq!(
        string_set(
            &schema("ipc/frame.schema.json")["$defs"]["event"]["properties"]["event_type"]["enum"]
        ),
        declared
    );
}

/// The prose table's rows under `heading`, each split into its cells.
fn table_rows(heading: &str) -> Vec<Vec<String>> {
    let text = std::fs::read_to_string(root().join("architecture/contracts/LOCAL-IPC.md"))
        .expect("LOCAL-IPC.md");
    let section = text
        .split(&format!("\n## {heading}\n"))
        .nth(1)
        .unwrap_or_else(|| panic!("no section {heading}"));
    let section = section.split("\n## ").next().expect("section body");
    section
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| {
            line.trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_owned())
                .collect()
        })
        .collect()
}

/// The first `code` span of a cell, backticks removed.
fn first_code(cell: &str) -> &str {
    cell.split('`').nth(1).unwrap_or(cell)
}

#[test]
fn the_method_table_in_the_prose_is_the_rust_table() {
    let rows = table_rows("Method catalogue");
    assert_eq!(rows.len(), Method::ALL.len(), "one prose row per method");
    assert_eq!(
        rows.iter()
            .map(|row| first_code(&row[0]).to_owned())
            .collect::<BTreeSet<_>>(),
        names(Method::ALL),
        "the prose names every method, each once"
    );
    for row in rows {
        let method = Method::parse(first_code(&row[0])).unwrap_or_else(|| panic!("{row:?}"));
        let entry = method.entry();
        let domain = match entry.domain {
            AuthorityDomain::Data => "data",
            AuthorityDomain::Admin => "admin",
        };
        assert_eq!(row[1], domain, "{} domain", method.as_str());
        assert_eq!(
            first_code(&row[2]),
            names([entry.capability]).pop_first().expect("one"),
            "{} capability",
            method.as_str()
        );
        assert_prose_since(&row[5], entry.since, method.as_str());
    }
}

/// A prose `Since` cell against the code's pair (`LOCAL-IPC.md` §Version
/// negotiation and phases): a cell of the major the code speaks names the
/// code's pair exactly; a cell of major 2 is the row's history -- at or
/// below 2.13, where major 2 ended -- and every such row is 3.0 in the
/// code, since 3.0 holds everything 2.13 held.
fn assert_prose_since(cell: &str, since: Since, what: &str) {
    let (major, minor) = cell
        .split_once('.')
        .and_then(|(a, b)| Some((a.parse::<u64>().ok()?, b.parse::<u64>().ok()?)))
        .unwrap_or_else(|| panic!("{what}: since {cell:?} is not <major>.<minor>"));
    if major == IPC_MAJOR {
        assert_eq!(Since { major, minor }, since, "{what} since");
    } else {
        assert!(
            major == 2 && minor <= 13,
            "{what}: {cell} is no pair of major 2's history"
        );
        assert_eq!(since, Since::THREE_ZERO, "{what}: a 2.x row is a 3.0 row");
    }
}

#[test]
fn the_prose_params_column_is_the_request_schemas_binding() {
    let doc = schema("ipc/request.schema.json");
    let alternatives = doc["oneOf"].as_array().expect("oneOf");
    for row in table_rows("Method catalogue") {
        let method = first_code(&row[0]);
        let alt = alternatives
            .iter()
            .find(|alt| alt["properties"]["method"]["const"] == serde_json::json!(method))
            .unwrap_or_else(|| panic!("{method} has no alternative"));
        let bound = alt["properties"]["params"]["$ref"]
            .as_str()
            .expect("params $ref");
        let prose = first_code(&row[3]);
        let expected = if prose == "none" {
            "empty-result"
        } else {
            prose
        };
        assert_eq!(
            bound,
            format!("urn:interweave:schemas:ipc:{expected}"),
            "{method}"
        );
    }
}

#[test]
fn the_prose_event_table_is_the_event_schemas_binding() {
    let rows = table_rows("Event catalogue");
    assert_eq!(rows.len(), EventType::ALL.len(), "one prose row per type");
    assert_eq!(
        rows.iter()
            .map(|row| first_code(&row[0]).to_owned())
            .collect::<BTreeSet<_>>(),
        names(EventType::ALL),
        "the prose names every type, each once"
    );
    let doc = schema("ipc/event.schema.json");
    for row in rows {
        let kind = EventType::parse(first_code(&row[0])).unwrap_or_else(|| panic!("{row:?}"));
        // The type's version leads the cell; a SHAPE added later is named
        // after it, as history -- `peer.path_changed`'s notice with no
        // `previous` (2.4), in 3.0 since 3.0 holds all of 2.13 -- and no
        // other type carries one.
        let (since, later) = row[3]
            .split_once(' ')
            .map_or((row[3].as_str(), None), |(a, b)| (a, Some(b)));
        assert_prose_since(since, kind.since(), kind.as_str());
        if kind == EventType::PathChanged {
            assert!(
                later.is_some_and(|l| l.contains("2.4")),
                "the route notice's history named: {}",
                row[3]
            );
        } else {
            assert_eq!(later, None, "{}: no later shape", kind.as_str());
        }
        let alt = doc["oneOf"]
            .as_array()
            .expect("oneOf")
            .iter()
            .find(|alt| {
                alt["properties"]["event_type"]["const"] == serde_json::json!(kind.as_str())
            })
            .expect("an alternative");
        let data = &alt["properties"]["data"];
        match data["$ref"].as_str() {
            Some(reference) => assert_eq!(
                reference,
                format!("urn:interweave:schemas:{}", first_code(&row[1])),
                "{}",
                kind.as_str()
            ),
            // `peer.disconnected` is the one inline body.
            None => assert_eq!(
                string_set(&data["required"]),
                BTreeSet::from(["peer".to_owned(), "reason_class".to_owned()])
            ),
        }
    }
}

#[test]
fn hello_matches_its_schema_shape() {
    let doc = schema("ipc/hello.schema.json");
    let required = string_set(&doc["required"]);
    for field in ["type", "ipc_version", "client"] {
        assert!(required.contains(field), "{field} should be required");
    }
    // `endpoint` is optional: diagnostics clients omit it, and admin
    // connections must.
    assert!(!required.contains("endpoint"));
    assert_eq!(doc["additionalProperties"], serde_json::json!(false));
    // Any positive major is a well-formed hello (hello 1.1.0): an
    // unsupported one is answered, not rejected as malformed. Which
    // major the server speaks is IPC_MAJOR; `negotiate` answers the rest.
    assert_eq!(
        doc["properties"]["ipc_version"]["properties"]["major"]["minimum"],
        serde_json::json!(1)
    );
    assert_eq!(IPC_MAJOR, 3, "the server speaks IPC major 3");
    assert_eq!(
        doc["properties"]["requested_capabilities"]["maxItems"],
        serde_json::json!(MAX_REQUESTED)
    );
    assert_eq!(
        doc["properties"]["features"]["maxItems"],
        serde_json::json!(MAX_REQUESTED)
    );

    // A minimal schema-shaped hello deserializes into the Rust type.
    let minimal = serde_json::json!({
        "type": "hello",
        "ipc_version": { "major": 3, "minor": 0 },
        "client": { "kind": "human-client" }
    });
    let parsed: Hello = serde_json::from_value(minimal).expect("de");
    assert_eq!(parsed.client.kind, "human-client");
    assert!(parsed.endpoint.is_none());

    // And a Rust-built hello serializes into that shape.
    let built = Hello {
        frame_type: HelloTag::Hello,
        ipc_version: IpcVersion { major: 3, minor: 0 },
        client: ClientInfo {
            kind: "human-client".to_owned(),
            version: None,
        },
        endpoint: None,
        requested_capabilities: BTreeSet::new(),
        features: BTreeSet::new(),
    };
    let json = serde_json::to_value(&built).expect("ser");
    assert_eq!(json["type"], "hello");
    // Empty optional collections are omitted, not emitted empty.
    assert!(json.get("requested_capabilities").is_none());
    assert!(json.get("features").is_none());
}

#[test]
fn the_frame_ceiling_matches_the_contract_and_the_fixture() {
    // The number lives in three places and must be one number: this
    // crate, the prose contract, and the frozen payload-fit vectors.
    assert_eq!(MAX_BODY_BYTES, 131_072);

    let fixture = json_at("fixtures/ipc-v3/ipc-v3-payload-fit.json");
    let vectors = fixture["vectors"].as_array().expect("vectors");
    assert!(!vectors.is_empty(), "the payload-fit fixture is empty");

    for v in vectors {
        let body =
            usize::try_from(v["body_bytes"].as_u64().expect("body_bytes")).expect("fits usize");
        let headroom = usize::try_from(
            v["envelope_headroom_bytes"]
                .as_u64()
                .expect("envelope_headroom_bytes"),
        )
        .expect("fits usize");
        // The invariant the fixture exists to prove.
        assert!(
            body <= MAX_BODY_BYTES,
            "{} exceeds the frame ceiling",
            v["name"]
        );
        assert_eq!(body + headroom, MAX_BODY_BYTES, "{} headroom", v["name"]);

        // And the codec agrees: a body of that size frames successfully,
        // with the prefix the fixture recorded.
        let prefix = v["frame_length_prefix_hex"].as_str().expect("prefix");
        // A real object of exactly that size: `encode_frame` enforces
        // every rule the decoder does, so a body of raw padding is no
        // longer a frame this process will produce.
        let padded = format!(r#"{{"pad":"{}"}}"#, "x".repeat(body - 10));
        assert_eq!(padded.len(), body);
        let framed = encode_frame(&padded).expect("encodes");
        assert_eq!(hex(&framed[..4]), prefix, "{} prefix", v["name"]);
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = std::fmt::Write::write_fmt(&mut s, format_args!("{b:02x}"));
        s
    })
}

#[test]
fn the_authority_domain_is_not_a_frame_field() {
    // The structural claim behind ADR-0037, asserted rather than described:
    // the same bytes yield different authority depending only on the
    // socket they arrived on.
    let json = serde_json::json!({
        "type": "hello",
        "ipc_version": { "major": 3, "minor": 0 },
        "client": { "kind": "admin" },
        "requested_capabilities": ["admin.shutdown"]
    });
    let hello: Hello = serde_json::from_value(json).expect("de");

    assert!(hello.evaluate(AuthorityDomain::Data, false).is_err());
    let granted = hello
        .evaluate(AuthorityDomain::Admin, false)
        .expect("admin socket grants");
    assert!(!granted.granted_admin.is_empty());
}

// ---------------------------------------------------------------------
// Every frame this crate emits validates against the frozen schemas.
// ---------------------------------------------------------------------

/// Every schema of the IPC family, by path: an inventory, which
/// `the_ipc_schema_inventory_is_complete` holds to the directory. It
/// covers nothing by itself -- `check_schemas_are_tested.sh` skips
/// this list and counts only the sites below that read each schema, so a
/// new schema needs a test that reads it, not only a line here.
const IPC_SCHEMAS: [&str; 41] = [
    "architecture/contracts/schemas/ipc/admin-status.schema.json",
    "architecture/contracts/schemas/ipc/broadcast-received.schema.json",
    "architecture/contracts/schemas/ipc/capability.schema.json",
    "architecture/contracts/schemas/ipc/channel-params.schema.json",
    "architecture/contracts/schemas/ipc/close.schema.json",
    "architecture/contracts/schemas/ipc/empty-result.schema.json",
    "architecture/contracts/schemas/ipc/endpoint-list.schema.json",
    "architecture/contracts/schemas/ipc/endpoint-params.schema.json",
    "architecture/contracts/schemas/ipc/error-code.schema.json",
    "architecture/contracts/schemas/ipc/event.schema.json",
    "architecture/contracts/schemas/ipc/frame.schema.json",
    "architecture/contracts/schemas/ipc/hello-response.schema.json",
    "architecture/contracts/schemas/ipc/hello.schema.json",
    "architecture/contracts/schemas/ipc/lease-changed.schema.json",
    "architecture/contracts/schemas/ipc/method.schema.json",
    "architecture/contracts/schemas/ipc/path-changed.schema.json",
    "architecture/contracts/schemas/ipc/payload.schema.json",
    "architecture/contracts/schemas/ipc/peer-list-params.schema.json",
    "architecture/contracts/schemas/ipc/peer-list.schema.json",
    "architecture/contracts/schemas/ipc/peers-address-list.schema.json",
    "architecture/contracts/schemas/ipc/peers-address-params.schema.json",
    "architecture/contracts/schemas/ipc/peers-address-result.schema.json",
    "architecture/contracts/schemas/ipc/peers-addresses-params.schema.json",
    "architecture/contracts/schemas/ipc/peers-addresses-result.schema.json",
    "architecture/contracts/schemas/ipc/peers-own-addresses.schema.json",
    "architecture/contracts/schemas/ipc/publish-params.schema.json",
    "architecture/contracts/schemas/ipc/query-params.schema.json",
    "architecture/contracts/schemas/ipc/request.schema.json",
    "architecture/contracts/schemas/ipc/send-params.schema.json",
    "architecture/contracts/schemas/ipc/send-result.schema.json",
    "architecture/contracts/schemas/ipc/session-losses-result.schema.json",
    "architecture/contracts/schemas/ipc/set-default-params.schema.json",
    "architecture/contracts/schemas/ipc/set-enabled-params.schema.json",
    "architecture/contracts/schemas/ipc/set-enabled-result.schema.json",
    "architecture/contracts/schemas/ipc/shutdown-params.schema.json",
    "architecture/contracts/schemas/ipc/trust-list-params.schema.json",
    "architecture/contracts/schemas/ipc/trust-list.schema.json",
    "architecture/contracts/schemas/ipc/trust-seed-params.schema.json",
    "architecture/contracts/schemas/ipc/trust-seed-result.schema.json",
    "architecture/contracts/schemas/ipc/trust-set-params.schema.json",
    "architecture/contracts/schemas/ipc/trust-set-result.schema.json",
];

fn all_schema_docs(dir: &std::path::Path, out: &mut Vec<Value>) {
    for entry in std::fs::read_dir(dir).expect("schema dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            all_schema_docs(&path, out);
        } else if path.extension().is_some_and(|x| x == "json")
            && path.file_name().is_some_and(|n| n != "manifest.json")
        {
            let text = std::fs::read_to_string(&path).expect("read");
            out.push(serde_json::from_str(&text).expect("json"));
        }
    }
}

/// A validator for one schema, resolving every `urn:` reference against
/// the whole schema tree (the tests/transport-contract pattern).
fn validator(path: &str) -> jsonschema::Validator {
    static REGISTRY: std::sync::OnceLock<jsonschema::Registry<'_>> = std::sync::OnceLock::new();
    let registry = REGISTRY.get_or_init(|| {
        let mut docs = Vec::new();
        all_schema_docs(&root().join("architecture/contracts/schemas"), &mut docs);
        let pairs: Vec<(String, jsonschema::Resource)> = docs
            .into_iter()
            .filter_map(|doc| {
                let id = doc.get("$id").and_then(Value::as_str)?.to_owned();
                Some((id, jsonschema::Resource::from_contents(doc)))
            })
            .collect();
        jsonschema::Registry::new()
            .extend(pairs)
            .expect("schemas register")
            .prepare()
            .expect("registry prepares")
    });
    jsonschema::options()
        .with_registry(registry)
        .build(&json_at(path))
        .expect("schema compiles")
}

fn assert_valid(path: &str, instance: &Value) {
    let errors: Vec<String> = validator(path)
        .iter_errors(instance)
        .map(|e| e.to_string())
        .collect();
    assert!(errors.is_empty(), "{path} refuses {instance}: {errors:?}");
}

/// `ipc/trust-list` 1.1.0 ties `persisted` to `source` both ways, and
/// the mirror refuses exactly what the schema refuses: the two shapes it
/// names pass both, the two mixtures fail both.
#[test]
fn the_trust_row_shapes_agree_with_the_schema_both_ways() {
    let path = "architecture/contracts/schemas/ipc/trust-list.schema.json";
    let schema = validator(path);
    for (row, legal) in [
        (serde_json::json!({"peer": PEER, "persisted": false}), true),
        (
            serde_json::json!({"peer": PEER, "persisted": true, "source": "configured"}),
            true,
        ),
        (
            serde_json::json!({"peer": PEER, "persisted": true, "source": "administered"}),
            true,
        ),
        (serde_json::json!({"peer": PEER, "persisted": true}), false),
        (
            serde_json::json!({"peer": PEER, "persisted": false, "source": "configured"}),
            false,
        ),
        (
            serde_json::json!({"peer": PEER, "persisted": true, "source": "elsewhere"}),
            false,
        ),
    ] {
        let page = serde_json::json!({ "allowed": [row] });
        assert_eq!(schema.is_valid(&page), legal, "the schema on {page}");
        assert_eq!(
            serde_json::from_value::<TrustList>(page.clone()).is_ok(),
            legal,
            "the mirror on {page}"
        );
    }
}

/// `ipc/endpoint-list` 1.1.0 widens `persisted` from the literal
/// `false` to a boolean -- the 2.0 row and the 2.5 row -- and the mirror
/// reads exactly what the schema admits: both values pass both, and a
/// row missing it or naming it as anything else fails both.
/// Major 3 carries the derived `EndpointId` alone (ADR-0067 decision 7):
/// in every IPC document that holds one, a label of the retired grammar
/// -- well formed under it, inside every length bound -- is refused by the
/// schema and by the mirror alike, and the derived id in the same place is
/// the control both accept. The hello's claim is the one place a label
/// parses, to be answered `InvalidArgument` by the handshake
/// (`handshake.rs`'s `a_label_claim_is_invalid_argument`).
#[test]
fn a_label_is_refused_wherever_an_ipc_document_carries_an_endpoint_id() {
    type Reads = fn(&Value) -> bool;
    type Mirror = Box<dyn Fn(&Value) -> bool>;
    fn params(method: Method) -> impl Fn(&Value) -> bool {
        move |v| {
            let raw = serde_json::value::to_raw_value(v).expect("raw");
            Request::decode(method, Some(&raw)).is_ok()
        }
    }
    fn event(kind: &'static str) -> impl Fn(&Value) -> bool {
        move |v| {
            let raw = serde_json::value::to_raw_value(v).expect("raw");
            Event::decode(kind, Some(&raw), supported()[0]).is_ok()
        }
    }
    let message = serde_json::json!({"media_type": "text/plain", "bytes": "aGk"});
    let id = "018f3c2a5e6d7b8c9dabcdef01234567";
    let cases: Vec<(&str, Value, &str, Mirror)> = vec![
        (
            "ipc/send-params",
            serde_json::json!({"peer": PEER, "endpoint": HUMAN, "message_id": id, "payload": message}),
            "/endpoint",
            Box::new(params(Method::DirectSend)),
        ),
        (
            "ipc/endpoint-params",
            serde_json::json!({"endpoint": HUMAN}),
            "/endpoint",
            Box::new(params(Method::AdminEndpointsRevoke)),
        ),
        (
            "ipc/set-enabled-params",
            serde_json::json!({"endpoint": HUMAN, "enabled": false}),
            "/endpoint",
            Box::new(params(Method::AdminEndpointsSetEnabled)),
        ),
        (
            "ipc/set-default-params",
            serde_json::json!({"endpoint": HUMAN}),
            "/endpoint",
            Box::new(params(Method::AdminEndpointsSetDefault)),
        ),
        (
            "ipc/send-result",
            serde_json::json!({"resolved_endpoint": HUMAN}),
            "/resolved_endpoint",
            Box::new((|v| serde_json::from_value::<SendResult>(v.clone()).is_ok()) as Reads),
        ),
        (
            "ipc/endpoint-list",
            serde_json::json!({"endpoints": [{"id": HUMAN, "enabled": true, "default": true, "persisted": true}]}),
            "/endpoints/0/id",
            Box::new((|v| serde_json::from_value::<EndpointList>(v.clone()).is_ok()) as Reads),
        ),
        (
            "endpoints/directory-response",
            serde_json::json!({"endpoints": [BOT, HUMAN], "ttl_ms": 1, "generated_at_ms": 2}),
            "/endpoints/1",
            Box::new((|v| serde_json::from_value::<DirectoryResult>(v.clone()).is_ok()) as Reads),
        ),
        (
            "ipc/lease-changed",
            serde_json::json!({"endpoint": HUMAN, "revoked_epoch": "AAAAAAAAAAAAAAAAAAAAAQ"}),
            "/endpoint",
            Box::new(event("endpoint.lease_changed")),
        ),
        (
            "endpoints/message-received",
            serde_json::json!({"message_id": id, "mode": "direct", "source_peer": PEER,
                "source_endpoint": HUMAN, "destination_endpoint": BOT, "payload": message,
                "received_at": 1}),
            "/source_endpoint",
            Box::new(event("message.direct")),
        ),
        (
            "endpoints/message-received",
            serde_json::json!({"message_id": id, "mode": "direct", "source_peer": PEER,
                "source_endpoint": BOT, "destination_endpoint": HUMAN, "payload": message,
                "received_at": 1}),
            "/destination_endpoint",
            Box::new(event("message.direct")),
        ),
        (
            "ipc/frame",
            serde_json::json!({"type": "hello_response", "ipc_version": {"major": 3, "minor": 0},
                "transport_contract_version": "3.0", "peer": PEER, "endpoint": HUMAN,
                "endpoint_lease_epoch": "AAAAAAAAAAAAAAAAAAAAAQ", "event_queue": 256,
                "granted_capabilities": ["events"]}),
            "/endpoint",
            Box::new((|v| Frame::parse(&v.to_string()).is_ok()) as Reads),
        ),
    ];
    for (name, derived, pointer, reads) in cases {
        let path = format!("architecture/contracts/schemas/{name}.schema.json");
        let schema = validator(&path);
        assert!(
            schema.is_valid(&derived),
            "{name}: the schema takes the derived id"
        );
        assert!(reads(&derived), "{name}: and so does the mirror");
        for label in ["human", "claude.agent-1", "a"] {
            let mut labelled = derived.clone();
            *labelled.pointer_mut(pointer).expect("the endpoint") = Value::from(label);
            assert!(
                !schema.is_valid(&labelled),
                "{name}{pointer}: the schema refuses {label}"
            );
            assert!(
                !reads(&labelled),
                "{name}{pointer}: and so does the mirror, {label}"
            );
        }
    }
}

#[test]
fn the_endpoint_row_shapes_agree_with_the_schema_both_ways() {
    let path = "architecture/contracts/schemas/ipc/endpoint-list.schema.json";
    let schema = validator(path);
    let row = |persisted: Option<Value>| {
        let mut row = serde_json::json!({"id": HUMAN, "enabled": true, "default": false});
        if let Some(p) = persisted {
            row["persisted"] = p;
        }
        serde_json::json!({ "endpoints": [row] })
    };
    for (list, legal) in [
        (row(Some(Value::Bool(false))), true),
        (row(Some(Value::Bool(true))), true),
        (row(None), false),
        (row(Some(Value::Null)), false),
        (row(Some(serde_json::json!("true"))), false),
    ] {
        assert_eq!(schema.is_valid(&list), legal, "the schema on {list}");
        assert_eq!(
            serde_json::from_value::<EndpointList>(list.clone()).is_ok(),
            legal,
            "the mirror on {list}"
        );
    }
}

const PEER: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
const OTHER_PEER: &str = "12D3KooWHsy9ZMqTYfTPpJd8YXhZGKrLWzkWT9BgX9DeyF8Fs3GQ";

fn peer() -> TransportIdentity {
    TransportIdentity::parse(PEER).expect("peer")
}

fn ep(name: &str) -> EndpointId {
    interweave_test_support::endpoint_keys::endpoint_id(name)
}

/// TEST-ONLY derived ids, `ep("human")` and `ep("bot")` as text
/// (`the_test_only_ids_are_the_support_crates`).
const HUMAN: &str = "12D3KooWK26ch85cLfzj6U4MegkVaGjpTcecHrnDX1zeFV3DCycr";
const BOT: &str = "12D3KooWBcY4QvsUdzcWLKkthqMjuYpRpAQxDF3hAwiD3SJbtvfV";

#[test]
fn the_test_only_ids_are_the_support_crates() {
    assert_eq!(ep("human").as_str(), HUMAN);
    assert_eq!(ep("bot").as_str(), BOT);
}

fn epoch() -> Generation {
    Generation::parse("AAAAAAAAAAAAAAAAAAAAAQ").expect("epoch")
}

fn payload() -> Payload {
    Payload::at_ceiling(
        Some(MediaType::parse("text/plain").expect("media type")),
        b"hello".to_vec(),
    )
    .expect("payload")
}

fn channel() -> ChannelId {
    ChannelId::parse("ops/alerts").expect("channel")
}

fn id(text: &str) -> RequestId {
    RequestId::new(text).expect("request id")
}

fn connectivity() -> ConnectivitySummary {
    ConnectivitySummary {
        direct_inbound: DirectInboundState::VerifiedPublic,
        relay_inbound: PathReadiness::Ready,
        active_relay_reservations: 1,
        target_relay_reservations: 2,
        active_relayed_peer_paths: 3,
        hole_punch_inflight: 0,
        preferred_path_policy: PreferredPathPolicy::DirectFirst,
        updated_at: 1_700_000_000_000,
    }
}

/// One request per method, every one of them.
fn every_request() -> Vec<Request> {
    let message_id = MessageId::from_bytes([7; 16]);
    let requests = vec![
        Request::ChannelJoin(ChannelParams { channel: channel() }),
        Request::ChannelLeave(ChannelParams { channel: channel() }),
        Request::BroadcastPublish(PublishParams {
            channel: channel(),
            message_id,
            payload: payload(),
        }),
        Request::DirectSend(SendParams {
            peer: peer(),
            endpoint: Some(ep("human")),
            message_id,
            payload: payload(),
        }),
        Request::EndpointsQuery(QueryParams { peer: peer() }),
        Request::AdminStatus,
        Request::AdminEndpointsList,
        Request::AdminEndpointsRevoke(EndpointParams {
            endpoint: ep("human"),
        }),
        Request::AdminEndpointsSetEnabled(SetEnabledParams {
            endpoint: ep("human"),
            enabled: false,
        }),
        Request::AdminEndpointsSetDefault(SetDefaultParams { endpoint: None }),
        Request::AdminShutdown(ShutdownParams {
            grace_ms: Some(1000),
        }),
        Request::AdminTrustList(TrustListParams {
            after: Some(peer()),
            endpoint: Some(ep("human")),
            direction: Some(TrustDirection::Outbound),
        }),
        Request::AdminTrustSet(TrustSetParams {
            peer: peer(),
            allowed: false,
            endpoint: Some(ep("human")),
        }),
        Request::AdminTrustSeed(TrustSeedParams {
            endpoint: ep("human"),
        }),
        Request::AdminPeersList(PeerListParams {
            after: Some(peer()),
        }),
        Request::AdminPeersAddAddress(AddressParams {
            address: format!("/ip4/192.168.1.20/tcp/4001/p2p/{}", peer().as_str()),
        }),
        Request::AdminPeersRemoveAddress(AddressParams {
            address: format!("/ip4/192.168.1.20/tcp/4001/p2p/{}", peer().as_str()),
        }),
        Request::AdminPeersListAddresses,
        Request::AdminPeersOwnAddresses,
        Request::AdminPeersAddAddresses(AddressesParams {
            addresses: vec![
                format!("/ip4/192.168.1.20/tcp/4001/p2p/{}", peer().as_str()),
                format!("/ip4/192.168.1.21/tcp/4001/p2p/{}", peer().as_str()),
            ],
        }),
        Request::SessionLosses,
    ];
    assert_eq!(
        requests.iter().map(Request::method).collect::<Vec<_>>(),
        Method::ALL,
        "one request per method"
    );
    requests
}

/// One event per type, every one of them, built from the session's.
fn every_event() -> Vec<Event> {
    let events: Vec<Event> = [
        SessionEvent::Direct(ReceivedDirect {
            source_peer: peer(),
            source_endpoint: ep("bot"),
            destination_endpoint: ep("human"),
            message_id: MessageId::from_bytes([1; 16]),
            payload: payload(),
            received_at_ms: 1,
        }),
        SessionEvent::Broadcast(ReceivedBroadcast {
            source_peer: peer(),
            channel: channel(),
            message_id: MessageId::from_bytes([2; 16]),
            payload: payload(),
            received_at_ms: 2,
        }),
        SessionEvent::Local(LocalSessionEvent::EndpointLeaseChanged {
            endpoint: ep("human"),
            revoked_epoch: epoch(),
        }),
        SessionEvent::Local(LocalSessionEvent::PeerDisconnected {
            peer: peer(),
            reason_class: "policy".into(),
        }),
        SessionEvent::Local(LocalSessionEvent::PeerPathChanged {
            peer: peer(),
            previous: Some(interweave_transport_api::PeerPath::Relayed),
            current: interweave_transport_api::PeerPath::Direct,
            reason_class: "dcutr".into(),
            observed_at: 3,
        }),
    ]
    .into_iter()
    .map(|e| Event::from_session(e).expect("maps").expect("an event"))
    .collect();
    assert_eq!(
        events.iter().map(Event::event_type).collect::<Vec<_>>(),
        EventType::ALL,
        "one event per type"
    );
    events
}

/// Every result type, with the schema the method table names for it.
fn every_result() -> Vec<(&'static str, Value)> {
    let admin = AdminStatusResult::new(
        AdminStatus {
            health: Health::Degraded,
            peer: peer(),
            connectivity: connectivity(),
            active_leases: 1,
            pre_auth: Some(interweave_local_client_api::PreAuthCounts {
                tracked_sources: 3,
                pending: 1,
            }),
            ingress: Some(interweave_local_client_api::IngressCounts {
                direct_tracked_peers: 2,
                broadcast_tracked_peers: 0,
            }),
            events_dropped: Some(4),
            events_refused: Some(5),
        },
        ServerCounters::default(),
    );
    let list = EndpointList::from_views(vec![
        EndpointAdminView {
            endpoint: ep("bot"),
            enabled: false,
            default: false,
            persisted: false,
            lease: None,
        },
        EndpointAdminView {
            endpoint: ep("human"),
            enabled: true,
            default: true,
            persisted: true,
            lease: Some(LeaseRecord {
                endpoint: ep("human"),
                epoch: epoch(),
                client_kind: "human-client".into(),
                session_id: Some("s".into()),
            }),
        },
    ])
    .expect("rows");
    let trust = TrustAdminView {
        local_peer: Some(
            TransportIdentity::parse("12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5")
                .expect("peer"),
        ),
        allowed: [
            peer(),
            TransportIdentity::parse("QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N")
                .expect("peer"),
        ]
        .into_iter()
        .zip([
            interweave_local_client_api::TrustSource::Configured,
            interweave_local_client_api::TrustSource::Administered,
        ])
        .map(|(peer, source)| interweave_local_client_api::TrustedPeer {
            peer,
            persisted: true,
            source,
        })
        .collect(),
    };
    let directory = DirectoryResult::from(EndpointDirectoryV1 {
        generated_at_ms: 3,
        ttl_ms: u32::MAX,
        endpoints: vec![ep("human"), ep("bot")],
    });
    vec![
        (
            "architecture/contracts/schemas/ipc/empty-result.schema.json",
            json(&EmptyResult {}),
        ),
        (
            "architecture/contracts/schemas/ipc/send-result.schema.json",
            json(&SendResult {
                resolved_endpoint: ep("human"),
            }),
        ),
        (
            "architecture/contracts/schemas/endpoints/directory-response.schema.json",
            json(&directory),
        ),
        (
            "architecture/contracts/schemas/ipc/admin-status.schema.json",
            json(&admin),
        ),
        (
            "architecture/contracts/schemas/ipc/endpoint-list.schema.json",
            json(&list),
        ),
        (
            "architecture/contracts/schemas/ipc/set-enabled-result.schema.json",
            json(&SetEnabledResult {
                revoked_epoch: Some(epoch()),
            }),
        ),
        (
            "architecture/contracts/schemas/ipc/set-enabled-result.schema.json",
            json(&SetEnabledResult {
                revoked_epoch: None,
            }),
        ),
        // The first page names the local peer; a later one does not; an
        // empty allowlist is a page with no rows. The persisted row with
        // its source, the one row shape 3.0 serves.
        (
            "architecture/contracts/schemas/ipc/trust-list.schema.json",
            json(&TrustList::page(trust.clone(), None)),
        ),
        (
            "architecture/contracts/schemas/ipc/trust-list.schema.json",
            json(&TrustList::page(trust, Some(&peer()))),
        ),
        (
            "architecture/contracts/schemas/ipc/trust-list.schema.json",
            json(&TrustList::page(
                TrustAdminView {
                    local_peer: None,
                    allowed: Vec::new(),
                },
                None,
            )),
        ),
        // A row with every field and one with none of the optional ones,
        // a later page, and an empty allowlist.
        (
            "architecture/contracts/schemas/ipc/peer-list.schema.json",
            json(&PeerList::page(
                vec![
                    PeerGateView {
                        peer: peer(),
                        connected: false,
                        last_seen: interweave_local_client_api::PeerLastSeen::At(1_791_329_940_227),
                        backoff_until: Some(1_791_329_950_227),
                        quarantined_until: Some(u64::MAX),
                        route_held_until: Some(1_791_329_960_227),
                        last_outcome: Some(PeerOutcome::IdentityMismatch),
                    },
                    PeerGateView {
                        peer: TransportIdentity::parse(OTHER_PEER).expect("a peer"),
                        connected: true,
                        last_seen: interweave_local_client_api::PeerLastSeen::Never,
                        backoff_until: None,
                        quarantined_until: None,
                        route_held_until: None,
                        last_outcome: None,
                    },
                ],
                None,
            )),
        ),
        // A port that does not say when it last saw the peer: no
        // `last_seen_ms`, the route hold still there.
        (
            "architecture/contracts/schemas/ipc/peer-list.schema.json",
            json(&PeerList::page(
                vec![PeerGateView {
                    peer: peer(),
                    connected: false,
                    last_seen: interweave_local_client_api::PeerLastSeen::Unreported,
                    backoff_until: None,
                    quarantined_until: None,
                    route_held_until: Some(1_791_329_960_227),
                    last_outcome: Some(PeerOutcome::DialFailed),
                }],
                None,
            )),
        ),
        (
            "architecture/contracts/schemas/ipc/peer-list.schema.json",
            json(&PeerList {
                peers: Vec::new(),
                next: Some(peer()),
            }),
        ),
        (
            "architecture/contracts/schemas/ipc/peer-list.schema.json",
            json(&PeerList::page(Vec::new(), None)),
        ),
        (
            "architecture/contracts/schemas/ipc/session-losses-result.schema.json",
            json(&SessionLossesResult {
                queue_overflow: u64::MAX,
                ingress_refused: 0,
            }),
        ),
    ]
}

/// `session-losses-result` both ways: the two counts and nothing else,
/// a negative or fractional count refused by the schema and the type
/// alike, the largest count read back whole.
#[test]
fn the_session_losses_shape_agrees_with_the_schema_both_ways() {
    let schema = "architecture/contracts/schemas/ipc/session-losses-result.schema.json";
    let good =
        serde_json::json!({"queue_overflow": 3, "ingress_refused": 18_446_744_073_709_551_615_u64});
    assert_valid(schema, &good);
    let read: SessionLossesResult = serde_json::from_value(good.clone()).expect("reads");
    assert_eq!(read.ingress_refused, u64::MAX);
    assert_eq!(json(&read), good, "round-trips");
    for bad in [
        serde_json::json!({"queue_overflow": 1}),
        serde_json::json!({"queue_overflow": -1, "ingress_refused": 0}),
        serde_json::json!({"queue_overflow": 1.5, "ingress_refused": 0}),
        serde_json::json!({"queue_overflow": 0, "ingress_refused": 0, "channel": "general"}),
    ] {
        assert!(
            !validator(schema).is_valid(&bad),
            "the schema refuses {bad}"
        );
        assert!(
            serde_json::from_value::<SessionLossesResult>(bad.clone()).is_err(),
            "the type refuses {bad}"
        );
    }
}

fn json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("ser")
}

/// Every frame class this crate builds, in every shape it builds it.
fn every_frame() -> Vec<Frame> {
    let hello: Hello = serde_json::from_value(serde_json::json!({
        "type": "hello", "ipc_version": {"major": 3, "minor": 0},
        "client": {"kind": "human-client", "version": "0.1.0"},
        "endpoint": {"id": HUMAN},
        "requested_capabilities": ["events", "commands"], "features": ["keepalive"]
    }))
    .expect("hello");
    let data_outcome = HandshakeOutcome {
        granted_data: [DataCapability::Events, DataCapability::Commands].into(),
        granted_admin: BTreeSet::new(),
        endpoint: Some(ep("human")),
    };
    let admin_outcome = HandshakeOutcome {
        granted_data: BTreeSet::new(),
        granted_admin: [AdminCapability::Status, AdminCapability::Shutdown].into(),
        endpoint: None,
    };
    let version = IpcVersion { major: 3, minor: 0 };
    let nonce = Nonce::new("_-0123456789abcdefghij").expect("nonce");
    let ping = Ping::new(nonce);
    let mut frames = vec![
        Frame::Hello(hello),
        Frame::HelloResponse(HelloResponse::new(
            version,
            peer(),
            Some(GrantedLease {
                endpoint: ep("human"),
                endpoint_lease_epoch: epoch(),
                event_queue: std::num::NonZeroU32::new(256).expect("positive"),
            }),
            &data_outcome,
        )),
        Frame::HelloResponse(HelloResponse::new(version, peer(), None, &admin_outcome)),
        Frame::Close(Close::version_incompatible(UnsupportedMajor(3))),
        Frame::Close(Close::new(TransportError::Timeout).with_message("hello არ მოვიდა")),
        Frame::Response(ResponseFrame::failure(id("e"), TransportError::Overloaded)),
        Frame::Cancel(Cancel::new(id("c"))),
        Frame::ServerState(ServerState::new(Health::Healthy, None)),
        Frame::ServerState(ServerState::new(Health::Degraded, Some(connectivity()))),
        Frame::Pong(ping.echo()),
        Frame::Ping(ping),
    ];
    frames.extend(
        every_request()
            .into_iter()
            .map(|r| Frame::Request(r.into_frame(id("r"), Some(5000)))),
    );
    frames.extend(
        (0_u64..)
            .zip(every_event())
            .map(|(sequence, e)| Frame::Event(e.into_frame(sequence))),
    );
    frames.extend(
        every_result()
            .into_iter()
            .map(|(_, result)| Frame::Response(ResponseFrame::success(id("ok"), &result))),
    );
    frames
}

#[test]
fn the_ipc_schema_inventory_is_complete() {
    let mut on_disk: Vec<String> =
        std::fs::read_dir(root().join("architecture/contracts/schemas/ipc"))
            .expect("ipc schema dir")
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|name| name.ends_with(".schema.json"))
            .map(|name| format!("architecture/contracts/schemas/ipc/{name}"))
            .collect();
    on_disk.sort();
    assert_eq!(
        on_disk, IPC_SCHEMAS,
        "a schema added or removed changes this list"
    );
    for path in IPC_SCHEMAS {
        let _ = validator(path);
    }
}

#[test]
fn every_emitted_frame_validates_against_the_frame_schema() {
    let frames = every_frame();
    let classes: BTreeSet<String> = frames
        .iter()
        .map(|f| {
            serde_json::to_value(f).expect("ser")["type"]
                .as_str()
                .expect("type")
                .to_owned()
        })
        .collect();
    assert_eq!(classes.len(), 10, "every class is exercised: {classes:?}");
    for frame in frames {
        let body = frame.to_body();
        let value: Value = serde_json::from_str(&body).expect("json");
        assert_valid(
            "architecture/contracts/schemas/ipc/frame.schema.json",
            &value,
        );
        // And what this crate emits it also reads back, to the same bytes.
        let back = Frame::parse(&body).expect("parses");
        assert_eq!(back.to_body(), body);
    }
}

/// A data hello otherwise valid, with this kind and claim.
fn hello_with(kind: &str, endpoint: Option<&str>) -> Value {
    let mut hello = serde_json::json!({"type": "hello", "ipc_version": {"major": 3, "minor": 0},
        "client": {"kind": kind}, "requested_capabilities": ["events"],
        "features": ["keepalive"]});
    if let Some(id) = endpoint {
        hello["endpoint"] = serde_json::json!({"id": id});
    }
    hello
}

/// The other edge of hello 1.2.0's label bounds: at them, the schema and
/// the parser both ACCEPT -- 64 characters, counted as characters and not
/// bytes (64 `é` are 128 bytes), for the kind and the claimed id alike.
/// Within the bounds the claim's grammar is the handshake's: 64 `é` is a
/// well-formed frame and an `InvalidArgument` claim.
/// A label carrying an escaped lone surrogate is not well-formed JSON
/// text: `serde_json` refuses it, so the frame is a framing error, never a
/// one-code-point label a laxer parser would count (the #165 review's
/// risk; LOCAL-IPC.md says the two sides agree on well-formed JSON).
#[test]
fn a_lone_surrogate_in_a_hello_label_is_a_framing_error() {
    for (kind, id) in [(r"\ud800", HUMAN), ("human-client", r"\udfff")] {
        let body = format!(
            r#"{{"type":"hello","ipc_version":{{"major":3,"minor":0}},"client":{{"kind":"{kind}"}},"endpoint":{{"id":"{id}"}},"requested_capabilities":["events"],"features":["keepalive"]}}"#
        );
        assert!(Frame::parse(&body).is_err(), "refused at parse: {body}");
    }
    // The control: the same frame with a paired surrogate parses.
    let paired = r#"{"type":"hello","ipc_version":{"major":3,"minor":0},"client":{"kind":"k\ud83d\ude00"},"requested_capabilities":["events"]}"#;
    assert!(Frame::parse(paired).is_ok(), "{paired}");
}

#[test]
fn a_hello_at_its_label_bounds_parses() {
    let accepted = [
        hello_with(&"é".repeat(64), Some(HUMAN)),
        hello_with("k", Some(HUMAN)),
        hello_with("human-client", Some(&"e".repeat(64))),
        hello_with("human-client", Some("e")),
        hello_with("human-client", Some(&"é".repeat(64))),
    ];
    for value in &accepted {
        assert!(
            validator("architecture/contracts/schemas/ipc/frame.schema.json").is_valid(value),
            "the schema accepts {value}"
        );
        assert!(
            Frame::parse(&value.to_string()).is_ok(),
            "and so does the parser: {value}"
        );
    }
    let Ok(Frame::Hello(wide)) = Frame::parse(&accepted[4].to_string()) else {
        panic!("a hello");
    };
    assert_eq!(
        wide.evaluate(interweave_ipc_protocol::AuthorityDomain::Data, true),
        Err(TransportError::InvalidArgument),
        "inside the bounds, outside the grammar: the handshake's answer"
    );
}

#[test]
fn the_frame_schema_is_live_and_refuses_what_the_parser_refuses() {
    // The control: a schema that accepted everything would pass the test
    // above for free.
    let refused = [
        serde_json::json!({"type": "hello_response", "ipc_version": {"major": 3, "minor": 0},
               "transport_contract_version": "3.0", "peer": PEER, "endpoint": HUMAN,
               "granted_capabilities": []}),
        // hello-response 1.1.0: the event queue bound comes with the
        // lease and only with it, in both directions.
        serde_json::json!({"type": "hello_response", "ipc_version": {"major": 3, "minor": 0},
               "transport_contract_version": "3.0", "peer": PEER, "endpoint": HUMAN,
               "endpoint_lease_epoch": "AAAAAAAAAAAAAAAAAAAAAQ", "granted_capabilities": []}),
        serde_json::json!({"type": "hello_response", "ipc_version": {"major": 3, "minor": 0},
               "transport_contract_version": "3.0", "peer": PEER, "event_queue": 256,
               "granted_capabilities": []}),
        serde_json::json!({"type": "hello_response", "ipc_version": {"major": 3, "minor": 0},
               "transport_contract_version": "3.0", "peer": PEER, "endpoint": HUMAN,
               "endpoint_lease_epoch": "AAAAAAAAAAAAAAAAAAAAAQ", "event_queue": 0,
               "granted_capabilities": []}),
        serde_json::json!({"type": "close", "code": "VersionIncompatible"}),
        serde_json::json!({"type": "response", "id": "1", "ok": false}),
        serde_json::json!({"type": "ping", "nonce": "short"}),
        // hello 1.2.0's bounded labels: the claimed id and the client kind
        // are 1 to 64 characters, refused at parse by both sides.
        hello_with("human-client", Some("")),
        hello_with("human-client", Some(&"e".repeat(65))),
        hello_with("", Some(HUMAN)),
        hello_with(&"k".repeat(65), Some(HUMAN)),
    ];
    for value in refused {
        assert!(
            !validator("architecture/contracts/schemas/ipc/frame.schema.json").is_valid(&value),
            "the schema refuses {value}"
        );
        assert!(
            Frame::parse(&value.to_string()).is_err(),
            "and so does the parser: {value}"
        );
    }
}

#[test]
fn every_request_validates_against_its_catalogue_entry_and_params_schema() {
    let params_schema = |method: Method| -> Option<&'static str> {
        Some(match method {
            Method::ChannelJoin | Method::ChannelLeave => {
                "architecture/contracts/schemas/ipc/channel-params.schema.json"
            }
            Method::BroadcastPublish => {
                "architecture/contracts/schemas/ipc/publish-params.schema.json"
            }
            Method::DirectSend => "architecture/contracts/schemas/ipc/send-params.schema.json",
            Method::EndpointsQuery => "architecture/contracts/schemas/ipc/query-params.schema.json",
            Method::AdminStatus
            | Method::AdminEndpointsList
            | Method::AdminPeersListAddresses
            | Method::AdminPeersOwnAddresses
            | Method::SessionLosses => {
                return None;
            }
            Method::AdminEndpointsRevoke => {
                "architecture/contracts/schemas/ipc/endpoint-params.schema.json"
            }
            Method::AdminEndpointsSetEnabled => {
                "architecture/contracts/schemas/ipc/set-enabled-params.schema.json"
            }
            Method::AdminEndpointsSetDefault => {
                "architecture/contracts/schemas/ipc/set-default-params.schema.json"
            }
            Method::AdminShutdown => {
                "architecture/contracts/schemas/ipc/shutdown-params.schema.json"
            }
            Method::AdminTrustList => {
                "architecture/contracts/schemas/ipc/trust-list-params.schema.json"
            }
            Method::AdminTrustSet => {
                "architecture/contracts/schemas/ipc/trust-set-params.schema.json"
            }
            Method::AdminTrustSeed => {
                "architecture/contracts/schemas/ipc/trust-seed-params.schema.json"
            }
            Method::AdminPeersList => {
                "architecture/contracts/schemas/ipc/peer-list-params.schema.json"
            }
            Method::AdminPeersAddAddress | Method::AdminPeersRemoveAddress => {
                "architecture/contracts/schemas/ipc/peers-address-params.schema.json"
            }
            Method::AdminPeersAddAddresses => {
                "architecture/contracts/schemas/ipc/peers-addresses-params.schema.json"
            }
        })
    };
    for request in every_request() {
        let method = request.method();
        let params: Value = serde_json::from_str(request.params().get()).expect("json");
        let pair = serde_json::json!({"method": method.as_str(), "params": params});
        assert_valid(
            "architecture/contracts/schemas/ipc/request.schema.json",
            &pair,
        );
        assert_valid(
            "architecture/contracts/schemas/ipc/method.schema.json",
            &pair["method"],
        );
        if let Some(path) = params_schema(method) {
            assert_valid(path, &params);
        }
    }
    // The payload a publish and a send carry is ipc/payload's.
    assert_valid(
        "architecture/contracts/schemas/ipc/payload.schema.json",
        &serde_json::to_value(payload()).expect("ser"),
    );
}

/// A request with no `params` at all: the schema takes it for exactly
/// the methods the decoder takes it for -- a method taking none -- and
/// refuses it for every method that takes some (the control).
#[test]
fn a_request_without_params_is_valid_exactly_where_the_decoder_takes_it() {
    let request = validator("architecture/contracts/schemas/ipc/request.schema.json");
    let mut without = 0;
    for method in Method::ALL {
        let decoded = Request::decode(method, None).is_ok();
        let frame = serde_json::json!({"method": method.as_str()});
        assert_eq!(
            request.is_valid(&frame),
            decoded,
            "{}: the schema and the decoder disagree on an absent params",
            method.as_str()
        );
        without += usize::from(decoded);
    }
    assert!(
        without > 0 && without < Method::ALL.len(),
        "both sides are exercised: {without} of {} take no params",
        Method::ALL.len()
    );
}

#[test]
fn every_event_validates_against_its_catalogue_entry_and_body_schema() {
    for event in every_event() {
        let frame = event.clone().into_frame(0);
        let data: Value =
            serde_json::from_str(frame.data.as_deref().expect("data").get()).expect("json");
        let pair = serde_json::json!({"event_type": frame.event_type, "data": data});
        assert_valid(
            "architecture/contracts/schemas/ipc/event.schema.json",
            &pair,
        );
        let body = match event {
            Event::MessageDirect(_) => {
                Some("architecture/contracts/schemas/endpoints/message-received.schema.json")
            }
            Event::MessageBroadcast(_) => {
                Some("architecture/contracts/schemas/ipc/broadcast-received.schema.json")
            }
            Event::LeaseChanged(_) => {
                Some("architecture/contracts/schemas/ipc/lease-changed.schema.json")
            }
            Event::PeerDisconnected(_) => None,
            Event::PathChanged(_) => {
                Some("architecture/contracts/schemas/ipc/path-changed.schema.json")
            }
        };
        if let Some(path) = body {
            assert_valid(path, &data);
        }
    }
}

/// `ipc.path-changed` 1.1.0's optional member (A 2026-10-09): the schema
/// no longer requires `previous`, the Rust shape holds it as an `Option`,
/// and a notice with none -- a route's begin, a routed peer's reconnect --
/// serializes WITHOUT the member, never as `null`, and validates; read
/// back, it is the same notice. The control: one with `previous`
/// carries it, and a `null` is refused by the schema.
#[test]
fn a_path_notice_without_previous_omits_it_and_validates() {
    let schema = "architecture/contracts/schemas/ipc/path-changed.schema.json";
    let doc: Value =
        serde_json::from_str(&std::fs::read_to_string(root().join(schema)).expect("the schema"))
            .expect("json");
    let required = string_set(&doc["required"]);
    assert!(!required.contains("previous"), "previous is optional");
    for field in ["peer", "current", "reason_class", "observed_at"] {
        assert!(required.contains(field), "{field} stays required");
    }
    for (previous, class) in [
        (None, interweave_local_client_api::ROUTE_ESTABLISHED),
        (None, interweave_local_client_api::RECONNECTED),
        (
            Some(interweave_transport_api::PeerPath::Relayed),
            "direct_established",
        ),
    ] {
        let session = SessionEvent::Local(LocalSessionEvent::PeerPathChanged {
            peer: peer(),
            previous,
            current: interweave_transport_api::PeerPath::Direct,
            reason_class: class.into(),
            observed_at: 4,
        });
        let event = Event::from_session(session.clone())
            .expect("maps")
            .expect("an event");
        let frame = event.into_frame(0);
        let data: Value =
            serde_json::from_str(frame.data.as_deref().expect("data").get()).expect("json");
        assert_eq!(
            data.get("previous").is_some(),
            previous.is_some(),
            "{class}: absent exactly when there is none: {data}"
        );
        assert_valid(schema, &data);
        let read = Event::decode(&frame.event_type, frame.data.as_deref(), supported()[0])
            .expect("decodes at 3.0");
        assert_eq!(read.into_session(), session, "{class}: round trip");
    }
    let mut with_null = serde_json::json!({
        "peer": peer().as_str(), "current": "direct",
        "reason_class": "route_established", "observed_at": 1
    });
    with_null["previous"] = Value::Null;
    assert!(
        !validator(schema).is_valid(&with_null),
        "null is not absent"
    );
    // And the decoder refuses it as the schema does (#248 review F3).
    let raw = serde_json::value::to_raw_value(&with_null).expect("raw");
    assert_eq!(
        Event::decode("peer.path_changed", Some(&raw), supported()[0],).err(),
        Some(interweave_transport_api::TransportError::ProtocolViolation),
        "an explicit null previous is the server's violation"
    );
}

/// The decode half of the major: a notice without `previous` is read at
/// 3.0, where the shape 2.4 brought is the only one, and on a major-2
/// connection -- 2.13 among them, past 2.4 by its minor alone -- the event
/// type itself is unknown, so the server's violation.
#[test]
fn a_path_notice_without_previous_decodes_at_three_zero_and_not_on_major_two() {
    let data = serde_json::value::to_raw_value(&serde_json::json!({
        "peer": peer().as_str(), "current": "relayed",
        "reason_class": "route_established", "observed_at": 1
    }))
    .expect("raw");
    assert!(Event::decode("peer.path_changed", Some(&data), supported()[0]).is_ok());
    for minor in [3, 4, 13] {
        assert_eq!(
            Event::decode(
                "peer.path_changed",
                Some(&data),
                IpcVersion { major: 2, minor }
            )
            .err(),
            Some(interweave_transport_api::TransportError::ProtocolViolation),
            "2.{minor}"
        );
    }
}

#[test]
fn every_result_validates_against_the_schema_its_method_names() {
    for (path, result) in every_result() {
        assert_valid(path, &result);
    }
}

#[test]
fn the_hello_side_validates_against_its_own_schemas() {
    let frames = every_frame();
    for frame in &frames {
        let value = serde_json::to_value(frame).expect("ser");
        let path = match frame {
            Frame::Hello(_) => "architecture/contracts/schemas/ipc/hello.schema.json",
            Frame::HelloResponse(_) => {
                "architecture/contracts/schemas/ipc/hello-response.schema.json"
            }
            Frame::Close(_) => "architecture/contracts/schemas/ipc/close.schema.json",
            _ => continue,
        };
        assert_valid(path, &value);
    }
    for capability in variants::<RequestedCapability>() {
        assert_valid(
            "architecture/contracts/schemas/ipc/capability.schema.json",
            &Value::String(capability),
        );
    }
    for code in variants::<TransportError>() {
        assert_valid(
            "architecture/contracts/schemas/ipc/error-code.schema.json",
            &Value::String(code),
        );
    }
}

// ---------------------------------------------------------------------
// The golden frames and the payload-fit invariant with its envelope.
// ---------------------------------------------------------------------

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
        .collect()
}

#[test]
fn the_golden_frames_decode_and_re_encode_byte_exact() {
    let fixture = json_at("fixtures/ipc-v3/ipc-v3-frame-golden.json");
    let vectors = fixture["vectors"].as_array().expect("vectors");
    let mut classes = BTreeSet::new();
    for v in vectors {
        let name = v["name"].as_str().expect("name");
        let wire = unhex(v["frame_hex"].as_str().expect("frame_hex"));
        let decoded = interweave_ipc_protocol::decode_frame(&wire).expect(name);
        assert_eq!(decoded.consumed, wire.len(), "{name}: one frame, whole");
        let frame = Frame::parse(&decoded.body).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(frame.encode().expect(name), wire, "{name}: byte-exact");
        // The typed layer reads what the envelope carried.
        match &frame {
            Frame::Request(request) => {
                let method = Method::parse(&request.method).expect(name);
                Request::decode(method, request.params.as_deref()).expect(name);
            }
            Frame::Event(event) => {
                event.event(supported()[0]).expect(name);
            }
            _ => {}
        }
        classes.insert(serde_json::to_value(&frame).expect("ser")["type"].to_string());
    }
    assert_eq!(classes.len(), 10, "the goldens cover every class");
}

#[test]
fn a_golden_request_re_encodes_byte_exact_from_its_typed_form() {
    // The envelope keeps the bytes; this is the stronger claim, that the
    // TYPED request emits the schema's key order the goldens froze.
    let fixture = json_at("fixtures/ipc-v3/ipc-v3-frame-golden.json");
    for v in fixture["vectors"].as_array().expect("vectors") {
        let body = v["body"].to_string();
        let Ok(Frame::Request(frame)) = Frame::parse(&body) else {
            continue;
        };
        let method = Method::parse(&frame.method).expect("known");
        let typed = Request::decode(method, frame.params.as_deref()).expect("typed");
        let again = Frame::Request(typed.into_frame(frame.id.clone(), frame.deadline_ms));
        assert_eq!(
            again.encode().expect("encodes"),
            unhex(v["frame_hex"].as_str().expect("hex")),
            "{}",
            v["name"]
        );
    }
}

#[test]
fn the_largest_legal_payload_fits_with_its_whole_envelope() {
    // The payload-fit vectors measure the schema-defined object alone;
    // frame 2.0.0 now models the envelope around it, so the envelope is
    // measured here at ITS ceilings -- the widest request id, deadline and
    // sequence -- and added to the fixture's worst case.
    let fixture = json_at("fixtures/ipc-v3/ipc-v3-payload-fit.json");
    let worst = |direction: &str| -> usize {
        fixture["vectors"]
            .as_array()
            .expect("vectors")
            .iter()
            .filter(|v| v["direction"] == direction)
            .map(|v| usize::try_from(v["body_bytes"].as_u64().expect("bytes")).expect("usize"))
            .max()
            .expect("a vector")
    };
    // The widest id a SENDER may write: 128 characters, each as a
    // surrogate-pair escape (`\ud834\udd1e`, twelve bytes) -- wider than
    // a control character's six-byte escape or any character's UTF-8.
    // This crate never writes that form, so the envelope is measured
    // around 128 one-byte characters and widened by the difference.
    let widest_id_bytes: usize = 128 * 12;
    let narrow_id = RequestId::new("i".repeat(128)).expect("id");
    let small = Request::ChannelJoin(ChannelParams { channel: channel() });
    let params_len = small.params().get().len();
    let request_frame = Frame::Request(small.into_frame(narrow_id, Some(u64::MAX))).to_body();
    let escaped = r#"{"id":"\ud834\udd1e"}"#;
    assert_eq!(
        serde_json::from_str::<Value>(escaped).expect("json")["id"],
        Value::String("\u{1D11E}".to_owned()),
        "a surrogate-pair escape is one character"
    );
    let request_envelope = request_frame.len() - params_len - 128 + widest_id_bytes;
    assert!(
        worst("send-params") + request_envelope <= MAX_BODY_BYTES,
        "a maximal direct.send is {} bytes",
        worst("send-params") + request_envelope
    );

    let event = every_event().remove(0);
    let frame = event.into_frame(u64::MAX);
    let data_len = frame.data.as_deref().expect("data").get().len();
    let event_envelope = Frame::Event(frame).to_body().len() - data_len;
    assert!(
        worst("message-received") + event_envelope <= MAX_BODY_BYTES,
        "a maximal message.direct is {} bytes",
        worst("message-received") + event_envelope
    );
}

// ---------------------------------------------------------------------
// Parsing is as strict as the schemas at every depth (#147 review, F2).
// ---------------------------------------------------------------------

/// `base` with the value at `pointer` replaced.
fn with(base: &Value, pointer: &str, replacement: Value) -> Value {
    let mut value = base.clone();
    *value
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("{pointer} in {base}")) = replacement;
    value
}

/// Every nested object a frame carries refuses an array, and every enum
/// refuses `{"Variant": null}`, through `Frame::parse` -- each beside its
/// base, which parses, so the one replaced field is what is refused.
#[test]
fn a_frame_refuses_an_array_for_an_object_and_an_object_for_an_enum() {
    use serde_json::json;
    let hello = json!({"type": "hello", "ipc_version": {"major": 3, "minor": 0},
        "client": {"kind": "k"}, "endpoint": {"id": HUMAN},
        "requested_capabilities": ["events"]});
    let response = json!({"type": "hello_response", "ipc_version": {"major": 3, "minor": 0},
        "transport_contract_version": "3.0", "peer": PEER, "granted_capabilities": ["events"]});
    let close = json!({"type": "close", "code": "VersionIncompatible",
        "supported": [{"major": 3, "minor": 0}]});
    let failed = json!({"type": "response", "id": "1", "ok": false, "error": {"code": "Timeout"}});
    let state = json!({"type": "server_state", "health": "healthy",
        "connectivity": serde_json::to_value(connectivity()).expect("ser")});
    let cases = [
        (&hello, "/ipc_version", json!([2, 0])),
        (&hello, "/client", json!(["k"])),
        (&hello, "/endpoint", json!([HUMAN])),
        (&hello, "/requested_capabilities/0", json!({"events": null})),
        (&response, "/ipc_version", json!([2, 0])),
        (
            &response,
            "/granted_capabilities/0",
            json!({"events": null}),
        ),
        (&close, "/supported/0", json!([2, 0])),
        (&close, "/code", json!({"VersionIncompatible": null})),
        (&failed, "/error", json!(["Timeout"])),
        (&failed, "/error/code", json!({"Timeout": null})),
        (
            &state,
            "/connectivity",
            json!(["verified_public", "ready", 1, 2, 3, 0, "direct_first", 0]),
        ),
        (&state, "/health", json!({"healthy": null})),
        (
            &state,
            "/connectivity/relay_inbound",
            json!({"ready": null}),
        ),
    ];
    for base in [&hello, &response, &close, &failed, &state] {
        assert!(
            Frame::parse(&base.to_string()).is_ok(),
            "the base parses: {base}"
        );
    }
    for (base, pointer, replacement) in cases {
        let bad = with(base, pointer, replacement);
        assert_eq!(
            Frame::parse(&bad.to_string()).err(),
            Some(TransportError::ProtocolViolation),
            "{bad}"
        );
    }
    assert!(
        Frame::parse(r#"["hello"]"#).is_err(),
        "a whole body as an array"
    );
    assert!(Frame::parse(r#"["request", "1", "admin.status"]"#).is_err());
}

/// The same for what the envelope carries raw: params, event data and
/// results, read through `Request::decode`, `Event::decode` and
/// `ResponseFrame::outcome`.
#[test]
fn params_data_and_results_refuse_an_array_for_an_object() {
    use serde_json::json;
    let raw = |value: &Value| serde_json::value::to_raw_value(value).expect("raw");
    // A payload inside params.
    let send = json!({"peer": PEER, "message_id": "00000000000000000000000000000001",
        "payload": {"media_type": "text/plain", "bytes": "aGk"}});
    assert!(Request::decode(Method::DirectSend, Some(&raw(&send))).is_ok());
    let bad = with(&send, "/payload", json!(["text/plain", "aGk"]));
    assert_eq!(
        Request::decode(Method::DirectSend, Some(&raw(&bad))),
        Err(TransportError::InvalidArgument)
    );
    // A payload inside an event body.
    let event = every_event().remove(0).into_frame(0);
    let data: Value =
        serde_json::from_str(event.data.as_deref().expect("data").get()).expect("json");
    assert!(Event::decode(&event.event_type, Some(&raw(&data)), supported()[0]).is_ok());
    let bad = with(&data, "/payload", json!(["text/plain", "aGk"]));
    assert_eq!(
        Event::decode(&event.event_type, Some(&raw(&bad)), supported()[0]),
        Err(TransportError::ProtocolViolation)
    );
    // Results: each shape's array form, beside its object form.
    let outcome = |result: &Value| {
        let body = json!({"type": "response", "id": "1", "ok": true, "result": result});
        let Ok(Frame::Response(response)) = Frame::parse(&body.to_string()) else {
            panic!("a response: {body}")
        };
        response
    };
    assert!(outcome(&json!({})).outcome::<EmptyResult>().is_ok());
    assert!(outcome(&json!([])).outcome::<EmptyResult>().is_err());
    assert!(
        outcome(&json!({"resolved_endpoint": HUMAN}))
            .outcome::<SendResult>()
            .is_ok()
    );
    assert!(outcome(&json!([HUMAN])).outcome::<SendResult>().is_err());
    assert!(outcome(&json!({})).outcome::<SetEnabledResult>().is_ok());
    assert!(outcome(&json!([])).outcome::<SetEnabledResult>().is_err());
    assert!(
        outcome(&json!({"endpoints": [HUMAN], "ttl_ms": 1, "generated_at_ms": 2}))
            .outcome::<DirectoryResult>()
            .is_ok()
    );
    assert!(
        outcome(&json!([[HUMAN], 1, 2]))
            .outcome::<DirectoryResult>()
            .is_err()
    );
    let [(_, _), (_, _), (_, _), (_, admin), (_, list), ..] = &every_result()[..] else {
        unreachable!()
    };
    assert!(outcome(admin).outcome::<AdminStatusResult>().is_ok());
    assert!(
        outcome(&with(admin, "/ipc", json!([1, 2, 3])))
            .outcome::<AdminStatusResult>()
            .is_err()
    );
    assert!(
        outcome(&with(admin, "/health", json!({"degraded": null})))
            .outcome::<AdminStatusResult>()
            .is_err()
    );
    assert!(outcome(list).outcome::<EndpointList>().is_ok());
    assert!(
        outcome(&with(
            list,
            "/endpoints/0",
            json!([BOT, false, false, false])
        ))
        .outcome::<EndpointList>()
        .is_err()
    );
    assert!(
        outcome(&with(
            list,
            "/endpoints/1/lease",
            json!(["AAAAAAAAAAAAAAAAAAAAAQ", "k"])
        ))
        .outcome::<EndpointList>()
        .is_err()
    );
}

/// The bounds only the schemas state are read in the schemas' unit,
/// characters (#147 review, F4): a multi-byte id, reason class or client
/// kind the schema admits is admitted here too.
#[test]
fn a_schema_only_bound_counts_characters() {
    use serde_json::json;
    let id = "ა".repeat(128); // 384 bytes, 128 characters
    let request = json!({"type": "request", "id": id, "method": "admin.status"});
    assert_valid(
        "architecture/contracts/schemas/ipc/frame.schema.json",
        &request,
    );
    assert!(Frame::parse(&request.to_string()).is_ok());
    let over = json!({"type": "request", "id": "ა".repeat(129), "method": "admin.status"});
    assert!(!validator("architecture/contracts/schemas/ipc/frame.schema.json").is_valid(&over));
    assert!(Frame::parse(&over.to_string()).is_err());

    let class = "ა".repeat(128);
    let data = json!({"peer": PEER, "reason_class": class});
    let raw = serde_json::value::to_raw_value(&data).expect("raw");
    assert!(Event::decode("peer.disconnected", Some(&raw), supported()[0]).is_ok());

    let kind = "ა".repeat(64);
    let list = json!({"endpoints": [{"id": HUMAN, "enabled": true, "default": true,
        "persisted": false, "lease": {"epoch": "AAAAAAAAAAAAAAAA", "client_kind": kind}}]});
    assert_valid(
        "architecture/contracts/schemas/ipc/endpoint-list.schema.json",
        &list,
    );
    let body = json!({"type": "response", "id": "1", "ok": true, "result": list});
    let Ok(Frame::Response(response)) = Frame::parse(&body.to_string()) else {
        panic!("a response")
    };
    assert!(response.outcome::<EndpointList>().is_ok());
}

/// The `admin.peers` address shapes (ADR-0062) and their Rust mirror:
/// each holds what LOCAL-IPC.md says of it, a refusal tied to its named
/// reason both ways, a row's `not_dialled` tied to `dialled: false` both
/// ways, and a list held to the provider's bound -- the schema and the
/// parser refusing the same instances.
/// `trust-set-result` (2.10): `set`, or a refusal naming its rule from
/// the contract crate's closed set, never the two mixed; and the Rust
/// mirror agrees both ways.
#[test]
fn the_trust_set_result_holds_what_the_contract_says() {
    use interweave_local_client_api::{TrustChange, TrustRefusal};
    let result = "architecture/contracts/schemas/ipc/trust-set-result.schema.json";
    let refuses = |instance: Value| {
        assert!(
            !validator(result).is_valid(&instance),
            "{result} admits {instance}"
        );
        assert!(
            serde_json::from_value::<TrustSetResult>(instance.clone()).is_err(),
            "the mirror admits {instance}"
        );
    };
    assert_valid(result, &serde_json::json!({ "outcome": "set" }));
    for reason in TrustRefusal::ALL.map(TrustRefusal::as_str) {
        assert_valid(
            result,
            &serde_json::json!({ "outcome": "refused", "refused": reason, "detail": "why" }),
        );
    }
    refuses(serde_json::json!({}));
    refuses(serde_json::json!({ "outcome": "refused" }));
    refuses(serde_json::json!({ "outcome": "set", "refused": "superseded" }));
    refuses(serde_json::json!({ "outcome": "refused", "refused": "grammar" }));
    refuses(serde_json::json!({ "outcome": "unchanged" }));
    refuses(serde_json::json!({ "outcome": "set", "peer": "x" }));
    refuses(
        serde_json::json!({ "outcome": "refused", "refused": "superseded", "detail": "d".repeat(513) }),
    );

    let changes =
        std::iter::once(TrustChange::Set).chain(TrustRefusal::ALL.map(TrustChange::Refused));
    for change in changes {
        let wire = serde_json::to_value(TrustSetResult::from(change)).expect("ser");
        assert_valid(result, &wire);
        let back: TrustChange = serde_json::from_value::<TrustSetResult>(wire)
            .expect("parses")
            .into();
        assert_eq!(change, back);
    }
}

/// `trust-seed-params` and `trust-seed-result` (IPC 3.0, ADR-0067 decision
/// 6): the endpoint required and nothing else, the outcome closed at its
/// two values, the schema and the mirror refusing the same instances.
#[test]
fn the_trust_seed_shapes_hold_what_the_contract_says() {
    use interweave_local_client_api::TrustSeed;
    let params = "architecture/contracts/schemas/ipc/trust-seed-params.schema.json";
    let result = "architecture/contracts/schemas/ipc/trust-seed-result.schema.json";
    let seed = |v: &Value| {
        let raw = serde_json::value::to_raw_value(v).expect("raw");
        Request::decode(Method::AdminTrustSeed, Some(&raw)).is_ok()
    };
    let good = serde_json::json!({ "endpoint": HUMAN });
    assert_valid(params, &good);
    assert!(seed(&good), "the control");
    for bad in [
        serde_json::json!({}),
        serde_json::json!({ "endpoint": null }),
        serde_json::json!({ "endpoint": "human" }),
        serde_json::json!({ "endpoint": HUMAN, "peer": PEER }),
        serde_json::json!({ "endpoint": HUMAN.to_lowercase() }),
    ] {
        assert!(!validator(params).is_valid(&bad), "{params} admits {bad}");
        assert!(!seed(&bad), "the mirror admits {bad}");
    }

    for outcome in [TrustSeed::Seeded, TrustSeed::Present] {
        let wire = serde_json::to_value(TrustSeedResult::from(outcome)).expect("ser");
        assert_valid(result, &wire);
        let back: TrustSeed = serde_json::from_value::<TrustSeedResult>(wire)
            .expect("parses")
            .into();
        assert_eq!(outcome, back);
    }
    for bad in [
        serde_json::json!({}),
        serde_json::json!({ "outcome": "refused" }),
        serde_json::json!({ "outcome": "set" }),
        serde_json::json!({ "outcome": "seeded", "detail": "x" }),
    ] {
        assert!(!validator(result).is_valid(&bad), "{result} admits {bad}");
        assert!(
            serde_json::from_value::<TrustSeedResult>(bad.clone()).is_err(),
            "the mirror admits {bad}"
        );
    }
}

/// The endpoint dimension of `admin.trust.list` and `admin.trust.set`
/// params: `direction` is required with `endpoint` and refused without
/// it, the endpoint a derived id, every member refused as `null`, and the
/// schema and the mirror agree on each instance, a positive control beside
/// every refusal.
#[test]
fn the_trust_params_hold_the_endpoint_dimension_both_ways() {
    let list_schema = "architecture/contracts/schemas/ipc/trust-list-params.schema.json";
    let set_schema = "architecture/contracts/schemas/ipc/trust-set-params.schema.json";
    let decode = |method, v: &Value| {
        let raw = serde_json::value::to_raw_value(v).expect("raw");
        Request::decode(method, Some(&raw)).is_ok()
    };
    for (instance, legal) in [
        (serde_json::json!({}), true),
        (serde_json::json!({ "after": PEER }), true),
        (
            serde_json::json!({ "endpoint": HUMAN, "direction": "inbound" }),
            true,
        ),
        (
            serde_json::json!({ "after": PEER, "endpoint": HUMAN, "direction": "outbound" }),
            true,
        ),
        (serde_json::json!({ "endpoint": HUMAN }), false),
        (serde_json::json!({ "direction": "inbound" }), false),
        (
            serde_json::json!({ "endpoint": HUMAN, "direction": "both" }),
            false,
        ),
        (
            serde_json::json!({ "endpoint": HUMAN, "direction": null }),
            false,
        ),
        (
            serde_json::json!({ "endpoint": null, "direction": "inbound" }),
            false,
        ),
        (
            serde_json::json!({ "endpoint": "human", "direction": "inbound" }),
            false,
        ),
        (
            serde_json::json!({ "endpoint": HUMAN, "direction": "inbound", "set": "own" }),
            false,
        ),
    ] {
        assert_eq!(
            validator(list_schema).is_valid(&instance),
            legal,
            "schema on {instance}"
        );
        assert_eq!(
            decode(Method::AdminTrustList, &instance),
            legal,
            "mirror on {instance}"
        );
    }
    for (instance, legal) in [
        (serde_json::json!({ "peer": PEER, "allowed": true }), true),
        (
            serde_json::json!({ "peer": PEER, "allowed": true, "endpoint": HUMAN }),
            true,
        ),
        (
            serde_json::json!({ "peer": PEER, "allowed": true, "endpoint": null }),
            false,
        ),
        (
            serde_json::json!({ "peer": PEER, "allowed": true, "endpoint": "human" }),
            false,
        ),
        (
            serde_json::json!({ "peer": PEER, "allowed": true, "endpoint": HUMAN, "direction": "inbound" }),
            false,
        ),
    ] {
        assert_eq!(
            validator(set_schema).is_valid(&instance),
            legal,
            "schema on {instance}"
        );
        assert_eq!(
            decode(Method::AdminTrustSet, &instance),
            legal,
            "mirror on {instance}"
        );
    }
}

/// `trust-list` 1.2.0's `set`: an endpoint answer has no `local_peer`,
/// an `own` set's rows are administered and persisted, and the mirror and
/// the schema refuse the same pages; the constructor of an endpoint answer
/// sets `set`, so the converse the schema cannot state holds of every
/// page this crate writes.
#[test]
fn a_trust_list_set_holds_what_the_contract_says() {
    use interweave_local_client_api::{
        EndpointSetKind, EndpointTrustView, TrustSource, TrustedPeer,
    };
    let schema = "architecture/contracts/schemas/ipc/trust-list.schema.json";
    let admin = serde_json::json!({"peer": PEER, "persisted": true, "source": "administered"});
    let configured = serde_json::json!({"peer": PEER, "persisted": true, "source": "configured"});
    let both = |instance: &Value, legal: bool| {
        assert_eq!(
            validator(schema).is_valid(instance),
            legal,
            "schema on {instance}"
        );
        assert_eq!(
            serde_json::from_value::<TrustList>(instance.clone()).is_ok(),
            legal,
            "mirror on {instance}"
        );
    };
    for kind in ["own", "authored", "inherited"] {
        both(
            &serde_json::json!({ "set": kind, "allowed": [admin.clone()] }),
            true,
        );
        both(
            &serde_json::json!({ "set": kind, "allowed": [], "local_peer": PEER }),
            false,
        );
    }
    both(
        &serde_json::json!({ "local_peer": PEER, "allowed": [] }),
        true,
    );
    both(
        &serde_json::json!({ "set": "authored", "allowed": [configured.clone()] }),
        true,
    );
    both(
        &serde_json::json!({ "set": "inherited", "allowed": [configured.clone()] }),
        true,
    );
    both(
        &serde_json::json!({ "set": "own", "allowed": [configured] }),
        false,
    );
    both(
        &serde_json::json!({ "set": "own", "allowed": [{"peer": PEER, "persisted": false}] }),
        false,
    );
    both(
        &serde_json::json!({ "set": "everyone", "allowed": [] }),
        false,
    );
    both(&serde_json::json!({ "set": null, "allowed": [] }), false);

    for kind in EndpointSetKind::ALL {
        let page = TrustList::page_endpoint(
            EndpointTrustView {
                set: kind,
                allowed: vec![TrustedPeer {
                    peer: TransportIdentity::parse(PEER).expect("peer"),
                    persisted: true,
                    source: if kind == EndpointSetKind::Authored {
                        TrustSource::Configured
                    } else {
                        TrustSource::Administered
                    },
                }],
            },
            None,
        );
        let wire = serde_json::to_value(&page).expect("ser");
        assert_eq!(wire["set"], kind.as_str());
        assert!(wire.get("local_peer").is_none(), "{wire}");
        both(&wire, true);
    }
}

/// A superseded write is a refusal by name, in both address results: on
/// 3.0 the 2.10 shape is the only one, so no connection is answered the
/// error `Internal` for it.
#[test]
fn superseded_is_named_in_both_address_results() {
    let superseded = || AddressChange::Refused {
        reason: AddressRefusal::Superseded,
        detail: "a later connection's write stands".to_owned(),
    };
    let named = AddressResult::from(superseded());
    assert_valid(
        "architecture/contracts/schemas/ipc/peers-address-result.schema.json",
        &serde_json::to_value(&named).expect("ser"),
    );
    assert_eq!(AddressChange::from(named), superseded());
    // And `own_peer`, which 2.9 brought, by its own name: never `grammar`.
    let own = AddressChange::Refused {
        reason: AddressRefusal::OwnPeer,
        detail: String::new(),
    };
    assert_eq!(AddressChange::from(AddressResult::from(own.clone())), own);

    let listed = || AddressesChange::Refused {
        reason: AddressRefusal::Superseded,
        line: Some(1),
        detail: String::new(),
    };
    let wire =
        serde_json::to_value(AddressesResult::from_view(listed(), 2).expect("named")).expect("ser");
    assert_valid(
        "architecture/contracts/schemas/ipc/peers-addresses-result.schema.json",
        &wire,
    );
    assert_eq!(
        AddressesChange::from(serde_json::from_value::<AddressesResult>(wire).expect("parses")),
        listed()
    );
}

#[test]
fn the_peers_address_shapes_hold_what_the_contract_says() {
    let params = "architecture/contracts/schemas/ipc/peers-address-params.schema.json";
    let result = "architecture/contracts/schemas/ipc/peers-address-result.schema.json";
    let list = "architecture/contracts/schemas/ipc/peers-address-list.schema.json";
    let refuses = |path: &str, instance: Value| {
        assert!(
            !validator(path).is_valid(&instance),
            "{path} admits {instance}"
        );
    };
    let peer = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
    let address = format!("/ip4/192.168.1.20/tcp/4001/p2p/{peer}");

    assert_valid(params, &serde_json::json!({ "address": address }));
    refuses(params, serde_json::json!({}));
    refuses(params, serde_json::json!({ "address": "" }));
    refuses(params, serde_json::json!({ "address": "x".repeat(1025) }));
    refuses(
        params,
        serde_json::json!({ "address": address, "source": "scan" }),
    );

    for outcome in ["added", "removed", "unchanged"] {
        assert_valid(result, &serde_json::json!({ "outcome": outcome }));
        refuses(
            result,
            serde_json::json!({ "outcome": outcome, "refused": "bound" }),
        );
    }
    // Every reason the contract crate names, so one added there and not
    // to the schema fails here.
    for reason in interweave_local_client_api::AddressRefusal::ALL
        .map(interweave_local_client_api::AddressRefusal::as_str)
    {
        assert_valid(
            result,
            &serde_json::json!({ "outcome": "refused", "refused": reason, "detail": "why" }),
        );
    }
    refuses(result, serde_json::json!({ "outcome": "refused" }));
    refuses(
        result,
        serde_json::json!({ "outcome": "refused", "refused": "dns" }),
    );

    let row = |source: &str, dialled: bool| serde_json::json!({ "address": address, "source": source, "dialled": dialled });
    assert_valid(list, &serde_json::json!({ "addresses": [] }));
    assert_valid(
        list,
        &serde_json::json!({ "addresses": [
            row("authored", true),
            { "address": address, "source": "added", "dialled": false,
              "not_dialled": "provider_disabled" }
        ] }),
    );
    refuses(
        list,
        serde_json::json!({ "addresses": vec![row("added", true); 65] }),
    );
    refuses(
        list,
        serde_json::json!({ "addresses": [row("scanned", true)] }),
    );

    // THE RUST MIRROR: what the server builds validates, and what the
    // schema refuses a client refuses too.
    let changes = [
        AddressChange::Added,
        AddressChange::Removed,
        AddressChange::Unchanged,
        AddressChange::Refused {
            reason: AddressRefusal::SpecialUse,
            detail: "d".repeat(600),
        },
    ];
    for change in changes {
        let wire = serde_json::to_value(AddressResult::from(change.clone())).expect("ser");
        assert_valid(result, &wire);
        let back: AddressChange = serde_json::from_value::<AddressResult>(wire)
            .expect("parses")
            .into();
        match (&change, back) {
            (
                AddressChange::Refused { reason, .. },
                AddressChange::Refused { reason: r, detail },
            ) => {
                assert_eq!(*reason, r);
                assert_eq!(
                    detail.chars().count(),
                    512,
                    "a long detail is cut to the bound"
                );
            }
            (change, back) => assert_eq!(*change, back),
        }
    }
    for wrong in [
        serde_json::json!({ "outcome": "refused" }),
        serde_json::json!({ "outcome": "added", "refused": "bound" }),
        serde_json::json!({ "outcome": "refused", "refused": "dns" }),
        serde_json::json!({ "outcome": "moved" }),
        serde_json::json!({ "outcome": "refused", "refused": "bound", "detail": "d".repeat(513) }),
        // An explicit null is not an absent key: refused by both.
        serde_json::json!({ "outcome": "added", "refused": null }),
        serde_json::json!({ "outcome": "refused", "refused": "bound", "detail": null }),
    ] {
        refuses(result, wrong.clone());
        assert!(serde_json::from_value::<AddressResult>(wrong).is_err());
    }
    let views = vec![
        BootstrapAddress {
            address: address.clone(),
            source: AddressSource::Authored,
            dialled: true,
        },
        BootstrapAddress {
            address: address.clone(),
            source: AddressSource::Added,
            dialled: false,
        },
    ];
    let wire =
        serde_json::to_value(AddressList::from_views(views.clone()).expect("rows")).expect("ser");
    assert_valid(list, &wire);
    let back: Vec<BootstrapAddress> = serde_json::from_value::<AddressList>(wire)
        .expect("parses")
        .addresses
        .into_iter()
        .map(Into::into)
        .collect();
    assert_eq!(back, views);
    assert!(AddressList::from_views(vec![views[0].clone(); 65]).is_err());
    for wrong in [
        serde_json::json!({ "addresses": vec![row("added", true); 65] }),
        serde_json::json!({ "addresses": [row("scanned", true)] }),
        // `not_dialled` without `dialled: false`, and the reverse.
        serde_json::json!({ "addresses": [{ "address": address, "source": "added",
            "dialled": true, "not_dialled": "provider_disabled" }] }),
        serde_json::json!({ "addresses": [row("added", false)] }),
        serde_json::json!({ "addresses": [{ "address": address, "source": "added",
            "dialled": true, "not_dialled": null }] }),
    ] {
        refuses(list, wrong.clone());
        assert!(serde_json::from_value::<AddressList>(wrong).is_err());
    }
}

/// The show side's read (ADR-0062 decision 4), `approved` before its
/// wire: one ordered list whose rows name their kind from a closed set,
/// held to its 64-row bound, with this node's `PeerId` and whether rows
/// were dropped -- and the Rust mirror agrees both ways: what it writes
/// validates and reads back, and every instance the schema refuses it
/// refuses too.
#[test]
fn the_own_addresses_shape_holds_what_the_contract_says() {
    let own = "architecture/contracts/schemas/ipc/peers-own-addresses.schema.json";
    let refuses = |instance: Value| {
        assert!(
            !validator(own).is_valid(&instance),
            "{own} admits {instance}"
        );
        assert!(
            serde_json::from_value::<OwnAddressList>(instance.clone()).is_err(),
            "the mirror admits {instance}"
        );
    };
    let peer = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
    let row = |kind: &str| serde_json::json!({ "address": format!("/ip4/192.168.1.20/tcp/4001/p2p/{peer}"), "kind": kind });

    assert_valid(
        own,
        &serde_json::json!({ "peer": peer, "addresses": [], "truncated": false }),
    );
    assert_valid(
        own,
        &serde_json::json!({ "peer": peer, "addresses": [
            row("confirmed_external"), row("relay_circuit"), row("listen")
        ], "truncated": false }),
    );
    assert_valid(
        own,
        &serde_json::json!({ "peer": peer, "addresses": vec![row("listen"); 64], "truncated": true }),
    );
    refuses(
        serde_json::json!({ "peer": peer, "addresses": vec![row("listen"); 65], "truncated": true }),
    );
    refuses(
        serde_json::json!({ "peer": peer, "addresses": [row("observed")], "truncated": false }),
    );
    refuses(serde_json::json!({ "peer": peer, "addresses": [] }));
    refuses(serde_json::json!({ "addresses": [], "truncated": false }));
    refuses(serde_json::json!({ "peer": "not-a-peer", "addresses": [], "truncated": false }));
    refuses(serde_json::json!({ "peer": peer, "addresses": [
        { "address": "", "kind": "listen" }
    ], "truncated": false }));

    let view = |kind: OwnAddressKind| OwnAddress {
        address: format!("/ip4/192.168.1.20/tcp/4001/p2p/{peer}"),
        kind,
    };
    let answered = OwnAddresses {
        peer: TransportIdentity::parse(peer.to_owned()).expect("peer"),
        addresses: vec![
            view(OwnAddressKind::ConfirmedExternal),
            view(OwnAddressKind::RelayCircuit),
            view(OwnAddressKind::Listen),
        ],
        truncated: false,
    };
    let wire = serde_json::to_value(OwnAddressList::from_view(answered.clone()).expect("in bound"))
        .expect("ser");
    assert_valid(own, &wire);
    assert_eq!(
        wire["addresses"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|r| r["kind"].as_str().expect("kind"))
            .collect::<Vec<_>>(),
        ["confirmed_external", "relay_circuit", "listen"],
        "the wire names are OwnAddressKind::as_str's"
    );
    for kind in [
        OwnAddressKind::ConfirmedExternal,
        OwnAddressKind::RelayCircuit,
        OwnAddressKind::Listen,
    ] {
        let one = OwnAddressList::from_view(OwnAddresses {
            addresses: vec![view(kind)],
            ..answered.clone()
        })
        .expect("in bound");
        assert_eq!(
            serde_json::to_value(&one).expect("ser")["addresses"][0]["kind"],
            kind.as_str()
        );
    }
    let back: OwnAddressList = serde_json::from_value(wire).expect("reads back");
    assert_eq!(OwnAddresses::from(back), answered);
    assert_eq!(
        OwnAddressList::from_view(OwnAddresses {
            addresses: vec![view(OwnAddressKind::Listen); 65],
            ..answered
        }),
        Err(TransportError::Internal),
        "a port past the bound is answered Internal, never cut"
    );
}

/// The all-or-none add (ADR-0062 decision 4): 1 to 16 lines in, and out
/// either one row per line, each added or unchanged, or a refusal naming
/// its rule and -- for every rule but the provider's, which refuses the
/// list whole -- the line's index, the two halves never mixed. And the
/// Rust mirror agrees both ways: every instance the schema refuses it
/// refuses too, what the server builds validates and reads back, and a
/// port answer the contract cannot carry is `Internal`, never mended.
#[test]
fn the_all_or_none_add_shapes_hold_what_the_contract_says() {
    let params = "architecture/contracts/schemas/ipc/peers-addresses-params.schema.json";
    let result = "architecture/contracts/schemas/ipc/peers-addresses-result.schema.json";
    let refuses = |path: &str, instance: Value| {
        assert!(
            !validator(path).is_valid(&instance),
            "{path} admits {instance}"
        );
        let mirror = if path == params {
            serde_json::from_value::<AddressesParams>(instance.clone()).is_err()
        } else {
            serde_json::from_value::<AddressesResult>(instance.clone()).is_err()
        };
        assert!(mirror, "the mirror admits {instance}");
    };
    let peer = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
    let address = format!("/ip4/192.168.1.20/tcp/4001/p2p/{peer}");

    assert_valid(params, &serde_json::json!({ "addresses": [address] }));
    assert_valid(
        params,
        &serde_json::json!({ "addresses": vec![address.clone(); 16] }),
    );
    refuses(
        params,
        serde_json::json!({ "addresses": vec![address.clone(); 17] }),
    );
    refuses(params, serde_json::json!({ "addresses": [] }));
    refuses(params, serde_json::json!({}));
    refuses(params, serde_json::json!({ "addresses": [""] }));
    refuses(
        params,
        serde_json::json!({ "addresses": ["x".repeat(1025)] }),
    );
    refuses(params, serde_json::json!({ "addresses": address }));
    refuses(
        params,
        serde_json::json!({ "addresses": [address], "source": "scan" }),
    );

    let row = |outcome: &str| serde_json::json!({ "outcome": outcome });
    assert_valid(
        result,
        &serde_json::json!({ "outcome": "added", "lines": [row("added"), row("unchanged")] }),
    );
    assert_valid(
        result,
        &serde_json::json!({ "outcome": "added", "lines": vec![row("unchanged"); 16] }),
    );
    refuses(result, serde_json::json!({ "outcome": "added" }));
    refuses(
        result,
        serde_json::json!({ "outcome": "added", "lines": [] }),
    );
    refuses(
        result,
        serde_json::json!({ "outcome": "added", "lines": vec![row("added"); 17] }),
    );
    refuses(
        result,
        serde_json::json!({ "outcome": "added", "lines": [row("removed")] }),
    );
    refuses(
        result,
        serde_json::json!({ "outcome": "added", "lines": [row("added")], "refused": "bound", "index": 0 }),
    );
    refuses(
        result,
        serde_json::json!({ "outcome": "added", "lines": [row("added")], "index": 0 }),
    );
    refuses(
        result,
        serde_json::json!({ "outcome": "unchanged", "lines": [row("unchanged")] }),
    );

    // Every reason that names a line: all but the provider's, which
    // refuses the list whole, and `authored`, which only a removal answers.
    for reason in interweave_local_client_api::AddressRefusal::ALL
        .into_iter()
        .filter(|r| {
            !matches!(
                r,
                interweave_local_client_api::AddressRefusal::ProviderDisabled
                    | interweave_local_client_api::AddressRefusal::Authored
            )
        })
        .map(interweave_local_client_api::AddressRefusal::as_str)
    {
        assert_valid(
            result,
            &serde_json::json!({ "outcome": "refused", "refused": reason, "index": 15, "detail": "why" }),
        );
        refuses(
            result,
            serde_json::json!({ "outcome": "refused", "refused": reason }),
        );
        refuses(
            result,
            serde_json::json!({ "outcome": "refused", "refused": reason, "index": 16 }),
        );
        refuses(
            result,
            serde_json::json!({ "outcome": "refused", "refused": reason, "index": 0, "lines": [row("added")] }),
        );
    }
    assert_valid(
        result,
        &serde_json::json!({ "outcome": "refused", "refused": "provider_disabled" }),
    );
    refuses(
        result,
        serde_json::json!({ "outcome": "refused", "refused": "provider_disabled", "index": 0 }),
    );
    refuses(
        result,
        serde_json::json!({ "outcome": "refused", "refused": "authored", "index": 0 }),
    );
    refuses(
        result,
        serde_json::json!({ "outcome": "refused", "index": 0 }),
    );
    refuses(
        result,
        serde_json::json!({ "outcome": "refused", "refused": "grammar", "index": 0, "detail": "d".repeat(513) }),
    );

    // THE RUST MIRROR, the server's side: each answer beside the length
    // of the list it answers.
    let line = |n: usize| AddressesChange::Refused {
        reason: AddressRefusal::Bound,
        line: Some(n),
        detail: "d".repeat(600),
    };
    let whole = |reason| AddressesChange::Refused {
        reason,
        line: None,
        detail: String::new(),
    };
    let at = |reason, n| AddressesChange::Refused {
        reason,
        line: Some(n),
        detail: String::new(),
    };
    for (change, lines) in [
        (
            AddressesChange::Added(vec![LineChange::Added, LineChange::Unchanged]),
            2,
        ),
        (AddressesChange::Added(vec![LineChange::Unchanged; 16]), 16),
        (line(15), 16),
        (whole(AddressRefusal::ProviderDisabled), 3),
    ] {
        let wire = serde_json::to_value(
            AddressesResult::from_view(change.clone(), lines).expect("carried"),
        )
        .expect("ser");
        assert_valid(result, &wire);
        let back: AddressesChange = serde_json::from_value::<AddressesResult>(wire)
            .expect("reads back")
            .into();
        match (&change, &back) {
            (
                AddressesChange::Refused {
                    reason,
                    line,
                    detail,
                },
                AddressesChange::Refused {
                    reason: r,
                    line: l,
                    detail: d,
                },
            ) => {
                assert_eq!((reason, line), (r, l));
                assert_eq!(d.chars().count(), detail.chars().count().min(512));
            }
            _ => assert_eq!(change, back),
        }
    }
    assert_eq!(
        serde_json::to_value(
            AddressesResult::from_view(
                AddressesChange::Added(vec![LineChange::Added, LineChange::Unchanged]),
                2,
            )
            .expect("carried")
        )
        .expect("ser")["lines"],
        serde_json::json!([{ "outcome": LineChange::Added.as_str() }, { "outcome": LineChange::Unchanged.as_str() }]),
        "the wire names are LineChange::as_str's"
    );
    for (broken, lines) in [
        (AddressesChange::Added(Vec::new()), 0),
        (AddressesChange::Added(vec![LineChange::Added; 2]), 3),
        (AddressesChange::Added(vec![LineChange::Added; 3]), 2),
        (AddressesChange::Added(vec![LineChange::Added; 17]), 17),
        (line(16), 17),
        (line(2), 2),
        (whole(AddressRefusal::Grammar), 1),
        (at(AddressRefusal::ProviderDisabled, 0), 1),
        (at(AddressRefusal::Authored, 0), 1),
    ] {
        assert_eq!(
            AddressesResult::from_view(broken.clone(), lines),
            Err(TransportError::Internal),
            "{broken:?} for {lines} lines"
        );
    }
}
