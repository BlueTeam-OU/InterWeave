// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `identity show` (ADR-0063 decision 6: show an identity's fingerprint;
//! human-client-ui.md section 13). The identity in front -- the profile's
//! first human endpoint, whose key every run opens -- is the model's.
//! Another human endpoint of the profile has no model: its key file is
//! read, never made or changed. No verb makes a new key in an identity's
//! place: its id derives from the key, so a new key is a new identity
//! (ADR-0067 decision 5; `no_verb_makes_a_new_key_for_an_identity`).

use interweave_human_app_core::KeySource;
use interweave_human_store::identity_key::{self, FilesystemCustody};
use interweave_human_ui_model::OwnKey;
use interweave_local_client_api::AdminBinding;
use interweave_profile_config::ProfilePaths;
use interweave_transport_api::EndpointId;
use serde_json::json;

use crate::Failure;
use crate::cards::key_problem_name;
use crate::output::json_line;
use crate::session::Session;
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
pub(crate) async fn show<B: interweave_human_transport_client::SessionBinding, A: AdminBinding>(
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
/// has none. Nothing is made or changed: no client makes a key
/// (ADR-0067 decision 5; `transportctl endpoint init` mints one)
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
