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
}
