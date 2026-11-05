// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The profile's configuration file is owner-only, as the app's
//! start requires, however it was written: the first provisioning, and a
//! case's replacement over a file another account could change.

#![allow(clippy::expect_used, clippy::panic)]
#![cfg(all(unix, feature = "dev-stand-ins"))]

use std::os::unix::fs::PermissionsExt as _;

use interweave_human_android_platform::{provision_profile, stand_in};
use interweave_profile_config::{ProfilePaths, TrustBoundary};

const PROFILE: &str = "human";

fn mode(path: &std::path::Path) -> u32 {
    std::fs::metadata(path)
        .expect("the file")
        .permissions()
        .mode()
        & 0o777
}

/// The file the A40 was left with (j89): 0666 after a replacement, which
/// the next start refused. A replacement over such a file leaves it
/// 0600, and so does the first write, under the test's own umask.
#[test]
fn a_provisioned_profile_is_owner_only_and_stays_so() {
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("tempdir");
    let dir = root.path().join("app");
    std::fs::create_dir(&dir).expect("mkdir");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).expect("chmod");
    let config =
        ProfilePaths::resolve_embedded(PROFILE, TrustBoundary::new(&dir).expect("boundary"))
            .expect("paths")
            .config_file();

    provision_profile(&dir, PROFILE).expect("provisioned");
    assert_eq!(mode(&config), 0o600, "the first write");
    let document = std::fs::read_to_string(&config).expect("read");

    std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o666)).expect("chmod");
    stand_in::provision_replacing(&dir, PROFILE, &document).expect("replaced");
    assert_eq!(mode(&config), 0o600, "a replacement over a 0666 file");
}
