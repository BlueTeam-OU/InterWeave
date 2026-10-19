// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The entry points the Kotlin side calls (`Native.kt`): the network
//! service's start, its wait for the end, its stop, the notices it posts,
//! the texts it shows, and what its network callback reports. Each catches a panic at the boundary
//! (`EnvUnowned::with_env`) and answers Java with a value, or a
//! `RuntimeException` on a JNI failure, never an unwind.

use std::ffi::CString;
use std::time::Duration;

use interweave_human_android_platform::AvailabilityMode;
use interweave_human_android_platform::{
    EmbeddedRefused, Ended, FirstRun, IdentityRefused, Link, NetworkReport, ServiceHost,
    ServiceLaunch, StartRefused, Told, addresses_from_wire, start_identity,
};
use interweave_human_ui_model::{UiText, fill, placeholder_en};
use interweave_human_ui_slint::{TouchSettings, snackbar_ms};
use interweave_profile_identity::ProfileIdentity;
use jni::{Env, EnvUnowned};

use crate::keystore::KeystoreCipher;
use jni::errors::ThrowRuntimeExAndDefault;
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jboolean, jfloat, jint, jlong};

/// The profile the app runs, as the android-e2e harness names it.
pub(crate) const PROFILE: &str = "human-android";

/// How long a stop lets exchanges in flight settle: what the runtime's
/// own tests use, and p2p-network-dev-01's word that it is enough (relay
/// 01a12498).
const GRACE: Duration = Duration::from_secs(1);

/// `Native.start`'s refusals, negative so they never read as a mode.
mod refused {
    use super::jint;
    pub(super) const NO_IDENTITY: jint = -1;
    #[cfg_attr(
        not(feature = "dev-stand-ins"),
        allow(dead_code, reason = "only the stand-in writes a profile today")
    )]
    pub(super) const PROFILE: jint = -2;
    pub(super) const RUNTIME: jint = -3;
    pub(super) const ENDPOINT: jint = -4;
    pub(super) const STORE_RECOVERY: jint = -5;
    pub(super) const STORE_PRIVATE: jint = -6;
    pub(super) const STORE_UNAVAILABLE: jint = -7;
    pub(super) const THREAD: jint = -8;
    pub(super) const RECOVERY_REQUIRED: jint = -9;
    pub(super) const USER_NOT_AUTHENTICATED: jint = -10;
    pub(super) const IDENTITY_UNAVAILABLE: jint = -11;
    /// Native.kt's `REFUSED_NETWORK_DENIED`: the Service answers it with
    /// the network-denied posture.
    pub(super) const NETWORK_DENIED: jint = -12;
}

/// One line to the platform's log, under the app's tag. A started
/// Service has no Activity, so nothing has redirected stderr there.
pub(crate) fn log(line: &str) {
    log_at(ndk_sys::android_LogPriority::ANDROID_LOG_INFO, line);
}

/// The runtime's events to the platform's log, at their own priority
/// (`crate::platform_log` filters them).
fn log_event(level: tracing::Level, line: &str) {
    use ndk_sys::android_LogPriority as P;
    let priority = match level {
        tracing::Level::ERROR => P::ANDROID_LOG_ERROR,
        tracing::Level::WARN => P::ANDROID_LOG_WARN,
        tracing::Level::INFO => P::ANDROID_LOG_INFO,
        tracing::Level::DEBUG => P::ANDROID_LOG_DEBUG,
        tracing::Level::TRACE => P::ANDROID_LOG_VERBOSE,
    };
    log_at(priority, line);
}

fn log_at(priority: ndk_sys::android_LogPriority, line: &str) {
    let (Ok(tag), Ok(text)) = (
        CString::new("interweave"),
        CString::new(line.replace('\0', " ")),
    ) else {
        return;
    };
    #[allow(
        unsafe_code,
        reason = "the NDK's logging call: two valid C strings, read during the call only"
    )]
    // SAFETY: both pointers are to NUL-terminated strings that outlive the call.
    unsafe {
        ndk_sys::__android_log_write(
            i32::try_from(priority.0).unwrap_or(4),
            tag.as_ptr(),
            text.as_ptr(),
        );
    }
}

