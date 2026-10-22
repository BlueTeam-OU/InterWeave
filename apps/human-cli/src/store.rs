// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `store health` and `store discard`: the store as ADR-0060 has it, at
//! rest under a key the platform's custody holds. Both run under the
//! instance lock and never reach the daemon: health is the store's and the
//! key's, and discard and recreate is the one way back from recovery
//! (decision 6), taken only on the person's word -- here the scope they
//! name with `--confirm`, which must be the one the plan lists.

use std::fmt::Write as _;

use interweave_human_store::at_rest::{
    self, AtRestReport, DiscardPlan, DiscardScope, KeyPresence, StoreFiles,
};
use interweave_human_store::key_custody::{Custody, KeyCipher};
use interweave_human_store::{HumanStore, StorageHealth, StoreError};
use interweave_profile_config::ProfilePaths;
use serde_json::json;

use crate::startup::{Blocked, open_store_at};
use crate::{Failure, Outcome, output};

const fn custody_token(custody: Custody) -> &'static str {
    match custody {
        Custody::FilesystemOnly => "filesystem-only",
        Custody::Keystore => "keystore",
    }
}

const fn key_token(key: &KeyPresence) -> &'static str {
    match key {
        KeyPresence::Absent => "absent",
        KeyPresence::Present => "present",
        KeyPresence::Invalidated => "invalidated",
        KeyPresence::Unusable(_) => "unusable",
    }
}

const fn files_token(files: StoreFiles) -> &'static str {
    match files {
        StoreFiles::None => "none",
        StoreFiles::Plaintext => "plaintext",
        StoreFiles::Encrypted => "encrypted",
        StoreFiles::ExportUnfinished => "export-unfinished",
        StoreFiles::ScrubUnfinished => "scrub-unfinished",
        StoreFiles::RenameUnfinished => "rename-unfinished",
        StoreFiles::Unlisted(_) => "unlisted",
    }
}

/// The scope's token, as `--confirm` takes it.
pub(crate) const fn scope_token(scope: DiscardScope) -> &'static str {
    match scope {
        DiscardScope::Everything => "everything",
        DiscardScope::UnfinishedPlaintext => "unfinished-plaintext",
    }
}

fn unavailable(e: &StoreError) -> Failure {
    Failure::Store(format!("the message store: {e}"))
}

/// `store health`: the store opened -- which recreates nothing it does
/// not open anyway -- and re-checked as the window's re-check does, with
/// what it holds counted; then its key and files at rest, as that open
/// left them. A store that does not open is reported by its class, with
/// no counts. Anything but healthy is exit 5.
pub(crate) fn health(paths: &ProfilePaths, cipher: &dyn KeyCipher, json: bool) -> Outcome {
    let opened = open_store_at(paths, cipher);
    let (state, counts, failure) = match opened {
        Ok(mut store) => {
            let healthy = matches!(store.recheck_health(), Ok(StorageHealth::Healthy));
            let counts = match counts(&mut store) {
                Ok(counts) => counts,
                Err(failure) => return failure.into(),
            };
            if healthy {
                ("healthy", Some(counts), None)
            } else {
                (
                    "degraded",
                    Some(counts),
                    Some(Failure::Store(
                        "the message store cannot hold new messages now".to_owned(),
                    )),
                )
            }
        }
        Err((blocked, failure)) => (blocked.token(), None, Some(failure)),
    };
    let report = match at_rest::inspect(paths, cipher) {
        Ok(report) => report,
        Err(e) => return unavailable(&e).into(),
    };
    Outcome {
        out: health_lines(state, &report, counts, json),
        result: failure.map_or(Ok(()), Err),
    }
}

fn counts(store: &mut HumanStore) -> Result<(usize, usize, usize), Failure> {
    Ok((
        store.pending_outbound().map_err(|e| unavailable(&e))?.len(),
        store.unread_inbound().map_err(|e| unavailable(&e))?.len(),
        store.kept_inbound().map_err(|e| unavailable(&e))?.len(),
    ))
}

fn health_lines(
    state: &str,
    report: &AtRestReport,
    counts: Option<(usize, usize, usize)>,
    json: bool,
) -> String {
    let (protection, key, files) = (
        custody_token(report.protection),
        key_token(&report.key),
        files_token(report.files),
    );
    if json {
        return output::json_line(&json!({
            "health": state,
            "protection": protection,
            "key": key,
            "files": files,
            "pending": counts.map(|c| c.0),
            "unread": counts.map(|c| c.1),
            "kept": counts.map(|c| c.2),
        }));
    }
    let mut out = format!(
        "health      {state}\nprotection  {protection}\nkey         {key}\nfiles       {files}\n"
    );
    if let Some((pending, unread, kept)) = counts {
        let _ = write!(
            out,
            "pending     {pending}\nunread      {unread}\nkept        {kept}\n"
        );
    }
    out
}

