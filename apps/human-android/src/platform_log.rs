// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The app's log: the runtime's `tracing` events to the platform's log,
//! each admitted exactly when the embedded host's own filter admits it
//! (`log_admits`) at the running profile's level -- so a trust set's
//! audit record (`interweave::audit`) is written whatever the level, as
//! the daemon writes it (plan section 20, the audit sink carried from
//! section 18; LOCAL-CLIENT.md section 5). The writer is the platform's
//! (`jni.rs` hands it logcat); everything else is here, so the host
//! tests it.

use std::fmt::Write as _;
use std::sync::{PoisonError, RwLock};

use interweave_human_android_platform::log_admits;
use interweave_profile_config::sections::LogLevel;
use tracing::field::{Field, Visit};

/// The level the running profile names; the default's until a start
/// sets it.
static LEVEL: RwLock<LogLevel> = RwLock::new(LogLevel::Info);

/// Filter at `level` from now on; a start sets it through [`follow`].
pub(crate) fn set_level(level: LogLevel) {
    *LEVEL.write().unwrap_or_else(PoisonError::into_inner) = level;
}

/// The level to filter at: the running runtime's where one runs, since
/// a start while one runs starts nothing and the file may have changed
/// since; else the one the profile names, which the next start loads;
/// else the default's (the start then says why the profile is unread).
/// A Service that runs without a runtime (the network-denied posture)
/// has none running, so it takes the file's.
fn level_to_follow(running: Option<LogLevel>, file: impl FnOnce() -> Option<LogLevel>) -> LogLevel {
    running.or_else(file).unwrap_or(LogLevel::Info)
}

/// Run `start` with the level followed on both sides: before, so the
/// runtime's first line is judged at the level it will load; after, so
/// the level is the one it did load. Every start path goes through it,
/// so none can leave the level stale.
pub(crate) fn around_start<T>(
    host: &interweave_human_android_platform::ServiceHost,
    app_data_dir: &std::path::Path,
    profile: &str,
    start: impl FnOnce() -> T,
) -> T {
    follow(host, app_data_dir, profile);
    let started = start();
    follow(host, app_data_dir, profile);
    started
}

/// Follow `host`'s level, reading `app_data_dir`'s profile only when no
/// runtime runs.
fn follow(
    host: &interweave_human_android_platform::ServiceHost,
    app_data_dir: &std::path::Path,
    profile: &str,
) {
    set_level(level_to_follow(host.log_level(), || {
        interweave_human_android_platform::profile_log_level(app_data_dir, profile)
    }));
}

fn level() -> LogLevel {
    *LEVEL.read().unwrap_or_else(PoisonError::into_inner)
}

/// An event as one line: its target, its message, then each field.
#[derive(Default)]
struct Line {
    message: String,
    fields: String,
}

impl Visit for Line {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            let _ = write!(self.fields, " {}={value}", field.name());
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
        } else {
            let _ = write!(self.fields, " {}={value:?}", field.name());
        }
    }
}

/// The layer: what the filter admits, to `write` as one line.
pub(crate) struct Platform<W> {
    write: W,
}

impl<W: Fn(tracing::Level, &str) + Send + Sync + 'static> Platform<W> {
    /// A layer writing through `write`.
    pub(crate) const fn new(write: W) -> Self {
        Self { write }
    }
}

impl<S, W> tracing_subscriber::Layer<S> for Platform<W>
where
    S: tracing::Subscriber,
    W: Fn(tracing::Level, &str) + Send + Sync + 'static,
{
    // Judged per event, never cached: the level changes at each start
    // (`set_level`), and an `always`/`never` interest would freeze the
    // first answer for the callsite's life.
    fn register_callsite(
        &self,
        _: &'static tracing::Metadata<'static>,
    ) -> tracing::subscriber::Interest {
        tracing::subscriber::Interest::sometimes()
    }

    // Before the fields are evaluated or a span is allocated: what the
    // filter refuses costs one check, in the release build too.
    fn enabled(
        &self,
        meta: &tracing::Metadata<'_>,
        _: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        log_admits(meta.target(), *meta.level(), level())
    }

    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        let meta = event.metadata();
        if !log_admits(meta.target(), *meta.level(), level()) {
            return;
        }
        let mut line = Line::default();
        event.record(&mut line);
        (self.write)(
            *meta.level(),
            &format!("{}: {}{}", meta.target(), line.message, line.fields),
        );
    }
}

