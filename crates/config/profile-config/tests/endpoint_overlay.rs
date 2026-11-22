// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The endpoint overlay on disk (ADR-0028 A 2026-10-11): each
//! normalisation at load with the file it rewrites, byte for byte; the
//! default/disabled predicate; the set moves; and every reason a present
//! overlay stops the start -- each beside the control that the same
//! file, made right, loads.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use interweave_profile_config::endpoint_overlay::{
    Carried, ENDPOINT_OVERLAY_FILE, EndpointOverlay, EndpointOverlayError,
    MAX_ENDPOINT_OVERLAY_BYTES, Normalised, OverlayMember,
};
use interweave_profile_config::{EndpointsConfig, MAX_ENDPOINTS, TrustBoundary};
use interweave_transport_api::EndpointId;

/// The TEST-ONLY derived id named `s` (tests/support): the entries below
/// carry `s` as their alias.
fn id(s: &str) -> EndpointId {
    interweave_test_support::endpoint_keys::endpoint_id(s)
}

/// The names the overlay texts below write in place of ids, so that each
/// reads as the case it is. The one rewrite that names two endpoints
/// names `a` and `b`, whose ids sort as the names do, so the rewrite the
/// overlay orders by id reads in the same order.
const NAMES: [&str; 4] = ["a", "b", "c", "gone"];

/// `text` with each quoted name replaced by its quoted id: what is put on
/// disk.
fn sub(text: &str) -> String {
    NAMES.iter().fold(text.to_owned(), |t, n| {
        t.replace(&format!("\"{n}\""), &format!("\"{}\"", id(n).as_str()))
    })
}

/// [`sub`] undone: what a file read back says, in names.
fn unsub(text: &str) -> String {
    NAMES.iter().fold(text.to_owned(), |t, n| {
        t.replace(&format!("\"{}\"", id(n).as_str()), &format!("\"{n}\""))
    })
}

#[test]
fn the_ids_of_a_and_b_sort_as_the_names_do() {
    assert!(id("a") < id("b"), "the two-endpoint rewrite's order");
}

/// `a` and `b` enabled, `c` disabled; the default `default`, an alias.
fn config(default: Option<&str>) -> EndpointsConfig {
    let key = |n: &str| interweave_profile_config::endpoint_key::key_file_name(&id(n));
    serde_json::from_value(serde_json::json!({
        "entries": [
            {"key": key("a"), "alias": "a", "enabled": true, "advertise": false},
            {"key": key("b"), "alias": "b", "enabled": true, "advertise": false},
            {"key": key("c"), "alias": "c", "enabled": false, "advertise": false},
        ],
        "default_direct_endpoint": default,
    }))
    .expect("an endpoints block")
}

/// A private state directory and the overlay's path in it.
fn state() -> (tempfile::TempDir, PathBuf) {
    let dir = private_tempdir().expect("a temporary directory");
    let path = dir.path().join(ENDPOINT_OVERLAY_FILE);
    (dir, path)
}

/// Write `text` at `path` as the daemon would: owner-only.
fn put(path: &Path, text: &str) {
    std::fs::write(path, sub(text)).expect("written");
    chmod(path, 0o600);
}

fn chmod(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
}

fn load(
    path: &Path,
    config: &EndpointsConfig,
) -> Result<(EndpointOverlay, Vec<Normalised>), EndpointOverlayError> {
    EndpointOverlay::load_within(path, config, &TrustBoundary::root())
}

fn read(path: &Path) -> String {
    unsub(&std::fs::read_to_string(path).expect("read"))
}

/// One normalisation case: what it is, the configured default, the file
/// loaded, what normalising reports, the file left, `a`/`b`/`c` enabled
/// and the effective default.
type Case = (
    &'static str,
    Option<&'static str>,
    &'static str,
    Vec<Normalised>,
    &'static str,
    [bool; 3],
    Option<&'static str>,
);

