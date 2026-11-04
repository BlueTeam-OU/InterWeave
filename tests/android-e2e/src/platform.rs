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
//! The parsers are pure, and each is unit-tested against AOSP's dump
//! layout (`ServiceRecord.dump`, `WindowManager.LayoutParams.toString`,
//! `Bmgr`). Where a device record quotes a line the fixture is that
//! line; the records quote no bound-only service record and no `fl=`
//! line, so those fixtures follow AOSP's source and the first device
//! run captures them. A dump whose shape a parser does not recognise is
//! an error naming the dump, never a "no": a renamed field on another
//! API level must fail the case, not pass its negative.

use std::process::Command;

use crate::adb::PACKAGE;

/// The app's network service, as the activity manager names it.
pub const NETWORK_SERVICE: &str = "org.interweave.human/.NetworkService";

/// `ServiceInfo.FOREGROUND_SERVICE_TYPE_REMOTE_MESSAGING`.
pub const REMOTE_MESSAGING: u32 = 0x200;

/// One service record, as `dumpsys activity services` prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRecord {
    /// `isForeground=`; false where AOSP prints no such line, which it
    /// prints only for a service foreground now or before.
    pub foreground: bool,
    /// `types=`, the foreground service types; absent on levels that do
    /// not print it.
    pub types: Option<u32>,
    /// The foreground notice's channel, from `foregroundNoti=`.
    pub channel: Option<String>,
    /// `startRequested=`: started, not only bound; false where AOSP
    /// prints no such line, which it prints only for a service started
    /// now or before.
    pub start_requested: bool,
}

/// The record of `component` in a `dumpsys activity services` dump:
/// `Ok(None)` when the dump says the service is not running -- AOSP's
/// `(nothing)`, or other records with none of `component` -- and an
/// error when it says neither, or the record is there and incomplete.
///
/// # Errors
/// A dump with no record and no `(nothing)`; a record with no
/// `packageName=` (which AOSP always prints, so the record was cut); an
/// `isForeground`, `startRequested` or `types` that is not a value.
pub fn service_record(dump: &str, component: &str) -> Result<Option<ServiceRecord>, String> {
    let header = format!(" {component}}}");
    let is_record = |l: &str| l.contains("* ServiceRecord{");
    let Some(start) = dump
        .lines()
        .position(|l| is_record(l) && l.ends_with(&header))
    else {
        // A "not running" read off a dump this does not recognise would
        // pass a negative for free.
        return if dump
            .lines()
            .any(|l| l.trim() == "(nothing)" || is_record(l))
        {
            Ok(None)
        } else {
            Err(format!(
                "no record of {component} and no (nothing):\n{dump}"
            ))
        };
    };
    // The record runs to the next record's header or the dump's end.
    let body: Vec<&str> = dump
        .lines()
        .skip(start + 1)
        .take_while(|l| !is_record(l))
        .collect();
    let field = |name: &str| -> Option<&str> {
        body.iter().find_map(|l| {
            l.split_whitespace()
                .find_map(|w| w.strip_prefix(name)?.strip_prefix('='))
        })
    };
    if field("packageName").is_none() {
        return Err(format!(
            "{component}'s record has no packageName=, so it is cut:\n{}",
            body.join("\n")
        ));
    }
    let flag = |name: &str| -> Result<bool, String> {
        match field(name) {
            Some("true") => Ok(true),
            Some("false") | None => Ok(false),
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

/// `FLAG_SECURE`.
const SECURE_BIT: u32 = 0x2000;

/// Whether an `fl=` value names `SECURE`: AOSP prints the flags'
/// names apart by spaces (`ViewDebug.flagsToString`), up to the next
/// `key=`; a `|`-joined value and a hex mask are read too.
fn secure_flag(value: &str) -> bool {
    value
        .split_whitespace()
        .take_while(|w| !w.contains('='))
        .flat_map(|w| w.trim_end_matches('}').split('|'))
        .any(|f| {
            f == "SECURE"
                || f.strip_prefix('#')
                    .or_else(|| f.strip_prefix("0x"))
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .is_some_and(|m| m & SECURE_BIT != 0)
        })
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
                Some(rest)
            })
            .ok_or_else(|| format!("no fl= line for the window {title}:\n{dump}"))?;
        out.push((title, secure_flag(fl)));
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

/// The embedded profile's configuration, under the app's data
/// directory, as `run-as` opens it.
pub const CONFIG: &str = "interweave/config/profiles/human-android/config.yaml";

/// The value of a `key:` at any depth in a profile's YAML, from
/// its one line that sets it; a comment that names the key or its value
/// is not that line. `None` when no line sets it, or more than one does.
#[must_use]
pub fn config_value<'a>(config: &'a str, key: &str) -> Option<&'a str> {
    let head = format!("{key}:");
    let mut found = config.lines().filter_map(|l| {
        let t = l.trim_start();
        let v = t.strip_prefix(&head)?;
        let v = v.split_once(" #").map_or(v, |(v, _)| v).trim();
        (!v.is_empty()).then_some(v)
    });
    let one = found.next()?;
    found.next().is_none().then_some(one)
}

