// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Every identity of the profile, held by the shipped desktop (plan §30;
//! architect-cto's REPLY 01a216b6): an entry the endpoint overlay `added`
//! for the human client, planted by hand with its key minted by
//! `transportctl endpoint init`, is leased beside the first identity, and
//! each lease is taken after the advertise assertion, so neither endpoint
//! is advertised (ADR-0061 decision 5).
//!
//! What this does NOT prove: the identities on screen (the switcher's
//! rows are the view's, `ui-slint`'s tests), a Try again, an added entry
//! whose key is missing (app-core's
//! `an_added_identity_without_its_key_is_leased_and_given_no_key`).

use std::net::Ipv4Addr;
use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};

use interweave_local_client_api::{AdminCapability, AdminPort as _};
use interweave_profile_config::endpoint_overlay::EndpointOverlay;
use interweave_transport_api::EndpointId;

use crate::common::{Home, example, free_port, human, stranger};
use crate::harness::AdminOpen as _;
use crate::harness::{self as human_app, until_lease_of};

/// Mint a key for `alias` with `transportctl endpoint init`, and add its
/// entry to the endpoint overlay as the administrative add writes it:
/// open to the human client, unadvertised, with its own empty set.
fn plant_added(home: &Home, alias: &str) -> EndpointId {
    let out = home.transportctl(&["endpoint", "init", "--alias", alias], "");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let id = EndpointId::parse_derived(stdout.lines().next().expect("the id line"))
        .expect("a derived id");
    let file = EndpointOverlay::path_for(&home.paths);
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(file.parent().expect("a state directory"))
        .expect("the state directory");
    std::fs::write(
        &file,
        format!(
            r#"{{"enabled":{{}},"added":[{{"key":"{id}.key","alias":"{alias}","enabled":true,"advertise":false,"allowed_client_kinds":["human-client"]}}],"inbound":{{"{id}":[]}},"outbound":{{"{id}":[]}}}}"#,
            id = id.as_str()
        ),
    )
    .expect("the overlay");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    id
}

/// The first identity and the added one are both leased by the one
/// window, and neither endpoint is advertised. Mutation: the desktop
/// composing the first identity alone (`ipc.rs` with `others` empty) --
/// the added lease never comes.
#[tokio::test(flavor = "multi_thread")]
async fn an_added_identity_is_leased_unadvertised_beside_the_first() {
    let home = Home::new("human-desktop");
    home.write_key();
    let listen = format!("/ip4/127.0.0.1/tcp/{}", free_port(Ipv4Addr::LOCALHOST));
    home.write_config(&example("human-desktop.yaml", &stranger(), &listen, None));
    let added = plant_added(&home, "work");
    let mut daemon = home.start(&[]);
    daemon.serving(&home).await;

    let mut app = human_app::start(&home);
    let binding = home.binding();
    until_lease_of(&binding, &human(), true, &app).await;
    until_lease_of(&binding, &added, true, &app).await;

    let admin = binding.admin_port(&[AdminCapability::Endpoints]).await;
    let rows = admin.leases().await.expect("admin.endpoints.list");
    for endpoint in [human(), added] {
        let row = rows
            .iter()
            .find(|r| r.endpoint == endpoint)
            .unwrap_or_else(|| panic!("{endpoint:?} listed: {rows:?}"));
        assert!(row.lease.is_some(), "{endpoint:?} leased: {}", app.log());
        assert!(!row.advertise, "{endpoint:?} not advertised: {rows:?}");
    }
    drop(admin);
    assert!(app.running(), "{}", app.report());
    assert!(app.terminate().success(), "{}", app.log());
    assert!(daemon.terminate().await.success(), "{}", daemon.log());
}
