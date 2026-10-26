// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Android half of `tests/android-e2e`'s cases (plan §20 gate (c)):
//! one body, two runners (architect-cto's DECISION of 2026-10-10). The
//! host stand-in runs a case over the embedded runtime's in-process
//! binding, which proves the body on the host first; the app's
//! instrumentation runs the same body in the app's own process, over the
//! binding of the runtime the app's service started, so a failure on the
//! phone is a platform fact and not a case bug. The host never holds a
//! binding to the phone: it names a case, passes its arguments as JSON,
//! and reads the result JSON back -- [`run`] is that whole seam.
//!
//! What a case here does NOT do is the desktop side, the relay and the
//! lifecycle: those are the host's (`tests/android-e2e`), which pairs
//! each case with its own half -- D replying to what the Android side
//! sends, and reading the path D was told.
//!
//! TEST-ONLY. Its normal dependencies reach no libp2p, test harness,
//! daemon or transport runtime, so it can be built for the phone
//! (`the_crate_reaches_no_libp2p_harness_daemon_or_runtime` below). That
//! the release APK's graph names it nowhere is DECISION 01a12770's
//! invariant, checked by the packaging batch's check, which does not
//! exist yet.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::time::Duration;

use interweave_local_client_api::{
    AdminBinding, AdminCapability, AdminPort as _, DataCapability, DataSessionBinding,
    DataSessionPort, LocalSessionEvent, ROUTE_ESTABLISHED, SessionEvent, SessionRequest,
    TrustChange,
};
use interweave_profile_config::{ProfilePaths, TrustBoundary, resolve_private_dir_within};
use interweave_transport_api::{
    DirectDestination, EndpointId, MediaType, MessageId, Payload, PeerPath, TransportError,
    TransportIdentity,
};
use serde_json::{Map, Value};

mod human_chat;
mod pairing;

pub use human_chat::{
    AppClient, Handed, HumanChatArgs, Recording, RecordingSession, Tap, chat_text,
    desktop_message_id, parse_payloads,
};
pub use pairing::{
    AddedAddressArgs, AwaitPeerArgs, PairArgs, PairExpect, PeerCacheArgs, pair_close, pair_message,
};

/// The result JSON's keys. The instrumentation copies each to the status
/// key `interweave.<key>` (the seam agreed with rust-ui-dev, 01a12771).
pub mod keys {
    /// The case's name, as asked.
    pub const CASE: &str = "case";
    /// [`PASS`] or [`FAIL`].
    pub const RESULT: &str = "result";
    /// Why a case failed; absent on a pass.
    pub const DETAIL: &str = "detail";
    /// The profile's `PeerId` (the `identity` case).
    pub const PEER: &str = "peer";
    /// The path a case observed to the desktop (`paths`).
    pub const PATH: &str = "path";
    /// Where the payloads the runtime handed the app's client were
    /// written, for the host to read and validate (`human_chat`; the
    /// file's shape is [`crate::parse_payloads`]'s).
    pub const PAYLOADS: &str = "payloads";
    /// The envelopes this side sent, as its facade committed them
    /// (`human_chat`): a JSON array, as TEXT -- every value crosses the
    /// instrumentation's status as a string, so a structured one would
    /// arrive as a string on a device and as structure on the stand-in.
    pub const SENT: &str = "sent";
    /// The runtime root the embedded runtime's private directories lie
    /// under (`trust_boundary`).
    pub const ROOT: &str = "runtime_root";
    /// The private directories found, each `<path> <octal mode>`
    /// (`trust_boundary`): a JSON array, as TEXT, as [`SENT`] is.
    pub const PRIVATE_DIRS: &str = "private_dirs";
    /// Why a directory under the platform's `files/` was refused
    /// (`trust_boundary`).
    pub const REFUSED: &str = "refused";
    /// What the ancestor walk ALONE said of that directory under
    /// `files/`, `accepted` or `refused: <why>` -- measured, not
    /// required (`trust_boundary`), since it rests on the platform's
    /// name service.
    pub const FILES_WALK_ALONE: &str = "files_walk_alone";
    /// The end of the hold on the desktop's circuit route, milliseconds
    /// since the Unix epoch, as TEXT (`route_hold`).
    pub const HELD_UNTIL: &str = "held_until_ms";
    /// How many reads of the peer row the watch made, every one finding
    /// the route held, its end within a second of the first read's, as
    /// TEXT (`route_hold`).
    pub const READS: &str = "reads";
    /// [`super::cases::ADDED_ADDRESS`] and [`super::cases::AWAIT_PEER`]:
    /// whether an address for the peer was held from an earlier run and
    /// removed first.
    pub const REMOVED_FIRST: &str = "removed_first";
    /// [`super::cases::ADDED_ADDRESS`] and [`super::cases::AWAIT_PEER`]:
    /// milliseconds from holding the address, or from the far end's
    /// start, to the peer's row reading connected.
    pub const CONNECTED_AFTER: &str = "connected_after_ms";
    /// [`super::cases::PAIR`]: the sends tried, and refused, while apart.
    pub const TRIES: &str = "tries";
    /// [`super::cases::PAIR`], the first side of a crossing step: its
    /// close's outcome, `accepted` or the error's class name.
    pub const CLOSE: &str = "close";
    /// [`super::cases::PAIR`]: whether this side trusts the peer at the
    /// step's end.
    pub const TRUSTED: &str = "trusted";
    /// [`super::cases::PAIR`]: the peer row's last outcome at the step's
    /// end, `none` with no row or no dial yet.
    pub const LAST_OUTCOME: &str = "last_outcome";
    /// [`super::cases::ADDED_ADDRESS`] with `remove_after`: whether the
    /// address was taken out of the overlay after the hold.
    pub const REMOVED_AFTER: &str = "removed_after";
    /// [`super::cases::PEER_CACHE`]: whether the peer cache's file exists.
    pub const CACHE_FILE: &str = "cache_file";
    /// [`super::cases::PEER_CACHE`]: whether that file names the peer.
    pub const CACHED: &str = "cached";
    /// [`super::cases::PEER_CACHE`]: whether the case removed the file.
    pub const FORGOTTEN: &str = "forgotten";
    /// [`RESULT`] of a case that held.
    pub const PASS: &str = "pass";
    /// [`RESULT`] of a case that did not, or could not run.
    pub const FAIL: &str = "fail";
}

/// The case names [`run`] answers.
pub mod cases {
    /// The profile's `PeerId`; needs no runtime.
    pub const IDENTITY: &str = "identity";
    /// Write the profile's `config.yaml` (argument `config`) through the
    /// runner's own provisioning, for the next start to read; needs no
    /// runtime.
    pub const PROVISION: &str = "provision";
    /// One direct message to the desktop and its reply, and the path this
    /// side was told the route began on. Arguments: [`super::PathsArgs`].
    pub const PATHS: &str = "paths";
    /// Plan §20 gate (d), in the app's process: the runtime root sits
    /// directly under the app data directory, it and every private
    /// directory of the profile (argument `profile`) are owner-only, and a
    /// private directory under the platform's `files/` is refused by the
    /// runtime root -- beside the control, a directory beside the runtime
    /// root that the ancestor walk alone accepts and the runtime root
    /// refuses.
    pub const TRUST_BOUNDARY: &str = "trust_boundary";
    /// Plan §20 gate (g)'s Android half: set a peer (argument `peer`)
    /// trusted through the admin port, which the runtime records on its
    /// audit target. Whether the record reached the platform's log under
    /// the profile's stricter level is the host's to read, with
    /// `Device::log`.
    pub const AUDIT: &str = "audit";

    /// ADR-0011 A 2026-10-21 in the app's process: this side's only route
    /// to the desktop is a circuit through a relay that is down, so the
    /// failed hop holds the route. The case waits for the hold on the
    /// admin port's peer row, then reads the row through
    /// [`super::RouteHoldArgs::watch`] -- several of the reconnect round's
    /// one-second ticks -- and fails if the hold is set again (a hop tried
    /// inside it) or is lifted; a lifted hold is waited on until the
    /// desktop connects, and the failure says whether it did. Arguments:
    /// [`super::RouteHoldArgs`]; answers [`super::keys::HELD_UNTIL`] and
    /// [`super::keys::READS`].
    pub const ROUTE_HOLD: &str = "route_hold";

    /// `HumanChatV2` with the desktop on the app's own client, both ways,
    /// plain and `;ce=br`. Arguments: [`super::HumanChatArgs`].
    pub const HUMAN_CHAT: &str = "human_chat";

    /// Plan §29 (e), the first device row: an address added for a trusted
    /// peer through the binding is dialled, and survives a restart as an
    /// added entry that is dialled again, with no entry authored for the
    /// peer; each time the connection is held up for a while before the
    /// case ends. For the peer cache's rows, the address can be taken out
    /// of the overlay after the hold, and a later start can require it
    /// absent, and then either connect (from the cache alone) or stay
    /// unconnected for a window (no source left). Arguments:
    /// [`super::AddedAddressArgs`]; answers [`super::keys::REMOVED_FIRST`]
    /// on an add, [`super::keys::CONNECTED_AFTER`] when it connects and
    /// [`super::keys::REMOVED_AFTER`] with `remove_after`.
    pub const ADDED_ADDRESS: &str = "added_address";