/// `config` with its one `availability_mode` set to `mode`, and nothing
/// else changed.
///
/// # Errors
/// No line, or more than one, sets `availability_mode`.
pub fn with_availability(config: &str, mode: &str) -> Result<String, String> {
    config_value(config, "availability_mode")
        .ok_or_else(|| format!("no one availability_mode in:\n{config}"))?;
    let mut out: Vec<String> = Vec::new();
    for l in config.lines() {
        let t = l.trim_start();
        if t.starts_with("availability_mode:") {
            let indent = &l[..l.len() - t.len()];
            out.push(format!("{indent}availability_mode: {mode}"));
        } else {
            out.push(l.to_owned());
        }
    }
    Ok(out.join("\n") + "\n")
}

/// The app's profile as the phone held it before a case provisioned its
/// own, written back when this is dropped, a failing case's unwind
/// included.
///
/// A case provisions a profile derived from this one with its
/// `key_unlock_policy` unchanged: the app reseals its identity under the
/// policy its profile names and deletes the other policy's key
/// (ADR-0042 A 2026-10-10, `custody::reseal`), so a profile with another
/// policy would destroy the person's identity setup, and no restore of
/// the file brings that back.
pub struct ProfileKept {
    found: String,
}

impl ProfileKept {
    /// The phone's profile now.
    ///
    /// # Panics
    /// If `adb` cannot run, or the app holds no profile to keep: a phone
    /// with no identity is set up first, by a person.
    #[must_use]
    pub fn take() -> Self {
        let out = Command::new("adb")
            .args(["exec-out", "run-as", PACKAGE, "cat", CONFIG])
            .output()
            .expect("adb");
        assert!(
            out.status.success() && !out.stdout.is_empty(),
            "the app holds no profile to keep (a phone with an identity a person made): {}",
            String::from_utf8_lossy(&out.stderr)
        );
        Self {
            found: String::from_utf8_lossy(&out.stdout).into_owned(),
        }
    }

    /// The profile as found.
    #[must_use]
    pub fn found(&self) -> &str {
        &self.found
    }

    /// The found profile's key policy, which the case's own keeps.
    ///
    /// # Panics
    /// If no one line sets `key_unlock_policy`.
    #[must_use]
    pub fn unlock_policy(&self) -> &str {
        config_value(&self.found, "key_unlock_policy")
            .unwrap_or_else(|| panic!("no one key_unlock_policy in the phone's profile"))
    }
}

impl Drop for ProfileKept {
    fn drop(&mut self) {
        // Never a second panic while unwinding: reported, not asserted.
        let _ = Command::new("adb")
            .args(["shell", "am", "force-stop", PACKAGE])
            .output();
        let write = Command::new("adb")
            .args([
                "exec-in",
                "run-as",
                PACKAGE,
                "sh",
                "-c",
                &format!("'cat > {CONFIG}'"),
            ])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write as _;
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(self.found.as_bytes())?;
                }
                child.wait()
            });
        if !matches!(write, Ok(status) if status.success()) {
            eprintln!(
                "the phone's profile was NOT put back ({write:?}); it was:\n{}",
                self.found
            );
        }
    }
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
    intent={cmp=com.example/.Other}
    packageName=com.example
    createTime=-5m0s startingBgTimeout=--