/// `store discard [--confirm <scope>]`. Only a store that needs recovery
/// is discarded: one that opens is refused, as the desktop offers the
/// action on its recovery screen alone. The plan is listed whatever
/// happens; nothing is removed unless `confirm` names its scope, so a
/// script that confirmed one loss never takes another
/// (`a_discard_is_refused_without_the_scope_it_lists_and_removes_nothing`).
/// After a discard the store is opened again, which recreates it.
pub(crate) fn discard(
    paths: &ProfilePaths,
    cipher: &dyn KeyCipher,
    confirm: Option<DiscardScope>,
    json: bool,
) -> Outcome {
    match open_store_at(paths, cipher) {
        Ok(_store) => {
            return Failure::Refused(
                "the message store opens, so there is nothing to recover: discard is for a \
                 store that needs recovery. Nothing was removed"
                    .to_owned(),
            )
            .into();
        }
        Err((Blocked::Recovery, _)) => {}
        Err((_, failure)) => return failure.into(),
    }
    let plan = match at_rest::plan_discard(paths, cipher) {
        Ok(plan) => plan,
        Err(e) => return unavailable(&e).into(),
    };
    let listed = |done: bool, recreated: bool| plan_lines(&plan, done, recreated, json);
    let refused = |why: &str| Outcome {
        out: listed(false, false),
        result: Err(Failure::Refused(format!("{why}. Nothing was removed"))),
    };
    match confirm {
        None => {
            return refused(&format!(
                "a discard needs the person's word: run it again with --confirm {}",
                scope_token(plan.scope())
            ));
        }
        Some(scope) if scope != plan.scope() => {
            return refused(&format!(
                "--confirm named {}, but the discard now removes {}",
                scope_token(scope),
                scope_token(plan.scope())
            ));
        }
        Some(_) => {}
    }
    if let Err(e) = at_rest::discard(paths, cipher, plan.clone().confirm()) {
        return Outcome {
            out: listed(false, false),
            result: Err(match e {
                StoreError::DiscardChanged => Failure::Refused(
                    "the store's files changed while the discard was being made: nothing was \
                     removed; list it again"
                        .to_owned(),
                ),
                other => Failure::Store(format!(
                    "the discard stopped part way, some files removed: {other}"
                )),
            }),
        };
    }
    match open_store_at(paths, cipher) {
        Ok(_store) => Outcome {
            out: listed(true, true),
            result: Ok(()),
        },
        Err((_, failure)) => Outcome {
            out: listed(true, false),
            result: Err(failure),
        },
    }
}