    /// The far end of [`ADDED_ADDRESS`]: hold no bootstrap address for the
    /// peer, wait until its row reads connected -- the peer's dial -- and
    /// then until the peer has left, so this side's runtime outlives the
    /// dialler's reading of its own row. With `apart`, stay up and
    /// listening for the window with the peer unconnected instead: the
    /// far end of [`ADDED_ADDRESS`]'s control.
    /// Arguments: [`super::AwaitPeerArgs`]; answers
    /// [`super::keys::REMOVED_FIRST`] and [`super::keys::CONNECTED_AFTER`].
    pub const AWAIT_PEER: &str = "await_peer";

    /// Plan §29 (e), the second device row, one step of it: add the other
    /// phone's address (in place of the scan), trust it or not, and expect
    /// the pair apart for a window or a message across each way.
    /// Arguments: [`super::PairArgs`]; answers [`super::keys::TRIES`] when
    /// apart, [`super::keys::CLOSE`] on a crossing's first side,
    /// [`super::keys::TRUSTED`] and [`super::keys::LAST_OUTCOME`].
    pub const PAIR: &str = "pair";

    /// The peer cache on disk between two starts, with no runtime serving
    /// (it fails if one is, since a runtime rewrites the file): whether
    /// the profile's cache file exists and names a peer, and with
    /// `forget` the file removed, so the next start begins with an empty
    /// cache. Arguments: [`super::PeerCacheArgs`]; answers
    /// [`super::keys::CACHE_FILE`], [`super::keys::CACHED`] and
    /// [`super::keys::FORGOTTEN`].
    pub const PEER_CACHE: &str = "peer_cache";

    /// The cases a runner must have the runtime ALONE serving for: the
    /// app's runtime with the app's launch, identity and paths, and no
    /// store and no facade, so the `human` lease is free for the case
    /// (01a128a6/01a128a8). On the host stand-in that is an `EmbeddedHost`
    /// started so. On a phone it is a runtime-alone start in the app's
    /// service host (`ServiceHost::start_runtime_alone`, called by the
    /// instrumentation in apps/human-android/src/e2e.rs). The others run
    /// before it starts, as [`PROVISION`] must.
    pub const NEED_A_RUNTIME: &[&str] = &[
        PATHS,
        TRUST_BOUNDARY,
        AUDIT,
        ROUTE_HOLD,
        ADDED_ADDRESS,
        AWAIT_PEER,
        PAIR,
    ];

    /// The cases a runner must have the app's service serving for in
    /// full -- runtime, store, facade and hub -- handing the case
    /// [`super::CaseCtx::client`].
    pub const NEED_THE_CLIENT: &[&str] = &[HUMAN_CHAT];
}

/// How long a case waits for a route, a message or a notice when its
/// arguments name no deadline: the harness's own patience.
pub const DEFAULT_DEADLINE: Duration = Duration::from_secs(30);

/// What a runner hands a case.
pub struct CaseCtx<B> {
    /// The profile's `PeerId`.
    pub peer: TransportIdentity,
    /// The app's data directory: the trust boundary the platform
    /// supplies (ADR-0028), under which the runtime keeps its own.
    pub app_data_dir: PathBuf,
    /// The binding of the runtime now serving, or `None` when none is up:
    /// a case that needs one fails, saying so, rather than starting a
    /// runtime of its own -- the app's service holds the profile's lock,
    /// and a second runtime is not what the app ships.
    pub binding: Option<B>,
    /// The runner's own provisioning: writes `config.yaml` where the
    /// runtime's next start reads it, under the profile's private
    /// directories, as the app's first start does. `None` where the
    /// runner cannot provision.
    pub provision: Option<Provision>,
    /// The app's own client, for a case of [`cases::NEED_THE_CLIENT`]:
    /// `None` where the app's service is not serving in full.
    pub client: Option<Box<dyn AppClient>>,
}

/// How a runner writes the profile's configuration ([`CaseCtx::provision`]).
pub type Provision = Box<dyn FnOnce(&str) -> Result<(), String> + Send>;

/// Run `case` with `args_json`, and answer the result JSON: [`keys::CASE`],
/// [`keys::RESULT`], [`keys::DETAIL`] on a failure, and what the case
/// reports. A panic inside a case is that case's failure, caught here,
/// since the instrumentation calls this across JNI, where an unwind must
/// not cross.
pub fn run<B: DataSessionBinding + AdminBinding>(
    case: &str,
    args_json: &str,
    ctx: CaseCtx<B>,
) -> String {
    let outcome = catch_unwind(AssertUnwindSafe(|| dispatch(case, args_json, ctx)));
    answer(
        case,
        outcome.unwrap_or_else(|panic| Err(panic_text(panic.as_ref()))),
    )
}

/// A case's outcome as the result JSON both runners hand back. Every value
/// is text: the instrumentation's status carries strings alone, so a
/// structured value would reach the host as text from a phone and as
/// structure from the stand-in, and a host reader written against one
/// would break on the other. A case that answers one fails here, on
/// either runner, naming the key.
fn answer(case: &str, outcome: Result<Map<String, Value>, String>) -> String {
    let mut out = match outcome {
        Ok(fields) => {
            if let Some((key, value)) = fields.iter().find(|(_, value)| !value.is_string()) {
                failed(&format!(
                    "{key} is not text ({value}): every value crosses the instrumentation's status as a string"
                ))
            } else {
                let mut fields = fields;
                fields.insert(keys::RESULT.to_owned(), keys::PASS.into());
                fields
            }
        }
        Err(detail) => failed(&detail),
    };
    out.insert(keys::CASE.to_owned(), case.into());
    Value::Object(out).to_string()
}

fn failed(detail: &str) -> Map<String, Value> {
    let mut out = Map::new();
    out.insert(keys::RESULT.to_owned(), keys::FAIL.into());
    out.insert(keys::DETAIL.to_owned(), detail.into());
    out
}

fn panic_text(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "the case panicked".to_owned())
}

fn dispatch<B: DataSessionBinding + AdminBinding>(
    case: &str,
    args_json: &str,
    ctx: CaseCtx<B>,
) -> Result<Map<String, Value>, String> {
    let args: Value =
        serde_json::from_str(args_json).map_err(|e| format!("arguments are not JSON: {e}"))?;
    let mut out = Map::new();
    match case {
        cases::IDENTITY => {
            out.insert(keys::PEER.to_owned(), ctx.peer.as_str().into());
        }
        cases::PROVISION => {
            let config = args
                .get("config")
                .and_then(Value::as_str)
                .ok_or("argument \"config\" is missing")?;
            let provision = ctx
                .provision
                .ok_or("this runner cannot provision a profile")?;
            provision(config)?;
        }
        cases::PATHS => {
            let args = PathsArgs::from_json(&args)?;
            let binding = ctx
                .binding
                .ok_or_else(|| "no runtime is serving: start the app's service first".to_owned())?;
            let path = block_on(paths(&binding, &args))??;
            out.insert(keys::PATH.to_owned(), path.label().into());
        }
        cases::TRUST_BOUNDARY => {
            let profile = args
                .get("profile")
                .and_then(Value::as_str)
                .ok_or("argument \"profile\" is missing")?;
            if ctx.binding.is_none() {
                return Err("no runtime is serving: start the app's service first".to_owned());
            }
            out.extend(trust_boundary(&ctx.app_data_dir, profile)?);
        }
        cases::AUDIT => {
            let peer = args
                .get("peer")
                .and_then(Value::as_str)
                .and_then(|p| TransportIdentity::parse(p).ok())
                .ok_or("argument \"peer\" is not a PeerId")?;
            let binding = ctx
                .binding
                .ok_or_else(|| "no runtime is serving: start the app's service first".to_owned())?;
            block_on(audit(&binding, peer))??;
        }
        cases::ROUTE_HOLD => {
            let args = RouteHoldArgs::from_json(&args)?;
            let binding = ctx
                .binding
                .ok_or_else(|| "no runtime is serving: start the app's service first".to_owned())?;
            out.extend(block_on(route_hold(&binding, &args))??);
        }
        cases::ADDED_ADDRESS => {
            let args = AddedAddressArgs::from_json(&args)?;
            let binding = ctx
                .binding
                .ok_or_else(|| "no runtime is serving: start the app's service first".to_owned())?;
            out.extend(block_on(pairing::added_address(&binding, &args))??);
        }
        cases::AWAIT_PEER => {
            let args = AwaitPeerArgs::from_json(&args)?;
            let binding = ctx
                .binding
                .ok_or_else(|| "no runtime is serving: start the app's service first".to_owned())?;
            out.extend(block_on(pairing::await_peer(&binding, &args))??);
        }
        cases::PAIR => {
            let args = PairArgs::from_json(&args)?;
            let binding = ctx
                .binding
                .ok_or_else(|| "no runtime is serving: start the app's service first".to_owned())?;
            out.extend(block_on(pairing::pair(&binding, &args))??);
        }
        cases::PEER_CACHE => {
            let args = PeerCacheArgs::from_json(&args)?;
            if ctx.binding.is_some() {
                return Err("a runtime is serving: it rewrites the cache file".to_owned());
            }
            out.extend(pairing::peer_cache(&ctx.app_data_dir, &args)?);
        }
        cases::HUMAN_CHAT => {
            let args = HumanChatArgs::from_json(&args)?;
            let mut client = ctx.client.ok_or_else(|| {
                "the app's client is not serving: start the app's service first".to_owned()
            })?;
            out.extend(human_chat::human_chat(
                client.as_mut(),
                &args,
                &ctx.app_data_dir,
            )?);
        }
        other => return Err(format!("no case is named {other:?}")),
    }
    Ok(out)
}