";

    // Bound by the window and never started or foreground: AOSP's
    // `ServiceRecord.dump` then prints no `isForeground=` line (only when
    // `isForeground || foregroundId != 0`) and no `startRequested=` line
    // (only when `startRequested || delayedStop || lastStartId != 0`).
    const BOUND: &str = "\
ACTIVITY MANAGER SERVICES (dumpsys activity services)
  User 0 active services:
  * ServiceRecord{8c1e2f1 u0 org.interweave.human/.NetworkService}
    intent={cmp=org.interweave.human/.NetworkService}
    packageName=org.interweave.human
    processName=org.interweave.human
    createTime=-4s012ms startingBgTimeout=--
    lastActivity=-4s012ms restartTime=-4s012ms createdFromFg=true
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
    fn not_running_is_read_only_off_a_dump_that_says_so() {
        assert_eq!(
            service_record("ACTIVITY MANAGER SERVICES\n  (nothing)\n", NETWORK_SERVICE)
                .expect("parsed"),
            None
        );
        // Only other records of the package: this one is not running.
        let others = FOREGROUND.replace(
            " org.interweave.human/.NetworkService}",
            " org.interweave.human/.Other}",
        );
        assert_eq!(
            service_record(&others, NETWORK_SERVICE).expect("parsed"),
            None
        );
        // Control: neither a record nor (nothing) is no answer at all.
        assert!(service_record("ACTIVITY MANAGER SERVICES\n", NETWORK_SERVICE).is_err());
        assert!(service_record("", NETWORK_SERVICE).is_err());
    }

    #[test]
    fn a_cut_record_or_an_unreadable_value_is_an_error() {
        let cut = BOUND.replace("    packageName=org.interweave.human\n", "");
        assert!(service_record(&cut, NETWORK_SERVICE).is_err());
        let odd = FOREGROUND.replace("isForeground=true", "isForeground=maybe");
        assert!(service_record(&odd, NETWORK_SERVICE).is_err());
        let hex = FOREGROUND.replace("types=0x00000200", "types=00000200");
        assert!(service_record(&hex, NETWORK_SERVICE).is_err());
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

    // AOSP's layout from API 30: `LayoutParams.toString` puts `fl=` on
    // its own line, the names apart by spaces (`ViewDebug.flagsToString`),
    // and `pfl=` on the next.
    const WINDOWS: &str = "\
WINDOW MANAGER WINDOWS (dumpsys window windows)
  Window #3 Window{a1 u0 StatusBar}:
    mAttrs={(0,0)(fillxWRAP) gr=TOP ty=STATUS_BAR fmt=TRANSLUCENT
      fl=NOT_FOCUSABLE SECURE
      pfl=COLOR_SPACE_AGNOSTIC}
  Window #2 Window{b2 u0 org.interweave.human/org.interweave.human.MainActivity}:
    mDisplayId=0 rootTaskId=12 mSession=Session{f1 9009:u0a234} mClient=android.os.BinderProxy@e2
    mAttrs={(0,0)(fillxfill) sim={adjust=resize forwardNavigation} ty=BASE_APPLICATION wanim=0x10302fe
      fl=LAYOUT_IN_SCREEN LAYOUT_INSET_DECOR SPLIT_TOUCH SECURE HARDWARE_ACCELERATED DRAWS_SYSTEM_BAR_BACKGROUNDS
      pfl=NO_MOVE_ANIMATION USE_BLAST
      bhv=DEFAULT}
  Window #1 Window{c3 u0 org.interweave.human/org.interweave.human.MainActivity}:
    mAttrs={(0,0)(fillxfill) ty=BASE_APPLICATION wanim=0x10302fe
      fl=LAYOUT_IN_SCREEN LAYOUT_INSET_DECOR SPLIT_TOUCH HARDWARE_ACCELERATED DRAWS_SYSTEM_BAR_BACKGROUNDS
      pfl=NO_MOVE_ANIMATION USE_BLAST
      bhv=DEFAULT}
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
            "      fl=LAYOUT_IN_SCREEN LAYOUT_INSET_DECOR SPLIT_TOUCH HARDWARE_ACCELERATED DRAWS_SYSTEM_BAR_BACKGROUNDS\n",
            "",
        );
        assert!(app_windows(&cut).is_err());
    }

    #[test]
    fn secure_is_read_anywhere_in_the_flags_and_nowhere_after_them() {
        assert!(secure_flag("LAYOUT_IN_SCREEN LAYOUT_INSET_DECOR SECURE"));
        assert!(!secure_flag("LAYOUT_IN_SCREEN HARDWARE_ACCELERATED"));
        // On one line, the next key ends the flags.
        assert!(!secure_flag("LAYOUT_IN_SCREEN pfl=SECURE"));
        assert!(secure_flag("LAYOUT_IN_SCREEN|SECURE|HARDWARE_ACCELERATED"));
        assert!(secure_flag("SECURE}"));
        assert!(secure_flag("#81812100"), "a hex mask with the bit");
        assert!(!secure_flag("#81810100"), "and without it");
    }

    // The example the cases' profiles come from, as shipped: its comment
    // names user-presence, its key says otherwise.
    const EXAMPLE: &str = include_str!("../../../architecture/config/examples/human-android.yaml");

    #[test]
    fn a_profile_value_is_read_off_its_line_and_never_a_comment() {
        assert!(
            EXAMPLE.contains("user-presence"),
            "the fixture's trap is there"
        );
        assert_eq!(
            config_value(EXAMPLE, "key_unlock_policy"),
            Some("background-compatible")
        );
        assert_eq!(
            config_value(
                "    key_unlock_policy: user-presence  # asks\n",
                "key_unlock_policy"
            ),
            Some("user-presence")
        );
        // A comment that names the key and the other value is not a line
        // that sets it.
        assert_eq!(
            config_value(
                "  # key_unlock_policy: user-presence asks the person\n  key_unlock_policy: background-compatible\n",
                "key_unlock_policy"
            ),
            Some("background-compatible")
        );
        assert_eq!(
            config_value("a: x\n  a: y\n", "a"),
            None,
            "two lines set it: no answer"
        );
        assert_eq!(config_value("b: x\n", "a"), None);
    }

    #[test]
    fn availability_changes_its_line_and_nothing_else() {
        let changed = with_availability(EXAMPLE, "foreground-only").expect("one mode");
        assert_eq!(
            config_value(&changed, "availability_mode"),
            Some("foreground-only")
        );
        let differ = EXAMPLE
            .lines()
            .zip(changed.lines())
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(differ, 1, "{changed}");
        assert_eq!(EXAMPLE.lines().count(), changed.lines().count());
        assert!(with_availability("x: 1\n", "stay-reachable").is_err());
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
