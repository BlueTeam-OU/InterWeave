// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `identity show` and `identity new-key` (ADR-0063 decision 6: show an
//! identity's fingerprint; human-client-ui.md section 13). The identity in
//! front -- the profile's first human endpoint, whose key every run opens
//! -- is the model's, and its new key is the model's key step, offered
//! only where the window offers Make a new key
//! (design/2026-10-24-identity-key.md sections 5 and 7). Another human
//! endpoint of the profile has no model: its key file is read, never made
//! or changed.

use std::fmt::Write as _;

use interweave_human_app_core::KeySource;
use interweave_human_store::identity_key::{self, FilesystemCustody};
use interweave_human_ui_model::{ConversationKey, Intent, KeyProblem, KeyStep, OwnKey, ViewEvent};
use interweave_local_client_api::{AdminBinding, DataSessionBinding};
use interweave_profile_config::ProfilePaths;
use interweave_transport_api::EndpointId;
use serde_json::json;

use crate::Failure;
use crate::cards::key_problem_name;
use crate::cli::conversation_text;
use crate::output::json_line;
use crate::session::{STEP, Session};
use crate::verbs::{Run, look};

/// What `identity show` prints of one identity's key: its fingerprint, or
/// the stable name of why there is none.
fn key_lines(endpoint: &EndpointId, key: Result<&str, &str>, json: bool) -> String {
    if json {
        return json_line(&json!({
            "endpoint": endpoint.as_str(),
            "fingerprint": key.ok(),
            "key_problem": key.err(),
        }));
    }
    format!(
        "identity     {}\nfingerprint  {}\n",
        endpoint.as_str(),
        match key {
            Ok(fingerprint) => fingerprint.to_owned(),
            Err(problem) => format!("none: the key cannot be used ({problem})"),
        }
    )
}

/// The failure of a show that printed an identity with no usable key.
fn no_key(endpoint: &EndpointId, problem: &str) -> Failure {
    Failure::Refused(format!(
        "{} has no key this client can use ({problem})",
        endpoint.as_str()
    ))
}

/// `identity show` of the identity in front, as the model holds it.
pub(crate) async fn show<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    out: &mut String,
) -> Result<(), Failure> {
    look(s, r).await;
    let i = s
        .model()
        .identity()
        .ok_or_else(|| Failure::Refused("the profile has no human identity".to_owned()))?;
    let key = match &i.key {
        OwnKey::Ready { fingerprint } => Ok(fingerprint.as_str()),
        OwnKey::Unusable(problem) => Err(key_problem_name(problem)),
    };
    out.push_str(&key_lines(&i.endpoint, key, r.json));
    key.map(|_| ())
        .map_err(|problem| no_key(&i.endpoint, problem))
}

/// `identity show <endpoint>` of a human endpoint that is not the one in
/// front: its key file read under the desktop's custody, `absent` when it
/// has none. Nothing is made or changed: a key is made only for the
/// identity in front, at its first run
/// (`another_identitys_key_is_read_and_never_made`).
pub(crate) fn show_file(
    paths: &ProfilePaths,
    endpoint: &EndpointId,
    json: bool,
    out: &mut String,
) -> Result<(), Failure> {
    let key = match identity_key::open(paths, endpoint, &FilesystemCustody) {
        Ok(Some(key)) => Ok(key.fingerprint().to_string()),
        Ok(None) => Err("absent"),
        Err(refused) => {
            let source =
                KeySource::new(paths.clone(), endpoint.clone(), Box::new(FilesystemCustody));
            Err(key_problem_name(&source.problem(&refused)))
        }
    };
    out.push_str(&key_lines(endpoint, key.as_deref().map_err(|p| *p), json));
    key.map(|_| ()).map_err(|problem| no_key(endpoint, problem))
}

/// Turn until the key step is one of `done`, or `--wait` ends.
async fn until_step<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    done: fn(&KeyStep) -> bool,
) -> Option<KeyStep> {
    loop {
        s.step().await;
        if let Some(step) = s.model().key_step().filter(|k| done(k)) {
            return Some(step.clone());
        }
        if std::time::Instant::now() >= r.until {
            return None;
        }
        tokio::time::sleep(STEP).await;
    }
}

