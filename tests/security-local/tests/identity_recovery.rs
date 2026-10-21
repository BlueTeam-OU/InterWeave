// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! S11: identity recovery through `transportctl` and the shipped daemon
//! (plan section 21; threat-model.md's "Private-key theft",
//! "Recovery-phrase theft", "Recovery typo/wrong phrase", "Log leakage"
//! and "cloned recovery phrase used on two active devices" rows).
//!
//! `apps/transportctl/tests/identity.rs` holds the command's own
//! refusals one at a time. What is new here is the whole path a phrase
//! takes -- `transportctl` restores it, the daemon runs on it, both are
//! driven over IPC and over the network, the daemon is made to fail on a
//! damaged key -- and a scan of every stream that path wrote.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::Path;
use std::process::Output;
use std::time::Duration;

use interweave_local_client_api::{
    AdminBinding as _, AdminCapability, AdminPort as _, DataSessionBinding as _,
    DataSessionPort as _, SessionEvent,
};
use interweave_profile_identity::{ProfileIdentity, RecoveryPhrase, RecoveryRecord};
use interweave_security_local_tests::secret::Secret;
use interweave_security_local_tests::tap::Tap;
use interweave_test_support::e2e::{Home, PATIENCE, human, lease_request, stranger};
use interweave_transport_api::{
    BroadcastMessageV1, ChannelId, DirectDestination, MessageId, TransportIdentity,
};

mod common;

use common::{Pair, arriving, payload};

