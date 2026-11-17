// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A measurement, not a pass/fail case: thirty source daemons each send
//! `k` direct messages of 32 KiB to one endpoint of one sink daemon
//! inside five seconds -- the shape of the fleet's `fabric-ctl all`, one
//! request out and thirty replies back to the coordinator's
//! `fabric-control` endpoint -- with a second endpoint on the same sink
//! receiving two a second from a thirty-first source meanwhile, since
//! both share the sink's ingress buckets (ADR-0026). For each `k` it
//! prints what each sender was answered, by class, what the sink's client
//! read, and how long the last accepted message took to be read.
//!
//! Run by hand and recorded under `tests/automated-client/runs/`; with
//! `INTERWEAVE_PACKAGE_BIN` naming a released archive's `bin/` it
//! measures that release's programs instead of the workspace's.
//!
//! What it cannot show, by name: every daemon is on one host's loopback,
//! so there is no network latency and all 32 daemons share the host's
//! cores; the figures say what the sink's daemon admits, not what a
//! network between two hosts adds.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::time::Duration;

use interweave_automated_client_tests::{Client, ClientConfig, payload};
use interweave_local_client_api::{AdminBinding as _, AdminCapability, AdminPort as _};
use interweave_test_support::e2e::{Daemon, Home, PATIENCE, free_port, private_tempdir};
use interweave_transport_api::{EndpointId, MessageId, TransportError, TransportIdentity};
use tokio::task::JoinSet;
use tokio::time::Instant;

/// The sink's endpoint the thirty sources send to.
const CONTROL: &str = "fabric-control";
/// The sink's second endpoint, the rest of its data plane.
const SIDE: &str = "side";
/// Every source's own endpoint, the lease its client sends from.
const SOURCE: &str = "automation";
const CLIENT_KIND: &str = "automated-test-client";
const CHANNEL: &str = "example:automation";

const SOURCES: usize = 30;
/// One reply part, as the fabric's control link cuts a large one.
const SIZE: usize = 32 * 1024;
/// The window every round's sends fall in.
const WINDOW: Duration = Duration::from_secs(5);
const KS: [usize; 4] = [1, 4, 9, 16];
/// Between two rounds: ADR-0026's per-peer bucket (32 at 2 a second)
/// refills in 16 s and the global one (256 at 20 a second) in 12.8 s, so
/// a round starts on full buckets.
const REST: Duration = Duration::from_secs(20);
/// The side endpoint's sender: two a second.
const SIDE_EVERY: Duration = Duration::from_millis(500);

/// How a source spaces its `k` sends.
#[derive(Clone, Copy, Debug)]
enum Pace {
    /// Evenly over the window, the thirty sources interleaved.
    Spread,
    /// Back to back from the window's start, each answered before the
    /// next is sent.
    Burst,
}

struct Profile {
    home: Home,
    peer: TransportIdentity,
    port: u16,
    daemon: Option<Daemon>,
}

fn loopback() -> std::net::Ipv4Addr {
    std::net::Ipv4Addr::LOCALHOST
}

fn endpoint(name: &str) -> EndpointId {
    EndpointId::parse(name).expect("an endpoint id")
}

