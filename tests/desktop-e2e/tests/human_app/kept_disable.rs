// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! An administrator's disable outlives the daemon (ADR-0028 A 2026-10-11,
//! #261), so the client cannot take which endpoint is enabled from
//! `config.yaml`: with `human` disabled over admin and the daemon
//! restarted, the shipped binary leases the profile's next human-client
//! endpoint rather than being refused for good.

use std::net::Ipv4Addr;
use std::time::Duration;

use interweave_local_client_api::{AdminBinding as _, AdminCapability, AdminPort as _};
use interweave_transport_api::EndpointId;

use crate::common::{Home, PATIENCE, example, free_port, human, stranger};
use crate::harness as human_app;

fn spare() -> EndpointId {
    EndpointId::parse("human-spare").expect("an endpoint")
}

/// The shipped example with a second endpoint open to the human client,
/// after `human` in the file.
fn home() -> Home {
    let home = Home::new("human-desktop");
    home.write_key();
    let listen = format!("/ip4/127.0.0.1/tcp/{}", free_port(Ipv4Addr::LOCALHOST));
    let shipped = example("human-desktop.yaml", &stranger(), &listen, None);
    let claude = "    - { id: claude,";
    assert_eq!(shipped.matches(claude).count(), 1, "the example's shape");
    let with_spare = shipped.replace(
        claude,
        "    - { id: human-spare, enabled: true, advertise: true, \
         allowed_client_kinds: [human-client], inbound: inherit_profile_trust, \
         outbound: inherit_profile_trust }\n    - { id: claude,",
    );
    home.write_config(&with_spare);
    home
}

/// An admin port for the endpoints. The daemon admits four admin
/// connections, each slot freed when its connection's task ends, so under
/// load a fresh open can be answered `Overloaded` while the last ones
/// close: that answer means try again, and it is, within `PATIENCE` (the
/// full suite once failed here on it).
async fn endpoints_admin(home: &Home) -> impl interweave_local_client_api::AdminPort {
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        match home
            .binding()
            .admin([AdminCapability::Endpoints].into())
            .await
        {
            Ok(admin) => return admin,
            Err(interweave_transport_api::TransportError::Overloaded)
                if tokio::time::Instant::now() < deadline =>
            {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Err(e) => panic!("an admin port: {e:?}"),
        }
    }
}

/// Each endpoint's (enabled, persisted, leased) as the daemon reports it.
async fn rows(home: &Home) -> Vec<(EndpointId, bool, bool, bool)> {
    let admin = endpoints_admin(home).await;
    admin
        .leases()
        .await
        .expect("the endpoints")
        .into_iter()
        .map(|v| (v.endpoint, v.enabled, v.persisted, v.lease.is_some()))
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_disable_kept_across_a_daemon_restart_moves_the_client_to_its_next_endpoint() {
    let home = home();
    let mut daemon = home.start(&[]);
    daemon.serving(&home).await;
    endpoints_admin(&home)
        .await
        .set_endpoint_enabled(human(), false)
        .await
        .expect("disabled");
    assert!(daemon.terminate().await.success(), "{}", daemon.log());

    let mut daemon = home.start(&[]);
    daemon.serving(&home).await;
    let row = |rows: &[(EndpointId, bool, bool, bool)], id: &EndpointId| {
        rows.iter()
            .find(|r| &r.0 == id)
            .cloned()
            .expect("a configured endpoint")
    };
    // The control: the disable survived the restart while `config.yaml`
    // still says enabled -- the state the client used to misread.
    let before = rows(&home).await;
    assert_eq!(row(&before, &human()), (human(), false, true, false));
    assert!(
        std::fs::read_to_string(home.paths.config_file())
            .expect("config")
            .contains("{ id: human, enabled: true,"),
        "the file still says enabled"
    );

    let mut app = human_app::start(&home);
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while !row(&rows(&home).await, &spare()).3 {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the client never leased `human-spare`: {}\n{}",
            app.log(),
            daemon.log()
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let after = rows(&home).await;
    assert_eq!(
        row(&after, &human()),
        (human(), false, true, false),
        "the disabled endpoint stays unleased"
    );
    assert!(app.running(), "{}", app.report());
    assert!(app.terminate().success(), "{}", app.log());
    assert!(daemon.terminate().await.success(), "{}", daemon.log());
}
