// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! An administrator's disable outlives the daemon (ADR-0028 A 2026-10-11,
//! #261), so the client cannot take which endpoint is enabled from
//! `config.yaml`: with `human` disabled over admin and the daemon
//! restarted, the shipped binary is refused the identity its store speaks
//! as, says so, and leases no other -- not the profile's next human-client
//! endpoint, which would send the first identity's rows as another
//! (`human-client-ui.md` §13: a disabled identity is off, nothing sending
//! as it; ADR-0061 decisions 1 and 9).

use std::net::Ipv4Addr;

use interweave_human_ui_model::{ErrorClass, RuntimeHost, UiText, fill, placeholder_en};
use interweave_local_client_api::{AdminBinding as _, AdminCapability, AdminPort};
use interweave_transport_api::EndpointId;

use crate::a11y::Bus;
use crate::common::{Home, example, free_port, human, stranger};
use crate::harness as human_app;

fn spare() -> EndpointId {
    interweave_test_support::endpoint_keys::endpoint_id("human-spare")
}

/// The shipped example with a second endpoint open to the human client,
/// after `human` in the file.
fn home() -> Home {
    let home = Home::new("human-desktop");
    home.write_key();
    let listen = format!("/ip4/127.0.0.1/tcp/{}", free_port(Ipv4Addr::LOCALHOST));
    let shipped = example("human-desktop.yaml", &stranger(), &listen, None);
    let claude = format!(
        "    - {{ key: {}.key,",
        interweave_test_support::endpoint_keys::endpoint_id("claude").as_str()
    );
    assert_eq!(shipped.matches(&claude).count(), 1, "the example's shape");
    let with_spare = shipped.replace(
        &claude,
        &format!(
            "    - {{ key: {}.key, alias: human-spare, enabled: true, advertise: true, \
             allowed_client_kinds: [human-client], inbound: inherit_profile_trust, \
             outbound: inherit_profile_trust }}\n{claude}",
            spare().as_str()
        ),
    );
    home.write_config(&with_spare);
    home
}

/// An admin port for the endpoints, held for as long as the case asks the
/// daemon anything (why: `harness::until_lease`).
async fn endpoints_admin(home: &Home) -> impl AdminPort {
    home.binding()
        .admin([AdminCapability::Endpoints].into())
        .await
        .expect("an admin port")
}

/// Each endpoint's (enabled, persisted, leased) as the daemon reports it.
async fn rows(admin: &impl AdminPort) -> Vec<(EndpointId, bool, bool, bool)> {
    admin
        .leases()
        .await
        .expect("the endpoints")
        .into_iter()
        .map(|v| (v.endpoint, v.enabled, v.persisted, v.lease.is_some()))
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_disable_kept_across_a_daemon_restart_refuses_the_client_and_leases_no_other() {
    let home = home();
    let mut daemon = home.start(&[]);
    daemon.serving(&home).await;
    let admin = endpoints_admin(&home).await;
    admin
        .set_endpoint_enabled(human(), false)
        .await
        .expect("disabled");
    drop(admin);
    assert!(daemon.terminate().await.success(), "{}", daemon.log());

    let mut daemon = home.start(&[]);
    daemon.serving(&home).await;
    let admin = endpoints_admin(&home).await;
    let row = |rows: &[(EndpointId, bool, bool, bool)], id: &EndpointId| {
        rows.iter()
            .find(|r| &r.0 == id)
            .cloned()
            .expect("a configured endpoint")
    };
    // The control: the disable survived the restart while `config.yaml`
    // still says enabled -- the state the client used to misread.
    let before = rows(&admin).await;
    assert_eq!(row(&before, &human()), (human(), false, true, false));
    assert!(
        std::fs::read_to_string(home.paths.config_file())
            .expect("config")
            .contains("alias: human, enabled: true,"),
        "the file still says enabled"
    );

    let bus = Bus::connect().await;
    let mut app = human_app::start(&home);
    // Shown, not only kept: the session's refusal is on screen, as the
    // desktop words it for a daemon's runtime.
    let refused = fill(
        placeholder_en::text(UiText::Refused),
        &[(
            "reason",
            placeholder_en::error(RuntimeHost::Daemon, ErrorClass::EndpointNotAvailable),
        )],
    );
    let window = bus.window_of(app.child.id(), || app.log()).await;
    window
        .until("the session refused", |e| e.name == refused, || app.log())
        .await;
    let after = rows(&admin).await;
    assert_eq!(
        row(&after, &human()),
        (human(), false, true, false),
        "the disabled endpoint stays unleased"
    );
    assert!(
        !row(&after, &spare()).3,
        "`human-spare` is not leased in its place: {}",
        app.log()
    );
    assert!(app.running(), "{}", app.report());
    assert!(app.terminate().success(), "{}", app.log());
    drop(admin);
    assert!(daemon.terminate().await.success(), "{}", daemon.log());
}
