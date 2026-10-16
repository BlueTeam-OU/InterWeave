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

/// Filter at `level` from now on: set before each start, from the
/// profile the start will run, so the runtime's first line is judged at
/// it too.
pub(crate) fn set_level(level: LogLevel) {
    *LEVEL.write().unwrap_or_else(PoisonError::into_inner) = level;
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
}
