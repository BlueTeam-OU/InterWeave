// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The endpoint-key commands as a process (ADR-0067 decision 4,
//! ENDPOINTS.md section Configured endpoint set) and the endpoint named
//! by its alias on the admin commands, against a scripted admin socket.
//!
//! **TEST-ONLY KEY MATERIAL.** Every secret planted here is a constant
//! (`[n; 32]`) or a key minted into a temporary directory; none is an
//! identity anywhere else.
//!
//! The old identity-key record is built byte by byte as the human store's
//! writer laid it out (`crates/human/store/src/identity_key.rs`, `header`
//! and `to_record`: `IDK1 | 1 | custody | len | label | public key | body`,
//! the body the 32-byte secret at filesystem custody, the line the
//! standard base64 of it all), not through profile-config's encoder,
//! which is the code under test. The store's own writer is not a
//! dev-dependency: it compiles the `SQLCipher` build for one file's layout,
//! and it is moving off labels in the same stage.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::fs::DirBuilder;
use std::io::{BufRead as _, Read as _, Write as _};
use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::SigningKey;
use interweave_ipc_protocol::{DecodedFrame, FrameError, decode_frame, encode_frame};
use interweave_profile_config::endpoint_key::{
    Custody, ENDPOINT_KEYS_DIR, KeyHeader, mint_filesystem_at, pending_key_path,
};
use interweave_profile_config::endpoint_overlay::EndpointOverlay;
use interweave_profile_config::{
    EndpointAlias, EndpointKeyState, ProfileConfig, ProfilePaths, XdgRoots,
};
use interweave_transport_api::EndpointId;
use serde_json::{Value, json};

const PEER: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";

struct Home {
    root: tempfile::TempDir,
    paths: ProfilePaths,
}

fn private_dir(path: &Path) {
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .expect("a private directory");
}

impl Home {
    fn new() -> Self {
        let root = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .expect("tempdir");
        let at = |name: &str| root.path().join(name);
        let roots = XdgRoots {
            config_home: at("config"),
            data_home: at("data"),
            state_home: at("state"),
            cache_home: at("cache"),
            runtime_dir: Some(at("run")),
        };
        let paths = ProfilePaths::resolve("p", &roots).expect("paths");
        private_dir(paths.state_dir());
        Self { root, paths }
    }