/// What a discard removes and what it loses, as ADR-0060 has the clients
/// list it before doing it.
fn plan_lines(plan: &DiscardPlan, done: bool, recreated: bool, json: bool) -> String {
    let files: Vec<String> = plan
        .files()
        .iter()
        .map(|f| output::printable(&f.display().to_string()))
        .collect();
    if json {
        return output::json_line(&json!({
            "discard": scope_token(plan.scope()),
            "removes_key": plan.removes_key(),
            "files": files,
            "done": done,
            "recreated": recreated,
        }));
    }
    let mut out = format!("discard     {}\n", scope_token(plan.scope()));
    for file in &files {
        let _ = writeln!(out, "file        {file}");
    }
    let _ = writeln!(
        out,
        "key         {}",
        if plan.removes_key() {
            "removed"
        } else {
            "kept"
        }
    );
    out.push_str(match plan.scope() {
        DiscardScope::Everything => {
            "lost        every contact and its routes, pending, unread and kept messages, \
             the conversation list, the record of messages already shown, the client's \
             settings and your own cards\n\
             kept        your identities and their keys, trust and the transport's \
             configuration: the store holds none of them\n"
        }
        DiscardScope::UnfinishedPlaintext => {
            "lost        nothing you had: the unfinished plaintext copy goes, and the next \
             open takes the complete encrypted copy beside it\n"
        }
    });
    if done {
        out.push_str(if recreated {
            "done        discarded and recreated\n"
        } else {
            "done        discarded; the store did not open again\n"
        });
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use interweave_human_store::key_custody::FilesystemCustody;
    use interweave_human_store::store_key;
    use interweave_human_store::{AppMessageId, InboundOrigin, NewInbound};
    use interweave_profile_config::TrustBoundary;
    use interweave_transport_api::TransportIdentity;
    use serde_json::Value;

    use super::*;

    const PEER: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";

    /// A profile's paths in an owner-only directory, its human directory
    /// made as the clients make it.
    fn profile() -> (tempfile::TempDir, ProfilePaths) {
        use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};
        let mut builder = tempfile::Builder::new();
        builder.permissions(std::fs::Permissions::from_mode(0o700));
        let dir = builder.tempdir().expect("tempdir");
        let paths = ProfilePaths::resolve_embedded(
            "default",
            TrustBoundary::new(dir.path()).expect("a boundary"),
        )
        .expect("paths");
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(paths.human_dir())
            .expect("the human directory");
        (dir, paths)
    }

    /// The store made, with one unread message in it.
    fn with_one_unread(paths: &ProfilePaths) {
        let mut store = open_store_at(paths, &FilesystemCustody)
            .map_err(|(_, f)| f)
            .expect("opens");
        store
            .commit_unread_inbound(&NewInbound {
                app_message_id: AppMessageId::parse(format!("{:032x}", 7)).expect("id"),
                origin: InboundOrigin {
                    peer: TransportIdentity::parse(PEER).expect("peer"),
                    endpoint: None,
                    channel: None,
                },
                media_type: None,
                payload: b"kept until read".to_vec(),
                received_at: 1_000,
            })
            .expect("commits");
    }

    /// The store made and its key file removed: recovery, decision 6's
    /// missing key.
    fn needing_recovery() -> (tempfile::TempDir, ProfilePaths) {
        let (dir, paths) = profile();
        with_one_unread(&paths);
        std::fs::remove_file(store_key::key_path(&paths)).expect("the key file");
        (dir, paths)
    }

    fn health_json(paths: &ProfilePaths) -> (Value, Result<(), Failure>) {
        let Outcome { out, result } = health(paths, &FilesystemCustody, true);
        (serde_json::from_str(&out).expect("one JSON line"), result)
    }

    fn files_in(paths: &ProfilePaths) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(paths.human_dir())
            .expect("the directory")
            .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_discard_is_refused_without_the_scope_it_lists_and_removes_nothing() {
        let (_dir, paths) = needing_recovery();
        let before = files_in(&paths);
        assert!(!before.is_empty());
        for confirm in [None, Some(DiscardScope::UnfinishedPlaintext)] {
            let Outcome { out, result } = discard(&paths, &FilesystemCustody, confirm, false);
            assert!(
                matches!(result, Err(Failure::Refused(ref m)) if m.contains("Nothing was removed")),
                "{confirm:?}: {result:?}"
            );
            assert!(out.contains("discard     everything\n"), "{out}");
            assert!(out.contains("key         removed\n"), "{out}");
            assert!(out.contains("every contact"), "the loss is listed: {out}");
            assert!(!out.contains("done"), "{out}");
            assert_eq!(files_in(&paths), before, "{confirm:?}");
        }
        let (report, result) = health_json(&paths);
        assert!(matches!(result, Err(Failure::Store(_))));
        assert_eq!(report["health"], "needs-recovery");
        assert_eq!(report["key"], "absent");
        assert_eq!(report["files"], "encrypted");
        assert_eq!(
            report["unread"],
            Value::Null,
            "no count of a store not open"
        );
    }

    #[test]
    fn a_confirmed_discard_recreates_an_empty_store_under_a_new_key() {
        let (_dir, paths) = needing_recovery();
        let Outcome { out, result } = discard(
            &paths,
            &FilesystemCustody,
            Some(DiscardScope::Everything),
            true,
        );
        result.expect("discarded");
        let line: Value = serde_json::from_str(&out).expect("one JSON line");
        assert_eq!(line["discard"], "everything");
        assert_eq!(line["removes_key"], true);
        assert_eq!(line["done"], true);
        assert_eq!(line["recreated"], true);
        let (report, result) = health_json(&paths);
        result.expect("healthy");
        assert_eq!(report["health"], "healthy");
        assert_eq!(report["key"], "present");
        assert_eq!(report["files"], "encrypted");
        assert_eq!(report["protection"], "filesystem-only");
        assert_eq!(report["unread"], 0, "the old store's message is gone");
    }

    #[test]
    fn a_store_that_opens_is_never_discarded() {
        let (_dir, paths) = profile();
        with_one_unread(&paths);
        let Outcome { out, result } = discard(
            &paths,
            &FilesystemCustody,
            Some(DiscardScope::Everything),
            false,
        );
        assert!(matches!(result, Err(Failure::Refused(_))), "{result:?}");
        assert!(out.is_empty(), "{out}");
        let (report, result) = health_json(&paths);
        result.expect("healthy");
        assert_eq!(report["unread"], 1);
    }
}