/// Send every panic's message and location to the platform's log, once
/// per process. Without it a panic leaves no trace: the default hook
/// writes to stderr, which Android discards, and the boundary's
/// `ThrowRuntimeExAndDefault` keeps a `&'static str` payload alone, so a
/// formatted panic reached Java as "non-string panic payload" (the j66
/// device run of 2026-10-14). The default hook still runs after it.
pub(crate) fn log_panics() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let default = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let payload = info.payload();
            let message = payload
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("a panic with no message");
            let at = info
                .location()
                .map(|l| format!(" at {}:{}", l.file(), l.line()))
                .unwrap_or_default();
            let thread = std::thread::current();
            log(&format!(
                "panic on thread {}{at}: {message}",
                thread.name().unwrap_or("unnamed")
            ));
            default(info);
        }));
    });
}

/// The JVM calls it when `System.loadLibrary` loads this library: from
/// `Native.kt` for the Service, and from the instrumentation.
#[allow(unsafe_code, reason = "the JVM finds JNI_OnLoad by its unmangled name")]
#[unsafe(no_mangle)]
extern "system" fn JNI_OnLoad(
    _vm: *mut jni::sys::JavaVM,
    _reserved: *mut std::ffi::c_void,
) -> jint {
    log_panics();
    // The runtime's log, once per process: the Service's and the
    // instrumentation's alike load the library.
    let _ = crate::platform_log::install(log_event);
    jni::sys::JNI_VERSION_1_6
}

/// The identity the runtime starts with: the profile's record, unwrapped
/// by the Keystore through the embedded custody path (step 6) -- or the
/// refusal code of why there is none, after logging it. A first run is
/// provisioned in the debug build alone (android-platform's `identity`).
fn identity(env: &mut Env<'_>, app_data_dir: &str) -> Result<ProfileIdentity, jint> {
    let cipher = KeystoreCipher::new(env).map_err(|e| {
        log(&format!("the Keystore cipher could not be reached: {e}"));
        refused::IDENTITY_UNAVAILABLE
    })?;
    // A first run creates its identity in the debug build alone, until
    // step 7's onboarding lets the person choose.
    let first_run = if cfg!(feature = "dev-stand-ins") {
        FirstRun::Create
    } else {
        FirstRun::Refuse
    };
    start_identity(
        std::path::Path::new(app_data_dir),
        PROFILE,
        &cipher,
        first_run,
    )
    .map_err(|why| {
        log(&format!("no identity: {why:?}"));
        match why {
            IdentityRefused::Unprovisioned => refused::NO_IDENTITY,
            IdentityRefused::RecoveryRequired(_) => refused::RECOVERY_REQUIRED,
            IdentityRefused::UserNotAuthenticated => refused::USER_NOT_AUTHENTICATED,
            IdentityRefused::Paths(_)
            | IdentityRefused::Unavailable(_)
            | IdentityRefused::Provisioning(_) => refused::IDENTITY_UNAVAILABLE,
        }
    })
}

/// `Native.startWithoutNetwork`'s answer when the posture runs: neither
/// mode, since no runtime runs to have one (Native.kt's `NETWORK_DENIED`).
const NETWORK_DENIED: jint = 2;

/// Write the stand-in profile where the debug build needs one.
#[allow(
    clippy::unnecessary_wraps,
    reason = "Ok in a build without dev-stand-ins, which writes no profile"
)]
fn provision(app_data_dir: &str) -> Result<(), jint> {
    #[cfg(feature = "dev-stand-ins")]
    if let Err(e) = interweave_human_android_platform::stand_in::provision(
        std::path::Path::new(app_data_dir),
        PROFILE,
    ) {
        log(&format!("the stand-in profile could not be written: {e}"));
        return Err(refused::PROFILE);
    }
    #[cfg(not(feature = "dev-stand-ins"))]
    let _ = app_data_dir;
    Ok(())
}