/// A profile of one endpoint, `human`, on loopback.
fn profile() -> String {
    format!(
        "schema_version: 2
profile: {{ name: s11 }}
trust: {{ policy: static-allowlist, allowed_peers: [\"{}\"] }}
endpoints:
  default_direct_endpoint: human
  entries:
    - {{ id: human, enabled: true, advertise: false }}
channels: {{ desired: [] }}
discovery: {{ providers: [] }}
transport: {{ listen: {{ addresses: [\"/ip4/127.0.0.1/tcp/0\"] }} }}
observability: {{ log_level: debug }}
",
        stranger().as_str()
    )
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn mode(path: &Path) -> u32 {
    std::fs::symlink_metadata(path).expect("metadata").mode() & 0o777
}

/// The `PeerId` of the key the profile stores, if it stores one.
fn stored(home: &Home) -> Option<TransportIdentity> {
    let file = home.paths.identity_file();
    file.exists().then(|| {
        ProfileIdentity::load(&file)
            .expect("loads")
            .transport_identity()
            .expect("a peer id")
    })
}

/// The identity's recovery record as `transportctl` reads one.
fn record_of(identity: &ProfileIdentity) -> String {
    serde_json::to_string(&RecoveryRecord::of(identity).expect("a record")).expect("json")
}

/// `secret`'s phrase with its last word replaced by another valid word
/// that breaks the checksum: a typo the wordlist cannot catch.
fn tampered(secret: &Secret) -> String {
    let donors: Vec<String> = (0..4)
        .flat_map(|_| Secret::of(&ProfileIdentity::generate()).words().to_vec())
        .collect();
    for donor in donors {
        let mut words = secret.words().to_vec();
        if words[23] == donor {
            continue;
        }
        words[23] = donor;
        let phrase = words.join(" ");
        if RecoveryPhrase::parse(&phrase).is_err() {
            return phrase;
        }
    }
    panic!("no donor word broke the checksum")
}

/// The record with its `expected_peer_id` replaced by `peer`.
fn record_naming(identity: &ProfileIdentity, peer: &TransportIdentity) -> String {
    let mut record: serde_json::Value = serde_json::from_str(&record_of(identity)).expect("json");
    record["expected_peer_id"] = peer.as_str().into();
    record.to_string()
}

/// The restores a mistyped or wrong phrase is: each refused, nothing
/// written. As `transportctl` arguments and stdin.
fn wrong_restores(
    identity: &ProfileIdentity,
    secret: &Secret,
) -> Vec<(&'static str, Vec<String>, String)> {
    let other = stranger();
    let new = || {
        vec![
            "identity".to_owned(),
            "restore".to_owned(),
            "--new".to_owned(),
        ]
    };
    let mut expecting_other = new();
    expecting_other.extend(["--expected-peer-id".to_owned(), other.as_str().to_owned()]);
    vec![
        (
            "a phrase with a word changed",
            new(),
            format!("{}\n", tampered(secret)),
        ),
        (
            "the phrase against another PeerId",
            expecting_other,
            format!("{}\n", secret.phrase()),
        ),
        (
            "a record naming another PeerId",
            new(),
            record_naming(identity, &other),
        ),
    ]
}

fn run(home: &Home, args: &[String], input: &str) -> Output {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    home.transportctl(&args, input)
}

/// Threat-model.md, "Recovery typo/wrong phrase": through `transportctl`,
/// a phrase whose checksum a substituted word broke, the right phrase
/// against another `PeerId`, and a record naming another `PeerId` are
/// each refused and write no key; the daemon on the empty profile refuses
/// to start rather than mint one. The control: the right record restores
/// the identity, and the daemon serves as its `PeerId`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_tampered_phrase_or_a_wrong_peer_id_restores_nothing_and_the_daemon_mints_nothing() {
    let home = Home::new("s11");
    home.write_config(&profile());
    let identity = ProfileIdentity::generate();
    let secret = Secret::of(&identity);
    let peer = identity.transport_identity().expect("a peer id");

    for (what, args, input) in wrong_restores(&identity, &secret) {
        let out = run(&home, &args, &input);
        assert_eq!(out.status.code(), Some(1), "{what}: {}", text(&out.stderr));
        assert_eq!(stored(&home), None, "{what} wrote a key");
        for stream in [&out.stdout, &out.stderr] {
            assert_eq!(secret.found_in(stream), Vec::<String>::new(), "{what}");
        }
    }
    let mut refused = home.start(&[]);
    assert_eq!(refused.exit().await.code(), Some(1), "{}", refused.log());
    assert_eq!(stored(&home), None, "the daemon minted no identity");

    let out = home.transportctl(&["identity", "restore", "--new"], &record_of(&identity));
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(stored(&home), Some(peer.clone()));
    let mut daemon = home.start(&[]);
    daemon.serving(&home).await;
    let status = home
        .binding()
        .admin([AdminCapability::Status].into())
        .await
        .expect("a port")
        .status()
        .await
        .expect("status");
    assert_eq!(
        status.peer, peer,
        "the daemon serves as the restored identity"
    );
    assert!(daemon.terminate().await.success(), "{}", daemon.log());
}

/// Threat-model.md, "Private-key theft": under umask 000 the key
/// `transportctl` mints, the backup it writes and the key it restores
/// are each `0600`, in a key directory of `0700`. The control: a file the
/// same shell makes first is `0666`, so the umask reached the process.
#[test]
fn under_a_hostile_umask_every_file_holding_the_secret_is_owner_only() {
    let home = Home::new("s11");
    home.write_config(&profile());
    let probe = home.root.path().join("umask-probe");
    let umask = format!("umask 000\n: > '{}'", probe.display());
    let out = home.transportctl_after(&umask, &["identity", "init"], "");
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(mode(&probe), 0o666, "the control: the umask is live");
    let key = home.paths.identity_file();
    assert_eq!(mode(&key), 0o600, "the minted key");
    assert_eq!(
        mode(key.parent().expect("a key directory")),
        0o700,
        "the key directory"
    );

    let backup = home.root.path().join("backup.json");
    let out = home.transportctl_after(
        "umask 000",
        &[
            "identity",
            "backup",
            "--to-file",
            backup.to_str().expect("utf-8"),
        ],
        "",
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(mode(&backup), 0o600, "the backup");

    std::fs::remove_file(&key).expect("the key lost");
    let record = std::fs::read_to_string(&backup).expect("the backup read");
    let out = home.transportctl_after("umask 000", &["identity", "restore", "--new"], &record);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(mode(&key), 0o600, "the restored key");
}

/// Threat-model.md, "cloned recovery phrase used on two active devices",
/// the part one host can show: a restore cannot take a RUNNING profile's
/// place. With the daemon up, a replace naming the stored identity and a
/// new restore are both refused at the profile lock, the key file is as
/// it was and the daemon still serves as it. The control: the same
/// replace once the daemon has stopped installs the other identity, so
/// the refusal was the lock's.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_restore_cannot_replace_the_key_of_a_running_profile() {
    let home = Home::new("s11");
    let peer = home.write_key();
    home.write_config(&profile());
    let mut daemon = home.start(&[]);
    daemon.serving(&home).await;
    let key_before = std::fs::read(home.paths.identity_file()).expect("the key");
    let other = ProfileIdentity::generate();
    let other_peer = other.transport_identity().expect("a peer id");
    let replace = [
        "identity",
        "restore",
        "--replace",
        "--replacing",
        peer.as_str(),
    ];
    for args in [&replace[..], &["identity", "restore", "--new"][..]] {
        let out = home.transportctl(args, &record_of(&other));
        assert_eq!(
            out.status.code(),
            Some(1),
            "{args:?}: {}",
            text(&out.stderr)
        );
        assert!(
            text(&out.stderr).contains("lock is held"),
            "{args:?}: {}",
            text(&out.stderr)
        );
    }
    assert_eq!(
        std::fs::read(home.paths.identity_file()).expect("the key"),
        key_before,
        "the key file untouched"
    );
    let status = home
        .binding()
        .admin([AdminCapability::Status].into())
        .await
        .expect("a port")
        .status()
        .await
        .expect("status");
    assert_eq!(status.peer, peer, "still serving as itself");
    assert!(daemon.terminate().await.success(), "{}", daemon.log());

    let out = home.transportctl(&replace, &record_of(&other));
    assert_eq!(
        out.status.code(),
        Some(0),
        "the control: {}",
        text(&out.stderr)
    );
    assert_eq!(stored(&home), Some(other_peer));
}

/// Every stream one run wrote, by name.
#[derive(Default)]
struct Captured(Vec<(String, Vec<u8>)>);

impl Captured {
    fn keep(&mut self, what: impl Into<String>, bytes: impl Into<Vec<u8>>) {
        self.0.push((what.into(), bytes.into()));
    }

    fn output(&mut self, what: &str, out: &Output) {
        self.keep(format!("{what}: stdout"), out.stdout.clone());
        self.keep(format!("{what}: stderr"), out.stderr.clone());
    }

    fn tap(&mut self, what: &str, tap: Tap) {
        for (n, stream) in tap.streams().into_iter().enumerate() {
            self.keep(format!("{what}: stream {n}"), stream);
        }
        tap.restore();
    }

    fn containing(&self, needle: &[u8]) -> usize {
        self.0
            .iter()
            .filter(|(_, b)| b.windows(needle.len()).any(|w| w == needle))
            .count()
    }
}

/// Stopping a daemon that is to leave no core: the preamble every start
/// below makes, with the daemon's stdout sent to its log, so a line
/// written there is scanned too.
const NO_CORE: &str = "ulimit -c 0\nexec 1>&2";

/// The key file damaged in the ways a read of it could quote what it
/// read: each still holds the seed, so an error that printed the file
/// would carry it.
fn damaged_keys(key: &[u8]) -> Vec<(&'static str, Vec<u8>, u32)> {
    let mut flipped = key.to_vec();
    flipped[0] ^= 0xff;
    let mut longer = key.to_vec();
    longer.push(0);
    vec![
        ("a key one byte short", key[..key.len() - 1].to_vec(), 0o600),
        ("a key one byte long", longer, 0o600),
        ("a key whose header is damaged", flipped, 0o600),
        ("a key readable by others", key.to_vec(), 0o644),
    ]
}

/// S11's scan, and threat-model.md's "Log leakage", "Private-key theft"
/// and "Recovery-phrase theft" rows: neither the recovery phrase nor the
/// key it is reaches ANY stream the daemon or `transportctl` writes,
/// read whole rather than sampled. The run: `transportctl` refuses three
/// wrong restores and makes the right one; the daemon runs on it beside a
/// peer at debug level, and over its two sockets, recorded byte for byte,
/// `transportctl`'s every admin and identity command and a client's
/// session exchange direct messages and broadcasts with the peer; the
/// daemon is then killed by `SIGABRT` and started on each damaged key.
/// Scanned: both daemons' stderr and stdout, every `transportctl` stdout
/// and stderr, every IPC connection in each direction, and each crash's
/// output. The controls: the daemon logged at debug; the tapped streams
/// carried the hellos, the admin requests and the messages; and the scan
/// finds the secret in the record that was restored and in the key file.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_phrase_and_its_key_reach_no_stream_the_daemon_or_transportctl_writes() {
    let identity = ProfileIdentity::generate();
    let secret = Secret::of(&identity);
    let peer = identity.transport_identity().expect("a peer id");
    let record = record_of(&identity);
    let mut captured = Captured::default();

    let mut pair = Pair::start_with(
        |a| {
            for (what, args, input) in wrong_restores(&identity, &secret) {
                let out = run(a, &args, &input);
                assert_eq!(out.status.code(), Some(1), "{what}: {}", text(&out.stderr));
                captured.output(what, &out);
            }
            let out = a.transportctl(&["identity", "restore", "--new"], &record);
            assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
            captured.output("the restore", &out);
            let out = a.transportctl(&["identity", "show"], "");
            assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
            captured.output("identity show", &out);
            stored(a).expect("restored")
        },
        Some(NO_CORE),
    )
    .await;
    assert_eq!(pair.a_peer, peer);

    let data = Tap::interpose(&pair.a.data_socket());
    let admin = Tap::interpose(&pair.a.admin_socket());
    let someone = stranger();
    for args in [
        &["status"][..],
        &["status", "--json"],
        &["endpoints", "list"],
        &["endpoints", "list", "--json"],
        &["trust", "list"],
        &["trust", "list", "--json"],
        &["peers", "list"],
        &["peers", "list", "--json"],
        &["trust", "allow", someone.as_str()],
        &["trust", "revoke", someone.as_str()],
        &["endpoints", "disable", "claude"],
        &["endpoints", "enable", "claude"],
    ] {
        let out = pair.a.transportctl(args, "");
        assert_eq!(
            out.status.code(),
            Some(0),
            "{args:?}: {}",
            text(&out.stderr)
        );
        captured.output(&format!("transportctl {}", args.join(" ")), &out);
    }
    // The identity commands against a running profile: refused at the
    // lock, with the phrase on stdin for the restore.
    let backup = pair.a.root.path().join("during.json");
    let replace = [
        "identity",
        "restore",
        "--replace",
        "--replacing",
        peer.as_str(),
    ];
    for (args, input) in [
        (
            &[
                "identity",
                "backup",
                "--to-file",
                backup.to_str().expect("utf-8"),
            ][..],
            String::new(),
        ),
        (&replace[..], format!("{}\n", secret.phrase())),
    ] {
        let out = pair.a.transportctl(args, &input);
        assert_eq!(
            out.status.code(),
            Some(1),
            "{args:?}: {}",
            text(&out.stderr)
        );
        captured.output(&format!("transportctl {}", args.join(" ")), &out);
    }

    exchange(&pair, &peer).await;
    captured.tap("the data socket", data);
    captured.tap("the admin socket", admin);
    assert!(pair.a_daemon.terminate().await.success(), "{}", pair.logs());
    assert!(pair.b_daemon.terminate().await.success(), "{}", pair.logs());
    let a_log = pair.a_daemon.log();
    assert!(a_log.contains("DEBUG"), "the control: A logged at debug");
    captured.keep("A's stderr and stdout", a_log);
    captured.keep("B's stderr", pair.b_daemon.log());

    let intact = crash_outputs(&pair.a, &mut captured).await;

    // The controls: the captures are live, and the scan reads the shapes
    // the secret really takes here.
    for (what, needle) in [
        ("a hello", &b"\"hello\""[..]),
        ("transportctl's trust grant", b"admin.trust.set"),
        ("an endpoint change", b"admin.endpoints.set_enabled"),
        ("a direct send", b"direct.send"),
        ("a message pushed to the session", b"\"event\""),
    ] {
        assert!(captured.containing(needle) > 0, "the taps saw {what}");
    }
    assert!(
        !secret.found_in(record.as_bytes()).is_empty(),
        "the control: the restored record is found"
    );
    assert!(
        !secret.found_in(&intact).is_empty(),
        "the control: the key file is found"
    );

    let leaks: Vec<String> = captured
        .0
        .iter()
        .flat_map(|(what, bytes)| {
            secret
                .found_in(bytes)
                .into_iter()
                .map(move |form| format!("{what}: {form}"))
        })
        .collect();
    assert!(
        captured.0.len() > 40,
        "every stream kept: {}",
        captured.0.len()
    );
    assert_eq!(leaks, Vec::<String>::new(), "the secret reached a stream");
}

/// Direct messages both ways and a broadcast between A's session and
/// B's, and an admin port's reads, over A's (tapped) sockets.
async fn exchange(pair: &Pair, peer: &TransportIdentity) {
    let mine = pair
        .a
        .binding()
        .open(lease_request())
        .await
        .expect("A leases");
    let theirs = pair
        .b
        .binding()
        .open(lease_request())
        .await
        .expect("B leases");
    let marker = b"S11-SCAN-MARKER";
    let deadline = tokio::time::Instant::now() + PATIENCE;
    for (from, to, n) in [(&mine, &pair.b_peer, 1_u8), (&theirs, &pair.a_peer, 2)] {
        loop {
            let sent = from
                .send_direct(
                    DirectDestination {
                        peer: to.clone(),
                        endpoint: Some(human()),
                    },
                    MessageId::from_bytes([n; 16]),
                    payload(marker),
                )
                .await;
            if sent.is_ok() {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "no direct route: {sent:?}\n{}",
                pair.logs()
            );
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
    let general = ChannelId::parse("general").expect("a channel");
    mine.join(general.clone()).await.expect("A joins");
    theirs.join(general.clone()).await.expect("B joins");
    let mut got_direct = false;
    let mut got_broadcast = false;
    let mut n = 16_u8;
    while !(got_direct && got_broadcast) {
        n = n.wrapping_add(1);
        theirs
            .broadcast(
                general.clone(),
                BroadcastMessageV1 {
                    message_id: MessageId::from_bytes([n; 16]),
                    sent_at_ms: 0,
                    payload: payload(marker),
                },
            )
            .await
            .expect("accepted locally");
        for event in arriving(&mine, Duration::from_millis(500)).await {
            match event {
                SessionEvent::Direct(_) => got_direct = true,
                SessionEvent::Broadcast(_) => got_broadcast = true,
                SessionEvent::Local(_) => {}
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "A received direct {got_direct}, broadcast {got_broadcast}:\n{}",
            pair.logs()
        );
    }
    let port = pair
        .a
        .binding()
        .admin([AdminCapability::Status, AdminCapability::Trust].into())
        .await
        .expect("an admin port");
    assert_eq!(&port.status().await.expect("status").peer, peer);
    let _ = port.trust().await.expect("trust");
    drop((mine, theirs, port));
}

/// The daemon's crash outputs on `a`, kept: killed by `SIGABRT` while
/// running on the key, then started -- and `transportctl` run -- on each
/// damaged key. The intact key, put back, is returned.
async fn crash_outputs(a: &Home, captured: &mut Captured) -> Vec<u8> {
    // A crash: the daemon running on the key, killed by SIGABRT.
    let mut aborted = a.start_after(NO_CORE, &[]);
    aborted.serving(a).await;
    let killed = std::process::Command::new("kill")
        .args(["-ABRT", &aborted.child.id().to_string()])
        .status()
        .expect("kill runs");
    assert!(killed.success());
    let status = aborted.exit().await;
    assert!(!status.success(), "killed: {status}");
    captured.keep("the daemon killed by SIGABRT", aborted.log());

    // The daemon, and transportctl, failing on each damaged key.
    let key = a.paths.identity_file();
    let intact = std::fs::read(&key).expect("the key");
    for (what, bytes, file_mode) in damaged_keys(&intact) {
        std::fs::write(&key, &bytes).expect("damaged");
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(file_mode)).expect("chmod");
        let mut refused = a.start_after(NO_CORE, &[]);
        let status = refused.exit().await;
        assert_eq!(status.code(), Some(1), "{what}: {}", refused.log());
        let log = refused.log();
        assert!(
            log.contains("transport-daemon:"),
            "{what}: the refusal printed"
        );
        captured.keep(format!("the daemon on {what}"), log);
        let to = a
            .root
            .path()
            .join(format!("{}.json", what.replace(' ', "-")));
        let to = to.to_str().expect("utf-8");
        for args in [
            &["identity", "show"][..],
            &["identity", "backup", "--to-file", to],
        ] {
            let out = a.transportctl(args, "");
            assert_eq!(out.status.code(), Some(1), "{what}, {args:?}");
            captured.output(&format!("transportctl {} on {what}", args.join(" ")), &out);
        }
    }
    std::fs::write(&key, &intact).expect("put back");
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    intact
}
