// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! An endpoint entry names its key file (ADR-0067 decisions 2, 3, 5 and
//! 7; ENDPOINTS.md section Configured endpoint set): each refusal by
//! name beside the entry it accepts, fed what a caller holds -- a
//! `config.yaml` and a real state directory -- through the loader the
//! daemon takes; the key states the load phase sets; the default and the
//! client resolver by alias and by id; and the filesystem-custody mint.

#![allow(clippy::expect_used, clippy::panic)]
#![cfg(target_os = "linux")]

use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use ed25519_dalek::SigningKey;
use interweave_profile_config::endpoint_key::{
    self, Custody, KeyFileError, KeyHeader, KeyNameError, RawRecord, RecordError,
};
use interweave_profile_config::endpoint_overlay::EndpointOverlay;
use interweave_profile_config::{
    ConfigError, EndpointAlias, EndpointKeyState, EndpointRemedy, KeyMissing, LoadError,
    NotDerivedEntry, ProfileConfig, ProfilePaths, ResolveEndpointError, XdgRoots,
    create_private_dir, resolve_endpoint,
};
use interweave_test_support::endpoint_keys::endpoint_id;
use interweave_transport_api::EndpointId;

fn private_tempdir() -> tempfile::TempDir {
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("tempdir")
}

fn paths(base: &Path) -> ProfilePaths {
    let roots = XdgRoots {
        config_home: base.join("config"),
        data_home: base.join("data"),
        state_home: base.join("state"),
        cache_home: base.join("cache"),
        runtime_dir: None,
    };
    ProfilePaths::resolve_offline("work", &roots).expect("paths")
}

/// A `config.yaml` whose endpoint entries are `entries`, YAML list items
/// at four spaces, and whose `endpoints` block carries `extra` lines.
fn document(entries: &str, extra: &str) -> String {
    format!(
        "schema_version: 2
profile:
  name: work
trust:
  policy: static-allowlist
  allowed_peers: []
endpoints:
{extra}  entries:
{entries}"
    )
}

/// An entry naming `key`, with `rest` (alias and the like) after it.
fn entry(key: &str, rest: &str) -> String {
    format!("    - key: {key}\n{rest}      enabled: true\n      advertise: false\n")
}

fn key_of(id: &EndpointId) -> String {
    endpoint_key::key_file_name(id)
}

fn write(paths: &ProfilePaths, text: &str) {
    create_private_dir(paths.config_dir()).expect("config dir");
    std::fs::write(paths.config_file(), text).expect("write");
    std::fs::set_permissions(paths.config_file(), std::fs::Permissions::from_mode(0o644))
        .expect("chmod");
}

fn load_invalid(paths: &ProfilePaths) -> Vec<ConfigError> {
    match ProfileConfig::load(paths) {
        Err(LoadError::Invalid(errors)) => errors,
        other => panic!("expected Invalid, got {other:?}"),
    }
}

fn not_derived(errors: &[ConfigError]) -> Vec<&NotDerivedEntry> {
    errors
        .iter()
        .filter_map(|e| match e {
            ConfigError::EndpointIdNotDerived { found } => Some(found),
            _ => None,
        })
        .collect()
}

/// The control every refusal below stands beside: a `key` entry loads,
/// its id the stem, key-missing with no file.
#[test]
fn an_entry_naming_its_key_file_loads_as_the_stems_id() {
    let dir = private_tempdir();
    let p = paths(dir.path());
    let id = endpoint_id("human");
    write(
        &p,
        &document(&entry(&key_of(&id), "      alias: human\n"), ""),
    );
    let loaded = ProfileConfig::load(&p).expect("loads");
    let e = &loaded.endpoints.entries[0];
    assert_eq!(e.id, id);
    assert_eq!(e.alias, Some(EndpointAlias::parse("human").expect("alias")));
    assert_eq!(e.key_state, EndpointKeyState::Missing(KeyMissing::Absent));
}