/// The code a refusal answers Java with, after logging it.
fn refusal(why: &StartRefused) -> jint {
    log(&format!("the network service did not start: {why:?}"));
    match why {
        StartRefused::Runtime(EmbeddedRefused::NetworkDenied(_)) => refused::NETWORK_DENIED,
        StartRefused::Runtime(_) => refused::RUNTIME,
        StartRefused::NoHumanEndpoint => refused::ENDPOINT,
        StartRefused::StoreNeedsRecovery => refused::STORE_RECOVERY,
        StartRefused::StoreNotPrivate(_) => refused::STORE_PRIVATE,
        StartRefused::StoreUnavailable(_) => refused::STORE_UNAVAILABLE,
        StartRefused::Thread(_) => refused::THREAD,
    }
}

fn start_without_network(app_data_dir: &str) -> jint {
    if let Err(code) = provision(app_data_dir) {
        return code;
    }
    let host = ServiceHost::global();
    let dir = std::path::Path::new(app_data_dir);
    let started = crate::platform_log::around_start(host, dir, PROFILE, || {
        host.start_without_network(dir, PROFILE)
    });
    match started {
        Ok(()) => {
            log("no network access: the store is open and no runtime runs");
            NETWORK_DENIED
        }
        Err(why) => refusal(&why),
    }
}

fn start(env: &mut Env<'_>, app_data_dir: String) -> jint {
    // The profile first: a first run's identity is provisioned under the
    // unlock policy it configures.
    if let Err(code) = provision(&app_data_dir) {
        return code;
    }
    let identity = match identity(env, &app_data_dir) {
        Ok(identity) => identity,
        Err(code) => return code,
    };
    let host = ServiceHost::global();
    let dir = std::path::PathBuf::from(&app_data_dir);
    let started = crate::platform_log::around_start(host, &dir, PROFILE, || {
        host.start(ServiceLaunch {
            app_data_dir: dir.clone(),
            profile: PROFILE.to_owned(),
            identity,
        })
    });
    match started {
        Ok(()) => {
            // The networks reported before the runtime existed, which it
            // heard at its start: a count, no address.
            match host.network_told() {
                Some(told) => log(&format!(
                    "the runtime started with the networks: {}",
                    said(told)
                )),
                None => log("the runtime started before any network was reported"),
            }
            match host.availability() {
                Some(AvailabilityMode::StayReachable) => 1,
                _ => 0,
            }
        }
        Err(why) => refusal(&why),
    }
}

