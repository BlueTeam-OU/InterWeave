// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `claude-channel --profile <name> --endpoint <id> --delivery push|pull
//! [--log-level error|warn|info|debug]`: started by an MCP host over
//! stdio (`plugin/LIFECYCLE.md` §Startup) -- Claude Code with `push`, any
//! other host with `pull`. `claude-channel --version` alone prints the
//! version and exits.
//!
//! The profile name, the endpoint and the delivery mode are the host's
//! non-secret configuration (the plugin's `.mcp.json` arguments). From the name it derives the
//! daemon's data socket and reads the profile's desired channels for
//! `status`; it connects to nothing but the data socket.
//!
//! **The first signal is final.** Claude Code stops the bridge with
//! SIGINT, then SIGTERM about 100 ms later, then SIGKILL (SPIKE-001
//! fact 18). The bridge exits on the first, doing nothing it could not
//! finish: its lease and joins end with its IPC connection, which is the
//! daemon's act.

/// `claude-channel --version`: the workspace version, which the plugin's
/// manifest carries too, with the commit as build metadata
/// (`tools/stamp/commit_stamp.rs`).
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "+", env!("INTERWEAVE_COMMIT"));

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    if let (Some(flag), None) = (args.next(), args.next())
        && flag == "--version"
    {
        println!("claude-channel {VERSION}");
        return std::process::ExitCode::SUCCESS;
    }
    serve()
}

#[cfg(unix)]
fn serve() -> std::process::ExitCode {
    unix::main()
}

#[cfg(not(unix))]
fn serve() -> std::process::ExitCode {
    eprintln!(
        "claude-channel: the transport daemon's sockets are Unix sockets; this platform has none"
    );
    std::process::ExitCode::FAILURE
}

#[cfg(unix)]
mod unix {
    use std::process::ExitCode;

    use interweave_claude_channel::log::{Level, Line, Log};
    use interweave_claude_channel::refused::{Refusal, serve_refused};
    use interweave_claude_channel::serve::{CLIENT_KIND, Config, Env, serve};
    use interweave_claude_channel_core::Delivery;
    use interweave_ipc_client::{IpcBinding, SocketPaths};
    use interweave_profile_config::profile_format::ProfileFormatError;
    use interweave_profile_config::{
        LoadError, PersistError, ProfileConfig, ProfilePaths, XdgRoots,
    };
    use interweave_transport_api::EndpointId;
    use tokio::io::BufReader;

    const USAGE: &str = "usage: claude-channel --profile <name> --endpoint <id> \
         --delivery push|pull [--log-level error|warn|info|debug]";

    /// `status.log.file_error` when the arguments did not parse, so no
    /// profile and endpoint named a file.
    const NO_PROFILE: &str = "no_profile";

    pub(crate) fn main() -> ExitCode {
        let (mut log, prepared) = prepare();
        match run(&mut log, prepared) {
            Ok(()) => ExitCode::SUCCESS,
            Err(failure) => {
                log.write(&Line::Failed(failure));
                ExitCode::FAILURE
            }
        }
    }

    /// What a start that can work hands the serve loop.
    struct Prepared {
        sockets: SocketPaths,
        config: Config,
    }