/// Each normalisation of (4) and the two default/disabled cases: the
/// file loaded, what normalising reports, the file it leaves -- byte for
/// byte -- and the effective state the runtime would start with.
#[test]
fn each_normalisation_rewrites_the_file_exactly() {
    let cases: [Case; 9] = [
        (
            "an enabled entry equal to config.yaml's",
            Some("a"),
            r#"{"enabled":{"a":true,"c":true}}"#,
            vec![Normalised::EnabledEqualsConfig(id("a"))],
            "{\n  \"enabled\": {\n    \"c\": true\n  }\n}",
            [true, true, true],
            Some("a"),
        ),
        (
            "an enabled entry naming an endpoint config.yaml lacks",
            Some("a"),
            r#"{"enabled":{"gone":false,"b":false}}"#,
            vec![Normalised::UnknownEndpointDropped(id("gone"))],
            "{\n  \"enabled\": {\n    \"b\": false\n  }\n}",
            [true, false, false],
            Some("a"),
        ),
        (
            "a default equal to the configured default",
            Some("a"),
            r#"{"enabled":{},"default":"a"}"#,
            vec![Normalised::DefaultEqualsConfig(Some(id("a")))],
            "{\n  \"enabled\": {}\n}",
            [true, true, false],
            Some("a"),
        ),
        (
            "a null default where config.yaml names none",
            None,
            r#"{"enabled":{},"default":null}"#,
            vec![Normalised::DefaultEqualsConfig(None)],
            "{\n  \"enabled\": {}\n}",
            [true, true, false],
            None,
        ),
        (
            "a default naming an endpoint config.yaml lacks",
            Some("a"),
            r#"{"enabled":{},"default":"gone"}"#,
            vec![Normalised::UnknownDefaultDropped(id("gone"))],
            "{\n  \"enabled\": {}\n}",
            [true, true, false],
            Some("a"),
        ),
        (
            "the configured default disabled by the overlay",
            Some("a"),
            r#"{"enabled":{"a":false}}"#,
            vec![Normalised::ConfiguredDefaultCleared(id("a"))],
            "{\n  \"enabled\": {\n    \"a\": false\n  },\n  \"default\": null\n}",
            [false, true, false],
            None,
        ),
        (
            "a default naming an endpoint config.yaml disables",
            Some("a"),
            r#"{"enabled":{},"default":"c"}"#,
            vec![Normalised::DisabledDefaultDropped(id("c"))],
            "{\n  \"enabled\": {}\n}",
            [true, true, false],
            Some("a"),
        ),
        (
            "a default naming an endpoint the same overlay disables",
            Some("a"),
            r#"{"enabled":{"b":false},"default":"b"}"#,
            vec![Normalised::DisabledDefaultDropped(id("b"))],
            "{\n  \"enabled\": {\n    \"b\": false\n  }\n}",
            [true, false, false],
            Some("a"),
        ),
        (
            "that, with the configured default disabled too",
            Some("a"),
            r#"{"enabled":{"a":false,"b":false},"default":"b"}"#,
            vec![
                Normalised::DisabledDefaultDropped(id("b")),
                Normalised::ConfiguredDefaultCleared(id("a")),
            ],
            "{\n  \"enabled\": {\n    \"a\": false,\n    \"b\": false\n  },\n  \"default\": null\n}",
            [false, false, false],
            None,
        ),
    ];
    for (what, default, text, changes, rewritten, enabled, effective_default) in cases {
        let (_dir, path) = state();
        put(&path, text);
        let config = config(default);
        let (overlay, got) = load(&path, &config).expect(what);
        assert_eq!(got, changes, "{what}");
        assert_eq!(read(&path), rewritten, "{what}: the rewrite");
        let effective = overlay.effective(&config);
        let on: Vec<bool> = ["a", "b", "c"]
            .iter()
            .map(|e| effective.enabled[&id(e)])
            .collect();
        assert_eq!(on, enabled, "{what}");
        assert_eq!(effective.default, effective_default.map(id), "{what}");
        // The rewrite is itself normalised: loading it changes nothing.
        let (again, none) = load(&path, &config).expect(what);
        assert_eq!(again, overlay, "{what}");
        assert!(none.is_empty(), "{what}: {none:?}");
    }
}

