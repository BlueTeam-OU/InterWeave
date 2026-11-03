// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What the platform itself says about the app, read over `adb` from the
//! system services' own dumps rather than from anything the app writes:
//! the `NetworkService` record (`dumpsys activity services`), a backup
//! pass's per-package result (`bmgr backupnow`), and which of the app's
//! windows carry `FLAG_SECURE` (`dumpsys window windows`). The plan §20
//! Platform-tests lines that `device-runs/2026-10-20-gate-b.md` read by
//! hand are read here by these parsers, so a case asserts the platform's
//! state and not the app's account of it.
//!
//! The parsers are pure, and each is unit-tested against the lines the
//! device records quote. A dump whose shape a parser does not recognise
//! is `None` or an error naming the dump, never a "no": a renamed field
//! on another API level must fail the case, not pass its negative.

use std::process::Command;

use crate::adb::PACKAGE;

/// The app's network service, as the activity manager names it.
pub const NETWORK_SERVICE: &str = "org.interweave.human/.NetworkService";

/// `ServiceInfo.FOREGROUND_SERVICE_TYPE_REMOTE_MESSAGING`.
pub const REMOTE_MESSAGING: u32 = 0x200;

/// One service record, as `dumpsys activity services` prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRecord {
    /// `isForeground=`.
    pub foreground: bool,
    /// `types=`, the foreground service types; absent on levels that do
    /// not print it.
    pub types: Option<u32>,
    /// The foreground notice's channel, from `foregroundNoti=`.
    pub channel: Option<String>,
    /// `startRequested=`: started, not only bound.
    pub start_requested: bool,
}

/// The record of `component` in a `dumpsys activity services` dump:
/// `Ok(None)` when the dump lists no such service (it is not running),
/// an error when the record is there and a field this reads is not.
///
/// # Errors
/// The record's `isForeground`, `startRequested` or `types` missing or
/// unreadable.
pub fn service_record(dump: &str, component: &str) -> Result<Option<ServiceRecord>, String> {
    let header = format!(" {component}}}");
    let Some(start) = dump
        .lines()
        .position(|l| l.contains("* ServiceRecord{") && l.ends_with(&header))
    else {
        return Ok(None);
    };
    // The record runs to the next record's header or the dump's end.
    let body: Vec<&str> = dump
        .lines()
        .skip(start + 1)
        .take_while(|l| !l.contains("* ServiceRecord{"))
        .collect();
    let field = |name: &str| -> Option<&str> {
        body.iter().find_map(|l| {
            l.split_whitespace()
                .find_map(|w| w.strip_prefix(name)?.strip_prefix('='))
        })
    };
    let flag = |name: &str| -> Result<bool, String> {
        match field(name) {
            Some("true") => Ok(true),
            Some("false") => Ok(false),
            other => Err(format!(
                "{component}'s record has {name}={other:?}:\n{}",
                body.join("\n")
            )),
        }
    };
    let types = field("types")
        .map(|t| {
            t.strip_prefix("0x")
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .ok_or_else(|| format!("{component}'s types={t} is not hex"))
        })
        .transpose()?;
    let channel = body.iter().find_map(|l| {
        let (_, rest) = l.split_once("foregroundNoti=Notification(channel=")?;
        Some(rest.split([' ', ')']).next().unwrap_or_default().to_owned())
    });
    Ok(Some(ServiceRecord {
        foreground: flag("isForeground")?,
        types,
        channel,
        start_requested: flag("startRequested")?,
    }))
}

/// What one package's backup pass ended as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackupResult {
    /// `Success`: the transport took the package's data.
    Success,
    /// `Backup is not allowed`: the manifest refused it.
    NotAllowed,
    /// Any other result, verbatim: a disabled Backup Manager or an
    /// unavailable transport, which says nothing of the manifest.
    Other(String),
}

/// `package`'s result in a `bmgr backupnow` output, or `None` when the
/// output names no result for it.
#[must_use]
pub fn backup_result(output: &str, package: &str) -> Option<BackupResult> {
    let head = format!("Package {package} with result: ");
    output.lines().find_map(|l| {
        let r = l.trim().strip_prefix(&head)?.trim();
        Some(match r {
            "Success" => BackupResult::Success,
            "Backup is not allowed" => BackupResult::NotAllowed,
            other => BackupResult::Other(other.to_owned()),
        })
    })
}

