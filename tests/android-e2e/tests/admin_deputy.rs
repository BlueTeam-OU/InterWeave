// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! TM-42 (threat-model.md, "Android remote message triggers admin
//! action"): a remote message reaching the app's own client cannot act
//! as its admin port. D, a desktop daemon the Android side trusts, sends
//! the Android side's `human` endpoint admin requests as raw direct
//! payloads (trust a stranger, revoke `human`, add the stranger's
//! address, shut down) and as one `HumanChatV2` message. The Android
//! half is `interweave-android-e2e-cases`' `admin_shaped` case, run on
//! the app's own client: the facade, store and hub the app's service
//! composed. It asserts that every payload was handed to the facade (the
//! control showing the would-be deputy got them). It then asserts that
//! the trust list, endpoint leases and bootstrap addresses, read through
//! the Android side's own admin port, are unchanged. A change made
//! through that port is seen by the same reads (the positive control).
//! D's half asserts here that each send was accepted at the endpoint,
//! so nothing was refused before the facade could see it.
//!
//! Two runs. On the host, the Android side is a stand-in for the app's
//! process: the app's own `ServiceHost` over the embedded runtime,
//! started as the instrumentation starts it for a client case. On a
//! device (`#[ignore]`d, `src/adb.rs`), the phone runs the case in the
//! app's process over adb's reversed port.
//!
//! What this does NOT prove: the threat model's residual, code already
//! running in the app's process calling its internals. On the stand-in,
//! the device itself (the stand-in's limits, `src/lib.rs`).

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::net::Ipv4Addr;
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use interweave_android_e2e_cases::{
    ADMIN_SHAPED_CLOSE, AdminShapedArgs, CaseCtx, Tap, admin_shaped, admin_text, cases,
    close_exchange, keys, lease_request,
};
use interweave_android_e2e_tests::adb::AdbDevice;
use interweave_android_e2e_tests::{
    CaseRun, Desktop, Device, PATIENCE, PROFILE, free_port, human, log_capture, try_provision,
};
use interweave_human_android_platform::e2e::HubClient;
use interweave_human_android_platform::{ServiceHost, ServiceLaunch};
use interweave_human_chat_protocol::{HumanChatV2, MessageKind, encode_outbound};
use interweave_local_client_api::{DataSessionBinding as _, DataSessionPort as _};
use interweave_profile_config::sections::LogLevel;
use interweave_profile_identity::{ProfileIdentity, RecoveryPhrase};
use interweave_test_support::e2e::example;
use interweave_transport_api::{
    DirectDestination, MAX_PAYLOAD_BYTES, MediaType, MessageId, Payload, TransportIdentity,
};

/// Where an Android side listens: an `embedded-android` profile listens
/// on a wildcard only.
const WILDCARD: &str = "/ip4/0.0.0.0/tcp/0";

/// How long a stop lets exchanges in flight settle, as the app's own.
const GRACE: Duration = Duration::from_secs(1);

/// The app's process for a client case: its own service host -- runtime,
/// store, facade and hub -- under an app data directory of its own, with
/// the payload recorder, and the runtime's binding handed to the case as
/// the instrumentation hands it (`apps/human-android/src/e2e.rs`).
struct AppStandIn {
    _scratch: tempfile::TempDir,
    app_data_dir: PathBuf,
    peer: TransportIdentity,
    phrase: RecoveryPhrase,
    service: ServiceHost,
    tap: Tap,
}

impl AppStandIn {
    fn new() -> Self {
        let scratch = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .expect("a scratch root");
        let app_data_dir = scratch.path().join("app");
        std::fs::create_dir(&app_data_dir).expect("the app data directory");
        std::fs::set_permissions(&app_data_dir, std::fs::Permissions::from_mode(0o700))
            .expect("owner-only");
        let identity = ProfileIdentity::generate();
        Self {
            _scratch: scratch,
            app_data_dir,
            peer: identity.transport_identity().expect("a valid identity"),
            phrase: identity.recovery_phrase().expect("a phrase"),
            service: ServiceHost::new(),
            tap: Tap::default(),
        }
    }
}

/// `f` on a thread of its own: the service's start and stop block on the
/// runtime's executor, which may not be done on a thread that drives a
/// runtime, and this test's async body is one.
fn off_runtime<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| scope.spawn(f).join().expect("the host thread"))
}

impl Device for AppStandIn {
    fn peer(&self) -> TransportIdentity {
        self.peer.clone()
    }

    fn start(&mut self, config: &str) {
        try_provision(&self.app_data_dir, config).expect("provisioned");
        self.restart();
    }

    fn stop(&mut self) {
        off_runtime(|| self.service.stop(GRACE));
    }

    fn kill(&mut self) {
        off_runtime(|| self.service.stop(Duration::ZERO));
    }