/// A normalised overlay is not rewritten: the file loaded is the file
/// left, byte for byte, in whatever layout it was written. Among them
/// the two the predicate keeps because it reads the EFFECTIVE state: an
/// overlay default that is enabled while the configured default is
/// disabled, and an overlay default `config.yaml` disables that the
/// overlay itself enables.
#[test]
fn a_normalised_overlay_is_not_rewritten() {
    for (text, default) in [
        (r#"{"enabled":{"a":false},"default":"b"}"#, "b"),
        (r#"{"enabled":{"c":true},"default":"c"}"#, "c"),
        (r#"{"enabled":{"a":false,"c":true},"default":"c"}"#, "c"),
    ] {
        let (_dir, path) = state();
        put(&path, text);
        let (overlay, changes) = load(&path, &config(Some("a"))).expect("loads");
        assert!(changes.is_empty(), "{text}: {changes:?}");
        assert_eq!(read(&path), text);
        let effective = overlay.effective(&config(Some("a")));
        assert_eq!(effective.default, Some(id(default)), "{text}");
    }
}

/// The READ-ONLY reader (a client that resolves aliases writes nothing
/// in the profile): on a file `load_within` rewrites -- the control,
/// which does -- `read_within` returns the same normalised overlay and
/// the same changes, and leaves the file byte-identical, its inode and
/// mode unchanged.
#[test]
fn read_within_normalises_in_memory_and_writes_nothing() {
    use std::os::unix::fs::MetadataExt as _;
    let text = r#"{"enabled":{"a":true,"c":true},"default":"a"}"#;
    let (_dir, path) = state();
    put(&path, text);
    let before = std::fs::read(&path).expect("read");
    let meta = std::fs::metadata(&path).expect("present");
    let (read_only, read_changes) =
        EndpointOverlay::read_within(&path, &config(Some("a")), &TrustBoundary::root())
            .expect("reads");
    assert!(!read_changes.is_empty(), "a case normalising changes");
    assert_eq!(
        std::fs::read(&path).expect("read"),
        before,
        "byte-identical"
    );
    let after = std::fs::metadata(&path).expect("present");
    assert_eq!(
        (after.ino(), after.mode(), after.mtime_nsec(), after.mtime()),
        (meta.ino(), meta.mode(), meta.mtime_nsec(), meta.mtime()),
        "not replaced"
    );
    // The control: `load_within` on the same file does rewrite it, to
    // the overlay the reader returned in memory.
    let (loaded, load_changes) = load(&path, &config(Some("a"))).expect("loads");
    assert_eq!((loaded, load_changes), (read_only, read_changes));
    assert_ne!(std::fs::read(&path).expect("read"), before, "rewritten");
    assert_eq!(read(&path), "{\n  \"enabled\": {\n    \"c\": true\n  }\n}");
    // And an absent file is the empty overlay, not created.
    let (_dir, absent) = state();
    let (empty, none) =
        EndpointOverlay::read_within(&absent, &config(Some("a")), &TrustBoundary::root())
            .expect("reads");
    assert_eq!((empty, none), (EndpointOverlay::default(), Vec::new()));
    assert!(!absent.exists());
}

/// An absent overlay is the empty one and is not created.
#[test]
fn an_absent_overlay_is_empty_and_is_not_created() {
    let (_dir, path) = state();
    let (overlay, changes) = load(&path, &config(Some("a"))).expect("loads");
    assert_eq!(overlay, EndpointOverlay::default());
    assert!(changes.is_empty());
    assert!(!path.exists());
    assert_eq!(overlay.apply(&config(Some("a"))), config(Some("a")));
}

/// The set moves: disabling the default carries both deltas in one
/// overlay; enabling it again restores nothing; a set back to the
/// configuration leaves the overlay empty; a set that changes nothing is
/// `None`; an unknown or disabled endpoint is refused as the runtime
/// refuses it.
#[test]
fn the_set_moves_record_only_what_differs() {
    let config = config(Some("a"));
    let empty = EndpointOverlay::default();

    let off = empty
        .set_enabled(&config, &id("a"), false)
        .expect("known")
        .expect("a change");
    let effective = off.effective(&config);
    assert!(!effective.enabled[&id("a")]);
    assert_eq!(effective.default, None, "disabling the default clears it");

    let on = off
        .set_enabled(&config, &id("a"), true)
        .expect("known")
        .expect("a change");
    assert_eq!(
        on.effective(&config).default,
        None,
        "enabling restores nothing"
    );
    let back = on
        .set_default(&config, Some(&id("a")))
        .expect("enabled")
        .expect("a change");
    assert_eq!(
        back, empty,
        "back to the configuration is the empty overlay"
    );

    assert!(
        empty
            .set_enabled(&config, &id("b"), true)
            .expect("known")
            .is_none()
    );
    assert!(
        empty
            .set_default(&config, Some(&id("a")))
            .expect("enabled")
            .is_none()
    );

    assert!(matches!(
        empty.set_enabled(&config, &id("gone"), false),
        Err(EndpointOverlayError::Unknown)
    ));
    assert!(matches!(
        empty.set_default(&config, Some(&id("gone"))),
        Err(EndpointOverlayError::Unknown)
    ));
    assert!(matches!(
        empty.set_default(&config, Some(&id("c"))),
        Err(EndpointOverlayError::Disabled)
    ));
    // The control: enabled by the overlay, `c` may be the default.
    let c_on = empty
        .set_enabled(&config, &id("c"), true)
        .expect("known")
        .expect("a change");
    assert!(c_on.set_default(&config, Some(&id("c"))).is_ok());

    let none = empty
        .set_default(&config, None)
        .expect("none")
        .expect("a change");
    assert_eq!(none.effective(&config).default, None);
}

/// A written overlay is owner-only and loads back as itself, unchanged.
#[test]
fn a_written_overlay_is_owner_only_and_round_trips() {
    use std::os::unix::fs::PermissionsExt as _;
    let (_dir, path) = state();
    let config = config(Some("a"));
    let overlay = EndpointOverlay::default()
        .set_enabled(&config, &id("a"), false)
        .expect("known")
        .expect("a change");
    overlay
        .write_within(&path, &TrustBoundary::root())
        .expect("written");
    let mode = std::fs::metadata(&path).expect("meta").permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    let (back, changes) = load(&path, &config).expect("loads");
    assert_eq!(back, overlay);
    assert!(changes.is_empty(), "{changes:?}");
}

/// Each reason a present overlay stops the start, beside the control
/// that a well-formed private file of the same contents loads.
#[test]
fn a_present_overlay_that_cannot_be_trusted_stops_the_load() {
    let config = config(Some("a"));
    let good = r#"{"enabled":{"b":false}}"#;

    let (_dir, path) = state();
    put(&path, good);
    assert!(load(&path, &config).is_ok(), "the control");

    for (what, text) in [
        ("not json", "enabled: {}"),
        ("an unknown member", r#"{"enabled":{},"extra":1}"#),
        ("enabled missing", r#"{"default":"a"}"#),
        ("enabled not an object", r#"{"enabled":["a"]}"#),
        ("not an endpoint id", r#"{"enabled":{"Not An Id":false}}"#),
        ("a state not a bool", r#"{"enabled":{"b":"off"}}"#),
        ("a default not an id", r#"{"enabled":{},"default":7}"#),
        ("an array, not the object", r"[{}]"),
        // Last-key-wins would drop the first map silently.
        (
            "a member named twice",
            r#"{"enabled":{},"enabled":{"b":false}}"#,
        ),
        (
            "an endpoint named twice",
            r#"{"enabled":{"b":false,"b":true}}"#,
        ),
    ] {
        let (_dir, path) = state();
        put(&path, text);
        let loaded = load(&path, &config);
        assert!(
            matches!(loaded, Err(EndpointOverlayError::Parse(_))),
            "{what}: {loaded:?}"
        );
        assert_eq!(
            read(&path),
            text,
            "{what}: a refused file is left as it was"
        );
    }

    for mode in [0o644, 0o620, 0o602, 0o640] {
        let (_dir, path) = state();
        put(&path, good);
        chmod(&path, mode);
        assert!(
            matches!(
                load(&path, &config),
                Err(EndpointOverlayError::NotPrivate { .. })
            ),
            "mode {mode:o}"
        );
    }

    {
        let (dir, path) = state();
        let real = dir.path().join("real.json");
        put(&real, good);
        std::os::unix::fs::symlink(&real, &path).expect("a link");
        assert!(matches!(
            load(&path, &config),
            Err(EndpointOverlayError::NotPrivate { .. })
        ));
    }

    {
        let (_dir, path) = state();
        let bound = usize::try_from(MAX_ENDPOINT_OVERLAY_BYTES).expect("fits");
        // At the bound it loads (the control); one byte past, refused.
        // Measured on the text as put, ids in place of names.
        let good = sub(good);
        put(&path, &format!("{good}{}", " ".repeat(bound - good.len())));
        assert!(load(&path, &config).is_ok(), "at the bound");
        put(
            &path,
            &format!("{good}{}", " ".repeat(bound - good.len() + 1)),
        );
        assert!(matches!(
            load(&path, &config),
            Err(EndpointOverlayError::TooLarge)
        ));
    }
}

/// A normalisation rewrite that fails stops the load, and the file is
/// left as it was: the stale entry could undo the operator's next
/// `config.yaml` edit. The same file in a writable directory is the
/// control.
#[test]
fn a_failed_normalisation_rewrite_stops_the_load() {
    let config = config(Some("a"));
    let text = r#"{"enabled":{"a":true}}"#;
    for writable in [true, false] {
        let (dir, path) = state();
        put(&path, text);
        if !writable {
            chmod(dir.path(), 0o500);
        }
        let loaded = load(&path, &config);
        chmod(dir.path(), 0o700);
        if writable {
            assert!(loaded.is_ok(), "the control: {loaded:?}");
        } else {
            assert!(
                matches!(loaded, Err(EndpointOverlayError::Write(_))),
                "{loaded:?}"
            );
            assert_eq!(read(&path), text);
        }
    }
}

/// The largest overlay a normalised file can hold -- every configured
/// endpoint, each flipped, the default `null` -- fits well inside the
/// size bound and loads as itself, so the bound refuses only what no set
/// could have written. Every id is derived, so every id is 52 bytes: the
/// longest an overlay can name, under `EndpointId::MAX_BYTES`.
#[test]
fn the_largest_overlay_fits_its_bound() {
    let ids: Vec<EndpointId> = (0..MAX_ENDPOINTS).map(|i| id(&format!("e{i}"))).collect();
    assert!(
        ids.iter()
            .all(|s| s.as_str().len() == 52 && s.as_str().len() <= EndpointId::MAX_BYTES)
    );
    let entries: Vec<_> = ids
        .iter()
        .map(|s| {
            serde_json::json!({
                "key": interweave_profile_config::endpoint_key::key_file_name(s),
                "enabled": true,
                "advertise": false,
            })
        })
        .collect();
    let config: EndpointsConfig = serde_json::from_value(serde_json::json!({
        "entries": entries,
        "default_direct_endpoint": ids[0].as_str(),
    }))
    .expect("an endpoints block");
    let mut overlay = EndpointOverlay::default();
    for s in &ids {
        overlay = overlay
            .set_enabled(&config, s, false)
            .expect("known")
            .expect("a change");
    }
    let (_dir, path) = state();
    overlay
        .write_within(&path, &TrustBoundary::root())
        .expect("written");
    let size = std::fs::metadata(&path).expect("meta").len();
    assert!(size < 7 * 1024, "{size} bytes");
    assert!(size < MAX_ENDPOINT_OVERLAY_BYTES);
    let (back, changes) = load(&path, &config).expect("loads");
    assert_eq!(back, overlay);
    assert!(changes.is_empty(), "{changes:?}");
}

/// `config()` plus a fourth entry aliased `claude`, enabled: the entry a
/// release before Stage 22's overlay named by its label.
fn config_with_claude() -> EndpointsConfig {
    let mut config = config(Some("a"));
    let key = interweave_profile_config::endpoint_key::key_file_name(&id("claude"));
    let extra: EndpointsConfig = serde_json::from_value(serde_json::json!({
        "entries": [{"key": key, "alias": "claude", "enabled": true, "advertise": false}],
    }))
    .expect("an endpoints block");
    config.entries.extend(extra.entries);
    config
}

/// Write `text` AS IS (no name substitution): a label overlay.
fn put_raw(path: &Path, text: &str) {
    std::fs::write(path, text).expect("written");
    chmod(path, 0o600);
}

fn claude() -> interweave_profile_config::EndpointAlias {
    interweave_profile_config::EndpointAlias::parse("claude").expect("an alias")
}

/// A label equal to a configured alias is REFUSED, by `load_within` and
/// `read_within` alike, in `enabled` and in `default`, naming the label
/// and the command; the file is left as it was -- a refusal that dropped
/// the member would lose the operator's disable. Controls: the same
/// disable under the id loads and is effective.
#[test]
fn a_label_member_naming_a_configured_alias_is_refused_by_name() {
    let config = config_with_claude();
    let claude_id = id("claude");
    for (what, text, member) in [
        (
            "enabled",
            r#"{"enabled":{"claude":false}}"#.to_owned(),
            OverlayMember::Enabled,
        ),
        (
            "default",
            r#"{"enabled":{},"default":"claude"}"#.to_owned(),
            OverlayMember::Default,
        ),
    ] {
        let (_dir, path) = state();
        put_raw(&path, &text);
        let before = std::fs::read(&path).expect("read");
        let refused = [
            load(&path, &config).expect_err("load refuses"),
            EndpointOverlay::read_within(&path, &config, &TrustBoundary::root())
                .expect_err("read refuses"),
        ];
        for e in refused {
            let shown = e.to_string();
            assert!(
                matches!(&e, EndpointOverlayError::LabelMember { label, member: m }
                    if label == "claude" && *m == member),
                "{what}: {e:?}"
            );
            assert!(shown.contains("'claude'"), "{what}: {shown}");
            assert!(
                shown.contains("transportctl endpoint adopt --profile <profile> --alias claude"),
                "{what}: {shown}"
            );
        }
        assert_eq!(std::fs::read(&path).expect("read"), before, "{what}: kept");
    }
    // The control: the same disable written under the id.
    let (_dir, path) = state();
    put_raw(
        &path,
        &format!(r#"{{"enabled":{{"{}":false}}}}"#, claude_id.as_str()),
    );
    let (overlay, changes) = load(&path, &config).expect("loads");
    assert!(changes.is_empty(), "{changes:?}");
    assert!(!overlay.effective(&config).enabled[&claude_id]);
}

/// A label equal to NO configured alias is the retired endpoint's: it
/// leaves as an unconfigured id does, reported with its own words, and
/// `load_within` rewrites. Control: what stands beside it is kept.
#[test]
fn a_label_member_naming_no_alias_is_dropped_and_reported() {
    let (_dir, path) = state();
    put(
        &path,
        r#"{"enabled":{"claude":false,"b":false},"default":"claude"}"#,
    );
    let (overlay, changes) = load(&path, &config(Some("a"))).expect("loads");
    assert_eq!(
        changes,
        vec![
            Normalised::LabelWithoutAliasDropped {
                label: "claude".to_owned(),
                member: OverlayMember::Enabled
            },
            Normalised::LabelWithoutAliasDropped {
                label: "claude".to_owned(),
                member: OverlayMember::Default
            },
        ]
    );
    assert!(
        changes[0]
            .to_string()
            .contains("a label, and no configured endpoint has that alias"),
        "{}",
        changes[0]
    );
    assert_eq!(read(&path), "{\n  \"enabled\": {\n    \"b\": false\n  }\n}");
    assert!(!overlay.effective(&config(Some("a"))).enabled[&id("b")]);
}

/// A member in neither grammar is still a parse refusal, and a label is
/// judged by the derived grammar: a `Qm` text is not a label.
#[test]
fn a_member_in_neither_grammar_is_a_parse_refusal() {
    let config = config(Some("a"));
    for text in [
        r#"{"enabled":{"Claude":false}}"#,
        r#"{"enabled":{"Qm":false}}"#,
        r#"{"enabled":{},"default":"Not An Id"}"#,
    ] {
        let (_dir, path) = state();
        put_raw(&path, text);
        assert!(
            matches!(load(&path, &config), Err(EndpointOverlayError::Parse(_))),
            "{text}"
        );
    }
}

/// The carry moves the label's members to the id and the load is then
/// accepted, with the entry disabled -- the operator's disable survived.
#[test]
fn after_the_carry_the_load_gives_the_entry_disabled() {
    let config = config_with_claude();
    let claude_id = id("claude");
    let (_dir, path) = state();
    put_raw(&path, r#"{"enabled":{"claude":false},"default":"claude"}"#);
    load(&path, &config).expect_err("refused before the carry");
    assert_eq!(
        EndpointOverlay::carry_label_within(&path, &TrustBoundary::root(), &claude(), &claude_id)
            .expect("carried"),
        Carried::Moved
    );
    let on_disk = std::fs::read_to_string(&path).expect("read");
    assert!(on_disk.contains(claude_id.as_str()) && !on_disk.contains("\"claude\""));
    let (overlay, changes) = load(&path, &config).expect("loads");
    let effective = overlay.effective(&config);
    assert!(!effective.enabled[&claude_id], "disabled");
    // The default named `claude`, which the overlay disables: the
    // predicate drops it, as it would any disabled default.
    assert_eq!(
        changes,
        vec![Normalised::DisabledDefaultDropped(claude_id.clone())]
    );
    assert_eq!(effective.default, Some(id("a")));
}

fn json_of(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read")).expect("json")
}

/// The carry: idempotent, only the label's members move, the rest of the
/// file is as it was, a file with no member and an absent file are not
/// written, and equal states merge while different ones refuse.
#[test]
fn the_carry_moves_the_labels_members_and_nothing_else() {
    use std::os::unix::fs::MetadataExt as _;
    let claude_id = id("claude");
    let other = id("b");
    let carry = |path: &Path| {
        EndpointOverlay::carry_label_within(path, &TrustBoundary::root(), &claude(), &claude_id)
    };

    // Absent: nothing, and not created.
    let (_dir, path) = state();
    assert_eq!(carry(&path).expect("absent"), Carried::Nothing);
    assert!(!path.exists());

    // Moved, other members (an id, another label) untouched.
    put_raw(
        &path,
        &format!(
            r#"{{"enabled":{{"claude":false,"retired":true,"{}":false}}}}"#,
            other.as_str()
        ),
    );
    assert_eq!(carry(&path).expect("carried"), Carried::Moved);
    assert_eq!(
        json_of(&path),
        serde_json::json!({"enabled": {
            claude_id.as_str(): false, "retired": true, other.as_str(): false}})
    );
    // Idempotent: nothing the second time, the file not rewritten.
    let text = std::fs::read(&path).expect("read");
    let first = std::fs::metadata(&path).expect("meta");
    assert_eq!(carry(&path).expect("again"), Carried::Nothing);
    let second = std::fs::metadata(&path).expect("meta");
    assert_eq!(
        (first.ino(), first.mtime_nsec()),
        (second.ino(), second.mtime_nsec())
    );
    assert_eq!(std::fs::read(&path).expect("read"), text);

    // A default of the label moves; any other default stays.
    put_raw(&path, r#"{"enabled":{},"default":"claude"}"#);
    assert_eq!(carry(&path).expect("carried"), Carried::Moved);
    assert_eq!(
        json_of(&path),
        serde_json::json!({"enabled": {}, "default": claude_id.as_str()})
    );
    put_raw(&path, r#"{"enabled":{},"default":"retired"}"#);
    let before = std::fs::read(&path).expect("read");
    assert_eq!(carry(&path).expect("nothing"), Carried::Nothing);
    assert_eq!(std::fs::read(&path).expect("read"), before);

    // Both named, the same state: the label goes, the id's stays.
    put_raw(
        &path,
        &format!(
            r#"{{"enabled":{{"claude":false,"{}":false}}}}"#,
            claude_id.as_str()
        ),
    );
    assert_eq!(carry(&path).expect("merged"), Carried::Moved);
    assert_eq!(
        json_of(&path),
        serde_json::json!({"enabled": {claude_id.as_str(): false}})
    );

    // Both named, different states: refused, the file untouched.
    put_raw(
        &path,
        &format!(
            r#"{{"enabled":{{"claude":false,"{}":true}}}}"#,
            claude_id.as_str()
        ),
    );
    let before = std::fs::read(&path).expect("read");
    let refused = carry(&path).expect_err("conflict");
    assert!(
        matches!(&refused, EndpointOverlayError::CarryConflict { label, id }
            if label == "claude" && *id == claude_id),
        "{refused:?}"
    );
    assert_eq!(std::fs::read(&path).expect("read"), before, "untouched");
}

/// The carry reads as the load does: a file readable by others is
/// refused, a duplicate key is refused; the control is the same text
/// private.
#[test]
fn the_carry_refuses_an_untrusted_file() {
    let claude_id = id("claude");
    let carry = |path: &Path| {
        EndpointOverlay::carry_label_within(path, &TrustBoundary::root(), &claude(), &claude_id)
    };
    let (_dir, path) = state();
    put_raw(&path, r#"{"enabled":{"claude":false}}"#);
    chmod(&path, 0o644);
    let e = carry(&path).expect_err("not private");
    assert!(
        matches!(e, EndpointOverlayError::NotPrivate { .. }),
        "{e:?}"
    );
    chmod(&path, 0o600);
    assert_eq!(carry(&path).expect("the control"), Carried::Moved);
    put_raw(&path, r#"{"enabled":{"claude":false,"claude":true}}"#);
    let e = carry(&path).expect_err("named twice");
    assert!(matches!(e, EndpointOverlayError::Parse(_)), "{e:?}");
}

fn private_tempdir() -> std::io::Result<tempfile::TempDir> {
    use std::os::unix::fs::PermissionsExt as _;
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
}