/// The transports `bmgr list transports` lists, and the selected one
/// (the line marked `*`).
#[must_use]
pub fn transports(output: &str) -> (Vec<String>, Option<String>) {
    let mut all = Vec::new();
    let mut selected = None;
    for l in output.lines() {
        let t = l.trim();
        if t.is_empty() {
            continue;
        }
        let (name, star) = match t.strip_prefix('*') {
            Some(rest) => (rest.trim(), true),
            None => (t, false),
        };
        all.push(name.to_owned());
        if star {
            selected = Some(name.to_owned());
        }
    }
    (all, selected)
}

/// The titles of the app's windows in a `dumpsys window windows` dump,
/// each with whether its attributes carry `SECURE`. A window of the app
/// whose attributes line is not found is an error: a "not secure" read
/// off a missing line would pass the negative for free.
///
/// # Errors
/// A window of the app with no `fl=` attributes line.
pub fn app_windows(dump: &str) -> Result<Vec<(String, bool)>, String> {
    let mut out = Vec::new();
    let lines: Vec<&str> = dump.lines().collect();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim_start();
        if !(t.starts_with("Window #") && t.contains(&format!(" {PACKAGE}"))) {
            continue;
        }
        let title = t
            .split_whitespace()
            .find(|w| w.starts_with(PACKAGE))
            .unwrap_or_default()
            .trim_end_matches(['}', ':'])
            .to_owned();
        // The attributes come before the next window's header.
        let fl = lines[i + 1..]
            .iter()
            .take_while(|n| !n.trim_start().starts_with("Window #"))
            .find_map(|n| {
                let (_, rest) = n.split_once(" fl=")?;
                Some(rest.split_whitespace().next().unwrap_or_default())
            })
            .ok_or_else(|| format!("no fl= line for the window {title}:\n{dump}"))?;
        out.push((title, fl.split('|').any(|f| f == "SECURE")));
    }
    Ok(out)
}

/// The system's credential prompt's window title (`SystemUI`'s
/// `AuthContainerView` sets it on every level this runs on).
pub const BIOMETRIC_PROMPT: &str = "BiometricPrompt";

/// Whether a `dumpsys window windows` dump lists a window whose title is
/// exactly `title`.
#[must_use]
pub fn has_window(dump: &str, title: &str) -> bool {
    let tail = format!(" {title}}}:");
    dump.lines().any(|l| {
        let t = l.trim();
        t.starts_with("Window #") && t.ends_with(&tail)
    })
}

/// Whether the credential prompt shows now.
///
/// # Panics
/// As [`shell`].
#[must_use]
pub fn prompt_shown() -> bool {
    has_window(&shell(&["dumpsys", "window", "windows"]), BIOMETRIC_PROMPT)
}

/// What the runtime was told by the app's network callback, from the
/// app's log line (`apps/human-android/src/jni.rs`, `network`): the
/// addresses, and how many of them lie on no local network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Told {
    /// Every address the platform reported.
    pub addresses: u32,
    /// Those on no local network (a carrier's, ADR-0052 rule 10).
    pub off_local: u32,
}

impl Told {
    /// The addresses on a local network: Wi-Fi's, here.
    #[must_use]
    pub fn on_local(self) -> u32 {
        self.addresses.saturating_sub(self.off_local)
    }
}

/// The report a log line carries, or `None` when the line is not one.
#[must_use]
pub fn told(line: &str) -> Option<Told> {
    let (_, rest) = line.split_once("the runtime was told the networks: ")?;
    let (addresses, rest) = rest.split_once(" addresses (")?;
    let (off_local, _) = rest.split_once(" on no local network)")?;
    Some(Told {
        addresses: addresses.trim().parse().ok()?,
        off_local: off_local.trim().parse().ok()?,
    })
}

