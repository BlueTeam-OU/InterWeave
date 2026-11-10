// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! [`crate::cases::STORE_UNREAD`]: the human store's unread rows, read in
//! the app's own process. Since Stage 21 the store is encrypted under a
//! key the platform's cipher seals (ADR-0060), the Keystore on a phone,
//! and that key opens in no other process, so a copy pulled to the host
//! is ciphertext there: the read is the app's, under the cipher the
//! runner hands the case.

use interweave_human_store::at_rest;
use interweave_human_store::key_custody::KeyCipher;
use interweave_profile_config::{ProfilePaths, TrustBoundary};
use serde_json::{Map, Value};

use crate::keys;

/// The arguments of [`crate::cases::STORE_UNREAD`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreUnreadArgs {
    /// The profile whose store is read, as the runner names it.
    pub profile: String,
}

impl StoreUnreadArgs {
    /// The JSON a host passes: `profile`.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut out = Map::new();
        out.insert("profile".to_owned(), self.profile.clone().into());
        Value::Object(out)
    }

    /// Parse what [`to_json`](Self::to_json) wrote.
    ///
    /// # Errors
    /// A missing or malformed field, by name.
    pub fn from_json(args: &Value) -> Result<Self, String> {
        Ok(Self {
            profile: args
                .get("profile")
                .and_then(Value::as_str)
                .ok_or("argument \"profile\" is missing")?
                .to_owned(),
        })
    }
}

/// [`crate::cases::STORE_UNREAD`]'s body: the application ids of the
/// unread rows, read-only under the store's key -- ids, never content.
/// Ids are joined by [`UNREAD_SEPARATOR`], since every value crosses the
/// instrumentation's status as text.
pub(crate) fn store_unread(
    app_data_dir: &std::path::Path,
    cipher: &dyn KeyCipher,
    args: &StoreUnreadArgs,
) -> Result<Map<String, Value>, String> {
    let canonical = std::fs::canonicalize(app_data_dir)
        .map_err(|e| format!("the app data directory {}: {e}", app_data_dir.display()))?;
    let boundary = TrustBoundary::new(&canonical)
        .map_err(|e| format!("the app data directory as a boundary: {e}"))?;
    let paths = ProfilePaths::resolve_embedded(&args.profile, boundary)
        .map_err(|e| format!("the profile's paths: {e}"))?;
    let conn = at_rest::open_read_only(&paths, cipher)
        .map_err(|e| format!("the store, read-only under its key: {e}"))?;
    let mut stmt = conn
        .prepare("SELECT app_message_id FROM unread_inbound ORDER BY app_message_id")
        .map_err(|e| format!("the unread select: {e}"))?;
    let ids = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .and_then(Iterator::collect::<Result<Vec<_>, _>>)
        .map_err(|e| format!("the unread rows: {e}"))?;
    let mut out = Map::new();
    out.insert(keys::UNREAD.to_owned(), ids.join(UNREAD_SEPARATOR).into());
    Ok(out)
}

/// What joins the ids in [`keys::UNREAD`]: the store keeps each as an
/// `AppMessageId`, 32 lowercase hex characters (`AppMessageId::parse`),
/// which a comma is not.
pub const UNREAD_SEPARATOR: &str = ",";

