// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The android-e2e cases' client handle (feature `e2e-cases`) over the
//! real embedded runtime on the host: a case attached to the app's hub
//! hears the listing, its commands reach the facade, and the service's
//! stop ends its reading rather than leaving it waiting.

#![allow(clippy::expect_used, clippy::panic)]
#![cfg(all(unix, feature = "e2e-cases"))]

use std::os::unix::fs::PermissionsExt as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

use interweave_android_e2e_cases::{AppClient as _, Tap};
use interweave_human_android_platform::e2e::HubClient;
use interweave_human_android_platform::stand_in;
use interweave_human_android_platform::{ServiceHost, ServiceLaunch};
use interweave_human_app_core::{Command, Update};

const PROFILE: &str = "human";
const WAIT: Duration = Duration::from_secs(20);

#[test]
fn a_case_on_the_hub_hears_the_listing_commands_the_facade_and_hears_the_stop() {
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("tempdir");
    let dir = root.path().join("app");
    std::fs::create_dir(&dir).expect("mkdir");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).expect("chmod");
    stand_in::provision(&dir, PROFILE).expect("provisioned");

    let service = ServiceHost::new();
    let tap = Tap::default();
    service
        .start_recording(
            ServiceLaunch {
                app_data_dir: dir.clone(),
                profile: PROFILE.to_owned(),
                identity: interweave_profile_identity::ProfileIdentity::generate(),
                store_cipher: Box::new(interweave_human_store::key_custody::FilesystemCustody),
            },
            Arc::clone(&tap),
        )
        .expect("started");
    let mut client = HubClient::attach(service.hub(), Arc::clone(&tap));

    let deadline = Instant::now() + WAIT;
    let mut listed = false;
    let mut done = false;
    let mut asked = false;
    while !(listed && done) {
        assert!(Instant::now() < deadline, "listed {listed}, done {done}");
        for update in client.updates(Duration::from_millis(100)).expect("reading") {
            match update {
                Update::Listed(_) => listed = true,
                Update::Done(Command::RecheckStorage) => done = true,
                _ => {}
            }
        }
        if listed && !asked {
            assert!(client.command(Command::RecheckStorage), "the facade runs");
            asked = true;
        }
    }
    assert!(client.handed().is_empty(), "nobody sent anything");

    let _ = service.stop(Duration::from_secs(1));
    let deadline = Instant::now() + WAIT;
    loop {
        match client.updates(Duration::from_millis(100)) {
            Err(why) => {
                assert!(why.contains("stopped"), "{why}");
                break;
            }
            Ok(_) => assert!(Instant::now() < deadline, "the stop was never heard"),
        }
    }
    assert!(!client.command(Command::RecheckStorage), "no facade runs");
}

/// The runtime alone (`NEED_A_RUNTIME`): no facade holds the `human`
/// lease, so a case leases it itself over the host's binding; with the
/// app's full start the same lease is refused, the control.
#[test]
fn on_the_runtime_alone_a_case_leases_human_itself_and_not_beside_the_facade() {
    use interweave_local_client_api::{DataCapability, DataSessionBinding as _, SessionRequest};

    let request = || {
        SessionRequest::new(
            "human-client",
            Some(interweave_transport_api::EndpointId::parse("human").expect("endpoint")),
            [DataCapability::Events, DataCapability::Commands],
        )
        .expect("a request")
    };
    let lease = |service: &ServiceHost| {
        let binding = service.runtime_binding().expect("a runtime serves");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime for the case");
        rt.block_on(async { binding.open(request()).await.map(|_| ()) })
    };
    for alone in [true, false] {
        let root = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .expect("tempdir");
        let dir = root.path().join("app");
        std::fs::create_dir(&dir).expect("mkdir");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).expect("chmod");
        stand_in::provision(&dir, PROFILE).expect("provisioned");
        let service = ServiceHost::new();
        let launch = ServiceLaunch {
            app_data_dir: dir.clone(),
            profile: PROFILE.to_owned(),
            identity: interweave_profile_identity::ProfileIdentity::generate(),
            store_cipher: Box::new(interweave_human_store::key_custody::FilesystemCustody),
        };
        if alone {
            service
                .start_runtime_alone(launch)
                .expect("the runtime starts");
        } else {
            service.start(launch).expect("the app starts");
            // Wait until the facade's session is Ready: its lease is held.
            let mut client = HubClient::attach(service.hub(), Tap::default());
            let deadline = Instant::now() + WAIT;
            let ready = |u: &Update| {
                matches!(
                    u,
                    Update::Client(interweave_human_client_api::ClientEvent::Session(
                        interweave_human_client_api::SessionState::Ready { .. }
                    ))
                )
            };
            while !client
                .updates(Duration::from_millis(100))
                .expect("reading")
                .iter()
                .any(ready)
            {
                assert!(Instant::now() < deadline, "the facade never became Ready");
            }
        }
        let leased = lease(&service);
        assert_eq!(leased.is_ok(), alone, "alone {alone}: {leased:?}");
        let stopped = service.stop(Duration::from_secs(1)).expect("it ran");
        assert!(stopped.session_closed, "nothing left open");
        assert!(
            service.runtime_binding().is_none(),
            "no runtime after the stop"
        );
    }
}