    /// The profile's document, `entries` its endpoint entries.
    fn write_config(&self, entries: &str) {
        let config = self.paths.config_file();
        private_dir(config.parent().expect("a config directory"));
        std::fs::write(
            &config,
            format!(
                "schema_version: 2
profile: {{ name: p }}
trust: {{ policy: static-allowlist, allowed_peers: [] }}
endpoints:
  entries:
{entries}
channels: {{ desired: [] }}
discovery: {{ providers: [] }}
transport: {{ listen: {{ addresses: [\"/ip4/127.0.0.1/tcp/0\"] }} }}
"
            ),
        )
        .expect("the profile written");
        std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o644)).expect("chmod");
    }

    fn run(&self, args: &[&str]) -> Output {
        let at = |name: &str| self.root.path().join(name);
        Command::new(env!("CARGO_BIN_EXE_transportctl"))
            .args(["--profile", "p"])
            .args(args)
            .env_clear()
            .env("XDG_CONFIG_HOME", at("config"))
            .env("XDG_DATA_HOME", at("data"))
            .env("XDG_STATE_HOME", at("state"))
            .env("XDG_CACHE_HOME", at("cache"))
            .env("XDG_RUNTIME_DIR", at("run"))
            .stdin(Stdio::null())
            .output()
            .expect("runs")
    }

    fn keys_dir(&self) -> PathBuf {
        self.paths.state_dir().join(ENDPOINT_KEYS_DIR)
    }

    /// The names under `endpoint-keys/`, none when it is absent.
    fn minted(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(self.keys_dir()) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .map(|e| {
                e.expect("an entry")
                    .file_name()
                    .into_string()
                    .expect("utf-8")
            })
            .collect();
        names.sort();
        names
    }

    fn legacy(&self, label: &str) -> PathBuf {
        self.paths
            .state_dir()
            .join("identity-keys")
            .join(format!("{label}.key"))
    }

    /// Plant `line` as `identity-keys/<label>.key`, owner-only, as the
    /// human store wrote it.
    fn plant_legacy(&self, label: &str, line: &str) -> PathBuf {
        let path = self.legacy(label);
        private_dir(path.parent().expect("a directory"));
        write_private(&path, line.as_bytes());
        path
    }
}

fn write_private(path: &Path, bytes: &[u8]) {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .expect("created");
    file.write_all(bytes).expect("written");
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// An IDK1 record's line, laid out byte by byte as the human store's
/// `identity_key::header` and `to_record` wrote it.
fn record(custody: u8, name: &[u8], public_key: &[u8; 32], body: &[u8]) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"IDK1");
    bytes.push(1);
    bytes.push(custody);
    bytes.push(u8::try_from(name.len()).expect("short"));
    bytes.extend_from_slice(name);
    bytes.extend_from_slice(public_key);
    bytes.extend_from_slice(body);
    STANDARD.encode(bytes)
}

/// TEST-ONLY: the key pair of the constant secret `[n; 32]`.
fn test_key(n: u8) -> ([u8; 32], [u8; 32]) {
    let secret = [n; 32];
    (
        secret,
        SigningKey::from_bytes(&secret).verifying_key().to_bytes(),
    )
}

/// The bytes of a key file's record, decoded whole.
fn decoded(path: &Path) -> Vec<u8> {
    let line = std::fs::read_to_string(path).expect("the key file");
    STANDARD
        .decode(line.trim_end_matches('\n'))
        .expect("base64")
}

/// The id on the first line of `init`'s or `adopt`'s output, and the
/// entry lines after it.
fn printed(out: &Output) -> (EndpointId, Vec<String>) {
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    let mut lines = stdout.lines().map(str::to_owned);
    let id = EndpointId::parse_derived(lines.next().expect("the id line")).expect("a derived id");
    (id, lines.collect())
}

/// The entry an operator writes from what was printed.
fn entry_from(lines: &[String]) -> String {
    format!(
        "    - {{ {}, enabled: true, advertise: false }}",
        lines.join(", ")
    )
}

fn loaded_state(home: &Home, id: &EndpointId) -> EndpointKeyState {
    let config = ProfileConfig::load(&home.paths).expect("the profile loads");
    config
        .endpoints
        .entries
        .iter()
        .find(|e| &e.id == id)
        .expect("the entry")
        .key_state
}

// ---------------------------------------------------------------- init

/// A fresh key: the id it prints is the one its file derives, the entry
/// it prints loads PRESENT once written, the file is owner-only in an
/// owner-only directory, and its secret is on neither stream.
#[test]
fn init_mints_a_key_whose_entry_loads_present() {
    let home = Home::new();
    let out = home.run(&["endpoint", "init", "--alias", "human"]);
    let (id, lines) = printed(&out);
    assert_eq!(
        lines,
        [
            format!("key: {}.key", id.as_str()),
            "alias: human".to_owned()
        ]
    );
    assert!(text(&out.stderr).is_empty(), "{}", text(&out.stderr));
    assert_eq!(home.minted(), [format!("{}.key", id.as_str())]);

    let file = home.keys_dir().join(format!("{}.key", id.as_str()));
    let mode = |p: &Path| std::fs::metadata(p).expect("meta").permissions().mode() & 0o777;
    assert_eq!(mode(&file), 0o600);
    assert_eq!(mode(&home.keys_dir()), 0o700);

    // The secret, in each form it could leak in, is on neither stream.
    let bytes = decoded(&file);
    let secret = &bytes[bytes.len() - 32..];
    let line = std::fs::read_to_string(&file).expect("line");
    let hex: String = secret.iter().fold(String::new(), |mut hex, b| {
        let _ = std::fmt::Write::write_fmt(&mut hex, format_args!("{b:02x}"));
        hex
    });
    for stream in [text(&out.stdout), text(&out.stderr)] {
        assert!(!stream.contains(line.trim()), "the record leaked");
        assert!(
            !stream.contains(&STANDARD.encode(secret)),
            "the secret leaked"
        );
        assert!(!stream.contains(&hex), "the secret leaked as hex");
    }

    home.write_config(&entry_from(&lines));
    assert_eq!(
        loaded_state(&home, &id),
        EndpointKeyState::Present {
            custody: Custody::FilesystemOnly
        }
    );
}

#[test]
fn two_inits_mint_two_keys() {
    let home = Home::new();
    let (a, a_lines) = printed(&home.run(&["endpoint", "init"]));
    let (b, _) = printed(&home.run(&["endpoint", "init"]));
    assert_ne!(a, b);
    assert_eq!(
        a_lines,
        [format!("key: {}.key", a.as_str())],
        "no alias line"
    );
    assert_eq!(home.minted().len(), 2);
}

#[test]
fn init_prints_one_json_object_with_json() {
    let home = Home::new();
    let out = home.run(&["endpoint", "init", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let value: Value = serde_json::from_slice(&out.stdout).expect("json");
    let id = value["id"].as_str().expect("the id");
    assert_eq!(value["key"], format!("{id}.key"));
    assert_eq!(value["alias"], Value::Null);
    assert_eq!(home.minted(), [format!("{id}.key")]);
}

/// An alias an entry of `config.yaml` carries is refused and nothing is
/// minted; the control, a free alias against the same document, mints.
#[test]
fn init_refuses_an_alias_config_yaml_already_carries() {
    let home = Home::new();
    let taken = EndpointId::from_ed25519_public(&test_key(1).1);
    home.write_config(&format!(
        "    - {{ key: {}.key, alias: human, enabled: true, advertise: false }}",
        taken.as_str()
    ));
    let out = home.run(&["endpoint", "init", "--alias", "human"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("'human' is taken"),
        "{}",
        text(&out.stderr)
    );
    assert!(home.minted().is_empty(), "nothing minted");

    let before = std::fs::read(home.paths.config_file()).expect("config.yaml");
    let (_, lines) = printed(&home.run(&["endpoint", "init", "--alias", "claude"]));
    assert_eq!(lines.last().map(String::as_str), Some("alias: claude"));
    assert_eq!(home.minted().len(), 1);
    assert_eq!(
        std::fs::read(home.paths.config_file()).expect("config.yaml"),
        before,
        "config.yaml is never written"
    );
}

/// Neither command takes the profile lock: a key is minted and one
/// adopted beside a holder of it, as beside a running daemon.
#[test]
fn neither_command_takes_the_profile_lock() {
    let home = Home::new();
    let _held = interweave_profile_config::ProfileLock::acquire(&home.paths, Duration::ZERO)
        .expect("the lock");
    printed(&home.run(&["endpoint", "init"]));
    let (secret, public) = test_key(16);
    home.plant_legacy("human", &record(1, b"human", &public, &secret));
    printed(&home.run(&["endpoint", "adopt", "--alias", "human"]));
}

/// A `config.yaml` that does not load -- a profile still on labels -- is
/// the migration's case: the mint goes on, and stderr says the alias was
/// not checked.
#[test]
fn init_beside_a_configuration_that_does_not_load_mints_and_says_so() {
    let home = Home::new();
    home.write_config("    - { id: human, enabled: true, advertise: false }");
    let out = home.run(&["endpoint", "init", "--alias", "human"]);
    printed(&out);
    assert!(
        text(&out.stderr).contains("the alias was not checked"),
        "{}",
        text(&out.stderr)
    );
}

#[test]
fn init_refuses_an_invalid_alias() {
    let home = Home::new();
    for bad in ["Human", "0x", "a/b", ""] {
        let out = home.run(&["endpoint", "init", "--alias", bad]);
        assert_eq!(out.status.code(), Some(2), "{bad:?}: {}", text(&out.stderr));
        assert!(text(&out.stderr).contains("not an alias"), "{bad:?}");
    }
    assert!(home.minted().is_empty());
}

// --------------------------------------------------------------- adopt

/// A filesystem-custody record sealed under `human` is adopted: the new
/// file's header derives its id from the same public key, its body is
/// the same secret, the old file is gone, and the printed entry loads
/// present.
#[test]
fn adopt_brings_a_filesystem_record_over_and_removes_the_old_file() {
    let home = Home::new();
    let (secret, public) = test_key(7);
    let old = home.plant_legacy("human", &record(1, b"human", &public, &secret));
    let out = home.run(&["endpoint", "adopt", "--alias", "human"]);
    let (id, lines) = printed(&out);
    assert_eq!(id, EndpointId::from_ed25519_public(&public));
    assert_eq!(id.ed25519_public(), Some(public), "the id names the key");
    assert_eq!(
        lines,
        [
            format!("key: {}.key", id.as_str()),
            "alias: human".to_owned()
        ]
    );
    assert!(!old.exists(), "the old file is removed");
    assert_eq!(home.minted(), [format!("{}.key", id.as_str())]);

    let new = home.keys_dir().join(format!("{}.key", id.as_str()));
    let line = std::fs::read_to_string(&new).expect("the new file");
    let header = KeyHeader::from_line(&line).expect("its header derives its id");
    assert_eq!(header.endpoint(), &id);
    assert_eq!(header.custody(), Custody::FilesystemOnly);
    assert_eq!(
        decoded(&new),
        STANDARD
            .decode(record(1, id.as_str().as_bytes(), &public, &secret))
            .expect("b64"),
        "the same record under the derived id, the same secret"
    );

    home.write_config(&entry_from(&lines));
    assert_eq!(
        loaded_state(&home, &id),
        EndpointKeyState::Present {
            custody: Custody::FilesystemOnly
        }
    );
}

#[test]
fn adopt_refuses_a_keystore_record_and_leaves_it() {
    let home = Home::new();
    let (_, public) = test_key(8);
    let line = record(2, b"human", &public, &[0xa5; 60]);
    let old = home.plant_legacy("human", &line);
    let out = home.run(&["endpoint", "adopt", "--alias", "human"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("Keystore"),
        "{}",
        text(&out.stderr)
    );
    assert_eq!(std::fs::read_to_string(&old).expect("kept"), line);
    assert!(home.minted().is_empty(), "nothing written");
}

/// No `identity-keys/<label>.key`: refused naming `endpoint init`, and
/// nothing minted -- a mistyped alias never makes a new identity.
#[test]
fn adopt_of_an_absent_alias_names_init_and_mints_nothing() {
    let home = Home::new();
    let out = home.run(&["endpoint", "adopt", "--alias", "claude"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("transportctl endpoint init --profile p --alias claude"),
        "{}",
        text(&out.stderr)
    );
    assert!(home.minted().is_empty());
    assert!(!home.keys_dir().exists(), "endpoint-keys/ not even created");
}

#[test]
fn adopt_refuses_a_header_whose_public_key_is_not_the_secrets() {
    let home = Home::new();
    let (secret, _) = test_key(9);
    let (_, other) = test_key(10);
    let line = record(1, b"human", &other, &secret);
    let old = home.plant_legacy("human", &line);
    let out = home.run(&["endpoint", "adopt", "--alias", "human"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("public key is not the secret's"),
        "{}",
        text(&out.stderr)
    );
    assert_eq!(std::fs::read_to_string(&old).expect("kept"), line);
    assert!(home.minted().is_empty());
}

/// A record in `human.key` whose header names another label was renamed
/// by hand: refused, left.
#[test]
fn adopt_refuses_a_record_whose_header_names_another_label() {
    let home = Home::new();
    let (secret, public) = test_key(11);
    let line = record(1, b"claude", &public, &secret);
    let old = home.plant_legacy("human", &line);
    let out = home.run(&["endpoint", "adopt", "--alias", "human"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("names another endpoint"),
        "{}",
        text(&out.stderr)
    );
    assert!(old.exists());
    assert!(home.minted().is_empty());
}

/// Interrupted between its two files: the new file is there (as the
/// write left it) beside the old one. A second run checks it, replaces
/// nothing, mints nothing, removes the old file and prints the entry.
#[test]
fn adopt_after_an_interruption_finishes_and_mints_nothing() {
    let home = Home::new();
    let (secret, public) = test_key(12);
    let id = EndpointId::from_ed25519_public(&public);
    let old = home.plant_legacy("human", &record(1, b"human", &public, &secret));
    private_dir(&home.keys_dir());
    let new = home.keys_dir().join(format!("{}.key", id.as_str()));
    let written = record(1, id.as_str().as_bytes(), &public, &secret);
    write_private(&new, written.as_bytes());

    let (printed_id, lines) = printed(&home.run(&["endpoint", "adopt", "--alias", "human"]));
    assert_eq!(printed_id, id);
    assert_eq!(lines.last().map(String::as_str), Some("alias: human"));
    assert!(!old.exists(), "the old file is removed");
    assert_eq!(home.minted(), [format!("{}.key", id.as_str())], "no mint");
    assert_eq!(
        std::fs::read_to_string(&new).expect("kept"),
        written,
        "not replaced"
    );
}

/// A new file under the derived name that holds ANOTHER secret is never
/// taken for the copy: both files are kept.
#[test]
fn adopt_beside_a_new_file_holding_another_secret_keeps_both() {
    let home = Home::new();
    let (secret, public) = test_key(13);
    let id = EndpointId::from_ed25519_public(&public);
    let old = home.plant_legacy("human", &record(1, b"human", &public, &secret));
    private_dir(&home.keys_dir());
    let new = home.keys_dir().join(format!("{}.key", id.as_str()));
    write_private(
        &new,
        record(1, id.as_str().as_bytes(), &public, &[0x55; 32]).as_bytes(),
    );
    let out = home.run(&["endpoint", "adopt", "--alias", "human"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("both files are kept"),
        "{}",
        text(&out.stderr)
    );
    assert!(old.exists() && new.exists());
}

/// THE ORDER: when the new file cannot be written, the old one is still
/// there -- the removal comes after the write and its read-back, never
/// before. Then, the directory writable again, a second run finishes.
#[test]
fn a_failed_write_leaves_the_old_file() {
    let home = Home::new();
    let (secret, public) = test_key(14);
    let line = record(1, b"human", &public, &secret);
    let old = home.plant_legacy("human", &line);
    private_dir(&home.keys_dir());
    std::fs::set_permissions(home.keys_dir(), std::fs::Permissions::from_mode(0o500))
        .expect("chmod");
    let out = home.run(&["endpoint", "adopt", "--alias", "human"]);
    std::fs::set_permissions(home.keys_dir(), std::fs::Permissions::from_mode(0o700))
        .expect("chmod back");
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stdout));
    assert_eq!(
        std::fs::read_to_string(&old).expect("the old file is still there"),
        line
    );
    assert!(home.minted().is_empty());

    let (id, _) = printed(&home.run(&["endpoint", "adopt", "--alias", "human"]));
    assert_eq!(id, EndpointId::from_ed25519_public(&public));
    assert!(!old.exists());
}

/// Run again once it finished: the old file is gone, so there is nothing
/// to adopt and it refuses -- naming `endpoint init` while `config.yaml`
/// names no entry with the alias, and naming the entry's key once it
/// does. Nothing is minted either way.
#[test]
fn adopt_run_again_after_it_finished_refuses_and_mints_nothing() {
    let home = Home::new();
    let (secret, public) = test_key(15);
    home.plant_legacy("human", &record(1, b"human", &public, &secret));
    let (id, lines) = printed(&home.run(&["endpoint", "adopt", "--alias", "human"]));

    let again = home.run(&["endpoint", "adopt", "--alias", "human"]);
    assert_eq!(again.status.code(), Some(1));
    assert!(
        text(&again.stderr).contains("nothing to adopt")
            && text(&again.stderr).contains("endpoint init"),
        "{}",
        text(&again.stderr)
    );

    home.write_config(&entry_from(&lines));
    let before = std::fs::read(home.paths.config_file()).expect("config.yaml");
    let again = home.run(&["endpoint", "adopt", "--alias", "human"]);
    assert_eq!(
        std::fs::read(home.paths.config_file()).expect("config.yaml"),
        before,
        "config.yaml is never written"
    );
    assert_eq!(again.status.code(), Some(1));
    assert!(
        text(&again.stderr).contains(&format!("already names key {}.key", id.as_str())),
        "{}",
        text(&again.stderr)
    );
    assert_eq!(home.minted(), [format!("{}.key", id.as_str())]);
}

#[test]
fn adopt_needs_an_alias() {
    let home = Home::new();
    let out = home.run(&["endpoint", "adopt"]);
    assert_eq!(out.status.code(), Some(2));
}

// ------------------------------------------- the admin socket, scripted

const PATIENCE: Duration = Duration::from_secs(10);

/// What the scripted daemon was asked: each request's method and params.
type Asked = Vec<(String, Value)>;

/// A scripted daemon on the profile's admin socket, for one connection:
/// it grants `granted`, records each request, and answers it with
/// `answer(method)`.
fn scripted(home: &Home, granted: &'static [&'static str], answer: fn(&str) -> Value) -> Script {
    let socket = home.paths.admin_socket().expect("the admin socket");
    private_dir(socket.parent().expect("the run directory"));
    // The previous script's socket, in a test that runs several.
    let _ = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket).expect("binds");
    listener.set_nonblocking(true).expect("nonblocking");
    let done = Arc::new(AtomicBool::new(false));
    let finished = Arc::clone(&done);
    let thread = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + PATIENCE;
        let stream = loop {
            // Read BEFORE the accept: a client that connected and exited
            // before `done` was set is still in the backlog, so this
            // round's accept sees it.
            let last = finished.load(Ordering::SeqCst);
            match listener.accept() {
                Ok((stream, _)) => break Some(stream),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if last || std::time::Instant::now() > deadline {
                        break None;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => panic!("accept: {e}"),
            }
        };
        let Some(stream) = stream else {
            return Vec::new();
        };
        serve(stream, granted, answer)
    });
    Script { thread, done }
}

struct Script {
    thread: JoinHandle<Asked>,
    done: Arc<AtomicBool>,
}

impl Script {
    /// What it was asked, once the client has exited; nothing when it
    /// never connected.
    fn asked(self) -> Asked {
        self.done.store(true, Ordering::SeqCst);
        self.thread.join().expect("the script")
    }
}

fn serve(stream: UnixStream, granted: &[&str], answer: fn(&str) -> Value) -> Asked {
    stream.set_nonblocking(false).expect("blocking");
    stream.set_read_timeout(Some(PATIENCE)).expect("timeout");
    let mut writer = stream.try_clone().expect("clone");
    let mut reader = std::io::BufReader::new(stream);
    let mut buf = Vec::new();
    let mut next = move || -> Option<Value> {
        loop {
            match decode_frame(&buf) {
                Ok(DecodedFrame { body, consumed }) => {
                    buf.drain(..consumed);
                    return Some(serde_json::from_str(&body).expect("a JSON frame"));
                }
                Err(FrameError::Incomplete { .. }) => {}
                Err(e) => panic!("a bad frame: {e:?}"),
            }
            let chunk = reader.fill_buf().ok()?.to_vec();
            if chunk.is_empty() {
                return None;
            }
            reader.consume(chunk.len());
            buf.extend_from_slice(&chunk);
        }
    };
    let mut write = |body: &Value| {
        let frame = encode_frame(&body.to_string()).expect("a frame");
        let _ = writer.write_all(&frame);
    };
    let hello = next().expect("a hello");
    assert_eq!(hello["type"], "hello", "{hello}");
    write(&json!({
        "type": "hello_response",
        "ipc_version": {"major": 3, "minor": 0},
        "transport_contract_version": "3.0",
        "peer": PEER,
        "granted_capabilities": granted
    }));
    let mut asked = Vec::new();
    while let Some(frame) = next() {
        if frame["type"] != "request" {
            break;
        }
        let method = frame["method"].as_str().expect("a method").to_owned();
        write(
            &json!({"type": "response", "id": frame["id"], "ok": true, "result": answer(&method)}),
        );
        asked.push((method, frame.get("params").cloned().unwrap_or(Value::Null)));
    }
    asked
}

fn empty(_: &str) -> Value {
    json!({})
}

/// TEST-ONLY ids: the derived ids of two constant keys.
fn ids() -> (EndpointId, EndpointId) {
    (
        EndpointId::from_ed25519_public(&test_key(21).1),
        EndpointId::from_ed25519_public(&test_key(22).1),
    )
}

fn two_entries(home: &Home) {
    let (human, bare) = ids();
    home.write_config(&format!(
        "    - {{ key: {}.key, alias: human, enabled: true, advertise: false }}
    - {{ key: {}.key, enabled: true, advertise: false }}",
        human.as_str(),
        bare.as_str()
    ));
}

fn listed(_: &str) -> Value {
    let (human, bare) = ids();
    let unconfigured = EndpointId::from_ed25519_public(&test_key(23).1);
    json!({"endpoints": [
        {"id": human.as_str(), "enabled": true, "default": true, "persisted": true},
        {"id": bare.as_str(), "enabled": false, "default": false, "persisted": true},
        {"id": unconfigured.as_str(), "enabled": true, "default": false, "persisted": true}
    ]})
}

/// `endpoints list` shows each row's alias beside its id: the alias,
/// `-` for an entry that carries none, `(unknown)` for an id the
/// configuration does not name -- in the text. `--json` is the daemon's
/// result as sent, which the `ipc/endpoint-list` schema describes: no
/// alias member.
#[test]
fn endpoints_list_shows_alias_and_id() {
    let home = Home::new();
    two_entries(&home);
    let (human, bare) = ids();
    let script = scripted(&home, &["admin.endpoints"], listed);
    let out = home.run(&["endpoints", "list"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(script.asked()[0].0, "admin.endpoints.list");
    let stdout = text(&out.stdout);
    let rows: Vec<Vec<&str>> = stdout
        .lines()
        .map(|l| l.split_whitespace().collect())
        .collect();
    assert_eq!(rows[0][..3], [human.as_str(), "human", "enabled,"]);
    assert_eq!(rows[1][..3], [bare.as_str(), "-", "disabled"]);
    assert_eq!(rows[2][1], "(unknown)");

    let script = scripted(&home, &["admin.endpoints"], listed);
    let out = home.run(&["endpoints", "list", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    script.asked();
    let value: Value = serde_json::from_slice(&out.stdout).expect("json");
    let rows = value["endpoints"].as_array().expect("rows");
    assert_eq!(rows[0]["id"], json!(human.as_str()));
    assert!(
        rows.iter().all(|row| row.get("alias").is_none()),
        "no alias member: {value}"
    );
    assert_eq!(rows[0]["default"], true, "the daemon's members kept");
}

/// `config.yaml` unreadable: every alias is shown as unknown, never as
/// absent or empty, and the list itself still answers; `--json` reads no
/// configuration and so answers as ever.
#[test]
fn endpoints_list_with_an_unreadable_configuration_shows_unknown() {
    let home = Home::new();
    home.write_config("    - { id: human, enabled: true, advertise: false }");
    let script = scripted(&home, &["admin.endpoints"], listed);
    let out = home.run(&["endpoints", "list"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    script.asked();
    assert!(
        text(&out.stderr).contains("the aliases are unknown"),
        "{}",
        text(&out.stderr)
    );
    for line in text(&out.stdout).lines() {
        assert_eq!(line.split_whitespace().nth(1), Some("(unknown)"), "{line}");
    }
    let script = scripted(&home, &["admin.endpoints"], listed);
    let out = home.run(&["endpoints", "list", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    script.asked();
    let value: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(value["endpoints"].as_array().expect("rows").len(), 3);
}

/// An alias on an endpoint write is resolved to the id the daemon
/// receives; an id is sent as typed.
#[test]
fn an_alias_resolves_to_the_id_the_daemon_receives() {
    let home = Home::new();
    two_entries(&home);
    let (human, bare) = ids();
    for (args, method, id) in [
        (
            vec!["endpoints", "revoke", "human"],
            "admin.endpoints.revoke",
            &human,
        ),
        (
            vec!["endpoints", "disable", "human"],
            "admin.endpoints.set_enabled",
            &human,
        ),
        (
            vec!["endpoints", "default", "human"],
            "admin.endpoints.set_default",
            &human,
        ),
        (
            vec!["endpoints", "enable", bare.as_str()],
            "admin.endpoints.set_enabled",
            &bare,
        ),
    ] {
        let script = scripted(&home, &["admin.endpoints"], empty);
        let out = home.run(&args);
        assert_eq!(
            out.status.code(),
            Some(0),
            "{args:?}: {}",
            text(&out.stderr)
        );
        let asked = script.asked();
        assert_eq!(asked.len(), 1, "{args:?}: {asked:?}");
        assert_eq!(asked[0].0, method);
        assert_eq!(asked[0].1["endpoint"], id.as_str(), "{args:?}");
        assert!(
            text(&out.stdout).contains(id.as_str()),
            "{}",
            text(&out.stdout)
        );
    }
}

/// An alias no entry carries, or one given against a configuration that
/// does not load, is refused naming the alias, and the daemon is never
/// connected to.
#[test]
fn an_unknown_alias_is_refused_with_nothing_sent() {
    let home = Home::new();
    two_entries(&home);
    let script = scripted(&home, &["admin.endpoints"], empty);
    let out = home.run(&["endpoints", "revoke", "claude"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("no configured endpoint has the alias 'claude'"),
        "{}",
        text(&out.stderr)
    );
    // Nothing connected.
    assert!(script.asked().is_empty());

    home.write_config("    - { id: human, enabled: true, advertise: false }");
    let out = home.run(&["endpoints", "default", "human"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("the alias 'human' cannot be resolved"),
        "{}",
        text(&out.stderr)
    );

    // An id is sent as typed and needs no readable configuration.
    let (human, _) = ids();
    let script = scripted(&home, &["admin.endpoints"], empty);
    let out = home.run(&["endpoints", "revoke", human.as_str()]);
    let asked = script.asked();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(asked[0].1["endpoint"], human.as_str());
}

/// Resolving reads no file under `endpoint-keys/`: at mode 000 it does
/// not stop an alias from resolving, and nothing in the profile is
/// written.
#[test]
fn endpoint_keys_at_mode_000_does_not_prevent_resolution() {
    let home = Home::new();
    two_entries(&home);
    let (human, _) = ids();
    private_dir(&home.keys_dir());
    std::fs::set_permissions(home.keys_dir(), std::fs::Permissions::from_mode(0o000))
        .expect("chmod");
    let before = snapshot(home.paths.state_dir());
    let script = scripted(&home, &["admin.endpoints"], empty);
    let out = home.run(&["endpoints", "revoke", "human"]);
    let asked = script.asked();
    let after = snapshot(home.paths.state_dir());
    std::fs::set_permissions(home.keys_dir(), std::fs::Permissions::from_mode(0o700))
        .expect("chmod back");
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(asked[0].1["endpoint"], human.as_str());
    assert_eq!(after, before, "nothing written");
}

/// Resolving reads no key header either: a key file under the entry's
/// name whose header derives ANOTHER id refuses the full load (the
/// control, the daemon's load) and does not touch the alias's resolution.
#[test]
fn a_key_file_the_daemon_would_refuse_does_not_touch_resolution() {
    let home = Home::new();
    two_entries(&home);
    let (human, _) = ids();
    private_dir(&home.keys_dir());
    let (secret, other) = test_key(30);
    let other_id = EndpointId::from_ed25519_public(&other);
    write_private(
        &home.keys_dir().join(format!("{}.key", human.as_str())),
        record(1, other_id.as_str().as_bytes(), &other, &secret).as_bytes(),
    );
    assert!(
        ProfileConfig::load(&home.paths).is_err(),
        "the control: a load that reads the header refuses it"
    );
    let script = scripted(&home, &["admin.endpoints"], empty);
    let out = home.run(&["endpoints", "revoke", "human"]);
    let asked = script.asked();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(asked[0].1["endpoint"], human.as_str());
}

/// Every file under `dir` with its bytes; a directory it cannot read
/// lists as itself.
fn snapshot(dir: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    let mut out = std::collections::BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&at) else {
            out.insert(at, Vec::new());
            continue;
        };
        for entry in entries {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let mut bytes = Vec::new();
                if let Ok(mut file) = std::fs::File::open(&path) {
                    let _ = file.read_to_end(&mut bytes);
                }
                out.insert(path, bytes);
            }
        }
    }
    out
}

// ------------------------------------------- the overlay's label members

/// What a release before Stage 22 wrote for an operator who disabled the
/// `claude` endpoint (`test-data/upgrade/d3a193e9`'s shape).
const FROZEN_OVERLAY: &str = r#"{"enabled":{"claude":false}}"#;

impl Home {
    fn overlay_path(&self) -> PathBuf {
        EndpointOverlay::path_for(&self.paths)
    }

    /// Plant the overlay, owner-only, replacing one that is there.
    fn plant_overlay(&self, text: &str) {
        let path = self.overlay_path();
        let _ = std::fs::remove_file(&path);
        write_private(&path, text.as_bytes());
    }

    fn overlay_json(&self) -> Value {
        serde_json::from_str(&std::fs::read_to_string(self.overlay_path()).expect("overlay"))
            .expect("json")
    }

    /// The composed endpoints with the overlay read as the daemon's load
    /// reads it: what `claude`'s state is once the entry is written.
    fn claude_enabled_as_loaded(&self, id: &EndpointId) -> bool {
        let config = ProfileConfig::load_document(&self.paths).expect("the document");
        let (overlay, _) = EndpointOverlay::read_within(
            &self.overlay_path(),
            &config.endpoints,
            self.paths.boundary(),
        )
        .expect("the overlay loads");
        overlay.effective(&config.endpoints).enabled[id]
    }

    fn claude_entry(&self, id: &EndpointId) {
        self.write_config(&format!(
            "    - {{ key: {}.key, alias: claude, enabled: true, advertise: false }}",
            id.as_str()
        ));
    }

    fn plant_pending(&self) -> EndpointId {
        let alias = EndpointAlias::parse("claude").expect("an alias");
        let pending = pending_key_path(&self.paths, &alias);
        mint_filesystem_at(&self.paths, |_| pending.clone()).expect("a pending key")
    }
}

/// `adopt --alias claude` carries the frozen overlay's `claude` member to
/// the adopted id, and the entry it prints then LOADS with the endpoint
/// disabled. The control: before the adopt, the same entry beside the
/// same overlay is refused for its label.
#[test]
fn adopt_carries_the_label_overlay_to_the_id() {
    let home = Home::new();
    let (secret, public) = test_key(30);
    let id = EndpointId::from_ed25519_public(&public);
    home.plant_legacy("claude", &record(1, b"claude", &public, &secret));
    home.plant_overlay(FROZEN_OVERLAY);

    home.claude_entry(&id);
    let config = ProfileConfig::load_document(&home.paths).expect("the document");
    let refused = EndpointOverlay::read_within(
        &home.overlay_path(),
        &config.endpoints,
        home.paths.boundary(),
    )
    .expect_err("the control: refused for its label");
    assert!(refused.to_string().contains("'claude'"), "{refused}");
    std::fs::remove_file(home.paths.config_file()).expect("config removed");

    let (printed_id, lines) = printed(&home.run(&["endpoint", "adopt", "--alias", "claude"]));
    assert_eq!(printed_id, id);
    assert_eq!(
        home.overlay_json(),
        json!({"enabled": {id.as_str(): false}})
    );
    home.write_config(&entry_from(&lines));
    assert!(!home.claude_enabled_as_loaded(&id), "the disable survived");
}

/// A carry that fails (the overlay names the label AND the id, with
/// different states) refuses, keeps the old file and the written new
/// one, and changes nothing else; once the operator has settled the
/// overlay, a second run finishes. Nothing is minted.
#[test]
fn adopt_whose_carry_fails_keeps_the_old_file_and_a_second_run_finishes() {
    let home = Home::new();
    let (secret, public) = test_key(31);
    let id = EndpointId::from_ed25519_public(&public);
    let old = home.plant_legacy("claude", &record(1, b"claude", &public, &secret));
    let conflicting = format!(r#"{{"enabled":{{"claude":false,"{}":true}}}}"#, id.as_str());
    home.plant_overlay(&conflicting);

    let out = home.run(&["endpoint", "adopt", "--alias", "claude"]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stdout));
    assert!(
        text(&out.stderr).contains("a second run finishes"),
        "{}",
        text(&out.stderr)
    );
    assert!(old.exists(), "the old file is what a rerun needs");
    assert_eq!(home.minted(), [format!("{}.key", id.as_str())]);
    assert_eq!(
        std::fs::read_to_string(home.overlay_path()).expect("overlay"),
        conflicting,
        "the overlay is as it was"
    );

    home.plant_overlay(FROZEN_OVERLAY);
    let (again, _) = printed(&home.run(&["endpoint", "adopt", "--alias", "claude"]));
    assert_eq!(again, id);
    assert!(!old.exists());
    assert_eq!(
        home.overlay_json(),
        json!({"enabled": {id.as_str(): false}})
    );
}

/// Interrupted AFTER the carry, before the old file's removal: the new
/// file is there and the overlay already names the id. The second run
/// finishes, and the overlay is left byte for byte as the carry wrote
/// it -- a carry that found nothing writes nothing.
#[test]
fn adopt_interrupted_after_the_carry_finishes_and_leaves_the_overlay() {
    let home = Home::new();
    let (secret, public) = test_key(32);
    let id = EndpointId::from_ed25519_public(&public);
    let old = home.plant_legacy("claude", &record(1, b"claude", &public, &secret));
    private_dir(&home.keys_dir());
    write_private(
        &home.keys_dir().join(format!("{}.key", id.as_str())),
        record(1, id.as_str().as_bytes(), &public, &secret).as_bytes(),
    );
    let carried = format!(r#"{{"enabled":{{"{}":false}}}}"#, id.as_str());
    home.plant_overlay(&carried);

    printed(&home.run(&["endpoint", "adopt", "--alias", "claude"]));
    assert!(!old.exists());
    assert_eq!(
        std::fs::read_to_string(home.overlay_path()).expect("overlay"),
        carried
    );
}

/// The entry was written by hand without the carry and no old file is
/// left: `adopt` carries the overlay to that entry's id and succeeds,
/// the refusal the loader gave now ended by the command it named. The
/// control, an overlay with no `claude` member, is the refusal as before.
#[test]
fn adopt_for_a_hand_written_entry_carries_the_overlay() {
    let home = Home::new();
    let id = EndpointId::from_ed25519_public(&test_key(33).1);
    home.claude_entry(&id);

    home.plant_overlay(r#"{"enabled":{"other":false}}"#);
    let out = home.run(&["endpoint", "adopt", "--alias", "claude"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains(&format!("already names key {}.key", id.as_str())),
        "{}",
        text(&out.stderr)
    );

    home.plant_overlay(FROZEN_OVERLAY);
    let (printed_id, lines) = printed(&home.run(&["endpoint", "adopt", "--alias", "claude"]));
    assert_eq!(printed_id, id);
    assert_eq!(lines.last().map(String::as_str), Some("alias: claude"));
    assert_eq!(
        home.overlay_json(),
        json!({"enabled": {id.as_str(): false}})
    );
    assert!(!home.claude_enabled_as_loaded(&id));
    assert!(home.minted().is_empty(), "nothing minted");
}

/// `init --alias claude` carries the frozen overlay to the minted id: one
/// key file, named by its id, no pending file left, and the entry it
/// prints loads with the endpoint disabled.
#[test]
fn init_carries_the_label_overlay_to_the_minted_id() {
    let home = Home::new();
    home.plant_overlay(FROZEN_OVERLAY);
    let (id, lines) = printed(&home.run(&["endpoint", "init", "--alias", "claude"]));
    assert_eq!(home.minted(), [format!("{}.key", id.as_str())]);
    assert_eq!(
        home.overlay_json(),
        json!({"enabled": {id.as_str(): false}})
    );
    home.write_config(&entry_from(&lines));
    assert!(!home.claude_enabled_as_loaded(&id));
}

/// THE CONSTRAINT: an `init --alias` cut short after minting and run
/// again mints nothing and ends with the overlay on the ONE key file's
/// id. The interruption is simulated by the pending file a cut-short run
/// leaves, planted by the mint that run makes; the overlay is as it was
/// (cut before the carry) or already carried (cut before the rename).
#[test]
fn init_rerun_after_an_interruption_mints_nothing() {
    for (what, overlay_names_id) in [("before the carry", false), ("after the carry", true)] {
        let home = Home::new();
        let id = home.plant_pending();
        assert_eq!(home.minted(), [".pending-claude.key".to_owned()]);
        let pending_line =
            std::fs::read_to_string(home.keys_dir().join(".pending-claude.key")).expect("pending");
        home.plant_overlay(&if overlay_names_id {
            format!(r#"{{"enabled":{{"{}":false}}}}"#, id.as_str())
        } else {
            FROZEN_OVERLAY.to_owned()
        });

        let (printed_id, lines) = printed(&home.run(&["endpoint", "init", "--alias", "claude"]));
        assert_eq!(printed_id, id, "{what}: the pending key's id");
        assert_eq!(lines.last().map(String::as_str), Some("alias: claude"));
        assert_eq!(
            home.minted(),
            [format!("{}.key", id.as_str())],
            "{what}: one key file, nothing minted, no pending file"
        );
        assert_eq!(
            std::fs::read_to_string(home.keys_dir().join(format!("{}.key", id.as_str())))
                .expect("published"),
            pending_line,
            "{what}: the pending key itself"
        );
        assert_eq!(
            home.overlay_json(),
            json!({"enabled": {id.as_str(): false}}),
            "{what}"
        );
    }
}

/// The `link` fallback cut short between its link and its unlink leaves
/// the pending key under its id's name as well: the rerun finds the same
/// bytes there, removes the pending name and prints the entry. The
/// control: another key under the id's name is refused, both kept.
#[test]
fn init_finishes_when_the_key_is_under_both_names() {
    let home = Home::new();
    let id = home.plant_pending();
    let pending = home.keys_dir().join(".pending-claude.key");
    let named = home.keys_dir().join(format!("{}.key", id.as_str()));
    let line = std::fs::read_to_string(&pending).expect("pending");
    std::fs::hard_link(&pending, &named).expect("a second name");
    let (printed_id, _) = printed(&home.run(&["endpoint", "init", "--alias", "claude"]));
    assert_eq!(printed_id, id);
    assert_eq!(home.minted(), [format!("{}.key", id.as_str())]);
    assert_eq!(std::fs::read_to_string(&named).expect("named"), line);

    // The control: the id's name holds other bytes.
    let other = Home::new();
    let id = other.plant_pending();
    let named = other.keys_dir().join(format!("{}.key", id.as_str()));
    std::fs::write(&named, "not the pending key\n").expect("planted");
    std::fs::set_permissions(&named, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    let out = other.run(&["endpoint", "init", "--alias", "claude"]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stdout));
    assert!(
        text(&out.stderr).contains("holds another key"),
        "{}",
        text(&out.stderr)
    );
    assert_eq!(other.minted().len(), 2, "both kept");
}

/// A carry that fails in `init` leaves the pending file, so the rerun
/// finds the same key; the control is the rerun finishing once the
/// overlay is settled.
#[test]
fn init_whose_carry_fails_keeps_the_pending_key() {
    let home = Home::new();
    let id = home.plant_pending();
    let conflicting = format!(r#"{{"enabled":{{"claude":false,"{}":true}}}}"#, id.as_str());
    home.plant_overlay(&conflicting);
    let out = home.run(&["endpoint", "init", "--alias", "claude"]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stdout));
    assert!(
        text(&out.stderr).contains("a second run finishes"),
        "{}",
        text(&out.stderr)
    );
    assert_eq!(home.minted(), [".pending-claude.key".to_owned()]);
    assert_eq!(
        std::fs::read_to_string(home.overlay_path()).expect("overlay"),
        conflicting
    );

    home.plant_overlay(FROZEN_OVERLAY);
    let (again, _) = printed(&home.run(&["endpoint", "init", "--alias", "claude"]));
    assert_eq!(again, id);
    assert_eq!(home.minted(), [format!("{}.key", id.as_str())]);
}

/// A pending file that is not a record of its own key is refused and
/// left, and nothing is minted over it.
#[test]
fn init_refuses_a_pending_file_that_is_not_a_key_record() {
    let home = Home::new();
    private_dir(&home.keys_dir());
    let pending = home.keys_dir().join(".pending-claude.key");
    write_private(&pending, b"not a record\n");
    let out = home.run(&["endpoint", "init", "--alias", "claude"]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stdout));
    assert_eq!(home.minted(), [".pending-claude.key".to_owned()]);
    assert_eq!(std::fs::read(&pending).expect("kept"), b"not a record\n");

    // The control: a record of its own key is taken.
    std::fs::remove_file(&pending).expect("removed");
    home.plant_pending();
    printed(&home.run(&["endpoint", "init", "--alias", "claude"]));
}

/// An entry with the alias already written by hand, the overlay still
/// naming the label: `init` carries and succeeds, minting nothing -- the
/// alias-taken refusal it gave before would have been a dead end. The
/// control, an overlay with no such member, still refuses it as taken.
#[test]
fn init_for_a_hand_written_entry_carries_the_overlay() {
    let home = Home::new();
    let id = EndpointId::from_ed25519_public(&test_key(34).1);
    home.claude_entry(&id);

    home.plant_overlay(r#"{"enabled":{"other":false}}"#);
    let out = home.run(&["endpoint", "init", "--alias", "claude"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("'claude' is taken"),
        "{}",
        text(&out.stderr)
    );

    home.plant_overlay(FROZEN_OVERLAY);
    let (printed_id, _) = printed(&home.run(&["endpoint", "init", "--alias", "claude"]));
    assert_eq!(printed_id, id);
    assert_eq!(
        home.overlay_json(),
        json!({"enabled": {id.as_str(): false}})
    );
    assert!(!home.claude_enabled_as_loaded(&id));
    assert!(home.minted().is_empty(), "nothing minted");
}

/// With no alias there is no label to carry: the overlay is left byte
/// for byte, and the control -- the same overlay with an alias -- is
/// carried (`init_carries_the_label_overlay_to_the_minted_id`).
#[test]
fn init_without_an_alias_leaves_the_overlay_alone() {
    let home = Home::new();
    home.plant_overlay(FROZEN_OVERLAY);
    let (id, _) = printed(&home.run(&["endpoint", "init"]));
    assert_eq!(home.minted(), [format!("{}.key", id.as_str())]);
    assert_eq!(
        std::fs::read_to_string(home.overlay_path()).expect("overlay"),
        FROZEN_OVERLAY
    );
}

/// No overlay at all: both commands do what they did, and no overlay is
/// created.
#[test]
fn no_overlay_is_created_by_either_command() {
    let home = Home::new();
    printed(&home.run(&["endpoint", "init", "--alias", "claude"]));
    let (secret, public) = test_key(35);
    home.plant_legacy("human", &record(1, b"human", &public, &secret));
    printed(&home.run(&["endpoint", "adopt", "--alias", "human"]));
    assert!(!home.overlay_path().exists());
}