/// [`keys::UNREAD`]'s text as the ids it joined; none for an empty text.
#[must_use]
pub fn unread_ids(text: &str) -> Vec<String> {
    text.split(UNREAD_SEPARATOR)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use std::os::unix::fs::PermissionsExt as _;

    use interweave_human_store::key_custody::FilesystemCustody;
    use interweave_human_store::{
        AppMessageId, HumanStore, InboundKind, InboundOrigin, NewInbound, StoreOptions,
    };
    use interweave_local_client_fake::{FakeConfig, FakeNetwork, FakeNode};
    use interweave_transport_api::TransportIdentity;

    use super::*;
    use crate::{CaseCtx, cases, desktop_message_id, run};

    const PROFILE: &str = "human-android";

    /// An app data directory the store's ancestor rule accepts, whatever
    /// the umask, and the profile's paths under it.
    fn app_dir() -> (tempfile::TempDir, ProfilePaths) {
        let dir = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .expect("a private directory");
        let canonical = std::fs::canonicalize(dir.path()).expect("canonical");
        let boundary = TrustBoundary::new(&canonical).expect("a boundary");
        let paths = ProfilePaths::resolve_embedded(PROFILE, boundary).expect("paths");
        (dir, paths)
    }

    fn sender() -> TransportIdentity {
        TransportIdentity::parse("12D3KooWJWoaqZhDaoEFshF7Rh1bpY9ohihFhzcW6d69Lr2NASuq")
            .expect("a PeerId")
    }

    fn unread(store: &mut HumanStore, id: &str) {
        store
            .commit_unread_inbound(&NewInbound {
                app_message_id: AppMessageId::parse(id).expect("an id"),
                origin: InboundOrigin {
                    peer: sender(),
                    endpoint: None,
                    channel: None,
                },
                media_type: None,
                payload: b"never answered".to_vec(),
                received_at: 1,
                kind: InboundKind::Text,
            })
            .expect("committed unread");
    }

    fn case(
        dir: &std::path::Path,
        cipher: Option<Box<dyn KeyCipher + Send>>,
    ) -> Map<String, Value> {
        case_serving(dir, cipher, None)
    }

    /// [`case`], with `binding` standing for a runtime serving the app.
    fn case_serving(
        dir: &std::path::Path,
        cipher: Option<Box<dyn KeyCipher + Send>>,
        binding: Option<FakeNode>,
    ) -> Map<String, Value> {
        let out = run::<FakeNode>(
            cases::STORE_UNREAD,
            &StoreUnreadArgs {
                profile: PROFILE.to_owned(),
            }
            .to_json()
            .to_string(),
            CaseCtx {
                peer: sender(),
                app_data_dir: dir.to_owned(),
                binding,
                provision: None,
                client: None,
                store_cipher: cipher,
            },
        );
        match serde_json::from_str(&out).expect("JSON") {
            Value::Object(map) => map,
            other => panic!("not an object: {other}"),
        }
    }

    #[test]
    fn the_unread_ids_are_read_under_the_stores_key_and_never_the_read_ones() {
        let (dir, paths) = app_dir();
        let mut store =
            HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default())
                .expect("an encrypted store");
        for serial in [2, 1, 3] {
            unread(&mut store, &desktop_message_id(serial));
        }
        let read = store
            .unread_inbound()
            .expect("rows")
            .into_iter()
            .find(|r| r.app_message_id.as_str() == desktop_message_id(3))
            .expect("the third");
        store.mark_read(read.row_id, 2).expect("read");
        drop(store);

        let out = case(dir.path(), Some(Box::new(FilesystemCustody)));
        assert_eq!(out[keys::RESULT], keys::PASS, "{out:?}");
        let text = out[keys::UNREAD].as_str().expect("text");
        assert_eq!(
            unread_ids(text),
            vec![desktop_message_id(1), desktop_message_id(2)],
            "{text}"
        );
    }

    #[test]
    fn no_store_and_no_cipher_each_fail_saying_why() {
        let (dir, _paths) = app_dir();
        let out = case(dir.path(), Some(Box::new(FilesystemCustody)));
        assert_eq!(out[keys::RESULT], keys::FAIL, "no store: {out:?}");
        let out = case(dir.path(), None);
        assert_eq!(out[keys::RESULT], keys::FAIL);
        assert_eq!(out[keys::DETAIL], "this runner holds no store cipher");
    }

    #[test]
    fn a_runtime_serving_refuses_the_read_of_a_store_that_opens() {
        let (dir, paths) = app_dir();
        let mut store =
            HumanStore::open_profile(&paths, &FilesystemCustody, StoreOptions::default())
                .expect("an encrypted store");
        unread(&mut store, &desktop_message_id(1));
        drop(store);
        let quiet = |peer| FakeConfig {
            peer,
            endpoints: Vec::new(),
            default_endpoint: None,
            queue_bound: 1,
        };
        let (serving, _other) = FakeNetwork::pair(quiet(sender()), quiet(sender()));

        let out = case_serving(dir.path(), Some(Box::new(FilesystemCustody)), Some(serving));
        assert_eq!(out[keys::RESULT], keys::FAIL, "{out:?}");
        assert_eq!(
            out[keys::DETAIL],
            "a runtime is serving: the app's store has a writer"
        );
        // The control: the same store, no runtime, reads.
        let out = case(dir.path(), Some(Box::new(FilesystemCustody)));
        assert_eq!(out[keys::RESULT], keys::PASS, "{out:?}");
    }

    #[test]
    fn an_empty_answer_is_no_ids() {
        assert!(unread_ids("").is_empty());
    }
}