/// `adb shell <args>`'s output, failing the test on a failing `adb`.
///
/// # Panics
/// If `adb` cannot run or exits non-zero.
#[allow(
    clippy::must_use_candidate,
    reason = "a command is also run for its effect, its output unread"
)]
pub fn shell(args: &[&str]) -> String {
    let out = Command::new("adb")
        .arg("shell")
        .args(args)
        .output()
        .expect("adb");
    assert!(
        out.status.success(),
        "adb shell {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The device's API level.
///
/// # Panics
/// As [`shell`], or if the property is not a number.
#[must_use]
pub fn sdk() -> u32 {
    shell(&["getprop", "ro.build.version.sdk"])
        .trim()
        .parse()
        .expect("an API level")
}

/// The app's `NetworkService` record as the platform holds it now.
///
/// # Panics
/// As [`shell`], or on a record [`service_record`] cannot read.
#[must_use]
pub fn network_service() -> Option<ServiceRecord> {
    let dump = shell(&["dumpsys", "activity", "services", PACKAGE]);
    service_record(&dump, NETWORK_SERVICE).unwrap_or_else(|e| panic!("{e}"))
}

/// The app's windows now, with their `SECURE` bit.
///
/// # Panics
/// As [`shell`], or on a dump [`app_windows`] cannot read.
#[must_use]
pub fn windows() -> Vec<(String, bool)> {
    app_windows(&shell(&["dumpsys", "window", "windows"])).unwrap_or_else(|e| panic!("{e}"))
}

/// The app's process id, or `None` when it has none.
///
/// # Panics
/// If `adb` cannot run.
#[must_use]
pub fn app_pid() -> Option<u32> {
    let out = Command::new("adb")
        .args(["shell", "pidof", PACKAGE])
        .output()
        .expect("adb");
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // As `device-runs/2026-10-20-gate-b.md` reads the Pixel's record at
    // API 37, in AOSP `ServiceRecord.dump`'s layout.
    const FOREGROUND: &str = "\
ACTIVITY MANAGER SERVICES (dumpsys activity services)
  User 0 active services:
  * ServiceRecord{8c1e2f1 u0 org.interweave.human/.NetworkService}
    intent={cmp=org.interweave.human/.NetworkService}
    packageName=org.interweave.human
    isForeground=true foregroundId=1 types=0x00000200 foregroundNoti=Notification(channel=reachable shortcut=null contentView=null vibrate=null sound=null defaults=0x0 flags=0x62 color=0x00000000 vis=PRIVATE)
    createTime=-1m2s startingBgTimeout=--
    startRequested=true delayedStop=false stopIfKilled=false callStart=true lastStartId=1
  * ServiceRecord{1a2b3c4 u0 com.example/.Other}
    isForeground=false foregroundId=0 foregroundNoti=null
    startRequested=false delayedStop=false stopIfKilled=false callStart=false lastStartId=0
";

    // Bound by the window, not started, and no `types=` (API 30 prints
    // none).
    const BOUND: &str = "\
  * ServiceRecord{8c1e2f1 u0 org.interweave.human/.NetworkService}
    isForeground=false foregroundId=0 foregroundNoti=null
    startRequested=false delayedStop=false stopIfKilled=false callStart=false lastStartId=0
    Bindings:
    * IntentBindRecord{5e6f7a8 CREATE}:
";

    #[test]
    fn a_foreground_record_is_read_with_its_type_and_channel() {
        let r = service_record(FOREGROUND, NETWORK_SERVICE)
            .expect("a record")
            .expect("running");
        assert_eq!(
            r,
            ServiceRecord {
                foreground: true,
                types: Some(REMOTE_MESSAGING),
                channel: Some("reachable".to_owned()),
                start_requested: true,
            }
        );
    }

    #[test]
    fn the_next_record_is_not_read_into_this_one() {
        let other = service_record(FOREGROUND, "com.example/.Other")
            .expect("a record")
            .expect("running");
        assert!(!other.foreground && other.channel.is_none() && other.types.is_none());
    }

    #[test]
    fn a_bound_record_is_neither_foreground_nor_started() {
        let r = service_record(BOUND, NETWORK_SERVICE)
            .expect("a record")
            .expect("running");
        assert!(!r.foreground && !r.start_requested && r.types.is_none());
    }

    #[test]
    fn no_record_is_not_running_and_a_record_missing_a_field_is_an_error() {
        assert_eq!(
            service_record("  (nothing)\n", NETWORK_SERVICE).expect("parsed"),
            None
        );
        let renamed = BOUND.replace("isForeground=", "isFg=");
        assert!(service_record(&renamed, NETWORK_SERVICE).is_err());
    }

    // `bmgr backupnow`'s lines, as the gate-b record quotes its results.
    const BACKUP: &str = "\
Running incremental backup for 2 requested packages.
Package org.interweave.human with result: Backup is not allowed
Package com.android.providers.settings with result: Success
Backup finished with result: Success
";

    #[test]
    fn each_packages_backup_result_is_read_by_its_name() {
        assert_eq!(
            backup_result(BACKUP, PACKAGE),
            Some(BackupResult::NotAllowed)
        );
        assert_eq!(
            backup_result(BACKUP, "com.android.providers.settings"),
            Some(BackupResult::Success)
        );
        assert_eq!(backup_result(BACKUP, "com.absent"), None);
        assert_eq!(
            backup_result(
                "Package org.interweave.human with result: Transport error\n",
                PACKAGE
            ),
            Some(BackupResult::Other("Transport error".to_owned()))
        );
    }

    #[test]
    fn the_selected_transport_is_the_starred_one() {
        let (all, selected) = transports(
            "    com.android.localtransport/.LocalTransport\n  * com.google.android.gms/.backup.BackupTransportService\n",
        );
        assert_eq!(all.len(), 2);
        assert_eq!(
            selected.as_deref(),
            Some("com.google.android.gms/.backup.BackupTransportService")
        );
    }

    const WINDOWS: &str = "\
WINDOW MANAGER WINDOWS (dumpsys window windows)
  Window #3 Window{a1 u0 StatusBar}:
    mAttrs={(0,0)(fillxWRAP) ty=STATUS_BAR fl=NOT_FOCUSABLE|SECURE}
  Window #2 Window{b2 u0 org.interweave.human/org.interweave.human.MainActivity}:
    mAttrs={(0,0)(fillxfill) sim={adjust=resize} ty=BASE_APPLICATION fl=LAYOUT_IN_SCREEN|SECURE|HARDWARE_ACCELERATED}
  Window #1 Window{c3 u0 org.interweave.human/org.interweave.human.MainActivity}:
    mAttrs={(0,0)(fillxfill) ty=BASE_APPLICATION fl=LAYOUT_IN_SCREEN|HARDWARE_ACCELERATED}
";

    #[test]
    fn only_the_apps_windows_are_read_each_with_its_secure_bit() {
        let w = app_windows(WINDOWS).expect("parsed");
        assert_eq!(
            w.len(),
            2,
            "the status bar's SECURE is not the app's: {w:?}"
        );
        assert!(w[0].1, "{w:?}");
        assert!(!w[1].1, "{w:?}");
    }

    #[test]
    fn a_window_with_no_flags_line_is_an_error_not_a_no() {
        let cut = WINDOWS.replace(
            "    mAttrs={(0,0)(fillxfill) ty=BASE_APPLICATION fl=LAYOUT_IN_SCREEN|HARDWARE_ACCELERATED}\n",
            "",
        );
        assert!(app_windows(&cut).is_err());
    }

    #[test]
    fn a_network_report_is_read_from_the_apps_line_and_nothing_else() {
        // As `device-runs/2026-10-31-pixel-9a.md` quotes them.
        let off = told("11-03 15:02:38.777 I/interweave( 9009): the runtime was told the networks: 0 addresses (0 on no local network), unobserved resolvers, strict Private DNS off")
            .expect("a report");
        assert_eq!(
            off,
            Told {
                addresses: 0,
                off_local: 0
            }
        );
        let lte = told("the runtime was told the networks: 1 addresses (1 on no local network), 4 resolvers, strict Private DNS off")
            .expect("a report");
        assert_eq!(lte.on_local(), 0, "the carrier's address is not Wi-Fi's");
        let wifi = told("the runtime was told the networks: 3 addresses (1 on no local network), 2 resolvers, strict Private DNS off")
            .expect("a report");
        assert_eq!(wifi.on_local(), 2);
        assert_eq!(told("the runtime was told the networks: many"), None);
        assert_eq!(told("something else"), None);
    }

    #[test]
    fn the_prompt_window_is_read_by_its_exact_title() {
        let with = format!(
            "{WINDOWS}  Window #4 Window{{d4 u0 BiometricPrompt}}:\n    mAttrs={{ty=STATUS_BAR_SUB_PANEL fl=SECURE}}\n"
        );
        assert!(has_window(&with, BIOMETRIC_PROMPT));
        assert!(!has_window(WINDOWS, BIOMETRIC_PROMPT), "control: absent");
        let other = with.replace("u0 BiometricPrompt}", "u0 BiometricPromptX}");
        assert!(
            !has_window(&other, BIOMETRIC_PROMPT),
            "a title is matched whole"
        );
    }
}