impl Profile {
    fn mint(base: &std::path::Path, name: &str) -> Self {
        let home = Home::within(base, name);
        let out = home.transportctl(&["identity", "init"], "");
        assert!(
            out.status.success(),
            "identity init: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let printed = String::from_utf8(out.stdout).expect("utf-8");
        let peer = TransportIdentity::parse(printed.trim_end()).expect("a PeerId printed");
        Self {
            home,
            peer,
            port: free_port(loopback()),
            daemon: None,
        }
    }

    fn address(&self) -> String {
        format!(
            "/ip4/127.0.0.1/tcp/{}/p2p/{}",
            self.port,
            self.peer.as_str()
        )
    }

    /// The sink: every source allowed, its two endpoints, no discovery
    /// provider -- the sources dial it. Its pre-auth bucket for one source
    /// address is widened from 30 starts a minute and 8 pending to 120 and
    /// 32: every source here dials from 127.0.0.1, one bucket, and at the
    /// defaults the 31st start in a minute is refused before its
    /// handshake (measured: 30 of 31 connected). Between hosts the bucket
    /// is each host's address, shared by every account on that host.
    fn render_sink(&self, sources: &[&Self]) {
        let allowed = sources
            .iter()
            .map(|o| format!("\"{}\"", o.peer.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        self.home.write_config(&format!(
            "schema_version: 2
profile: {{ name: {name} }}
trust: {{ policy: static-allowlist, allowed_peers: [{allowed}] }}
endpoints:
  entries:
    - {{ id: {CONTROL}, enabled: true, advertise: false, allowed_client_kinds: [{CLIENT_KIND}] }}
    - {{ id: {SIDE}, enabled: true, advertise: false, allowed_client_kinds: [{CLIENT_KIND}] }}
channels: {{ desired: [] }}
discovery: {{ providers: [] }}
transport:
  listen: {{ addresses: [\"/ip4/127.0.0.1/tcp/{port}\"] }}
  pre_auth: {{ max_pending_per_source_bucket: 32, max_attempts_per_source_bucket_per_minute: 120 }}
observability: {{ log_level: info }}
",
            name = self.home.paths.profile(),
            port = self.port,
        ));
    }

    /// A source: the sink allowed and its one static-bootstrap entry.
    fn render_source(&self, sink: &Self) {
        self.home.write_config(&format!(
            "schema_version: 2
profile: {{ name: {name} }}
trust: {{ policy: static-allowlist, allowed_peers: [\"{sink}\"] }}
endpoints:
  entries:
    - {{ id: {SOURCE}, enabled: true, advertise: false, allowed_client_kinds: [{CLIENT_KIND}] }}
channels: {{ desired: [] }}
discovery:
  providers:
    - {{ type: static-bootstrap, enabled: true, priority: 5, config: {{ peers: [\"{entry}\"] }} }}
transport: {{ listen: {{ addresses: [\"/ip4/127.0.0.1/tcp/{port}\"] }} }}
observability: {{ log_level: info }}
",
            name = self.home.paths.profile(),
            sink = sink.peer.as_str(),
            entry = sink.address(),
            port = self.port,
        ));
    }

    async fn start(&mut self) {
        let mut daemon = self.home.start(&[]);
        daemon.serving(&self.home).await;
        self.daemon = Some(daemon);
    }

    fn log(&self) -> String {
        self.daemon.as_ref().map(Daemon::log).unwrap_or_default()
    }

    /// How many peers this daemon's dial gate holds a connection to.
    async fn connected(&self) -> usize {
        self.home
            .binding()
            .admin([AdminCapability::Status].into())
            .await
            .expect("an admin port")
            .peers()
            .await
            .expect("admin.peers.list")
            .into_iter()
            .filter(|v| v.connected)
            .count()
    }

    async fn client(&self, endpoint: &str) -> Client {
        let mut client = Client::new(
            ClientConfig::new(endpoint, CLIENT_KIND, CHANNEL),
            self.home.binding(),
        );
        client.connect(PATIENCE).await.expect("the client leases");
        client
    }
}

/// A message's id: its round, its sender (the side sender is `SOURCES`)
/// and its number.
fn fan_id(round: u8, source: usize, n: usize) -> MessageId {
    let mut bytes = [0xf1; 16];
    bytes[0] = round;
    bytes[1] = u8::try_from(source).expect("under 256 sources");
    bytes[2..6].copy_from_slice(&u32::try_from(n).expect("n").to_be_bytes());
    MessageId::from_bytes(bytes)
}

fn round_of(id: MessageId) -> u8 {
    id.as_bytes()[0]
}

fn class(outcome: &Result<EndpointId, TransportError>) -> String {
    match outcome {
        Ok(_) => "Accepted".to_owned(),
        Err(e) => format!("{e:?}"),
    }
}

/// One send, as its sender saw it.
struct Answer {
    source: usize,
    id: MessageId,
    /// When it was sent.
    sent: Instant,
    /// When it was answered.
    answered: Instant,
    outcome: Result<EndpointId, TransportError>,
}

/// A client reading, each first copy stamped with the moment its pump
/// returned it.
struct Reader {
    client: Client,
    stamped: usize,
    read_at: HashMap<MessageId, Instant>,
}

impl Reader {
    async fn pump(&mut self) {
        let _ = self.client.pump(PATIENCE).await;
        let now = Instant::now();
        for m in &self.client.received.inbound[self.stamped..] {
            self.read_at.insert(m.message_id, now);
        }
        self.stamped = self.client.received.inbound.len();
    }
}

fn ms(d: Duration) -> u128 {
    d.as_millis()
}

fn percentile(sorted: &[Duration], p: usize) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    sorted[(sorted.len() - 1) * p / 100]
}

#[allow(clippy::too_many_lines)]
async fn round(
    sources: Vec<(usize, Client)>,
    side_from: Client,
    sink: &TransportIdentity,
    readers: (&mut Reader, &mut Reader),
    round: u8,
    k: usize,
    pace: Pace,
) -> (Vec<(usize, Client)>, Client, String) {
    let (control, side) = readers;
    let duplicates_before = (
        control.client.received.duplicates,
        side.client.received.duplicates,
    );
    let t0 = Instant::now() + Duration::from_millis(500);
    let body = "x".repeat(SIZE);
    let mut tasks = JoinSet::new();
    for (s, client) in sources {
        let sink = sink.clone();
        let body = body.clone();
        tasks.spawn(async move {
            let mut answers = Vec::with_capacity(k);
            for n in 0..k {
                let offset = match pace {
                    Pace::Spread => {
                        let slot = u32::try_from(n * SOURCES + s).expect("slot");
                        let slots = u32::try_from(k * SOURCES).expect("slots");
                        WINDOW * slot / slots
                    }
                    Pace::Burst => Duration::ZERO,
                };
                tokio::time::sleep_until(t0 + offset).await;
                let id = fan_id(round, s, n);
                let started = Instant::now();
                let outcome = client
                    .send_payload_once(&sink, endpoint(CONTROL), id, payload(&body))
                    .await;
                answers.push(Answer {
                    source: s,
                    id,
                    sent: started,
                    answered: Instant::now(),
                    outcome,
                });
            }
            (s, client, answers)
        });
    }
    let mut side_set = JoinSet::new();
    {
        let sink = sink.clone();
        side_set.spawn(async move {
            let start = t0.checked_sub(Duration::from_secs(1)).expect("after boot");
            let end = t0 + WINDOW + Duration::from_secs(2);
            let mut answers = Vec::new();
            for n in 0_u32.. {
                let at = start + SIDE_EVERY * n;
                if at > end {
                    break;
                }
                tokio::time::sleep_until(at).await;
                let id = fan_id(round, SOURCES, usize::try_from(n).expect("n"));
                let started = Instant::now();
                let outcome = side_from
                    .send_payload_once(&sink, endpoint(SIDE), id, payload("side"))
                    .await;
                answers.push(Answer {
                    source: SOURCES,
                    id,
                    sent: started,
                    answered: Instant::now(),
                    outcome,
                });
            }
            (side_from, answers)
        });
    }

    let mut back = Vec::new();
    let mut answers: Vec<Answer> = Vec::new();
    let mut side_back = None;
    let mut side_answers: Vec<Answer> = Vec::new();
    let mut settled: Option<Instant> = None;
    loop {
        control.pump().await;
        side.pump().await;
        while let Some(done) = tasks.try_join_next() {
            let (s, client, mut a) = done.expect("a source task");
            back.push((s, client));
            answers.append(&mut a);
        }
        while let Some(done) = side_set.try_join_next() {
            let (client, a) = done.expect("the side task");
            side_back = Some(client);
            side_answers = a;
        }
        if tasks.is_empty() && side_set.is_empty() {
            let at = *settled.get_or_insert_with(Instant::now);
            let all = |answers: &[Answer], reader: &Reader| {
                answers
                    .iter()
                    .filter(|a| a.outcome.is_ok())
                    .all(|a| reader.read_at.contains_key(&a.id))
            };
            if (all(&answers, control) && all(&side_answers, side)) || at.elapsed() > PATIENCE {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    back.sort_by_key(|(s, _)| *s);

    // What the senders were answered.
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    let mut refused_by_source: BTreeMap<usize, usize> = BTreeMap::new();
    for a in &answers {
        *classes.entry(class(&a.outcome)).or_default() += 1;
        if a.outcome.is_err() {
            *refused_by_source.entry(a.source).or_default() += 1;
        }
    }
    let accepted: HashSet<MessageId> = answers
        .iter()
        .filter(|a| a.outcome.is_ok())
        .map(|a| a.id)
        .collect();
    let lost = accepted
        .iter()
        .filter(|id| !control.read_at.contains_key(id))
        .count();
    let read_not_accepted = control
        .read_at
        .keys()
        .filter(|id| round_of(**id) == round && !accepted.contains(id))
        .count();
    let last_answer = answers.iter().map(|a| a.answered).max().expect("sends");
    let last_read = answers
        .iter()
        .filter_map(|a| control.read_at.get(&a.id))
        .max()
        .copied();
    let mut took: Vec<Duration> = answers.iter().map(|a| a.answered - a.sent).collect();
    took.sort_unstable();
    let refusals: Vec<&Answer> = answers.iter().filter(|a| a.outcome.is_err()).collect();
    let first_refusal = refusals.iter().map(|a| a.sent - t0).min();
    let last_refusal = refusals.iter().map(|a| a.sent - t0).max();
    let last_sent = answers.iter().map(|a| a.sent - t0).max().expect("sends");

    // The side endpoint.
    let mut side_classes: BTreeMap<String, usize> = BTreeMap::new();
    for a in &side_answers {
        *side_classes.entry(class(&a.outcome)).or_default() += 1;
    }
    let side_lost = side_answers
        .iter()
        .filter(|a| a.outcome.is_ok() && !side.read_at.contains_key(&a.id))
        .count();
    let side_worst = side_answers
        .iter()
        .filter_map(|a| side.read_at.get(&a.id).map(|r| *r - a.sent))
        .max()
        .unwrap_or_default();

    let mut out = String::new();
    let _ = writeln!(
        out,
        "round {round}: pace {pace:?}, k {k}: {} sent of {SIZE} bytes, last sent at {} ms",
        answers.len(),
        ms(last_sent)
    );
    let _ = writeln!(out, "  answered: {classes:?}");
    let _ = writeln!(
        out,
        "  senders refused at least once: {} of {SOURCES}; refusals per refused sender: {:?}",
        refused_by_source.len(),
        refused_by_source.values().collect::<Vec<_>>()
    );
    if let (Some(f), Some(l)) = (first_refusal, last_refusal) {
        let _ = writeln!(out, "  refusals sent between {} and {} ms", ms(f), ms(l));
    }
    let _ = writeln!(
        out,
        "  lost after Accepted: {lost}; read without an Accepted: {read_not_accepted}; \
         duplicates read: {}",
        control.client.received.duplicates - duplicates_before.0
    );
    let _ = writeln!(
        out,
        "  answer latency: p50 {} ms, p99 {} ms, max {} ms; last answer at {} ms",
        ms(percentile(&took, 50)),
        ms(percentile(&took, 99)),
        ms(percentile(&took, 100)),
        ms(last_answer - t0)
    );
    let _ = writeln!(
        out,
        "  every accepted message read by the {CONTROL} client at {} ms",
        last_read.map_or_else(|| "-".to_owned(), |r| ms(r - t0).to_string())
    );
    let _ = writeln!(
        out,
        "  side endpoint: {} sent, answered {side_classes:?}, lost after Accepted {side_lost}, \
         duplicates {}, worst send-to-read {} ms",
        side_answers.len(),
        side.client.received.duplicates - duplicates_before.1,
        ms(side_worst)
    );
    (back, side_back.expect("the side sender returned"), out)
}

/// See the module's documentation. The deployment: a sink with
/// `fabric-control` and `side`, thirty sources with an entry for it, and
/// a thirty-first source for `side`; every source connected before the
/// first round; then for each pace and each `k` one round, `REST` apart.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "a measurement of 32 daemons, run by hand and recorded under tests/automated-client/runs/"]
async fn thirty_sources_fan_in_to_one_endpoint() {
    let base = private_tempdir();
    let mut sink = Profile::mint(base.path(), "sink");
    let mut sources: Vec<Profile> = (0..=SOURCES)
        .map(|i| Profile::mint(base.path(), &format!("source-{i}")))
        .collect();
    sink.render_sink(&sources.iter().collect::<Vec<_>>());
    for s in &sources {
        s.render_source(&sink);
    }
    sink.start().await;
    for s in &mut sources {
        s.start().await;
    }
    let deadline = Instant::now() + Duration::from_secs(60);
    while sink.connected().await < SOURCES + 1 {
        assert!(
            Instant::now() < deadline,
            "{} of {} sources connected\n{}",
            sink.connected().await,
            SOURCES + 1,
            sink.log()
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let mut control = Reader {
        client: sink.client(CONTROL).await,
        stamped: 0,
        read_at: HashMap::new(),
    };
    let mut side = Reader {
        client: sink.client(SIDE).await,
        stamped: 0,
        read_at: HashMap::new(),
    };
    let mut clients = Vec::new();
    for (i, s) in sources.iter().enumerate().take(SOURCES) {
        clients.push((i, s.client(SOURCE).await));
    }
    let mut side_from = sources[SOURCES].client(SOURCE).await;

    let daemon = std::env::var(interweave_test_support::e2e::PACKAGE_BIN)
        .unwrap_or_else(|_| "the workspace's build".to_owned());
    let mut record = format!("daemons: {daemon}\n");
    let mut n = 0_u8;
    for pace in [Pace::Spread, Pace::Burst] {
        for k in KS {
            tokio::time::sleep(REST).await;
            n += 1;
            let (back, side_back, out) = round(
                clients,
                side_from,
                &sink.peer,
                (&mut control, &mut side),
                n,
                k,
                pace,
            )
            .await;
            clients = back;
            side_from = side_back;
            print!("{out}");
            record.push_str(&out);
        }
    }
    let path = std::env::current_exe()
        .expect("the test binary's path")
        .with_file_name("fan_in.txt");
    std::fs::write(&path, &record).expect("the record is written");
    println!("record: {}", path.display());
}