/// A current-thread runtime for one case. The bindings' futures talk to a
/// runtime driven elsewhere -- the embedded host's own executor -- so
/// this one needs only timers, and it is never the host's.
fn block_on<T>(f: impl Future<Output = T>) -> Result<T, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .map_err(|e| format!("no runtime for the case: {e}"))?;
    Ok(rt.block_on(f))
}

/// The arguments of [`cases::PATHS`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathsArgs {
    /// The desktop peer this side exchanges with.
    pub desktop: TransportIdentity,
    /// The path this side must be told its route to the desktop began on.
    pub path: PeerPath,
    /// Which exchange this is: it names both messages and their ids, so
    /// the host's half knows what to wait for and what to answer.
    pub serial: u8,
    /// How long each wait may take.
    pub deadline: Duration,
}

impl PathsArgs {
    /// The JSON a host passes: `desktop`, `path` (`relayed` or `direct`),
    /// `serial` (below 128), and `deadline_ms` (optional).
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut out = Map::new();
        out.insert("desktop".to_owned(), self.desktop.as_str().into());
        out.insert("path".to_owned(), self.path.label().into());
        out.insert("serial".to_owned(), self.serial.into());
        out.insert(
            "deadline_ms".to_owned(),
            u64::try_from(self.deadline.as_millis())
                .unwrap_or(u64::MAX)
                .into(),
        );
        Value::Object(out)
    }

    /// Parse what [`to_json`](Self::to_json) wrote.
    ///
    /// # Errors
    /// A missing or malformed field, by name.
    pub fn from_json(args: &Value) -> Result<Self, String> {
        let field = |name: &str| {
            args.get(name)
                .ok_or_else(|| format!("argument {name:?} is missing"))
        };
        let desktop = field("desktop")?
            .as_str()
            .and_then(|s| TransportIdentity::parse(s).ok())
            .ok_or("argument \"desktop\" is not a PeerId")?;
        let path = match field("path")?.as_str() {
            Some("relayed") => PeerPath::Relayed,
            Some("direct") => PeerPath::Direct,
            _ => return Err("argument \"path\" is neither \"relayed\" nor \"direct\"".to_owned()),
        };
        let serial = field("serial")?
            .as_u64()
            .and_then(|n| u8::try_from(n).ok())
            .filter(|n| *n < 0x80)
            .ok_or("argument \"serial\" is not below 128")?;
        let deadline = match args.get("deadline_ms") {
            None => DEFAULT_DEADLINE,
            Some(v) => Duration::from_millis(
                v.as_u64()
                    .ok_or("argument \"deadline_ms\" is not a count")?,
            ),
        };
        Ok(Self {
            desktop,
            path,
            serial,
            deadline,
        })
    }
}

/// The arguments of [`cases::ROUTE_HOLD`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteHoldArgs {
    /// The desktop peer whose circuit route must be held.
    pub desktop: TransportIdentity,
    /// How long the hold must stay as it was first read: longer than a
    /// few of the reconnect round's one-second ticks, shorter than the
    /// hold (the first one lasts the retry base, 30 s).
    pub watch: Duration,
    /// How long the hold may take to appear.
    pub deadline: Duration,
}

impl RouteHoldArgs {
    /// The JSON a host passes: `desktop`, `watch_ms`, and `deadline_ms`
    /// (optional).
    #[must_use]
    pub fn to_json(&self) -> Value {
        let ms = |d: Duration| u64::try_from(d.as_millis()).unwrap_or(u64::MAX);
        let mut out = Map::new();
        out.insert("desktop".to_owned(), self.desktop.as_str().into());
        out.insert("watch_ms".to_owned(), ms(self.watch).into());
        out.insert("deadline_ms".to_owned(), ms(self.deadline).into());
        Value::Object(out)
    }

    /// Parse what [`to_json`](Self::to_json) wrote.
    ///
    /// # Errors
    /// A missing or malformed field, by name.
    pub fn from_json(args: &Value) -> Result<Self, String> {
        let desktop = args
            .get("desktop")
            .and_then(Value::as_str)
            .and_then(|s| TransportIdentity::parse(s).ok())
            .ok_or("argument \"desktop\" is not a PeerId")?;
        let watch = args
            .get("watch_ms")
            .and_then(Value::as_u64)
            .map(Duration::from_millis)
            .ok_or("argument \"watch_ms\" is not a count")?;
        let deadline = match args.get("deadline_ms") {
            None => DEFAULT_DEADLINE,
            Some(v) => Duration::from_millis(
                v.as_u64()
                    .ok_or("argument \"deadline_ms\" is not a count")?,
            ),
        };
        Ok(Self {
            desktop,
            watch,
            deadline,
        })
    }
}

/// How far apart the holds one failed dial sets may end
/// ([`cases::ROUTE_HOLD`]); far below the 30 s retry base a new hold
/// would add.
const SAME_FAILURE_MS: u64 = 1_000;

