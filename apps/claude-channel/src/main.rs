// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `claude-channel --profile <name> --endpoint <alias|id> --delivery push|pull
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
    use interweave_profile_config::endpoint_overlay::{EndpointOverlay, EndpointOverlayError};
    use interweave_profile_config::profile_format::ProfileFormatError;
    use interweave_profile_config::{
        EndpointAlias, EndpointSelector, EndpointsConfig, LoadError, PersistError, ProfileConfig,
        ProfilePaths, ResolveEndpointError, XdgRoots, resolve_endpoint,
    };
    use interweave_transport_api::EndpointId;
    use tokio::io::BufReader;

    const USAGE: &str = "usage: claude-channel --profile <name> --endpoint <alias|id> \
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
        endpoint: EndpointSelector,
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
        // ONE load serves both the desired channels and the endpoint, so
        // the two cannot disagree about which configuration they read. It
        // is the document alone: the bridge holds no endpoint key, so it
        // reads nothing under `endpoint-keys/` (ENDPOINTS.md, the client
        // that resolves an alias; architect-cto's ruling of 2026-11-21).
        let loaded = ProfileConfig::load_document(&paths);
        let overlay = |configured: &EndpointsConfig| {
            EndpointOverlay::read_within(
                &EndpointOverlay::path_for(&paths),
                configured,
                paths.boundary(),
            )
            .map(|(overlay, _normalised)| overlay)
        };
        let (endpoint, endpoint_alias) = local_endpoint(endpoint, loaded.as_ref(), overlay)?;
        let desired_channels = loaded
            .map(|config| config.channels.desired)
            .map_err(|e| load_error_class(&e).to_owned());
        let config = Config {
            endpoint,
            endpoint_alias,
            desired_channels,
            delivery,
        };
        Ok(Prepared { sockets, config })
    }

    /// The id `--endpoint` names, and the alias `status` reports beside
    /// it, resolved ONCE, here, before any socket is opened: the hello
    /// carries the id alone, and a reconnect re-sends that id, never a
    /// re-resolved alias -- a `config.yaml` edit reaches the next session
    /// (ENDPOINTS.md §Configured endpoint set, A 2026-11-19).
    ///
    /// An alias needs the configuration: an alias no entry carries, or
    /// one given when the configuration does not load, refuses the start
    /// with class `endpoint_alias`. An id needs nothing -- a failed load
    /// is tolerated as before, and an id no entry holds starts and meets
    /// the daemon's lease refusal, which is the daemon's to give.
    ///
    /// `overlay` reads the endpoint overlay, and is called only for an
    /// alias: the alias resolves in the set the daemon composes, so an
    /// entry the overlay adds is found once overlays add entries. It is
    /// read and normalised in memory and never written back
    /// (`EndpointOverlay::read_within`) -- the bridge writes nothing in
    /// the profile; what normalising changed is the daemon's to warn. An
    /// overlay that cannot be read refuses an alias start as a failed
    /// load does, and is never read for an id.
    fn local_endpoint(
        endpoint: EndpointSelector,
        loaded: Result<&ProfileConfig, &LoadError>,
        overlay: impl FnOnce(&EndpointsConfig) -> Result<EndpointOverlay, EndpointOverlayError>,
    ) -> Result<(EndpointId, Option<EndpointAlias>), Refusal> {
        let refused = |reason: String| Refusal {
            class: "endpoint_alias",
            reason,
        };
        match (endpoint, loaded) {
            // The reasons name no alias: a refusal's reason is a log line
            // too, and a log line carries no endpoint identifier
            // (TOOL-SURFACE.md §Status visibility, `log`). The log's file
            // name says which one it was.
            (EndpointSelector::Alias(_), Err(e)) => Err(refused(format!(
                "--endpoint: an alias needs the profile's configuration, which did not load ({})",
                load_error_class(e)
            ))),
            (EndpointSelector::Alias(alias), Ok(config)) => {
                let overlay = overlay(&config.endpoints).map_err(|e| {
                    refused(format!(
                        "--endpoint: an alias needs the endpoint overlay, which did not load ({})",
                        overlay_error_class(&e)
                    ))
                })?;
                match resolve_endpoint(&config.endpoints, &overlay, alias.as_str()) {
                    Ok(id) => Ok((id, Some(alias))),
                    Err(ResolveEndpointError::UnknownAlias(_)) => Err(refused(
                        "--endpoint: no configured endpoint has that alias".to_owned(),
                    )),
                    // Unreachable for a parsed alias; refused all the same
                    // rather than started on a guess.
                    Err(_) => Err(refused(
                        "--endpoint: the alias resolved to no endpoint".to_owned(),
                    )),
                }
            }
            (EndpointSelector::Id(id), Ok(config)) => {
                let alias = config
                    .endpoints
                    .entries
                    .iter()
                    .find(|entry| entry.id == id)
                    .and_then(|entry| entry.alias.clone());
                Ok((id, alias))
            }
            (EndpointSelector::Id(id), Err(_)) => Ok((id, None)),
        }
    }

    /// A word for what was wrong with the overlay, never its `Display`:
    /// a refusal's reason is a log line, and the class is all an
    /// operator needs to look at the file.
    fn overlay_error_class(e: &EndpointOverlayError) -> &'static str {
        match e {
            EndpointOverlayError::Read(_) => "unreadable",
            EndpointOverlayError::NotPrivate { .. } => "not_private",
            EndpointOverlayError::TooLarge => "too_large",
            EndpointOverlayError::Parse(_) | EndpointOverlayError::LabelMember { .. } => "parse",
            // `too_large` stays the file's byte ceiling: a set or a row
            // count past its bound is a file that parsed and is not a
            // legal overlay, as an added entry that collides is.
            EndpointOverlayError::Unknown
            | EndpointOverlayError::Disabled
            | EndpointOverlayError::CarryConflict { .. }
            | EndpointOverlayError::SetInOneMember { .. }
            | EndpointOverlayError::DuplicatePeer { .. }
            | EndpointOverlayError::SetTooLarge { .. }
            | EndpointOverlayError::SetRowsExceeded { .. }
            | EndpointOverlayError::AddedEntryCarriesTrust { .. }
            | EndpointOverlayError::AddedIdCollides(_)
            | EndpointOverlayError::AddedAliasCollides(_)
            | EndpointOverlayError::TooManyEndpoints { .. }
            | EndpointOverlayError::Write(_) => "invalid",
        }
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

    /// `--profile <name> --endpoint <alias|id> --delivery push|pull`, in any
    /// order, and `--log-level` optionally, nothing else. `--delivery` is
    /// REQUIRED, with no default: a host does not declare whether it
    /// takes Claude Code's channel push, so a missing flag never puts the
    /// one harness that does into pull (architect-cto's ruling, relay seq
    /// 18691). A level that does not parse is a usage refusal, as any
    /// other setting that does not: a fallback would hide the mistake.
    fn arguments(
        mut args: impl Iterator<Item = String>,
    ) -> Result<(String, EndpointSelector, Delivery, Level), String> {
        let (mut profile, mut endpoint, mut delivery, mut level) = (None, None, None, None);
        while let Some(flag) = args.next() {
            let value = args.next().ok_or_else(|| USAGE.to_owned())?;
            match flag.as_str() {
                "--profile" if profile.is_none() => profile = Some(value),
                "--endpoint" if endpoint.is_none() => {
                    endpoint = Some(EndpointSelector::parse(&value).ok_or_else(|| {
                        "--endpoint: neither an EndpointId nor an alias".to_owned()
                    })?);
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
        use interweave_test_support::endpoint_keys::endpoint_id;

        fn args(list: &[&str]) -> Result<(String, EndpointSelector, Delivery), String> {
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
            assert_eq!(
                endpoint,
                EndpointSelector::Alias(EndpointAlias::parse("claude").expect("an alias"))
            );
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

        /// `--endpoint` takes an alias or a derived id and nothing else;
        /// what is neither is a usage refusal whose reason does not echo
        /// it (a reason is a log line too).
        #[test]
        fn the_endpoint_is_an_alias_or_an_id_and_nothing_else() {
            let with = |value: &str| {
                args(&["--profile", "w", "--endpoint", value, "--delivery", "push"])
                    .map(|(_, endpoint, _)| endpoint)
            };
            let id = endpoint_id("claude");
            assert_eq!(with(id.as_str()), Ok(EndpointSelector::Id(id.clone())));
            assert!(matches!(
                with("fabric-control"),
                Ok(EndpointSelector::Alias(_))
            ));
            let long = "a".repeat(65);
            for bad in ["Claude", "..", ".claude", "0claude", "a/b", &long] {
                let refused = with(bad).expect_err(bad);
                assert!(refused.starts_with("--endpoint:"), "{refused}");
                assert!(!refused.contains(bad), "echoes {bad:?}: {refused}");
            }
        }

        fn profile(yaml_endpoints: &str) -> ProfileConfig {
            let text = format!(
                "schema_version: 2\nprofile:\n  name: w\nidentity: {{}}\ntrust:\n  policy: static-allowlist\n  \
                 allowed_peers: []\n{yaml_endpoints}"
            );
            ProfileConfig::parse_yaml(&text).expect("a profile")
        }

        /// An entry's required members past its key and alias.
        const ENTRY: &str = "      enabled: true\n      advertise: true\n      \
             allowed_client_kinds: [claude-channel]\n      inbound: inherit_profile_trust\n      \
             outbound: inherit_profile_trust\n";

        fn endpoints() -> String {
            format!(
                "endpoints:\n  default_direct_endpoint: claude\n  entries:\n    \
                 - key: {}.key\n      alias: claude\n{ENTRY}    \
                 - key: {}.key\n{ENTRY}",
                endpoint_id("claude").as_str(),
                endpoint_id("bare").as_str()
            )
        }

        /// The start's resolution, every case of the contract
        /// (TOOL-SURFACE.md, the paragraph on Stage 22): an alias resolves
        /// to its entry's id and is reported; an unknown alias, or an
        /// alias without a configuration, is refused `endpoint_alias`
        /// with no identifier in the reason; an id starts with or without
        /// a configuration and with or without an entry, its alias
        /// reported only when the entry has one and the load succeeded.
        #[test]
        fn an_alias_is_resolved_once_and_an_id_needs_no_configuration() {
            let loaded = profile(&endpoints());
            let claude = endpoint_id("claude");
            let alias = |s: &str| EndpointSelector::Alias(EndpointAlias::parse(s).expect("alias"));
            let resolved = |s: EndpointSelector, l: Result<&ProfileConfig, &LoadError>| {
                local_endpoint(s, l, |_| Ok(EndpointOverlay::default()))
                    .map_err(|r| (r.class, r.reason))
            };
            assert_eq!(
                resolved(alias("claude"), Ok(&loaded)),
                Ok((
                    claude.clone(),
                    Some(EndpointAlias::parse("claude").expect("alias"))
                ))
            );
            let failed = LoadError::Parse("x".to_owned());
            for (selector, load) in [
                (alias("nobody"), Ok(&loaded)),
                (alias("claude"), Err(&failed)),
            ] {
                let (class, reason) = resolved(selector, load).expect_err("refused");
                assert_eq!(class, "endpoint_alias");
                assert!(
                    !reason.contains("nobody") && !reason.contains("claude"),
                    "{reason}"
                );
            }
            assert_eq!(
                resolved(EndpointSelector::Id(claude.clone()), Err(&failed)),
                Ok((claude.clone(), None))
            );
            let bare = endpoint_id("bare");
            assert_eq!(
                resolved(EndpointSelector::Id(bare.clone()), Ok(&loaded)),
                Ok((bare, None))
            );
            let stranger = endpoint_id("stranger");
            assert_eq!(
                resolved(EndpointSelector::Id(stranger.clone()), Ok(&loaded)),
                Ok((stranger, None))
            );
            assert_eq!(
                resolved(EndpointSelector::Id(claude.clone()), Ok(&loaded))
                    .map(|(_, a)| a.map(|a| a.as_str().to_owned())),
                Ok(Some("claude".to_owned()))
            );
        }

        /// The overlay is read for an alias and only for one: an id
        /// starts whatever the overlay holds, and an overlay that cannot
        /// be read refuses an alias start with no identifier in the
        /// reason.
        #[test]
        fn the_overlay_is_read_for_an_alias_alone() {
            let loaded = profile(&endpoints());
            let claude = endpoint_id("claude");
            let alias = EndpointSelector::Alias(EndpointAlias::parse("claude").expect("alias"));
            let unread = |_: &EndpointsConfig| -> Result<EndpointOverlay, EndpointOverlayError> {
                panic!("the overlay was read for an id")
            };
            assert_eq!(
                local_endpoint(EndpointSelector::Id(claude.clone()), Ok(&loaded), unread)
                    .map(|(id, _)| id)
                    .map_err(|r| r.reason),
                Ok(claude.clone())
            );
            let mut read = 0;
            let resolved = local_endpoint(alias.clone(), Ok(&loaded), |_| {
                read += 1;
                Ok(EndpointOverlay::default())
            })
            .map(|(id, _)| id)
            .map_err(|r| r.reason);
            assert_eq!((resolved, read), (Ok(claude), 1));
            let refused =
                local_endpoint(alias, Ok(&loaded), |_| Err(EndpointOverlayError::TooLarge))
                    .expect_err("refused");
            assert_eq!(refused.class, "endpoint_alias");
            assert!(refused.reason.contains("too_large"), "{}", refused.reason);
            assert!(!refused.reason.contains("claude"), "{}", refused.reason);
        }

        /// Each refusal the overlay's sets and added entries bring
        /// (ADR-0028 A 2026-11-22) is classed `invalid`, and an alias
        /// start refused on one carries the class and none of the
        /// identifiers the error holds.
        #[test]
        fn the_overlay_set_and_added_refusals_are_invalid_and_name_nothing() {
            use interweave_profile_config::endpoint_overlay::OverlayMember;
            use interweave_transport_api::TransportIdentity;
            let loaded = profile(&endpoints());
            let claude = endpoint_id("claude");
            let peer = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
            let refusals: Vec<fn(EndpointId) -> EndpointOverlayError> = vec![
                |endpoint| EndpointOverlayError::SetInOneMember {
                    endpoint,
                    only_in: OverlayMember::Inbound,
                },
                |endpoint| EndpointOverlayError::DuplicatePeer {
                    member: OverlayMember::Outbound,
                    endpoint,
                    peer: TransportIdentity::parse(
                        "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN",
                    )
                    .expect("peer"),
                },
                |endpoint| EndpointOverlayError::SetTooLarge {
                    member: OverlayMember::Inbound,
                    endpoint,
                    got: 1_000_000,
                },
                |_| EndpointOverlayError::SetRowsExceeded { got: 1_000_000 },
                |_| EndpointOverlayError::AddedEntryCarriesTrust {
                    member: OverlayMember::Outbound,
                },
                EndpointOverlayError::AddedIdCollides,
                |_| {
                    EndpointOverlayError::AddedAliasCollides(
                        EndpointAlias::parse("claude").expect("alias"),
                    )
                },
                |_| EndpointOverlayError::TooManyEndpoints { got: 1_000_000 },
            ];
            for make in refusals {
                let e = make(claude.clone());
                let display = e.to_string();
                assert_eq!(overlay_error_class(&e), "invalid", "{display}");
                let refused = local_endpoint(
                    EndpointSelector::Alias(EndpointAlias::parse("claude").expect("alias")),
                    Ok(&loaded),
                    |_| Err(make(claude.clone())),
                )
                .expect_err("refused");
                assert_eq!(refused.class, "endpoint_alias", "{display}");
                assert!(refused.reason.contains("(invalid)"), "{}", refused.reason);
                for held in [claude.as_str(), "claude", peer, "1000000"] {
                    assert!(!refused.reason.contains(held), "{held}: {}", refused.reason);
                }
            }
        }
    }
}