/// Install `write` as the process's log, once: the Service's and the
/// instrumentation's process alike load the library, and the first
/// install holds. False when a log was installed already.
pub(crate) fn install(write: impl Fn(tracing::Level, &str) + Send + Sync + 'static) -> bool {
    use tracing_subscriber::layer::SubscriberExt as _;
    let subscriber = tracing_subscriber::registry().with(Platform::new(write));
    tracing::subscriber::set_global_default(subscriber).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::layer::SubscriberExt as _;

    #[test]
    fn the_level_followed_is_the_running_runtimes_else_the_files() {
        use LogLevel::{Debug, Info, Warn};
        assert_eq!(
            level_to_follow(Some(Debug), || panic!(
                "the file is read while a runtime runs"
            )),
            Debug
        );
        assert_eq!(level_to_follow(None, || Some(Warn)), Warn);
        assert_eq!(
            level_to_follow(None, || None),
            Info,
            "an unread profile: the default's"
        );
    }

    /// One test, since the level is the process's: the filter follows it,
    /// the audit record passes at every level, and a line carries its
    /// fields.
    #[test]
    fn the_log_admits_what_the_hosts_filter_does_at_the_profiles_level() {
        let lines: Arc<Mutex<Vec<(tracing::Level, String)>>> = Arc::default();
        let held = Arc::clone(&lines);
        let subscriber =
            tracing_subscriber::registry().with(Platform::new(move |level, line: &str| {
                held.lock().expect("lines").push((level, line.to_owned()));
            }));
        tracing::subscriber::with_default(subscriber, || {
            set_level(LogLevel::Warn);
            tracing::info!(target: "interweave::connectivity", "below the level");
            tracing::warn!(target: "interweave::connectivity", "at the level");
            tracing::info!(
                target: "interweave::audit",
                peer = "12D3KooW",
                allowed = true,
                "admin.trust.set"
            );
            tracing::warn!(target: "libp2p_swarm", "another crate at WARN");
            // Refused before its fields are built, and not cached: the
            // same callsite is admitted once the level allows it.
            for (at, admitted) in [(LogLevel::Warn, false), (LogLevel::Debug, true)] {
                set_level(at);
                assert_eq!(
                    tracing::enabled!(target: "interweave::connectivity", tracing::Level::DEBUG),
                    admitted,
                    "DEBUG enabled at {at:?}"
                );
            }
            set_level(LogLevel::Warn);
            let built = std::cell::Cell::new(false);
            tracing::debug!(
                target: "interweave::connectivity",
                field = {
                    built.set(true);
                    1_u8
                },
                "never built"
            );
            assert!(!built.get(), "a refused event's field was evaluated");
            set_level(LogLevel::Debug);
            tracing::debug!(target: "interweave::connectivity", "debug once the level is");
            tracing::debug!(target: "libp2p_swarm", "another crate is capped at WARN");
            set_level(LogLevel::Error);
            tracing::info!(target: "interweave::audit", "admin.trust.set");
        });
        set_level(LogLevel::Info);
        let lines = lines.lock().expect("lines").clone();
        assert_eq!(
            lines,
            vec![
                (
                    tracing::Level::WARN,
                    "interweave::connectivity: at the level".to_owned()
                ),
                (
                    tracing::Level::INFO,
                    "interweave::audit: admin.trust.set peer=12D3KooW allowed=true".to_owned()
                ),
                (
                    tracing::Level::WARN,
                    "libp2p_swarm: another crate at WARN".to_owned()
                ),
                (
                    tracing::Level::DEBUG,
                    "interweave::connectivity: debug once the level is".to_owned()
                ),
                (
                    tracing::Level::INFO,
                    "interweave::audit: admin.trust.set".to_owned()
                ),
            ]
        );
    }

    /// An identity key the runtime logs reaches the platform's log as its
    /// fingerprint and nothing of its secret: its bytes as a log writes
    /// them, their base64 and their hex are in no line (human-client-ui.md
    /// section 13; ADR-0063 decision 2). On the audit target, which every
    /// level admits, so the other test's level cannot drop it.
    #[test]
    fn an_identity_key_logged_reaches_the_platform_as_its_fingerprint_alone() {
        use base64::Engine as _;
        use interweave_human_store::identity_key;
        use interweave_human_store::key_custody::FilesystemCustody;
        let scratch = std::env::temp_dir().join(format!(
            "interweave-log-key-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir(&scratch).expect("scratch");
        let paths = interweave_profile_config::TrustBoundary::new(&scratch)
            .and_then(|b| interweave_profile_config::ProfilePaths::resolve_embedded("test", b))
            .expect("paths");
        let human = interweave_transport_api::EndpointId::parse("human").expect("valid");
        let key = identity_key::create(&paths, &human, &FilesystemCustody).expect("a key");
        let record = base64::engine::general_purpose::STANDARD
            .decode(
                std::fs::read_to_string(identity_key::key_path(&paths, &human))
                    .expect("the key file")
                    .trim(),
            )
            .expect("base64");
        let _ = std::fs::remove_dir_all(&scratch);
        // The record's last 32 bytes are the secret at the filesystem floor.
        let secret = &record[record.len() - 32..];
        let forms = [
            format!("{secret:?}"),
            base64::engine::general_purpose::STANDARD.encode(secret),
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(secret),
            secret.iter().fold(String::new(), |mut s, b| {
                let _ = write!(s, "{b:02x}");
                s
            }),
        ];
        let lines: Arc<Mutex<Vec<String>>> = Arc::default();
        let held = Arc::clone(&lines);
        let subscriber =
            tracing_subscriber::registry().with(Platform::new(move |_, line: &str| {
                held.lock().expect("lines").push(line.to_owned());
            }));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(target: "interweave::audit", key = ?key, "identity opened");
        });
        let lines = lines.lock().expect("lines").clone();
        assert_eq!(lines.len(), 1, "the control: the line was written");
        assert!(
            lines[0].contains(&key.fingerprint().to_string()),
            "the control: the key was formatted into it: {}",
            lines[0]
        );
        for form in &forms {
            assert!(
                !lines[0].contains(form.as_str()),
                "the log holds the secret"
            );
        }
    }
}