/// `Native.start(appDataDir)`.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_start<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    app_data_dir: JString<'caller>,
) -> jint {
    env.with_env(|env| -> Result<jint, jni::errors::Error> {
        let app_data_dir = app_data_dir.try_to_string(env)?;
        Ok(start(env, app_data_dir))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

/// `Native.startWithoutNetwork(appDataDir)`.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_startWithoutNetwork<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    app_data_dir: JString<'caller>,
) -> jint {
    env.with_env(|env| -> Result<jint, jni::errors::Error> {
        Ok(start_without_network(&app_data_dir.try_to_string(env)?))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

/// `Native.waitNetworkAsk(timeoutMs)`: whether the person asked, from
/// the window, for the network access the platform withholds.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_waitNetworkAsk<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    timeout_ms: jlong,
) -> jboolean {
    env.with_env(|_| -> Result<jboolean, jni::errors::Error> {
        let timeout = Duration::from_millis(u64::try_from(timeout_ms).unwrap_or(0));
        Ok(crate::activity::NETWORK_ASK.wait(timeout))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

/// `Native.waitHaptic(timeoutMs)`: true when a swipe armed since the last
/// call, within the timeout.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_waitHaptic<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    timeout_ms: jlong,
) -> jboolean {
    env.with_env(|_| -> Result<jboolean, jni::errors::Error> {
        let timeout = Duration::from_millis(u64::try_from(timeout_ms).unwrap_or(0));
        Ok(crate::activity::HAPTIC.wait(timeout))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

/// `Native.touch(...)`: the platform's touch values, in dp -- which Slint's
/// logical pixels are on Android -- the snackbar's recommended time, and
/// the 12- or 24-hour setting.
#[allow(
    unsafe_code,
    clippy::too_many_arguments,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_touch<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    slop: jfloat,
    edge_left: jfloat,
    edge_right: jfloat,
    rtl: jboolean,
    reduced_motion: jboolean,
    recommended_ms: jlong,
    hour24: jboolean,
    long_press_ms: jlong,
) {
    env.with_env(|_| -> Result<(), jni::errors::Error> {
        crate::activity::touch_changed(TouchSettings {
            slop,
            edge_left,
            edge_right,
            rtl,
            reduced_motion,
            snackbar_ms: snackbar_ms(u64::try_from(recommended_ms).unwrap_or(0)),
            hour24,
            long_press_ms: u64::try_from(long_press_ms).unwrap_or(500),
        });
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}

/// `Native.waitEnded()`.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_waitEnded<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jint {
    env.with_env(|_| -> Result<jint, jni::errors::Error> {
        Ok(match ServiceHost::global().wait_ended() {
            // Native.kt's ENDED_ values.
            Ended::NotRunning => 0,
            Ended::ShutdownRequested { .. } => 1,
            Ended::RuntimeEnded => 2,
            Ended::AvailabilityChanged(_) => 3,
        })
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

/// `Native.stop()`.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_stop<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jboolean {
    env.with_env(|_| -> Result<jboolean, jni::errors::Error> {
        let Some(stopped) = ServiceHost::global().stop(GRACE) else {
            return Ok(false);
        };
        log(&format!(
            "stopped: session closed {}, runtime {:?}",
            stopped.session_closed, stopped.runtime
        ));
        Ok(true)
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

/// `Native.waitNotice(timeoutMs)`.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_waitNotice<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    timeout_ms: jlong,
) -> jint {
    env.with_env(|_| -> Result<jint, jni::errors::Error> {
        let timeout = Duration::from_millis(u64::try_from(timeout_ms).unwrap_or(0));
        Ok(ServiceHost::global()
            .hub()
            .notices()
            .wait_change(timeout)
            .map_or(-1, |count| jint::try_from(count).unwrap_or(jint::MAX)))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

/// The text `Native.text(key, count)` answers, by `Text.kt`'s keys.
fn text(key: jint, count: jint) -> String {
    let key = match key {
        0 => UiText::ReachableChannel,
        1 => UiText::ReachableTitle,
        2 => UiText::ReachableBody,
        3 => UiText::MessagesChannel,
        5 => UiText::NetworkOffTitle,
        6 => UiText::NetworkOffBody,
        // The credential prompt's title, before Show my phrase (S1).
        7 => UiText::IdS1Title,
        8 => UiText::UnlockPromptTitle,
        _ if count == 1 => UiText::NewMessage,
        _ => UiText::NewMessages,
    };
    fill(placeholder_en::text(key), &[("count", &count.to_string())])
}

/// `Native.text(key, count)`.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_text<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    key: jint,
    count: jint,
) -> JString<'caller> {
    env.with_env(|env| -> Result<JString<'caller>, jni::errors::Error> {
        JString::from_str(env, text(key, count))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

/// One report of the Service's network callback, to the runtime
/// (`network` in android-platform). Never waits on a start
/// (`a_network_report_never_waits_on_a_start` pins it).
fn network(report: NetworkReport) {
    if let Some(told) = ServiceHost::global().network_report(report) {
        log(&format!(
            "the runtime was told the networks: {}",
            said(told)
        ));
    }
}

/// What the runtime was told, by count: the log names no address,
/// resolver or host.
fn said(told: Told) -> String {
    format!(
        "{} addresses, {} resolvers, strict Private DNS {}",
        told.addresses,
        told.dns_servers,
        if told.strict_private_dns { "on" } else { "off" }
    )
}

/// `Native.networkWatching()`: the callback was registered.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_networkWatching<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) {
    env.with_env(|_| -> Result<(), jni::errors::Error> {
        network(NetworkReport::Watching);
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}

/// `Native.networkLink(network, addresses, dnsServers, privateDns)`: a
/// network, the whole of it -- its addresses and its resolvers, each its
/// length then its bytes, and its Private DNS host when strict, else
/// null.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_networkLink<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    handle: jlong,
    addresses: JByteArray<'caller>,
    dns_servers: JByteArray<'caller>,
    private_dns: JString<'caller>,
) {
    env.with_env(|env| -> Result<(), jni::errors::Error> {
        let addresses = addresses_from_wire(&env.convert_byte_array(&addresses)?);
        let dns_servers = addresses_from_wire(&env.convert_byte_array(&dns_servers)?);
        let private_dns = if private_dns.is_null() {
            None
        } else {
            Some(private_dns.try_to_string(env)?)
        };
        network(NetworkReport::Link {
            network: handle.cast_unsigned(),
            link: Link {
                addresses,
                dns_servers,
                private_dns,
            },
        });
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}

/// `Native.networkDefault(network)`: the default network, or 0 when there
/// is none (no network has handle 0: it is the platform's unset id).
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_networkDefault<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    handle: jlong,
) {
    env.with_env(|_| -> Result<(), jni::errors::Error> {
        network(NetworkReport::Default {
            network: (handle != 0).then(|| handle.cast_unsigned()),
        });
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}

/// `Native.networkLost(network)`.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_networkLost<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    handle: jlong,
) {
    env.with_env(|_| -> Result<(), jni::errors::Error> {
        network(NetworkReport::Lost {
            network: handle.cast_unsigned(),
        });
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}

/// `Native.networkUnwatched()`: the callback was unregistered.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_networkUnwatched<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) {
    env.with_env(|_| -> Result<(), jni::errors::Error> {
        network(NetworkReport::Unwatched);
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}

/// `Native.identityState(appDataDir)`: whether the identity opens, or
/// where the identity screens start (onboard.rs's `state`). The Keystore
/// cipher is made here, on a thread the JVM called from, and kept for the
/// window's flow.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_identityState<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    app_data_dir: JString<'caller>,
) -> jint {
    let state = env
        .with_env(|env| -> Result<jint, jni::errors::Error> {
            let app_data_dir = app_data_dir.try_to_string(env)?;
            if provision(&app_data_dir).is_err() {
                crate::onboard::clear_authentication();
                return Ok(crate::onboard::state::READY);
            }
            let cipher = match KeystoreCipher::new(env) {
                Ok(cipher) => cipher,
                Err(e) => {
                    log(&format!("the Keystore cipher could not be reached: {e}"));
                    crate::onboard::clear_authentication();
                    return Ok(crate::onboard::state::READY);
                }
            };
            Ok(crate::onboard::identity_state(
                std::path::PathBuf::from(app_data_dir),
                cipher,
            ))
        })
        .resolve::<ThrowRuntimeExAndDefault>();
    // Every return answers the window's wait, the early ones and a thrown
    // one included: onStart calls this before super.onStart(), which waits
    // on the window's thread (#309 review N5).
    crate::onboard::answered();
    state
}

/// `Native.waitIdentityAsk(timeoutMs)`: the identity screens' next ask of
/// the Activity (onboard.rs's `ask`), or 0.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_waitIdentityAsk<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    timeout_ms: jlong,
) -> jint {
    env.with_env(|_| -> Result<jint, jni::errors::Error> {
        let timeout = Duration::from_millis(u64::try_from(timeout_ms).unwrap_or(0));
        Ok(crate::onboard::ASKS.wait(timeout))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

/// `Native.prompted(granted)`: the credential prompt's answer.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_prompted<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    granted: jboolean,
) {
    env.with_env(|_| -> Result<(), jni::errors::Error> {
        crate::onboard::prompted(granted);
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}

/// `Native.offline(stopped)`: whether the service stopped for Show my
/// phrase.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_offline<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    stopped: jboolean,
) {
    env.with_env(|_| -> Result<(), jni::errors::Error> {
        crate::onboard::offline(stopped);
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}

/// `Native.back()`: the platform's Back, for the window (activity.rs).
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_back<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) {
    env.with_env(|_| -> Result<(), jni::errors::Error> {
        crate::activity::back();
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}

/// `Native.waitLeave(timeoutMs)`: true when Back found no way back and the
/// Activity is to finish.
#[allow(
    unsafe_code,
    reason = "the JVM finds a native method by its unmangled name"
)]
#[unsafe(no_mangle)]
extern "system" fn Java_org_interweave_human_Native_waitLeave<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    timeout_ms: jlong,
) -> jboolean {
    env.with_env(|_| -> Result<jboolean, jni::errors::Error> {
        let timeout = Duration::from_millis(u64::try_from(timeout_ms).unwrap_or(0));
        Ok(crate::activity::LEAVE.wait(timeout))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}