/// The retired `id` member is refused `EndpointIdNotDerived`, naming per
/// entry the one command that applies: `adopt` where
/// `identity-keys/<old id>.key` is there -- judged by its presence
/// alone, a file nobody may read included -- and `init` where it is not,
/// with the entry to write.
#[test]
fn the_retired_id_names_adopt_or_init_by_the_old_files_presence() {
    let dir = private_tempdir();
    let p = paths(dir.path());
    write(
        &p,
        &document(
            "    - id: human\n      enabled: true\n      advertise: false\n    - id: claude\n      enabled: true\n      advertise: false\n",
            "",
        ),
    );
    let old_dir = p.state_dir().join("identity-keys");
    create_private_dir(&old_dir).expect("identity-keys");
    let old = old_dir.join("human.key");
    std::fs::write(&old, b"not read").expect("old key");
    // Nobody may read it, and the loader still knows it is there.
    std::fs::set_permissions(&old, std::fs::Permissions::from_mode(0o000)).expect("chmod");

    let errors = load_invalid(&p);
    let alias = |a: &str| EndpointAlias::parse(a).expect("alias");
    assert_eq!(
        not_derived(&errors),
        vec![
            &NotDerivedEntry::RetiredId {
                id: "human".to_owned(),
                remedy: Some(EndpointRemedy::Adopt {
                    profile: "work".to_owned(),
                    alias: alias("human"),
                }),
            },
            &NotDerivedEntry::RetiredId {
                id: "claude".to_owned(),
                remedy: Some(EndpointRemedy::Init {
                    profile: "work".to_owned(),
                    alias: alias("claude"),
                }),
            },
        ]
    );
    let text: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert!(
        text[0].contains("transportctl endpoint adopt --profile work --alias human")
            && text[0].contains("alias: human")
            && text[0].contains("key: <the EndpointId it prints>.key"),
        "{}",
        text[0]
    );
    assert!(
        text[1].contains("transportctl endpoint init --profile work --alias claude"),
        "{}",
        text[1]
    );
    std::fs::set_permissions(&old, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    assert_eq!(
        std::fs::read(&old).expect("still there"),
        b"not read",
        "nothing rewrote it"
    );

    // Parsed with no state directory to ask, no command is named rather
    // than a guess.
    match ProfileConfig::parse_yaml(&std::fs::read_to_string(p.config_file()).expect("read")) {
        Err(LoadError::Invalid(errors)) => assert!(
            not_derived(&errors)
                .iter()
                .all(|f| matches!(f, NotDerivedEntry::RetiredId { remedy: None, .. }))
        ),
        other => panic!("expected Invalid, got {other:?}"),
    }
}

/// Every `key` value that is not `<EndpointId>.key` is refused by name,
/// beside the control that a derived id's file name parses.
#[test]
fn a_key_value_of_any_other_shape_is_refused_by_name() {
    let id = endpoint_id("human");
    let good = key_of(&id);
    ProfileConfig::parse_yaml(&document(&entry(&good, ""), "")).expect("the control parses");
    for (what, key, reason) in [
        (
            "a separator",
            format!("endpoint-keys/{good}"),
            Some(KeyNameError::Separator),
        ),
        (
            "a climb",
            format!("../{good}"),
            Some(KeyNameError::Separator),
        ),
        (
            "a bare `..`",
            format!("..{good}"),
            Some(KeyNameError::ParentDir),
        ),
        (
            "a bare id",
            id.as_str().to_owned(),
            Some(KeyNameError::Suffix),
        ),
        (
            "another suffix",
            format!("{}.pem", id.as_str()),
            Some(KeyNameError::Suffix),
        ),
        // 52 characters that are no canonical encoding of an Ed25519 key.
        (
            "a non-derived stem",
            "12D3KoowK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5.key".to_owned(),
            None,
        ),
        (
            "a Qm stem",
            "QmURMJzEuRzqD29nvrJ65AqxBeUmniGNMVN2zuwNSESg6d.key".to_owned(),
            None,
        ),
        ("a label stem", "human.key".to_owned(), None),
    ] {
        match ProfileConfig::parse_yaml(&document(&entry(&key, ""), "")) {
            Err(LoadError::Invalid(errors)) => match not_derived(&errors).as_slice() {
                [NotDerivedEntry::KeyName { key: k, reason: r }] => {
                    assert_eq!(k, &key, "{what}");
                    match reason {
                        Some(reason) => assert_eq!(r, &reason, "{what}"),
                        None => assert!(matches!(r, KeyNameError::Stem(_)), "{what}: {r:?}"),
                    }
                }
                other => panic!("{what}: {other:?}"),
            },
            other => panic!("{what}: expected Invalid, got {other:?}"),
        }
    }
    // And serde's path, which a JSON reader takes, refuses the same entry.
    let json = serde_json::json!({"key": format!("a/{good}"), "enabled": true, "advertise": false});
    let err = serde_json::from_value::<interweave_profile_config::EndpointConfig>(json)
        .expect_err("refused")
        .to_string();
    assert!(err.contains("EndpointIdNotDerived"), "{err}");
}

/// A key file whose header derives another id than its name is refused
/// `EndpointKeyMismatch`: a real minted file, renamed under another id's
/// name. The control: the same file under its own name loads present.
#[test]
fn a_key_file_renamed_under_another_ids_name_is_refused() {
    let dir = private_tempdir();
    let p = paths(dir.path());
    let minted = endpoint_key::mint_filesystem(&p).expect("minted");
    write(&p, &document(&entry(&key_of(&minted), ""), ""));
    let loaded = ProfileConfig::load(&p).expect("the control loads");
    assert_eq!(
        loaded.endpoints.entries[0].key_state,
        EndpointKeyState::Present {
            custody: Custody::FilesystemOnly
        }
    );

    let other = endpoint_id("other");
    std::fs::rename(
        endpoint_key::key_path(&p, &minted),
        endpoint_key::key_path(&p, &other),
    )
    .expect("renamed");
    write(&p, &document(&entry(&key_of(&other), ""), ""));
    let errors = load_invalid(&p);
    assert_eq!(
        errors,
        vec![ConfigError::EndpointKeyMismatch {
            endpoint: other,
            header_derives: Some(minted),
        }]
    );
    assert!(errors[0].to_string().starts_with("EndpointKeyMismatch"));
}

/// A header whose own id is not its key's is refused the same way.
#[test]
fn a_key_file_whose_header_names_another_keys_id_is_refused() {
    let dir = private_tempdir();
    let p = paths(dir.path());
    let id = endpoint_id("human");
    let header =
        endpoint_key::header_bytes(Custody::FilesystemOnly, id.as_str().as_bytes(), &[9; 32])
            .expect("header");
    plant_key(&p, &id, &endpoint_key::encode_record(&header, &[0; 32]));
    write(&p, &document(&entry(&key_of(&id), ""), ""));
    assert_eq!(
        load_invalid(&p),
        vec![ConfigError::EndpointKeyMismatch {
            endpoint: id,
            header_derives: None,
        }]
    );
}

fn plant_key(p: &ProfilePaths, id: &EndpointId, line: &str) {
    interweave_profile_config::create_private_exclusive_within(
        &endpoint_key::key_path(p, id),
        line.as_bytes(),
        p.boundary(),
    )
    .expect("planted");
}

/// THE LOADER READS THE PUBLIC PART ALONE: a Keystore-custody file whose
/// body is garbage, and one cut off where the body starts, load present
/// with their custody -- a body this crate cannot open is never decoded.
#[test]
fn a_keystore_files_header_is_read_whatever_its_body() {
    let pk = SigningKey::from_bytes(&[4; 32]).verifying_key().to_bytes();
    let id = EndpointId::from_ed25519_public(&pk);
    let header =
        endpoint_key::header_bytes(Custody::Keystore, id.as_str().as_bytes(), &pk).expect("h");
    let whole = endpoint_key::encode_record(&header, &[0x5a; 60]);
    // 91 header bytes are covered by the first 124 characters.
    for line in [
        format!("{}~~garbage, not base64~~", &whole[..124]),
        whole[..124].to_owned(),
    ] {
        let dir = private_tempdir();
        let p = paths(dir.path());
        plant_key(&p, &id, &line);
        write(&p, &document(&entry(&key_of(&id), ""), ""));
        let loaded = ProfileConfig::load(&p).expect("loads");
        assert_eq!(
            loaded.endpoints.entries[0].key_state,
            EndpointKeyState::Present {
                custody: Custody::Keystore
            }
        );
    }
}

/// No file is the key-missing state with the stem's id; so is a file
/// nobody can read and a directory in the file's place -- never a
/// refusal, never present.
#[test]
fn an_absent_or_unreadable_key_file_loads_key_missing() {
    for case in ["absent", "mode 000", "a directory", "not a record"] {
        let dir = private_tempdir();
        let p = paths(dir.path());
        let id = endpoint_id("human");
        let path = endpoint_key::key_path(&p, &id);
        match case {
            "absent" => {}
            "mode 000" => {
                let _ = endpoint_key::mint_filesystem(&p).expect("a key dir");
                plant_key(&p, &id, "anything");
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
                    .expect("chmod");
            }
            "a directory" => {
                create_private_dir(&path).expect("a directory in its place");
            }
            _ => plant_key(&p, &id, "SURLMQ=="),
        }
        write(&p, &document(&entry(&key_of(&id), ""), ""));
        let loaded = ProfileConfig::load(&p).expect(case);
        let e = &loaded.endpoints.entries[0];
        assert_eq!(e.id, id, "{case}");
        match (case, e.key_state) {
            ("absent", EndpointKeyState::Missing(KeyMissing::Absent))
            | (
                "mode 000",
                EndpointKeyState::Missing(KeyMissing::Unreadable(KeyFileError::Io(
                    std::io::ErrorKind::PermissionDenied,
                ))),
            )
            | ("a directory", EndpointKeyState::Missing(KeyMissing::Unreadable(_)))
            | (
                "not a record",
                EndpointKeyState::Missing(KeyMissing::Unreadable(KeyFileError::Record(
                    RecordError::Truncated,
                ))),
            ) => {}
            (case, state) => panic!("{case}: {state:?}"),
        }
    }
}

/// `load_document` READS NO KEY FILE: with `endpoint-keys/` shut (mode
/// 000) it loads and every entry stays `NotRead`, where `load` -- the
/// control -- reads the header and answers key-missing, unreadable; and
/// with a key file whose header derives another id, where `load` is
/// refused `EndpointKeyMismatch`, it loads `NotRead` too. Its other
/// guards stand: a retired `id` entry is refused by name.
#[test]
fn load_document_reads_no_endpoint_key() {
    // Shut: the directory itself unsearchable.
    let dir = private_tempdir();
    let p = paths(dir.path());
    let minted = endpoint_key::mint_filesystem(&p).expect("minted");
    write(&p, &document(&entry(&key_of(&minted), ""), ""));
    let keys = p.state_dir().join(endpoint_key::ENDPOINT_KEYS_DIR);
    std::fs::set_permissions(&keys, std::fs::Permissions::from_mode(0o000)).expect("shut");
    let read = ProfileConfig::load(&p);
    let document_only = ProfileConfig::load_document(&p);
    std::fs::set_permissions(&keys, std::fs::Permissions::from_mode(0o700)).expect("open");
    assert!(
        matches!(
            read.expect("load loads").endpoints.entries[0].key_state,
            EndpointKeyState::Missing(KeyMissing::Unreadable(_))
        ),
        "the control: load read the shut directory"
    );
    let document_only = document_only.expect("load_document loads");
    assert_eq!(document_only.endpoints.entries[0].id, minted);
    assert_eq!(
        document_only.endpoints.entries[0].key_state,
        EndpointKeyState::NotRead
    );

    // A header deriving another id: the minted file under another name.
    let other = endpoint_id("other");
    std::fs::rename(
        endpoint_key::key_path(&p, &minted),
        endpoint_key::key_path(&p, &other),
    )
    .expect("renamed");
    write(&p, &document(&entry(&key_of(&other), ""), ""));
    assert_eq!(
        load_invalid(&p),
        vec![ConfigError::EndpointKeyMismatch {
            endpoint: other.clone(),
            header_derives: Some(minted),
        }],
        "the control: load refuses the mismatch"
    );
    let document_only = ProfileConfig::load_document(&p).expect("load_document loads");
    assert_eq!(document_only.endpoints.entries[0].id, other);
    assert_eq!(
        document_only.endpoints.entries[0].key_state,
        EndpointKeyState::NotRead
    );

    // The guards are load's: a retired entry is refused by name.
    write(
        &p,
        &document(
            "    - id: human\n      enabled: true\n      advertise: false\n",
            "",
        ),
    );
    match ProfileConfig::load_document(&p) {
        Err(LoadError::Invalid(errors)) => {
            assert!(!not_derived(&errors).is_empty(), "{errors:?}");
        }
        other => panic!("refused EndpointIdNotDerived: {other:?}"),
    }
}

/// An entry parsed and not loaded has read no file: its state is
/// `NotRead`, never shown as missing or present.
#[test]
fn a_parsed_entry_has_read_no_key() {
    let doc = document(&entry(&key_of(&endpoint_id("human")), ""), "");
    let parsed = ProfileConfig::parse_yaml(&doc).expect("parses");
    assert_eq!(
        parsed.endpoints.entries[0].key_state,
        EndpointKeyState::NotRead
    );
}

#[test]
fn an_invalid_alias_is_refused_alias_invalid() {
    let key = key_of(&endpoint_id("human"));
    ProfileConfig::parse_yaml(&document(&entry(&key, "      alias: human\n"), ""))
        .expect("the control");
    for bad in ["Human", "0human", "hu/man", "\"\""] {
        match ProfileConfig::parse_yaml(&document(
            &entry(&key, &format!("      alias: {bad}\n")),
            "",
        )) {
            Err(LoadError::Invalid(errors)) => assert!(
                matches!(errors.as_slice(), [ConfigError::AliasInvalid { .. }]),
                "{bad}: {errors:?}"
            ),
            other => panic!("{bad}: expected Invalid, got {other:?}"),
        }
    }
}

#[test]
fn an_alias_used_twice_is_refused_alias_taken() {
    let one = entry(&key_of(&endpoint_id("one")), "      alias: chat\n");
    let other = entry(&key_of(&endpoint_id("two")), "      alias: chat\n");
    let renamed = entry(&key_of(&endpoint_id("two")), "      alias: talk\n");
    let ok = ProfileConfig::parse_yaml(&document(&format!("{one}{renamed}"), "")).expect("ok");
    assert!(ok.validate().is_empty(), "the control: {:?}", ok.validate());
    let taken = ProfileConfig::parse_yaml(&document(&format!("{one}{other}"), "")).expect("parses");
    assert_eq!(
        taken.validate(),
        vec![ConfigError::AliasTaken {
            alias: EndpointAlias::parse("chat").expect("alias")
        }]
    );
}

/// `default_direct_endpoint` takes an alias or an id and holds the id;
/// an alias no entry carries is the unknown-default refusal, named.
#[test]
fn the_default_is_an_alias_or_an_id() {
    let id = endpoint_id("human");
    let entries = entry(&key_of(&id), "      alias: human\n");
    for default in ["human", id.as_str()] {
        let doc = document(&entries, &format!("  default_direct_endpoint: {default}\n"));
        let c = ProfileConfig::parse_yaml(&doc).expect(default);
        assert_eq!(
            c.endpoints.default_direct_endpoint,
            Some(id.clone()),
            "{default}"
        );
        assert!(c.validate().is_empty(), "{default}");
    }
    let doc = document(&entries, "  default_direct_endpoint: nobody\n");
    match ProfileConfig::parse_yaml(&doc) {
        Err(LoadError::Invalid(errors)) => assert_eq!(
            errors,
            vec![ConfigError::DefaultEndpointUnknown {
                selector: "nobody".to_owned()
            }]
        ),
        other => panic!("expected Invalid, got {other:?}"),
    }
    // An id no entry has is the same refusal, from `validate`.
    let stranger = endpoint_id("stranger");
    let doc = document(
        &entries,
        &format!("  default_direct_endpoint: {}\n", stranger.as_str()),
    );
    assert_eq!(
        ProfileConfig::parse_yaml(&doc).expect("parses").validate(),
        vec![ConfigError::DefaultEndpointUnknown {
            selector: stranger.as_str().to_owned()
        }]
    );
}

/// The client resolver: an alias or an id to the configured id, over the
/// configuration with the overlay applied; an alias, an id or a text
/// that names nothing is refused by name.
#[test]
fn a_client_resolves_an_alias_or_an_id_or_is_refused() {
    let human = endpoint_id("human");
    let off = endpoint_id("off");
    let doc = document(
        &format!(
            "{}{}",
            entry(&key_of(&human), "      alias: human\n"),
            entry(&key_of(&off), "      alias: off\n")
        ),
        "",
    );
    let c = ProfileConfig::parse_yaml(&doc).expect("parses");
    let overlay = EndpointOverlay::default()
        .set_enabled(&c.endpoints, &off, false)
        .expect("known")
        .expect("a change");
    let resolve = |s: &str| resolve_endpoint(&c.endpoints, &overlay, s);
    assert_eq!(resolve("human"), Ok(human.clone()));
    assert_eq!(resolve(human.as_str()), Ok(human));
    assert_eq!(resolve("off"), Ok(off), "disabled still resolves");
    assert_eq!(
        resolve("nobody"),
        Err(ResolveEndpointError::UnknownAlias(
            EndpointAlias::parse("nobody").expect("alias")
        ))
    );
    let stranger = endpoint_id("stranger");
    assert_eq!(
        resolve(stranger.as_str()),
        Err(ResolveEndpointError::UnknownId(stranger))
    );
    assert_eq!(
        resolve("Not An Id!"),
        Err(ResolveEndpointError::NotASelector("Not An Id!".to_owned()))
    );
}

/// The mint: each key new from the CSPRNG, written owner-only in an
/// owner-only directory, and its id the one the written key derives.
#[test]
fn a_minted_key_is_new_owner_only_and_derives_its_id() {
    let dir = private_tempdir();
    let p = paths(dir.path());
    let a = endpoint_key::mint_filesystem(&p).expect("minted");
    let b = endpoint_key::mint_filesystem(&p).expect("minted");
    assert_ne!(a, b, "two mints differ");
    let path = endpoint_key::key_path(&p, &a);
    let mode = |q: &Path| std::fs::metadata(q).expect("meta").permissions().mode() & 0o777;
    assert_eq!(mode(&path), 0o600);
    assert_eq!(mode(path.parent().expect("dir")), 0o700);
    assert_eq!(
        path.parent()
            .and_then(Path::file_name)
            .and_then(|n| n.to_str()),
        Some(endpoint_key::ENDPOINT_KEYS_DIR)
    );

    let line = std::fs::read_to_string(&path).expect("read");
    let bytes = endpoint_key::decode_line(&line).expect("base64");
    let raw = RawRecord::parse(&bytes).expect("a record");
    assert_eq!(raw.custody, Custody::FilesystemOnly);
    let secret: [u8; 32] = raw.body.try_into().expect("a 32-byte secret");
    let public = SigningKey::from_bytes(&secret).verifying_key().to_bytes();
    assert_eq!(raw.public_key, public);
    assert_eq!(EndpointId::from_ed25519_public(&public), a);
    assert_eq!(raw.endpoint, a.as_str().as_bytes());
    let header = KeyHeader::from_line(&line).expect("header");
    assert_eq!(header.endpoint(), &a);
    assert_eq!(
        endpoint_key::read_header(&p, &a).expect("reads"),
        Some(header)
    );
}

/// An entry built in code with a label is refused by `validate`: ids are
/// derived, never chosen.
#[test]
fn a_label_id_built_in_code_is_refused() {
    let doc = document(&entry(&key_of(&endpoint_id("human")), ""), "");
    let mut c = ProfileConfig::parse_yaml(&doc).expect("parses");
    assert!(c.validate().is_empty(), "the control");
    c.endpoints.entries[0].id = EndpointId::parse("human").expect("a label still parses");
    assert!(c.validate().iter().any(|e| matches!(
        e,
        ConfigError::EndpointIdNotDerived {
            found: NotDerivedEntry::Label { .. }
        }
    )));
}

/// Written back, an entry is the file's shape: `key` and `alias`, no
/// `id`, and it parses as itself.
#[test]
fn an_entry_serializes_in_the_files_shape() {
    let id = endpoint_id("human");
    let doc = document(
        &entry(&key_of(&id), "      alias: human\n"),
        "  default_direct_endpoint: human\n",
    );
    let c = ProfileConfig::parse_yaml(&doc).expect("parses");
    let text = serde_norway::to_string(&c.endpoints).expect("serializes");
    assert!(text.contains(&format!("key: {}", key_of(&id))), "{text}");
    assert!(text.contains("alias: human"), "{text}");
    assert!(
        !text.contains("\n  id:") && !text.contains("- id:"),
        "{text}"
    );
    let back: interweave_profile_config::EndpointsConfig =
        serde_norway::from_str(&text).expect("parses back");
    assert_eq!(back, c.endpoints);
}
