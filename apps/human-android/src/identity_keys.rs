// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Whether the profile's state root holds the identities' keys, which K1
//! reads before it takes a phrase (src/onboard.rs): on every target, so
//! the host tests it.

use interweave_profile_config::ProfilePaths;

/// Whether the state root holds `identity-keys/`: without it, K1's
/// restore gives every identity a new key and says so first
/// (IDENTITY-RECOVERY.md section Restore semantics item 12). Anything
/// there but a directory is no identity key a restore keeps, so it counts
/// as none.
pub(crate) fn holds(paths: &ProfilePaths) -> bool {
    paths
        .state_dir()
        .join(interweave_human_store::identity_key::IDENTITY_KEYS_DIR)
        .is_dir()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use interweave_profile_config::TrustBoundary;

    use super::*;

    /// A scratch app data directory, removed however the test ends.
    struct Scratch(PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Item 12's fact as K1 reads it: no `identity-keys/` under the
    /// profile's state root, or a file in its place, is none; the
    /// directory is held. Mutation: `.exists()` for `.is_dir()` holds the
    /// file and this fails; a `!` dropped at the call site is
    /// `k1_says_new_keys_only_with_no_identity_keys`'s in ui-model.
    #[test]
    fn identity_keys_count_only_as_a_directory_under_the_state_root() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let scratch = Scratch(std::env::temp_dir().join(format!(
            "interweave-identity-keys-{}-{nanos}",
            std::process::id()
        )));
        std::fs::create_dir(&scratch.0).expect("scratch");
        let paths = TrustBoundary::new(&scratch.0)
            .and_then(|boundary| ProfilePaths::resolve_embedded("test", boundary))
            .expect("paths");
        let keys = paths
            .state_dir()
            .join(interweave_human_store::identity_key::IDENTITY_KEYS_DIR);
        assert!(!holds(&paths), "fresh state");

        std::fs::create_dir_all(paths.state_dir()).expect("state dir");
        std::fs::write(&keys, b"").expect("a file in its place");
        assert!(!holds(&paths), "a file is no key directory");

        std::fs::remove_file(&keys).expect("rm");
        std::fs::create_dir(&keys).expect("identity-keys/");
        assert!(holds(&paths), "the directory, even empty");
    }

    /// Every identity's key lies under the app data directory Android
    /// hands the runtime (`applicationInfo.dataDir`, the `root` domain of
    /// the backup rules), and the shipped rules exclude that domain, and
    /// its device-protected twin, from cloud backup and device transfer
    /// whole: no backup set carries a key (human-client-ui.md section 13;
    /// ADR-0063 decision 2; ADR-0060 decision 8). The rules' every domain
    /// is the tree check's (`check_android_contacts_and_backup.py`); this
    /// ties the key's place to one of them. Mutation: the `root`
    /// exclusion dropped from either section fails it.
    #[test]
    fn an_identity_key_lies_under_a_domain_every_backup_rule_excludes() {
        const RULES: &str =
            include_str!("../android/app/src/main/res/xml/data_extraction_rules.xml");
        // A scratch directory in `dataDir`'s place: the boundary needs it
        // to exist.
        let scratch = Scratch(std::env::temp_dir().join(format!(
            "interweave-backup-key-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        )));
        std::fs::create_dir(&scratch.0).expect("scratch");
        let app_data = scratch.0.clone();
        let paths = TrustBoundary::new(&app_data)
            .and_then(|boundary| ProfilePaths::resolve_embedded("default", boundary))
            .expect("paths");
        let human = interweave_transport_api::EndpointId::parse("human").expect("valid");
        let key = interweave_human_store::identity_key::key_path(&paths, &human);
        assert!(
            key.starts_with(&app_data),
            "the key outside the app data directory: {}",
            key.display()
        );
        for section in ["cloud-backup", "device-transfer"] {
            let body = RULES
                .split(&format!("<{section}>"))
                .nth(1)
                .and_then(|rest| rest.split(&format!("</{section}>")).next())
                .unwrap_or_else(|| panic!("no <{section}>"));
            for domain in ["root", "device_root"] {
                assert!(
                    body.contains(&format!("<exclude domain=\"{domain}\" path=\".\" />")),
                    "<{section}> keeps the {domain} domain, where the key is"
                );
            }
            assert!(!body.contains("<include"), "<{section}> re-admits data");
        }
    }
}