    fn run(log: &mut Log, prepared: Result<Prepared, Refusal>) -> Result<(), String> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .map_err(|e| format!("the runtime: {e}"))?;
        // A start that cannot work is served, not exited: Claude Code does
        // not start the bridge again (`refused`).
        let prepared = prepared.map_err(|refusal| {
            log.write(&Line::Refused(refusal.reason.clone()));
            (refusal, delivery_named(std::env::args().skip(1)))
        });
        // The serve loop's copy: the one here writes the signal's line,
        // after the loop is dropped.
        let mut serving = log.clone();
        let ended = runtime.block_on(async {
            let input = BufReader::new(tokio::io::stdin());
            let output = tokio::io::stdout();
            let served = async {
                match prepared {
                    Ok(Prepared { sockets, config }) => {
                        serving.write(&Line::Started {
                            delivery: config.delivery,
                        });
                        let env = Env {
                            now_ms: Box::new(wall_ms),
                            entropy: Box::new(rand::random),
                            log: serving,
                        };
                        let binding = IpcBinding::new(sockets, CLIENT_KIND);
                        serve(binding, config, env, input, output).await
                    }
                    Err((refusal, delivery)) => {
                        serve_refused(&refusal, delivery, &mut serving, input, output).await
                    }
                }
            };
            tokio::select! {
                served = served => served.map_err(|e| format!("stdio: {e}")),
                () = first_signal() => {
                    log.write(&Line::Ended("a signal"));
                    Ok(())
                }
            }
        });
        // NOT a drop: dropping the runtime waits for its blocking pool,
        // and stdin's read there cannot be cancelled -- with the host never
        // closing stdin, the first signal would not end the process
        // (`the_first_sigint_ends_the_process_with_stdin_held_open`).
        runtime.shutdown_background();
        ended
    }

    /// The log, and the arguments and the profile's paths or why they
    /// cannot be had. The log's file needs a profile and an endpoint and
    /// nothing else -- not `XDG_RUNTIME_DIR`, so a start refused for its
    /// absence still leaves its line in the file.
    fn prepare() -> (Log, Result<Prepared, Refusal>) {
        let (profile, endpoint, delivery, level) = match arguments(std::env::args().skip(1)) {
            Ok(arguments) => arguments,
            Err(reason) => {
                let refusal = Refusal {
                    class: "usage",
                    reason,
                };
                return (Log::stderr_only(Level::default(), NO_PROFILE), Err(refusal));
            }
        };
        let roots = match XdgRoots::from_env() {
            Ok(roots) => roots,
            Err(e) => {
                let refused = refusal("the XDG directories", &e);
                return (Log::stderr_only(level, refused.class), Err(refused));
            }
        };
        let log = match ProfilePaths::resolve_offline(&profile, &roots) {
            Ok(paths) => Log::open(level, paths.state_dir(), &endpoint),
            // The refusal's class: `profile_name`, which `resolve` below
            // refuses the start with too.
            Err(e) => Log::stderr_only(level, refusal("the profile's paths", &e).class),
        };
        (log, prepare_paths(&profile, endpoint, delivery, &roots))
    }

    fn prepare_paths(
        profile: &str,
        endpoint: EndpointId,
        delivery: Delivery,
        roots: &XdgRoots,
    ) -> Result<Prepared, Refusal> {
        let paths = ProfilePaths::resolve(profile, roots)
            .map_err(|e| refusal("the profile's paths", &e))?;
        let sockets = SocketPaths {
            data: paths
                .data_socket()
                .map_err(|e| refusal("the data socket", &e))?,
            // Required by the binding's shape; the bridge is generic over
            // the data side alone and never opens it.
            admin: paths
                .admin_socket()
                .map_err(|e| refusal("the admin socket path", &e))?,
        };
        let desired_channels = ProfileConfig::load(&paths)
            .map(|config| config.channels.desired)
            .map_err(|e| load_error_class(&e).to_owned());
        let config = Config {
            endpoint,
            desired_channels,
            delivery,
        };
        Ok(Prepared { sockets, config })
    }

    /// A path failure as a refusal, classed so a host can tell an unset
    /// directory from a bad profile name (`TOOL-SURFACE.md` §status).
    fn refusal(what: &str, error: &PersistError) -> Refusal {
        let class = match error {
            PersistError::MissingHome => "home_unset",
            PersistError::NoRuntimeDir => "runtime_dir_unset",
            PersistError::InvalidProfileName { .. } => "profile_name",
            _ => "paths",
        };
        Refusal {
            class,
            reason: format!("{what}: {error}"),
        }
    }

    /// The delivery mode the arguments name, read on its own so a refusal
    /// lists the tools of the mode asked for even when another argument
    /// is wrong.
    fn delivery_named(args: impl Iterator<Item = String>) -> Option<Delivery> {
        let args: Vec<String> = args.collect();
        args.windows(2)
            .find(|pair| pair[0] == "--delivery")
            .and_then(|pair| Delivery::parse(&pair[1]))
    }

    /// `--profile <name> --endpoint <id> --delivery push|pull`, in any
    /// order, and `--log-level` optionally, nothing else. `--delivery` is
    /// REQUIRED, with no default: a host does not declare whether it
    /// takes Claude Code's channel push, so a missing flag never puts the
    /// one harness that does into pull (architect-cto's ruling, relay seq
    /// 18691). A level that does not parse is a usage refusal, as any
    /// other setting that does not: a fallback would hide the mistake.
    fn arguments(
        mut args: impl Iterator<Item = String>,
    ) -> Result<(String, EndpointId, Delivery, Level), String> {
        let (mut profile, mut endpoint, mut delivery, mut level) = (None, None, None, None);
        while let Some(flag) = args.next() {
            let value = args.next().ok_or_else(|| USAGE.to_owned())?;
            match flag.as_str() {
                "--profile" if profile.is_none() => profile = Some(value),
                "--endpoint" if endpoint.is_none() => {
                    endpoint =
                        Some(EndpointId::parse(value).map_err(|e| format!("--endpoint: {e}"))?);
                }
                "--delivery" if delivery.is_none() => {
                    delivery = Some(
                        Delivery::parse(&value)
                            .ok_or_else(|| format!("--delivery is push or pull, not {value:?}"))?,
                    );
                }
                "--log-level" if level.is_none() => {
                    level = Some(Level::parse(&value).ok_or_else(|| {
                        format!("--log-level is error, warn, info or debug, not {value:?}")
                    })?);
                }
                _ => return Err(USAGE.to_owned()),
            }
        }
        match (profile, endpoint, delivery) {
            (Some(profile), Some(endpoint), Some(delivery)) => {
                Ok((profile, endpoint, delivery, level.unwrap_or_default()))
            }
            _ => Err(USAGE.to_owned()),
        }
    }

    /// The class of a profile load's failure, which `status` reports in
    /// place of the desired channels. A profile a newer build wrote
    /// (ADR-0028 A 2026-10-20) has a class of its own: the bridge writes
    /// nothing, so it reports the refusal here rather than refusing to
    /// start, and its remedy -- the newer build -- is not a damaged
    /// marker's.
    fn load_error_class(error: &LoadError) -> &'static str {
        match error {
            LoadError::Read(_) => "Read",
            LoadError::TooLarge { .. } => "TooLarge",
            LoadError::Parse(_) => "Parse",
            LoadError::NameMismatch { .. } => "NameMismatch",
            LoadError::Invalid(_) => "Invalid",
            LoadError::KeyFileInHumanDir { .. } => "KeyFileInHumanDir",
            LoadError::KeyFileUnresolved { .. } => "KeyFileUnresolved",
            LoadError::ConfigDirUnguarded(_) => "ConfigDirUnguarded",
            LoadError::ConfigFileUnguarded { .. } => "ConfigFileUnguarded",
            LoadError::ProfileFormat(ProfileFormatError::Newer { .. }) => "ProfileFormatNewer",
            LoadError::ProfileFormat(_) => "ProfileFormat",
        }
    }

    fn wall_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
    }

    async fn first_signal() {
        use tokio::signal::unix::{SignalKind, signal};
        let (Ok(mut interrupt), Ok(mut terminate)) = (
            signal(SignalKind::interrupt()),
            signal(SignalKind::terminate()),
        ) else {
            // No handler: the default action still ends the process.
            return std::future::pending().await;
        };
        tokio::select! {
            _ = interrupt.recv() => {}
            _ = terminate.recv() => {}
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn args(list: &[&str]) -> Result<(String, EndpointId, Delivery), String> {
            arguments(list.iter().map(|s| (*s).to_owned()))
                .map(|(profile, endpoint, delivery, _)| (profile, endpoint, delivery))
        }

        /// `--log-level` is optional, `info` when absent, one of the four
        /// names when present, and given once.
        #[test]
        fn the_log_level_is_optional_and_one_of_four() {
            let base = ["--profile", "w", "--endpoint", "c", "--delivery", "push"];
            let level = |extra: &[&str]| {
                let list: Vec<String> = base.iter().chain(extra).map(|s| (*s).to_owned()).collect();
                arguments(list.into_iter()).map(|(.., level)| level)
            };
            assert_eq!(level(&[]), Ok(Level::Info));
            assert_eq!(level(&["--log-level", "debug"]), Ok(Level::Debug));
            assert_eq!(level(&["--log-level", "error"]), Ok(Level::Error));
            let refused = level(&["--log-level", "loud"]).expect_err("refused");
            assert!(refused.contains("--log-level"), "{refused}");
            assert!(level(&["--log-level", "warn", "--log-level", "info"]).is_err());
            assert!(level(&["--log-level"]).is_err());
        }

        /// A profile a newer build wrote is its own class in `status`; a
        /// marker that cannot be trusted is the marker's.
        #[test]
        fn a_newer_profile_format_has_a_class_of_its_own() {
            let newer = LoadError::ProfileFormat(ProfileFormatError::Newer {
                path: "/s/profile-format.json".into(),
                found: 2,
                own: 1,
            });
            assert_eq!(load_error_class(&newer), "ProfileFormatNewer");
            let untrusted = LoadError::ProfileFormat(ProfileFormatError::NotPrivate {
                detail: "a link".to_owned(),
            });
            assert_eq!(load_error_class(&untrusted), "ProfileFormat");
        }

        /// A profile, an endpoint and a delivery mode, in any order and
        /// nothing else; `--delivery` is required, with no default.
        #[test]
        fn the_arguments_are_a_profile_an_endpoint_and_a_delivery_mode() {
            let (profile, endpoint, delivery) = args(&[
                "--endpoint",
                "claude",
                "--delivery",
                "push",
                "--profile",
                "work",
            ])
            .expect("parsed");
            assert_eq!(profile, "work");
            assert_eq!(endpoint.as_str(), "claude");
            assert_eq!(delivery, Delivery::Push);
            let (.., pull) =
                args(&["--profile", "w", "--endpoint", "c", "--delivery", "pull"]).expect("parsed");
            assert_eq!(pull, Delivery::Pull);
            let missing =
                args(&["--profile", "work", "--endpoint", "claude"]).expect_err("refused");
            assert!(
                missing.contains("--delivery push|pull"),
                "names both: {missing}"
            );
            for bad in [
                &[][..],
                &["--profile", "work", "--delivery", "push"],
                &["--profile", "work", "--endpoint"],
                &[
                    "--profile",
                    "a",
                    "--profile",
                    "b",
                    "--endpoint",
                    "claude",
                    "--delivery",
                    "push",
                ],
                &[
                    "--profile",
                    "work",
                    "--endpoint",
                    "claude",
                    "--delivery",
                    "auto",
                ],
                &[
                    "--profile",
                    "w",
                    "--endpoint",
                    "c",
                    "--delivery",
                    "push",
                    "--delivery",
                    "pull",
                ],
                &[
                    "--profile",
                    "work",
                    "--endpoint",
                    "claude",
                    "--delivery",
                    "push",
                    "--admin",
                    "x",
                ],
                &["--profile", "work", "--endpoint", "", "--delivery", "push"],
            ] {
                assert!(args(bad).is_err(), "{bad:?}");
            }
        }
    }
}
