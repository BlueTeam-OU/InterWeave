// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! TEST BUILDS ONLY (feature `e2e-cases`): the entry the app's
//! instrumentation calls to run one of `interweave-android-e2e-cases`'
//! cases in the app's own process (plan section 20 gate (c), the seam
//! agreed with p2p-network-dev-02, 01a12771 and 01a128a8). It starts what
//! the case needs as the app starts it -- nothing, the runtime alone, or
//! the app's service in full with its hub as the case's client -- on the
//! identity the app's start takes, runs the case, and stops.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use interweave_android_e2e_cases::{CaseCtx, Tap, cases, keys};
use interweave_human_android_platform::e2e::HubClient;
use interweave_human_android_platform::{
    FirstRun, ServiceHost, ServiceLaunch, stand_in, start_identity,
};
use jni::objects::{JClass, JString};
use jni::{Env, EnvUnowned};

use crate::jni::{PROFILE, log};
use crate::keystore::KeystoreCipher;

/// How long the stop after a case lets exchanges in flight settle.
const GRACE: Duration = Duration::from_secs(1);

/// The result JSON of a case that could not be run at all.
fn failed(case: &str, detail: &str) -> String {
    serde_json::json!({ keys::CASE: case, keys::RESULT: keys::FAIL, keys::DETAIL: detail })
        .to_string()
}

fn run(env: &mut Env<'_>, case: &str, args: &str, app_data_dir: &str) -> String {
    let dir = Path::new(app_data_dir).to_owned();
    let cipher = match KeystoreCipher::new(env) {
        Ok(cipher) => Arc::new(cipher),
        Err(e) => return failed(case, &format!("the Keystore cipher: {e}")),
    };
    // The host's provisioning: config.yaml replaced, under the app's
    // private directories, for the next start to read.
    let provision = {
        let dir = dir.clone();
        Box::new(move |config: &str| {
            stand_in::provision_replacing(&dir, PROFILE, config).map_err(|e| e.to_string())
        })
    };
    // The app's own start writes the stand-in profile when there is none,
    // and a first run reads its unlock policy; `identity` comes before any
    // `provision` (AdbDevice::connect), so the same here.
    if let Err(e) = stand_in::provision(&dir, PROFILE) {
        return failed(case, &format!("the stand-in profile: {e}"));
    }
    if case == cases::PROVISION {
        // Provisioning needs no identity and starts nothing; the PeerId a
        // CaseCtx carries is not read by it.
        let Ok(peer) =
            interweave_profile_identity::ProfileIdentity::generate().transport_identity()
        else {
            return failed(case, "a generated identity gave no PeerId");
        };
        let ctx = CaseCtx {
            peer,
            app_data_dir: dir,
            binding: ServiceHost::global().runtime_binding(),
            provision: Some(provision),
            client: None,
        };
        return interweave_android_e2e_cases::run(case, args, ctx);
    }
    let identity = match start_identity(&dir, PROFILE, &*cipher, FirstRun::Create) {
        Ok(identity) => identity,
        Err(why) => return failed(case, &format!("no identity: {why:?}")),
    };
    let peer = match identity.transport_identity() {
        Ok(peer) => peer,
        Err(e) => return failed(case, &format!("the identity's PeerId: {e}")),
    };
    let service = ServiceHost::global();
    // The store key under the same Keystore bridge, at its own alias and
    // background-compatible (ADR-0060 decision 3).
    let launch = ServiceLaunch {
        app_data_dir: dir.clone(),
        profile: PROFILE.to_owned(),
        identity,
        store_cipher: Box::new(crate::keystore::StoreKeystore(Arc::clone(&cipher))),
    };
    let mut client: Option<Box<dyn interweave_android_e2e_cases::AppClient>> = None;
    let started = crate::platform_log::around_start(service, &dir, PROFILE, || {
        if cases::NEED_A_RUNTIME.contains(&case) {
            service.start_runtime_alone(launch)
        } else if cases::NEED_THE_CLIENT.contains(&case) {
            let tap = Tap::default();
            let started = service.start_recording(launch, Arc::clone(&tap));
            client = Some(Box::new(HubClient::attach(service.hub(), tap)));
            started
        } else {
            Ok(())
        }
    });
    if let Err(why) = started {
        return failed(case, &format!("the start was refused: {why:?}"));
    }
    let ctx = CaseCtx {
        peer,
        app_data_dir: dir,
        binding: service.runtime_binding(),
        provision: Some(provision),
        client,
    };
    let out = interweave_android_e2e_cases::run(case, args, ctx);
    if let Some(stopped) = service.stop(GRACE) {
        log(&format!(
            "e2e {case}: stopped, runtime {:?}",
            stopped.runtime
        ));
    }
    out
}

/// `E2eCase.run(case, args, appDataDir)`: the result JSON.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_test_E2eCase_run<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    case: JString<'caller>,
    args: JString<'caller>,
    app_data_dir: JString<'caller>,
) -> JString<'caller> {
    env.with_env(|env| -> Result<JString<'caller>, jni::errors::Error> {
        let (case, args, dir) = (
            case.try_to_string(env)?,
            args.try_to_string(env)?,
            app_data_dir.try_to_string(env)?,
        );
        let out = run(env, &case, &args, &dir);
        JString::from_str(env, out)
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}