    fn restart(&mut self) {
        let launch = ServiceLaunch {
            app_data_dir: self.app_data_dir.clone(),
            profile: PROFILE.to_owned(),
            identity: ProfileIdentity::from_phrase(&self.phrase).expect("the same identity"),
            store_cipher: Box::new(interweave_human_store::key_custody::FilesystemCustody),
            unlock: None,
        };
        let tap = Arc::clone(&self.tap);
        off_runtime(|| self.service.start_recording(launch, tap)).expect("the service starts");
    }

    /// The process's capture, which D's daemon (a child process) does
    /// not write to: in this binary it is the Android side's alone.
    fn log(&self) -> String {
        log_capture::lines_from(0)
    }

    fn log_is_process_wide(&self) -> bool {
        true
    }

    fn run_case(&self, case: &str, args: &serde_json::Value) -> CaseRun {
        let ctx = CaseCtx {
            peer: self.peer.clone(),
            app_data_dir: self.app_data_dir.clone(),
            binding: self.service.runtime_binding(),
            provision: None,
            client: Some(Box::new(HubClient::attach(
                self.service.hub(),
                Arc::clone(&self.tap),
            ))),
            store_cipher: None,
        };
        let (name, args) = (case.to_owned(), args.to_string());
        CaseRun::on_thread(case, move || {
            interweave_android_e2e_cases::run(&name, &args, ctx)
        })
    }
}

impl Drop for AppStandIn {
    fn drop(&mut self) {
        self.kill();
    }
}

async fn admin_deputy<A: Device>(mut android: A) {
    let mut d = Desktop::new();
    let port = free_port(Ipv4Addr::LOCALHOST);
    let d_listen = format!("/ip4/127.0.0.1/tcp/{port}");
    d.start(&example(
        "human-desktop.yaml",
        &android.peer(),
        &d_listen,
        None,
    ))
    .await;
    android.reach(port);
    android.start(&example(
        "human-android.yaml",
        &d.peer,
        WILDCARD,
        Some(&format!("{d_listen}/p2p/{}", d.peer.as_str())),
    ));
    let stranger = ProfileIdentity::generate()
        .transport_identity()
        .expect("a PeerId");
    let args = AdminShapedArgs {
        desktop: d.peer.clone(),
        stranger: stranger.clone(),
        deadline: PATIENCE,
    };
    let run = android.run_case(cases::ADMIN_SHAPED, &args.to_json());

    let session = d
        .binding()
        .open(lease_request())
        .await
        .expect("D leases human");
    let to = DirectDestination {
        peer: android.peer(),
        endpoint: Some(human()),
    };
    let json = MediaType::parse("application/json").expect("a media type");
    let mut sends: Vec<(MessageId, Payload)> = admin_shaped(&stranger)
        .into_iter()
        .zip(0_u8..)
        .map(|(bytes, n)| {
            (
                MessageId::from_bytes([n | 0x10; 16]),
                Payload::at_ceiling(Some(json.clone()), bytes).expect("a payload"),
            )
        })
        .collect();
    let envelope = HumanChatV2 {
        v: 2,
        kind: MessageKind::Text,
        app_message_id: format!("{:032x}", 0x42),
        text: admin_text(&stranger),
        reply_to: None,
        sent_at_ms: None,
        from_endpoint: None,
    };
    let encoded = encode_outbound(&envelope, MAX_PAYLOAD_BYTES).expect("it fits");
    sends.push((
        MessageId::from_bytes([0x42; 16]),
        Payload::at_ceiling(
            Some(MediaType::parse(&encoded.media_type).expect("the v2 media type")),
            encoded.bytes,
        )
        .expect("a payload"),
    ));
    let log = || format!("D:\n{}\nandroid:\n{}", d.log(), android.log());
    for (id, payload) in sends {
        // Until a route exists; each send is then accepted at the
        // endpoint, never refused before the facade could see it.
        let until = tokio::time::Instant::now() + PATIENCE;
        loop {
            match session.send_direct(to.clone(), id, payload.clone()).await {
                Ok(endpoint) => {
                    assert_eq!(endpoint, human(), "accepted at the human endpoint");
                    break;
                }
                // The refusal is not printed: it can carry a peer's
                // identity (`send_until_routed`'s note).
                Err(_) => assert!(
                    tokio::time::Instant::now() < until,
                    "no route to the Android side\n{}",
                    log()
                ),
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
    let closed = close_exchange(&session, &android.peer(), ADMIN_SHAPED_CLOSE).await;
    let out = run.passed(log);
    eprintln!(
        "admin_deputy: close {closed:?}, handed {:?}",
        out.get(keys::HANDED)
    );
    android.stop();
    d.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admin_shaped_messages_on_the_stand_in_reach_the_facade_and_change_nothing() {
    log_capture::install();
    log_capture::filter_at(LogLevel::Debug);
    admin_deputy(AppStandIn::new()).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs a device with the app's androidTest build: ANDROID_SERIAL and INTERWEAVE_ANDROID_INSTRUMENTATION (src/adb.rs); one phone, so --test-threads 1"]
async fn admin_shaped_messages_on_a_device_reach_the_facade_and_change_nothing() {
    admin_deputy(AdbDevice::connect()).await;
}