/// `identity new-key`: the window's Make a new key, for the identity in
/// front whose key cannot be used. Without `confirm` it says what the new
/// key would do and changes nothing, exit 1; with it the key is made, as
/// the confirmation's Make a new key makes it
/// (`a_new_key_is_made_only_when_confirmed_and_offered`).
pub(crate) async fn new_key<B: DataSessionBinding, A: AdminBinding>(
    s: &mut Session<B, A>,
    r: &Run<'_>,
    confirm: bool,
    out: &mut String,
) -> Result<(), Failure> {
    look(s, r).await;
    let Some(i) = s.model().identity() else {
        return Err(Failure::Refused(
            "the profile has no human identity".to_owned(),
        ));
    };
    let endpoint = i.endpoint.clone();
    let problem = match &i.key {
        OwnKey::Ready { .. } => {
            return Err(Failure::Refused(format!(
                "{}'s key can be used: a new key is made only for a key this client cannot use",
                endpoint.as_str()
            )));
        }
        // The one kind the model offers no new key for: the folder is the
        // person's to fix, and running again opens the key once it is
        // (design/2026-10-24-identity-key.md section 3;
        // `a_key_folder_not_safe_is_refused_naming_the_folder`).
        OwnKey::Unusable(KeyProblem::DirUnsafe { folder }) => {
            return Err(Failure::Refused(format!(
                "{}'s key folder {folder} is not a folder only this account owns (key-folder-unsafe): \
                 make it one, then run the command again; no new key is made",
                endpoint.as_str()
            )));
        }
        OwnKey::Unusable(problem) if !problem.offers_new_key() => {
            return Err(Failure::Refused(format!(
                "{} is offered no new key ({})",
                endpoint.as_str(),
                key_problem_name(problem)
            )));
        }
        OwnKey::Unusable(problem) => problem.clone(),
    };
    s.will(|_| vec![ViewEvent::Intent(Intent::AskNewKey)]);
    let asked = until_step(s, r, |k| matches!(k, KeyStep::Confirming { .. })).await;
    let Some(KeyStep::Confirming { pending }) = asked else {
        return Err(Failure::Refused(
            "what a new key would cancel could not be read".to_owned(),
        ));
    };
    if !confirm {
        out.push_str(&would(&endpoint, &problem, pending, r.json));
        return Err(Failure::Refused(
            "nothing was changed: --confirm key-change makes the new key".to_owned(),
        ));
    }
    s.will(|_| vec![ViewEvent::Intent(Intent::MakeNewKey)]);
    let ended = until_step(s, r, |k| {
        matches!(k, KeyStep::Made { .. } | KeyStep::Failed { .. })
    })
    .await;
    match ended {
        Some(KeyStep::Made { share_again }) => {
            let fingerprint = match s.model().identity().map(|i| &i.key) {
                Some(OwnKey::Ready { fingerprint }) => fingerprint.clone(),
                _ => return Err(Failure::Refused("the new key is not shown".to_owned())),
            };
            out.push_str(&made(&endpoint, &fingerprint, &share_again, r.json));
            Ok(())
        }
        Some(KeyStep::Failed { gone: true }) => Err(Failure::Refused(
            "the new key was not made, and the old key file is gone: the next run says the key is missing"
                .to_owned(),
        )),
        _ => Err(Failure::Refused(
            "the new key was not made; the old key is as it was".to_owned(),
        )),
    }
}

/// What a new key would do, as the confirmation says it (section 5.1).
fn would(endpoint: &EndpointId, problem: &KeyProblem, pending: bool, json: bool) -> String {
    let deletes = !matches!(problem, KeyProblem::Missing);
    let newer = matches!(problem, KeyProblem::NewerVersion);
    if json {
        return json_line(&json!({
            "endpoint": endpoint.as_str(),
            "key_problem": key_problem_name(problem),
            "deletes_old_key": deletes,
            "cancels_pending_cards": pending,
        }));
    }
    let mut text = format!(
        "a new key for {} ({}):\n",
        endpoint.as_str(),
        key_problem_name(problem)
    );
    if newer {
        text.push_str(
            "  the old key file is deleted: a newer version may be able to use it, and once it is deleted, updating does not bring it back\n",
        );
    } else if deletes {
        text.push_str("  the old key file is deleted and cannot be used again\n");
    }
    if pending {
        text.push_str(
            "  your cards still on their way are cancelled; one already sent may arrive with the old key\n",
        );
    }
    text.push_str(
        "  contacts who have your card see that your key changed the next time you share it, and are told it might be someone else\n",
    );
    text
}

/// What a made key prints: its fingerprint and where to share the card
/// again, most recently shared first.
fn made(
    endpoint: &EndpointId,
    fingerprint: &str,
    share_again: &[ConversationKey],
    json: bool,
) -> String {
    let again: Vec<String> = share_again.iter().map(conversation_text).collect();
    if json {
        return json_line(&json!({
            "endpoint": endpoint.as_str(),
            "fingerprint": fingerprint,
            "share_again": again,
        }));
    }
    let mut text = format!(
        "identity     {}\nfingerprint  {fingerprint}\n",
        endpoint.as_str()
    );
    for conversation in again {
        let _ = writeln!(text, "share again  {conversation}");
    }
    text
}