/// [`cases::ROUTE_HOLD`]'s body, over the admin port's peer rows.
async fn route_hold(
    binding: &impl AdminBinding,
    args: &RouteHoldArgs,
) -> Result<Map<String, Value>, String> {
    let port = binding
        .admin(BTreeSet::from([AdminCapability::Status]))
        .await
        .map_err(|_| "the admin port with the status capability was refused".to_owned())?;
    let desktop = &args.desktop;
    let read = || async {
        let rows = port
            .peers()
            .await
            .map_err(|e| format!("the peer rows were refused: {e:?}"))?;
        rows.into_iter()
            .find(|row| &row.peer == desktop)
            .ok_or_else(|| format!("no peer row for {}", desktop.as_str()))
    };
    let wall_ms = || {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
    };
    let until = tokio::time::Instant::now() + args.deadline;
    let held = loop {
        let row = read().await?;
        if row.connected {
            return Err(format!("connected to the desktop, nothing held: {row:?}"));
        }
        if let Some(held) = row.route_held_until {
            break held;
        }
        if tokio::time::Instant::now() >= until {
            return Err(format!(
                "no hold on the desktop's route within {:?}: {row:?}",
                args.deadline
            ));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    };
    // A hold that ends inside the watch would be renewed by a retry the
    // policy allows, which this case could not tell from one it forbids.
    let watch_ms = u64::try_from(args.watch.as_millis()).unwrap_or(u64::MAX);
    let read_at = wall_ms();
    if held <= read_at.saturating_add(watch_ms) {
        return Err(format!(
            "the hold ends at {held}, inside the {watch_ms} ms watch from {read_at}"
        ));
    }
    let end = tokio::time::Instant::now() + args.watch;
    let mut reads = 0_u32;
    while tokio::time::Instant::now() < end {
        tokio::time::sleep(Duration::from_millis(250)).await;
        let row = read().await?;
        reads += 1;
        // One failed dial holds each circuit address of the peer through
        // the relay, milliseconds apart; the refusal is any of them and
        // the row reports the latest, so the reported end moves by those
        // milliseconds either way as entries come and go, with no dial
        // (measured on the stand-in, 2026-10-21: 1 ms earlier, and 1 ms
        // later with the last outcome `denied`). A hop tried inside the
        // hold would fail and set a new end the retry delay past ITS
        // moment, seconds later at least. The host's log check is what
        // rules a dial out; this rules out a new hold.
        let same_failure = held.saturating_add(SAME_FAILURE_MS);
        if !row.connected && row.route_held_until.is_some_and(|now| now <= same_failure) {
            continue;
        }
        // A hold the relay's connection lifted (`relay_reached`) is the
        // policy working, not a hop tried inside the hold: a first dial
        // can race the relay's connection at start. What follows the lift
        // is said, so a run with the relay up reads as connected.
        if row.route_held_until.is_none() {
            let lifted = row.clone();
            let connect_by = tokio::time::Instant::now() + args.deadline;
            while tokio::time::Instant::now() < connect_by {
                if read().await?.connected {
                    return Err(format!(
                        "the hold until {held} was lifted (read {reads}) and this side \
                         connected to the desktop: {lifted:?}"
                    ));
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            return Err(format!(
                "the hold until {held} was lifted (read {reads}) and nothing connected: {lifted:?}"
            ));
        }
        return Err(format!(
            "the hold until {held} moved later or connected inside the watch (read {reads}): \
             {row:?}"
        ));
    }
    let mut out = Map::new();
    out.insert(keys::HELD_UNTIL.to_owned(), held.to_string().into());
    out.insert(keys::READS.to_owned(), reads.to_string().into());
    Ok(out)
}

/// Gate (d) in the running app: what `resolve_embedded` resolves for
/// `profile` under the app data directory, judged on disk.
fn trust_boundary(app_data_dir: &Path, profile: &str) -> Result<Map<String, Value>, String> {
    // Where the directory resolves, not how it is spelled: the platform
    // hands over `/data/user/0/<package>`, a symlink to `/data/data` on
    // most devices, and the boundary below is canonical.
    let canonical = std::fs::canonicalize(app_data_dir)
        .map_err(|e| format!("the app data directory {}: {e}", app_data_dir.display()))?;
    let app_data_dir = canonical.as_path();
    let boundary = TrustBoundary::new(app_data_dir)
        .map_err(|e| format!("the app data directory as a boundary: {e}"))?;
    let walk_alone = boundary.clone();
    let paths = ProfilePaths::resolve_embedded(profile, boundary)
        .map_err(|e| format!("the profile's paths: {e}"))?;
    let root = paths
        .boundary()
        .runtime_root()
        .ok_or("the embedded boundary names no runtime root")?
        .to_path_buf();
    if root.parent() != Some(app_data_dir) {
        return Err(format!(
            "the runtime root {} is not directly under {}",
            root.display(),
            app_data_dir.display()
        ));
    }
    let mut found = vec![];
    for dir in [
        root.as_path(),
        paths.config_dir(),
        paths.state_dir(),
        paths.identity_dir(),
        paths.cache_dir(),
    ] {
        if !dir.exists() {
            continue;
        }
        if dir != root && !dir.starts_with(&root) {
            return Err(format!(
                "{} is outside the runtime root {}",
                dir.display(),
                root.display()
            ));
        }
        let mode = std::fs::metadata(dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .permissions()
            .mode()
            & 0o7777;
        if mode != 0o700 {
            return Err(format!("{} is {mode:o}, not 700", dir.display()));
        }
        found.push(format!("{} {mode:o}", dir.display()));
    }
    // The runtime is serving, so at least its config and state are made.
    for dir in [paths.config_dir(), paths.state_dir()] {
        if !dir.exists() {
            return Err(format!(
                "{} does not exist under a serving runtime",
                dir.display()
            ));
        }
    }
    let (refused, files_walk_alone) =
        refused_under_files(app_data_dir, &walk_alone, paths.boundary())?;
    let mut out = Map::new();
    out.insert(keys::ROOT.to_owned(), root.display().to_string().into());
    out.insert(
        keys::PRIVATE_DIRS.to_owned(),
        Value::from(found).to_string().into(),
    );
    out.insert(keys::REFUSED.to_owned(), refused.into());
    out.insert(keys::FILES_WALK_ALONE.to_owned(), files_walk_alone.into());
    Ok(out)
}

/// The probe under the platform's `files/`, and the control's beside the
/// runtime root.
const FILES_PROBE: &str = "interweave-gate-d-probe";
const CONTROL_PROBE: &str = "interweave-gate-d-control";

/// A private directory made under the platform's `files/`, refused by the
/// embedded boundary as outside the runtime root; answers that refusal
/// and what the walk alone said of the same directory.
///
/// The control is a private directory made directly under the app data
/// directory, beside the runtime root: the walk alone must accept it and
/// the embedded boundary refuse it as outside the runtime root, so the
/// rule the gate is about is shown live on a directory nothing else
/// refuses. `files/` itself is not the control, because what the walk
/// alone says of it rests on the platform's name service: `files/` is
/// `0771` in the app's own group, which bionic answers with the app as
/// its one listed member (`getgrgid_r`'s `gr_mem`,
/// `libc/bionic/grp_pwd.cpp`) and a host stand-in with none. ADR-0028
/// accepts both since A 2026-10-15; before it, the walk alone refused
/// `files/` on a Pixel 9a and the control placed there failed. So that
/// verdict is reported, not required.
/// On a stand-in, `files/` is made as Android makes it, `0771`, where it
/// is missing. Both probes are removed either way.
fn refused_under_files(
    app_data_dir: &Path,
    walk_alone: &TrustBoundary,
    embedded: &TrustBoundary,
) -> Result<(String, String), String> {
    let refused_by_root = |dir: &Path| match resolve_private_dir_within(dir, embedded) {
        Ok(_) => Err(format!(
            "{} was accepted by the embedded boundary",
            dir.display()
        )),
        Err(e) if e.to_string().contains("outside the runtime root") => Ok(e.to_string()),
        Err(e) => Err(format!(
            "{} refused, but not by the runtime root: {e}",
            dir.display()
        )),
    };
    with_probe(&app_data_dir.join(CONTROL_PROBE), |control| {
        resolve_private_dir_within(control, walk_alone).map_err(|e| {
            format!(
                "the control failed: the walk alone refused {}: {e}",
                control.display()
            )
        })?;
        refused_by_root(control)
            .map_err(|e| format!("the control failed: {e}"))
            .map(drop)
    })?;
    let files = app_data_dir.join("files");
    if !files.exists() {
        std::fs::create_dir(&files).map_err(|e| format!("files/: {e}"))?;
        std::fs::set_permissions(&files, std::fs::Permissions::from_mode(0o771))
            .map_err(|e| format!("files/: {e}"))?;
    }
    with_probe(&files.join(FILES_PROBE), |probe| {
        let refused = refused_by_root(probe)?;
        let alone = match resolve_private_dir_within(probe, walk_alone) {
            Ok(_) => "accepted".to_owned(),
            Err(e) => format!("refused: {e}"),
        };
        Ok((refused, alone))
    })
}

/// Make `probe` a `0700` directory, run `judge` on it and remove it,
/// whatever `judge` answered. A run killed between the making and the
/// removal leaves it in a device's persistent data directory; the next
/// run takes it back first.
fn with_probe<T>(
    probe: &Path,
    judge: impl FnOnce(&Path) -> Result<T, String>,
) -> Result<T, String> {
    match std::fs::remove_dir(probe) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(format!(
                "a stale probe {} could not be removed: {e}",
                probe.display()
            ));
        }
    }
    std::fs::create_dir(probe).map_err(|e| format!("the probe {}: {e}", probe.display()))?;
    let outcome = std::fs::set_permissions(probe, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| format!("the probe {}: {e}", probe.display()))
        .and_then(|()| judge(probe));
    let _ = std::fs::remove_dir(probe);
    outcome
}

/// Gate (g)'s Android half: one trust change through the admin port.
async fn audit(binding: &impl AdminBinding, peer: TransportIdentity) -> Result<(), String> {
    let port = binding
        .admin(BTreeSet::from([AdminCapability::Trust]))
        .await
        .map_err(|_| "the admin port with the trust capability was refused".to_owned())?;
    match port.set_trust(peer, true).await {
        Ok(TrustChange::Set) => Ok(()),
        // One port here, so nothing supersedes it; said if it ever does.
        Ok(TrustChange::Refused(reason)) => {
            Err(format!("the trust change was refused {}", reason.as_str()))
        }
        Err(_) => Err("the trust change was refused".to_owned()),
    }
}

/// What the Android side sends in exchange `serial`, and its id.
#[must_use]
pub fn to_desktop(serial: u8) -> (MessageId, String) {
    (
        MessageId::from_bytes([serial; 16]),
        format!("android {serial} to desktop"),
    )
}

/// What the desktop answers in exchange `serial`, and its id: the high
/// bit keeps it apart from every id the Android side sends.
#[must_use]
pub fn to_android(serial: u8) -> (MessageId, String) {
    (
        MessageId::from_bytes([serial | 0x80; 16]),
        format!("desktop {serial} to android"),
    )
}

/// What the desktop sends in exchange `serial` once its answer was
/// accepted, and its id: the high bit marks it the desktop's, as
/// [`to_android`]'s, and 0x40 keeps it apart from that answer (every
/// serial is below 0x40).
#[must_use]
pub fn closing(serial: u8) -> (MessageId, String) {
    (
        MessageId::from_bytes([serial | 0xC0; 16]),
        format!("desktop {serial} closes"),
    )
}

/// The desktop's last step of exchange `serial` with `android`, once its
/// answer was accepted: the closing message, sent once, the Android
/// half waiting for it before it ends.
///
/// WHY A CLOSE. On a device the Android side's process ends with its
/// case, and an answer the Android side has read can still be
/// unconfirmed at its sender: the acceptance travels back after the
/// remote admitted the message, and a connection closed in between is
/// "outcome unknown" (`TRANSPORT.md`, "Outcome unknown"), which the
/// desktop's half cannot tell from no route. The 2026-10-20 Pixel run
/// failed exactly so, the phone's case passed
/// (`tests/android-e2e/runs/2026-10-20-pixel-9a-api37.md`). The close is
/// for the caller to send once the answer is accepted, so the Android
/// side outlives that acceptance. No host test sees that order: a
/// stand-in's runtime outlives its case, so a close sent early passes
/// there, and only a device run tells.
///
/// Its own outcome is returned and not relied on: the Android side may
/// end as soon as it reads the close, so "outcome unknown" is expected.
/// A caller prints it beside the Android side's answer, to be read by
/// the three classes of `TRANSPORT.md`'s error model (not dispatched;
/// dispatched and refused; outcome unknown), which say which code is
/// which: only the first is a close that was not sent, and an
/// outcome-unknown one cannot tell a close the Android side read from
/// one it did not. The codes are not restated here, so they cannot drift
/// from the contract.
///
/// # Errors
/// The send's own outcome, a class with no identity in it.
///
/// # Panics
/// Never: the closing text is a valid payload.
pub async fn close_exchange(
    from: &impl DataSessionPort,
    android: &TransportIdentity,
    serial: u8,
) -> Result<(), TransportError> {
    let (id, text) = closing(serial);
    #[allow(clippy::expect_used, reason = "a fixed text, far under the ceiling")]
    let payload = payload(&text).expect("the closing text is a payload");
    from.send_direct(
        DirectDestination {
            peer: android.clone(),
            endpoint: Some(human()),
        },
        id,
        payload,
    )
    .await
    .map(|_| ())
}

/// The Android half of a path case: send to the desktop until a route
/// exists, take its answer, and read the path this side was told the
/// route began on. Which path that must be is the case's to assert, and
/// it is asserted here, on the side that was told it. It ends only on the
/// desktop's [`closing`] message, so its process outlives the desktop's
/// acceptance of the answer ([`close_exchange`]).
async fn paths(binding: &impl DataSessionBinding, args: &PathsArgs) -> Result<PeerPath, String> {
    // The refusal is not printed: it can carry a trusted peer's identity
    // (`send_until_routed`'s note), and what failing here means is said.
    let session = binding
        .open(lease_request())
        .await
        .map_err(|_| "the human endpoint's lease was refused".to_owned())?;
    let (id, text) = to_desktop(args.serial);
    send_until_routed(&session, &args.desktop, id, &text, args.deadline).await?;
    let mut told = Told::default();
    let (_, answer) = to_android(args.serial);
    told.take_message(&session, &args.desktop, &answer, args.deadline)
        .await?;
    let (_, (previous, current)) = told
        .path_notice(&session, &args.desktop, ROUTE_ESTABLISHED, 0, args.deadline)
        .await?;
    if previous.is_some() || current != args.path {
        return Err(format!(
            "the route to the desktop began as {previous:?} -> {current:?}, not None -> {:?}",
            args.path
        ));
    }
    let (_, close) = closing(args.serial);
    told.take_message(&session, &args.desktop, &close, args.deadline)
        .await?;
    Ok(current)
}

/// The `human` endpoint, which the profiles of both sides lease.
///
/// # Panics
/// Never: the literal is a valid endpoint id.
#[must_use]
pub fn human() -> EndpointId {
    #[allow(clippy::expect_used, reason = "a literal checked by every case")]
    EndpointId::parse("human").expect("endpoint")
}

/// A human client's session on the `human` endpoint, with events and
/// commands -- the request `interweave-test-support`'s `e2e` harness
/// makes, restated because that harness never builds for Android.
///
/// # Panics
/// Never: the request is a valid one.
#[must_use]
pub fn lease_request() -> SessionRequest {
    #[allow(clippy::expect_used, reason = "a fixed request checked by every case")]
    SessionRequest::new(
        "human-client",
        Some(human()),
        [DataCapability::Events, DataCapability::Commands],
    )
    .expect("a request")
}

fn payload(text: &str) -> Result<Payload, String> {
    let media = MediaType::parse("text/plain").map_err(|e| format!("a media type: {e}"))?;
    Payload::at_ceiling(Some(media), text.as_bytes().to_vec())
        .map_err(|e| format!("a payload: {e}"))
}

/// Send `text` as `id` from `from` to `to` until a route exists, within
/// `deadline`.
///
/// # Errors
/// No route by the deadline.
pub async fn send_until_routed(
    from: &impl DataSessionPort,
    to: &TransportIdentity,
    id: MessageId,
    text: &str,
    deadline: Duration,
) -> Result<(), String> {
    let until = tokio::time::Instant::now() + deadline;
    loop {
        let sent = from
            .send_direct(
                DirectDestination {
                    peer: to.clone(),
                    endpoint: Some(human()),
                },
                id,
                payload(text)?,
            )
            .await;
        if sent.is_ok() {
            return Ok(());
        }
        // The refusal itself is not printed: this helper is generic over
        // every binding, the in-memory fake's among them, whose refusal
        // carries a trusted peer's identity, and a static analysis reads
        // printing it as logging that identity in clear.
        if tokio::time::Instant::now() >= until {
            return Err(format!("no route to {} within the deadline", to.as_str()));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// What one session has been told so far, kept across reads -- shared by
/// both halves, so the desktop's reading and the Android side's are the
/// same reading.
#[derive(Default)]
pub struct Told {
    /// Path notices as (peer, previous, current, reason class), in order.
    pub paths: Vec<(TransportIdentity, Option<PeerPath>, PeerPath, String)>,
    /// Disconnects, by peer, each with how many path notices preceded it.
    pub gone: Vec<(TransportIdentity, usize)>,
    /// Every direct message read, as (source, payload): one read while
    /// waiting for something else is still there to take
    /// (`a_message_read_while_waiting_for_a_notice_is_still_taken`).
    pub direct: Vec<(TransportIdentity, Vec<u8>)>,
}

impl Told {
    /// Read `session` once, keeping what it said; whether a direct
    /// message `text` from `source` was among it.
    ///
    /// # Errors
    /// The session refused the read.
    pub async fn read(
        &mut self,
        session: &impl DataSessionPort,
        source: &TransportIdentity,
        text: &str,
    ) -> Result<bool, String> {
        let mut found = false;
        let events = session
            .events(64)
            .await
            .map_err(|_| "the session refused a read".to_owned())?;
        for event in events {
            match event {
                SessionEvent::Direct(m) => {
                    found |= &m.source_peer == source && m.payload.bytes() == text.as_bytes();
                    self.direct
                        .push((m.source_peer.clone(), m.payload.bytes().to_vec()));
                }
                SessionEvent::Local(LocalSessionEvent::PeerPathChanged {
                    peer,
                    previous,
                    current,
                    reason_class,
                    ..
                }) => self.paths.push((peer, previous, current, reason_class)),
                SessionEvent::Local(LocalSessionEvent::PeerDisconnected { peer, .. }) => {
                    self.gone.push((peer, self.paths.len()));
                }
                _ => {}
            }
        }
        Ok(found)
    }

    /// Read until `text` from `source` has arrived, within `deadline`:
    /// at once if an earlier read already held it.
    ///
    /// # Errors
    /// It did not arrive, or a read was refused.
    pub async fn take_message(
        &mut self,
        session: &impl DataSessionPort,
        source: &TransportIdentity,
        text: &str,
        deadline: Duration,
    ) -> Result<(), String> {
        let until = tokio::time::Instant::now() + deadline;
        let held = |told: &Self| {
            told.direct
                .iter()
                .any(|(from, bytes)| from == source && bytes == text.as_bytes())
        };
        while !held(self) {
            if tokio::time::Instant::now() >= until {
                return Err(format!("{text:?} from {} never arrived", source.as_str()));
            }
            self.read(session, source, text).await?;
            if !held(self) {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
        Ok(())
    }

    /// Read until a path notice about `peer` with reason `class` has been
    /// told at or after position `from`, within `deadline`; its position
    /// and the notice.
    ///
    /// # Errors
    /// None was told, or a read was refused.
    pub async fn path_notice(
        &mut self,
        session: &impl DataSessionPort,
        peer: &TransportIdentity,
        class: &str,
        from: usize,
        deadline: Duration,
    ) -> Result<(usize, (Option<PeerPath>, PeerPath)), String> {
        let until = tokio::time::Instant::now() + deadline;
        loop {
            if let Some((at, (_, previous, current, _))) = self
                .paths
                .iter()
                .enumerate()
                .skip(from)
                .find(|(_, (p, _, _, c))| p == peer && c == class)
            {
                return Ok((at, (*previous, *current)));
            }
            if tokio::time::Instant::now() >= until {
                let mut told = String::new();
                for (p, previous, current, c) in &self.paths {
                    let _ = write!(told, "[{} {previous:?}->{current:?} {c}] ", p.as_str());
                }
                return Err(format!("no {class} notice for {}: {told}", peer.as_str()));
            }
            self.read(session, peer, "").await?;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use interweave_local_client_api::MAX_EVENT_QUEUE;
    use interweave_local_client_fake::{FakeConfig, FakeEndpoint, FakeNetwork, FakeNode};

    fn peer() -> TransportIdentity {
        TransportIdentity::parse("12D3KooWD3eckifWpRn9wQpMG9R9hX3sD158z7EqHWmweQAJU5SA")
            .expect("a PeerId")
    }

    fn other_peer() -> TransportIdentity {
        TransportIdentity::parse("12D3KooWQYhTNQdmr3ArTeUHRYzFg94BKyTkoWBDWez9kSCVe2Xo")
            .expect("a PeerId")
    }

    fn config(peer: TransportIdentity) -> FakeConfig {
        FakeConfig {
            peer,
            endpoints: vec![FakeEndpoint::open(human(), false)],
            default_endpoint: Some(human()),
            queue_bound: MAX_EVENT_QUEUE,
        }
    }

    /// What the fake tells the Android side about its route to the
    /// desktop, once the route exists.
    #[derive(Clone, Copy)]
    enum Notice {
        /// A first connection on this path: `route_established`, no
        /// previous path.
        Connected(PeerPath),
        /// A `route_established` that nonetheless names a previous path.
        WithPrevious(PeerPath, PeerPath),
    }

    fn paths_on_the_fake(want: PeerPath, told: PeerPath) -> Map<String, Value> {
        paths_told(want, Notice::Connected(told))
    }

    /// The `paths` case on the fake pair -- its third runner: the Android
    /// side runs the case on a thread of its own, as the instrumentation
    /// does, while this test is the desktop's half, answering and then
    /// reporting the route as `told` from the Android side's view.
    fn paths_told(want: PeerPath, told: Notice) -> Map<String, Value> {
        paths_closing(want, told, true, Duration::from_secs(5))
    }

    /// [`paths_told`], the desktop closing the exchange or not, the case
    /// given `deadline`.
    fn paths_closing(
        want: PeerPath,
        told: Notice,
        closes: bool,
        deadline: Duration,
    ) -> Map<String, Value> {
        let (android, desktop) = FakeNetwork::pair(config(peer()), config(other_peer()));
        // The pair starts connected directly, and a first route would be
        // announced on that path; with none, it waits for `connected`.
        android.disconnected(&other_peer());
        let args = PathsArgs {
            desktop: other_peer(),
            path: want,
            serial: 3,
            deadline,
        };
        let case = {
            let android = android.clone();
            let args = args.to_json().to_string();
            std::thread::spawn(move || {
                run(
                    cases::PATHS,
                    &args,
                    CaseCtx {
                        peer: peer(),
                        app_data_dir: PathBuf::from("/nonexistent"),
                        binding: Some(android),
                        provision: None,
                        client: None,
                    },
                )
            })
        };
        let close = block_on(desktop_half(&android, &desktop, told, closes)).expect("a runtime");
        let mut out = result(&case.join().expect("the case thread"));
        // Beside the Android side's answer, as `close_exchange` asks of a
        // caller and `pairing`'s case records it.
        if let Some(sent) = close {
            out.insert(
                keys::CLOSE.to_owned(),
                sent.map_or_else(|e| format!("{e:?}"), |()| "accepted".to_owned())
                    .into(),
            );
        }
        out
    }

    /// The desktop's half; its close's outcome, when it closes.
    async fn desktop_half(
        android: &FakeNode,
        desktop: &FakeNode,
        told: Notice,
        closes: bool,
    ) -> Option<Result<(), TransportError>> {
        let session = desktop.open(lease_request()).await.expect("D leases");
        let (_, text) = to_desktop(3);
        Told::default()
            .take_message(&session, &peer(), &text, Duration::from_secs(5))
            .await
            .expect("the Android side's message");
        // Holding the message does not mean the Android side holds its
        // route: that is recorded once its send returns, and a change
        // before then is the path the route begins on, with no previous
        // (the fake's `routed_sessions`). Without this wait the
        // previous-path test passed by timing (#294's queue build).
        let until = std::time::Instant::now() + Duration::from_secs(5);
        while android.routed_sessions(&other_peer()) == 0 {
            assert!(
                std::time::Instant::now() < until,
                "the Android side's route"
            );
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        match told {
            Notice::Connected(path) => android.connected(&other_peer(), path),
            Notice::WithPrevious(previous, current) => {
                android.path_changed(&other_peer(), previous, current, ROUTE_ESTABLISHED, 0);
            }
        }
        let (id, answer) = to_android(3);
        send_until_routed(&session, &peer(), id, &answer, Duration::from_secs(5))
            .await
            .expect("D answers");
        // Its outcome is returned and not asserted, as [`close_exchange`]
        // says: an Android half that failed -- told the wrong path, or a
        // previous one -- ends without waiting for the close, and its
        // session can be gone before the close arrives (the fake's
        // `RemoteEndpointUnavailable`). A close a passing half needs and
        // does not read fails that half
        // (`the_android_half_waits_for_the_desktops_close`).
        if closes {
            Some(close_exchange(&session, &peer(), 3).await)
        } else {
            None
        }
    }

    /// The Android half ends only on the desktop's close: without it the
    /// case fails, naming the close; the control is the same exchange
    /// closed, which passes.
    #[test]
    fn the_android_half_waits_for_the_desktops_close() {
        let open = paths_closing(
            PeerPath::Relayed,
            Notice::Connected(PeerPath::Relayed),
            false,
            Duration::from_millis(500),
        );
        assert_eq!(open[keys::RESULT], keys::FAIL, "{open:?}");
        let detail = open[keys::DETAIL].as_str().expect("a detail");
        assert!(detail.contains(&closing(3).1), "{detail}");
        assert!(!open.contains_key(keys::CLOSE), "{open:?}");
        let closed = paths_closing(
            PeerPath::Relayed,
            Notice::Connected(PeerPath::Relayed),
            true,
            Duration::from_millis(500),
        );
        assert_eq!(closed[keys::RESULT], keys::PASS, "{closed:?}");
        assert_eq!(closed[keys::CLOSE], "accepted", "{closed:?}");
    }

    /// A direct message read while waiting for something else -- a path
    /// notice reads with no text -- is still there to take afterwards;
    /// the control is a text never sent, which is not.
    #[test]
    fn a_message_read_while_waiting_for_a_notice_is_still_taken() {
        let (android, desktop) = FakeNetwork::pair(config(peer()), config(other_peer()));
        block_on(async {
            let mine = android.open(lease_request()).await.expect("A leases");
            let theirs = desktop.open(lease_request()).await.expect("D leases");
            let (id, text) = closing(1);
            send_until_routed(&theirs, &peer(), id, &text, Duration::from_secs(5))
                .await
                .expect("D sends");
            let mut told = Told::default();
            let until = std::time::Instant::now() + Duration::from_secs(5);
            while told.direct.is_empty() {
                assert!(std::time::Instant::now() < until, "nothing was read");
                told.read(&mine, &other_peer(), "").await.expect("A reads");
            }
            told.take_message(&mine, &other_peer(), &text, Duration::ZERO)
                .await
                .expect("held from the earlier read");
            assert!(
                told.take_message(&mine, &other_peer(), "never sent", Duration::ZERO)
                    .await
                    .is_err()
            );
        })
        .expect("a runtime");
    }

    #[test]
    fn paths_passes_on_the_path_it_was_told_and_fails_on_another() {
        for path in [PeerPath::Relayed, PeerPath::Direct] {
            let out = paths_on_the_fake(path, path);
            assert_eq!(out[keys::RESULT], keys::PASS, "{path:?}: {out:?}");
            assert_eq!(out[keys::PATH], path.label());
        }
        let out = paths_on_the_fake(PeerPath::Relayed, PeerPath::Direct);
        assert_eq!(out[keys::RESULT], keys::FAIL, "{out:?}");
        let detail = out[keys::DETAIL].as_str().expect("a detail");
        assert!(
            detail.contains("Direct") && detail.contains("Relayed"),
            "{detail}"
        );
    }

    #[test]
    fn paths_fails_when_its_route_began_with_a_previous_path_even_the_right_one() {
        let out = paths_told(
            PeerPath::Relayed,
            Notice::WithPrevious(PeerPath::Direct, PeerPath::Relayed),
        );
        assert_eq!(out[keys::RESULT], keys::FAIL, "{out:?}");
        let detail = out[keys::DETAIL].as_str().expect("a detail");
        assert!(detail.contains("Some(Direct)"), "{detail}");
    }

    /// The packages this crate must never reach through a normal
    /// dependency: they do not build for the phone, or would put a second
    /// runtime or the test harness into the instrumentation.
    const NEVER: &[&str] = &[
        "interweave-test-support",
        "interweave-transport-daemon",
        "interweave-transport-libp2p",
        "interweave-transport-runtime",
        "interweave-transport-composition",
        "interweave-transport-embedded",
    ];

    #[test]
    fn the_crate_reaches_no_libp2p_harness_daemon_or_runtime() {
        // The graph as the phone's target resolves it -- the build the
        // claim is about -- which also keeps cargo from needing crates no
        // Android build uses (an iOS-only one is not in CI's cache).
        let out = std::process::Command::new(env!("CARGO"))
            .args([
                "metadata",
                "--format-version",
                "1",
                "--filter-platform",
                "aarch64-linux-android",
                "--manifest-path",
            ])
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
            .output()
            .expect("cargo metadata");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let meta: Value = serde_json::from_slice(&out.stdout).expect("metadata JSON");
        let name_of = |id: &str| {
            meta["packages"]
                .as_array()
                .expect("packages")
                .iter()
                .find(|p| p["id"] == id)
                .and_then(|p| p["name"].as_str())
                .expect("a package's name")
                .to_owned()
        };
        let nodes = meta["resolve"]["nodes"].as_array().expect("nodes");
        let me = nodes
            .iter()
            .find(|n| name_of(n["id"].as_str().expect("id")) == env!("CARGO_PKG_NAME"))
            .expect("this crate's node");
        // The normal-dependency closure: a dev or build edge never ships.
        let mut seen = std::collections::BTreeSet::new();
        let mut todo = vec![me["id"].as_str().expect("id").to_owned()];
        while let Some(id) = todo.pop() {
            let node = nodes
                .iter()
                .find(|n| n["id"] == id.as_str())
                .expect("a node");
            for dep in node["deps"].as_array().expect("deps") {
                let normal = dep["dep_kinds"]
                    .as_array()
                    .expect("dep_kinds")
                    .iter()
                    .any(|k| k["kind"].is_null());
                let pkg = dep["pkg"].as_str().expect("pkg").to_owned();
                if normal && seen.insert(name_of(&pkg)) {
                    todo.push(pkg);
                }
            }
        }
        // The walk is live: what the crate does depend on is in it.
        for wanted in [
            "interweave-local-client-api",
            "interweave-transport-api",
            "tokio",
        ] {
            assert!(seen.contains(wanted), "{wanted} missing from {seen:?}");
        }
        let forbidden: Vec<&String> = seen
            .iter()
            .filter(|n| n.starts_with("libp2p") || NEVER.contains(&n.as_str()))
            .collect();
        assert!(forbidden.is_empty(), "reached: {forbidden:?}");
    }

    /// An app data directory as the platform supplies it, owner-only, with
    /// the profile's config and state directories made by the profile
    /// crate itself, as a serving runtime leaves them.
    fn app_dir_as_a_runtime_leaves_it() -> (tempfile::TempDir, PathBuf) {
        let scratch = tempfile::tempdir().expect("scratch");
        let app = scratch.path().join("app");
        std::fs::create_dir(&app).expect("app");
        std::fs::set_permissions(&app, std::fs::Permissions::from_mode(0o700)).expect("0700");
        let paths = ProfilePaths::resolve_embedded(
            "human-android",
            TrustBoundary::new(&app).expect("boundary"),
        )
        .expect("paths");
        for dir in [paths.config_dir(), paths.state_dir()] {
            interweave_profile_config::create_private_dir_within(dir, paths.boundary())
                .expect("a private dir");
        }
        (scratch, app)
    }

    #[test]
    fn trust_boundary_passes_on_the_layout_a_runtime_leaves_and_reports_the_refusal() {
        let (_scratch, app) = app_dir_as_a_runtime_leaves_it();
        // `0711`, not Android's `0771`: whether the walk accepts a
        // group-writable `files/` rests on this host's primary group, and
        // the verdict asserted below must not.
        let files = app.join("files");
        std::fs::create_dir(&files).expect("files/");
        std::fs::set_permissions(&files, std::fs::Permissions::from_mode(0o711)).expect("0711");
        let out = trust_boundary(&app, "human-android").expect("gate (d) holds");
        // Where the root resolves: a host's temp directory may itself sit
        // behind a symlink, as /var/run does.
        let canonical = std::fs::canonicalize(&app).expect("canonical");
        assert_eq!(
            out[keys::ROOT],
            canonical.join("interweave").display().to_string()
        );
        let dirs: Vec<Value> =
            serde_json::from_str(out[keys::PRIVATE_DIRS].as_str().expect("dirs, as text"))
                .expect("a JSON array");
        assert!(dirs.len() >= 3, "the root, config and state: {dirs:?}");
        assert!(
            dirs.iter()
                .all(|d| d.as_str().is_some_and(|d| d.ends_with(" 700"))),
            "{dirs:?}"
        );
        let refused = out[keys::REFUSED].as_str().expect("refused");
        assert!(refused.contains("outside the runtime root"), "{refused}");
        assert_eq!(out[keys::FILES_WALK_ALONE], "accepted");
        assert!(
            !app.join("files").join(FILES_PROBE).exists(),
            "the probe is removed"
        );
        assert!(!app.join(CONTROL_PROBE).exists(), "the control is removed");
    }

    #[test]
    fn a_files_dir_the_walk_alone_refuses_still_passes_and_is_reported() {
        // Whatever the walk alone says of `files/`, the gate holds: as a
        // device's walk did before ADR-0028 A 2026-10-15, staged here by
        // the one bit the walk refuses on any host, other-write.
        let (_scratch, app) = app_dir_as_a_runtime_leaves_it();
        let files = app.join("files");
        std::fs::create_dir(&files).expect("files/");
        std::fs::set_permissions(&files, std::fs::Permissions::from_mode(0o773)).expect("0773");
        let out = trust_boundary(&app, "human-android").expect("gate (d) holds");
        let refused = out[keys::REFUSED].as_str().expect("refused");
        assert!(refused.contains("outside the runtime root"), "{refused}");
        let alone = out[keys::FILES_WALK_ALONE].as_str().expect("walk alone");
        assert!(
            alone.starts_with("refused: ") && alone.contains("other-writable"),
            "{alone}"
        );
        assert!(!files.join(FILES_PROBE).exists(), "the probe is removed");
    }

    #[test]
    fn the_control_fails_when_the_embedded_boundary_accepts_it() {
        // An embedded boundary with no runtime root accepts the control:
        // the case must fail there, before judging `files/`.
        let (_scratch, app) = app_dir_as_a_runtime_leaves_it();
        let walk_alone = TrustBoundary::new(&app).expect("boundary");
        let detail =
            refused_under_files(&app, &walk_alone, &walk_alone).expect_err("the control fails");
        assert!(detail.starts_with("the control failed: "), "{detail}");
        assert!(
            detail.contains("accepted by the embedded boundary"),
            "{detail}"
        );
        assert!(!app.join(CONTROL_PROBE).exists(), "the control is removed");
        assert!(!app.join("files").exists(), "files/ is not reached");
    }

    #[test]
    fn the_control_fails_when_the_walk_alone_refuses_the_app_data_dir() {
        // The control's own negative: an app data directory the walk
        // refuses cannot show the runtime root's refusal on its own.
        let (_scratch, app) = app_dir_as_a_runtime_leaves_it();
        let walk_alone = TrustBoundary::new(&app).expect("boundary");
        let paths =
            ProfilePaths::resolve_embedded("human-android", walk_alone.clone()).expect("paths");
        let mode = std::fs::metadata(&app).expect("app").permissions().mode();
        std::fs::set_permissions(&app, std::fs::Permissions::from_mode(0o777)).expect("0777");
        let failed = refused_under_files(&app, &walk_alone, paths.boundary());
        std::fs::set_permissions(&app, std::fs::Permissions::from_mode(mode)).expect("restored");
        let detail = failed.expect_err("the control fails");
        assert!(
            detail.starts_with("the control failed: the walk alone refused"),
            "{detail}"
        );
        assert!(!app.join(CONTROL_PROBE).exists(), "the control is removed");
    }

    #[test]
    fn trust_boundary_judges_where_a_symlinked_app_data_dir_resolves() {
        let (scratch, app) = app_dir_as_a_runtime_leaves_it();
        let link = scratch.path().join("data-user-0-link");
        std::os::unix::fs::symlink(&app, &link).expect("a symlink, as /data/user/0 is");
        let out = trust_boundary(&link, "human-android").expect("gate (d) holds through the link");
        let canonical = std::fs::canonicalize(&app).expect("canonical");
        assert_eq!(
            out[keys::ROOT],
            canonical.join("interweave").display().to_string()
        );
    }

    #[test]
    fn a_stale_probe_left_by_a_killed_run_is_taken_back() {
        let (_scratch, app) = app_dir_as_a_runtime_leaves_it();
        let files = app.join("files");
        std::fs::create_dir(&files).expect("files/");
        std::fs::set_permissions(&files, std::fs::Permissions::from_mode(0o771)).expect("0771");
        std::fs::create_dir(files.join(FILES_PROBE)).expect("a stale probe");
        std::fs::create_dir(app.join(CONTROL_PROBE)).expect("a stale control");
        trust_boundary(&app, "human-android").expect("gate (d) holds after a killed run");
        assert!(!files.join(FILES_PROBE).exists(), "the probe is removed");
        assert!(!app.join(CONTROL_PROBE).exists(), "the control is removed");
    }

    #[test]
    fn trust_boundary_fails_on_a_private_dir_that_is_not_owner_only() {
        let (_scratch, app) = app_dir_as_a_runtime_leaves_it();
        let paths = ProfilePaths::resolve_embedded(
            "human-android",
            TrustBoundary::new(&app).expect("boundary"),
        )
        .expect("paths");
        std::fs::set_permissions(paths.state_dir(), std::fs::Permissions::from_mode(0o750))
            .expect("chmod");
        let err = trust_boundary(&app, "human-android").expect_err("a 750 state dir");
        assert!(err.contains("is 750, not 700"), "{err}");
    }

    #[test]
    fn trust_boundary_fails_when_the_runtime_has_made_nothing() {
        let scratch = tempfile::tempdir().expect("scratch");
        let app = scratch.path().join("app");
        std::fs::create_dir(&app).expect("app");
        std::fs::set_permissions(&app, std::fs::Permissions::from_mode(0o700)).expect("0700");
        let err = trust_boundary(&app, "human-android").expect_err("nothing made");
        assert!(
            err.contains("does not exist under a serving runtime"),
            "{err}"
        );
    }

    #[test]
    fn audit_sets_the_peer_trusted_through_the_admin_port() {
        let (android, _desktop) = FakeNetwork::pair(config(peer()), config(other_peer()));
        let stranger =
            TransportIdentity::parse("12D3KooWJWoaqZhDaoEFshF7Rh1bpY9ohihFhzcW6d69Lr2NASuq")
                .expect("a PeerId");
        let args = serde_json::json!({ "peer": stranger.as_str() }).to_string();
        let out = result(&run(
            cases::AUDIT,
            &args,
            CaseCtx {
                peer: peer(),
                app_data_dir: PathBuf::from("/nonexistent"),
                binding: Some(android.clone()),
                provision: None,
                client: None,
            },
        ));
        assert_eq!(out[keys::RESULT], keys::PASS, "{out:?}");
        let admin = block_on(android.admin(BTreeSet::from([AdminCapability::Status])))
            .expect("rt")
            .expect("admin");
        let rows = block_on(admin.peers()).expect("rt").expect("rows");
        assert!(
            format!("{rows:?}").contains(stranger.as_str()),
            "the stranger is trusted now: {rows:?}"
        );
    }

    #[test]
    fn paths_arguments_round_trip_and_name_what_is_wrong() {
        let args = PathsArgs {
            desktop: peer(),
            path: PeerPath::Relayed,
            serial: 5,
            deadline: Duration::from_millis(1500),
        };
        assert_eq!(PathsArgs::from_json(&args.to_json()), Ok(args.clone()));
        for (field, bad, says) in [
            ("path", Value::from("both"), "\"path\""),
            ("serial", Value::from(128), "\"serial\""),
            ("desktop", Value::from("not a peer"), "\"desktop\""),
            ("deadline_ms", Value::from(-1), "\"deadline_ms\""),
        ] {
            let mut json = args.to_json();
            json[field] = bad;
            let err = PathsArgs::from_json(&json).expect_err(field);
            assert!(err.contains(says), "{field}: {err}");
        }
        let mut json = args.to_json();
        json.as_object_mut().unwrap().remove("deadline_ms");
        assert_eq!(
            PathsArgs::from_json(&json).map(|a| a.deadline),
            Ok(DEFAULT_DEADLINE)
        );
    }

    #[test]
    fn route_hold_arguments_round_trip_and_name_what_is_wrong() {
        let args = RouteHoldArgs {
            desktop: peer(),
            watch: Duration::from_millis(6000),
            deadline: Duration::from_millis(1500),
        };
        assert_eq!(RouteHoldArgs::from_json(&args.to_json()), Ok(args.clone()));
        for (field, bad) in [
            ("desktop", Value::from("not a peer")),
            ("watch_ms", Value::from(-1)),
            ("deadline_ms", Value::from(-1)),
        ] {
            let mut json = args.to_json();
            json[field] = bad;
            let err = RouteHoldArgs::from_json(&json).expect_err(field);
            assert!(err.contains(&format!("{field:?}")), "{field}: {err}");
        }
        let mut json = args.to_json();
        json.as_object_mut().unwrap().remove("deadline_ms");
        assert_eq!(
            RouteHoldArgs::from_json(&json).map(|a| a.deadline),
            Ok(DEFAULT_DEADLINE)
        );
    }

    #[test]
    fn the_two_messages_of_an_exchange_never_share_an_id() {
        for serial in 0..0x80 {
            assert_ne!(to_desktop(serial).0, to_android(serial).0, "{serial}");
            assert_ne!(to_desktop(serial).1, to_android(serial).1, "{serial}");
        }
    }

    fn result(json: &str) -> Map<String, Value> {
        match serde_json::from_str(json).expect("result JSON") {
            Value::Object(map) => map,
            other => panic!("not an object: {other}"),
        }
    }

    #[test]
    fn provision_hands_the_runner_exactly_the_config_and_reports_its_refusal() {
        let written = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let ctx = |answer: Result<(), String>| {
            let written = std::sync::Arc::clone(&written);
            CaseCtx::<FakeNode> {
                peer: peer(),
                app_data_dir: PathBuf::from("/nonexistent"),
                binding: None,
                provision: Some(Box::new(move |config: &str| {
                    written.lock().expect("lock").push(config.to_owned());
                    answer
                })),
                client: None,
            }
        };
        let config = "profile:\n  name: \"x\"\n";
        let args = serde_json::json!({ "config": config }).to_string();
        let out = result(&run(cases::PROVISION, &args, ctx(Ok(()))));
        assert_eq!(out[keys::RESULT], keys::PASS, "{out:?}");
        let out = result(&run(
            cases::PROVISION,
            &args,
            ctx(Err("disk full".to_owned())),
        ));
        assert_eq!(out[keys::RESULT], keys::FAIL);
        assert_eq!(out[keys::DETAIL], "disk full");
        assert_eq!(*written.lock().expect("lock"), [config, config]);
    }

    #[test]
    fn identity_answers_the_peer_without_a_runtime() {
        let out = result(&run::<FakeNode>(
            cases::IDENTITY,
            "{}",
            CaseCtx {
                peer: peer(),
                app_data_dir: PathBuf::from("/nonexistent"),
                binding: None,
                provision: None,
                client: None,
            },
        ));
        assert_eq!(out[keys::RESULT], keys::PASS);
        assert_eq!(out[keys::CASE], cases::IDENTITY);
        assert_eq!(out[keys::PEER], peer().as_str());
        assert!(!out.contains_key(keys::DETAIL));
    }

    /// A binding whose `open` panics: what a case body does when one of
    /// its own checks fails by panicking, as the bodies moved here from
    /// `tests/android-e2e` may.
    struct Panics;

    impl DataSessionBinding for Panics {
        type Session = interweave_local_client_fake::FakeSession;

        #[allow(
            clippy::panic,
            clippy::unused_async_trait_impl,
            reason = "the panic is the input under test, raised where the trait's future runs"
        )]
        async fn open(
            &self,
            _: SessionRequest,
        ) -> Result<Self::Session, interweave_transport_api::TransportError> {
            panic!("a check inside the case failed")
        }
    }

    impl AdminBinding for Panics {
        type Admin = interweave_local_client_fake::FakeAdmin;

        #[allow(
            clippy::panic,
            clippy::unused_async_trait_impl,
            reason = "the panic is the input under test, raised where the trait's future runs"
        )]
        async fn admin(
            &self,
            _: BTreeSet<AdminCapability>,
        ) -> Result<Self::Admin, interweave_transport_api::TransportError> {
            panic!("a check inside the case failed")
        }
    }

    #[test]
    fn a_panic_inside_a_case_is_its_failure_and_does_not_unwind_out_of_run() {
        let args = PathsArgs {
            desktop: peer(),
            path: PeerPath::Direct,
            serial: 1,
            deadline: DEFAULT_DEADLINE,
        }
        .to_json()
        .to_string();
        // `run` returning at all is the property: an unwind would end
        // this test here, as it would abort the app across JNI.
        let out = result(&run(
            cases::PATHS,
            &args,
            CaseCtx {
                peer: peer(),
                app_data_dir: PathBuf::from("/nonexistent"),
                binding: Some(Panics),
                provision: None,
                client: None,
            },
        ));
        assert_eq!(out[keys::RESULT], keys::FAIL);
        assert_eq!(out[keys::DETAIL], "a check inside the case failed");
    }

    #[test]
    fn a_structured_value_fails_its_case_on_either_runner_and_text_passes() {
        let fields = |value: Value| {
            let mut out = Map::new();
            out.insert(keys::ROOT.to_owned(), "/data/user/0/app/interweave".into());
            out.insert(keys::PRIVATE_DIRS.to_owned(), value);
            Ok(out)
        };
        // The control: the same fields, every one text, pass.
        let out = result(&answer(
            cases::TRUST_BOUNDARY,
            fields(r#"["/a 700"]"#.into()),
        ));
        assert_eq!(out[keys::RESULT], keys::PASS, "{out:?}");
        for value in [
            serde_json::json!(["/a 700"]),
            serde_json::json!({"a": 1}),
            7.into(),
            true.into(),
            Value::Null,
        ] {
            let out = result(&answer(cases::TRUST_BOUNDARY, fields(value.clone())));
            assert_eq!(out[keys::RESULT], keys::FAIL, "{value}");
            assert_eq!(out[keys::CASE], cases::TRUST_BOUNDARY);
            let detail = out[keys::DETAIL].as_str().expect("a detail");
            assert!(detail.starts_with(keys::PRIVATE_DIRS), "{detail}");
        }
    }

    #[test]
    fn a_case_that_cannot_run_fails_saying_why_and_never_unwinds() {
        let ctx = || CaseCtx::<FakeNode> {
            peer: peer(),
            app_data_dir: PathBuf::from("/nonexistent"),
            binding: None,
            provision: None,
            client: None,
        };
        let args = PathsArgs {
            desktop: peer(),
            path: PeerPath::Direct,
            serial: 1,
            deadline: DEFAULT_DEADLINE,
        }
        .to_json()
        .to_string();
        let chat = HumanChatArgs {
            desktop: peer(),
            path: PeerPath::Direct,
            side: "C".to_owned(),
            sends: [3, 7],
            answers: [4, 8],
            deadline: DEFAULT_DEADLINE,
        }
        .to_json()
        .to_string();
        for (case, args, says) in [
            ("nonsense", "{}", "no case is named"),
            (cases::PATHS, "not json", "not JSON"),
            (cases::PATHS, "{}", "\"desktop\" is missing"),
            (cases::PATHS, args.as_str(), "no runtime is serving"),
            (cases::PROVISION, "{}", "\"config\" is missing"),
            (cases::PROVISION, r#"{"config":"x"}"#, "cannot provision"),
            (cases::HUMAN_CHAT, "{}", "\"desktop\" is missing"),
            (
                cases::HUMAN_CHAT,
                chat.as_str(),
                "the app's client is not serving",
            ),
        ] {
            let out = result(&run(case, args, ctx()));
            assert_eq!(out[keys::RESULT], keys::FAIL, "{case} {args}");
            assert_eq!(out[keys::CASE], case);
            let detail = out[keys::DETAIL].as_str().expect("a detail");
            assert!(detail.contains(says), "{case} {args}: {detail}");
        }
    }
}
