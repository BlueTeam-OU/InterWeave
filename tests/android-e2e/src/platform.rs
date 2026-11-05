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
//! an error naming the dump, never a "no": no `(nothing)` and no record,
//! a record with no `packageName=`, a value that is not one. The one
//! exception is a service record's `isForeground` and `startRequested`,
//! which AOSP prints only when set, so their absence reads false and a
//! renamed one would too: what reads them true on the same phone is
//! `fgs_lifecycle_stay_reachable_is_foreground_and_restarts_after_a_crash`,
//! the control to run beside any case that asserts them false.

use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use interweave_profile_config::ProfileConfig;
use interweave_profile_config::runtime::{AvailabilityMode, KeyUnlockPolicy};

use crate::adb::PACKAGE;

/// The phone's serial once a case has followed it to a new one; until
/// then `None`, and `adb` picks the phone from `ANDROID_SERIAL`.
static MOVED: Mutex<Option<String>> = Mutex::new(None);

/// `adb`, aimed at the phone where it is now: at the serial
/// [`moved_to`] set, else as `ANDROID_SERIAL` says. Every call this
/// module makes goes through it, the profile's restore among them, so a
/// phone whose wireless-debugging port changed under a case is still
/// the one put back.
#[must_use]
pub fn adb() -> Command {
    let mut command = Command::new("adb");
    if let Some(serial) = moved() {
        command.args(["-s", &serial]);
    }
    command
}

/// The serial [`moved_to`] set, if any.
#[must_use]
pub fn moved() -> Option<String> {
    MOVED
        .lock()
        .map_or_else(|e| e.into_inner().clone(), |s| s.clone())
}

/// The phone answers at `serial` from now on: wireless debugging came
/// back on a new port.
pub fn moved_to(serial: &str) {
    let mut moved = MOVED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *moved = Some(serial.to_owned());
}

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
/// `(nothing)`, or other records with none of `component` among the
/// running ones -- and an error when it says neither, or the record is
/// there and incomplete. A record AOSP lists under a section of its own
/// (`* Destroy ServiceRecord{..}` under "Destroying services:", as the
/// A40 printed one 5 s into a stay-reachable start) is a service on its
/// way in or out, not a running one: it answers "not running", never
/// "no answer".
///
/// # Errors
/// A dump with no record and no `(nothing)`; a record with no
/// `packageName=` (which AOSP always prints, so the record was cut); an
/// `isForeground`, `startRequested` or `types` that is not a value.
pub fn service_record(dump: &str, component: &str) -> Result<Option<ServiceRecord>, String> {
    let header = format!(" {component}}}");
    let Some(start) = dump
        .lines()
        .position(|l| record_header(l) == Some(true) && l.ends_with(&header))
    else {
        // A "not running" read off a dump this does not recognise would
        // pass a negative for free.
        return if dump
            .lines()
            .any(|l| l.trim() == "(nothing)" || record_header(l).is_some())
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
        .take_while(|l| record_header(l).is_none())
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

/// Whether `line` opens a service record: `Some(true)` for a running
/// one (`* ServiceRecord{`), `Some(false)` for one a section lists under
/// a word of its own (`* Destroy ServiceRecord{`), `None` for any other
/// line. Read by shape rather than by the words AOSP uses, so a section
/// the fixtures do not name still bounds the record before it.
fn record_header(line: &str) -> Option<bool> {
    let rest = line.trim_start().strip_prefix("* ")?;
    let (before, _) = rest.split_once("ServiceRecord{")?;
    match before {
        "" => Some(true),
        word if word.ends_with(' ') && !word.trim().contains(' ') => Some(false),
        _ => None,
    }
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
/// `key=`; a `|`-joined value and a hex mask are read too. The A40
/// (Samsung, API 30) prints the mask as bare hex (`fl=1800518`, every
/// window of its dump of 2026-11-04), so a word of hex digits alone is
/// a mask: no flag's name is spelt in the letters A to F only.
fn secure_flag(value: &str) -> bool {
    value
        .split_whitespace()
        .take_while(|w| !w.contains('='))
        .flat_map(|w| w.trim_end_matches('}').split('|'))
        .any(|f| {
            f == "SECURE"
                || f.strip_prefix('#')
                    .or_else(|| f.strip_prefix("0x"))
                    .or_else(|| f.bytes().all(|b| b.is_ascii_hexdigit()).then_some(f))
                    .filter(|h| !h.is_empty())
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

/// Samsung's title for the same prompt: the A40 (One UI, API 30) draws
/// it as `BiometricPrompt3P` (its dump of 2026-11-05), so a check for
/// AOSP's title alone never sees the prompt there.
pub const SAMSUNG_BIOMETRIC_PROMPT: &str = "BiometricPrompt3P";

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
    prompt_in(&shell(&["dumpsys", "window", "windows"]))
}

/// Whether a `dumpsys window windows` dump lists the credential prompt,
/// under AOSP's title or Samsung's.
#[must_use]
pub fn prompt_in(dump: &str) -> bool {
    [BIOMETRIC_PROMPT, SAMSUNG_BIOMETRIC_PROMPT]
        .iter()
        .any(|title| has_window(dump, title))
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
    let out = adb().arg("shell").args(args).output().expect("adb");
    assert!(
        out.status.success(),
        "adb shell {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Whether the keyguard is up, from `dumpsys window`'s
/// `isKeyguardShowing=` (the A40, API 30, prints it in its policy
/// line).
///
/// # Errors
/// A dump with no such value: a level that prints it elsewhere is
/// added from its own dump, never read as unlocked.
pub fn keyguard_showing(dump: &str) -> Result<bool, String> {
    let said = dump
        .split_whitespace()
        .find_map(|w| w.strip_prefix("isKeyguardShowing="));
    match said {
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        other => Err(format!(
            "no isKeyguardShowing= value in dumpsys window ({other:?})"
        )),
    }
}

/// How long a person is given to unlock the phone.
pub const UNLOCK_PATIENCE: Duration = Duration::from_secs(300);

/// The phone awake and unlocked before a case brings the app to the
/// front: an `am start -W` behind a secure keyguard waits with no end
/// (the A40, 2026-11-04, 51 minutes). The screen is woken; a keyguard
/// still up asks the person on stderr and is waited out.
///
/// # Panics
/// As [`shell`]; on a dump [`keyguard_showing`] cannot read; or on a
/// phone still locked after [`UNLOCK_PATIENCE`].
pub fn until_unlocked() {
    shell(&["input", "keyevent", "KEYCODE_WAKEUP"]);
    let locked =
        || keyguard_showing(&shell(&["dumpsys", "window"])).unwrap_or_else(|e| panic!("{e}"));
    if !locked() {
        return;
    }
    eprintln!("PERSON: on the phone, unlock it (within {UNLOCK_PATIENCE:?})");
    let deadline = Instant::now() + UNLOCK_PATIENCE;
    while locked() {
        assert!(
            Instant::now() < deadline,
            "the phone stayed locked for {UNLOCK_PATIENCE:?}: the case needs it unlocked"
        );
        std::thread::sleep(Duration::from_secs(1));
    }
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
    let out = adb()
        .args(["shell", "pidof", PACKAGE])
        .output()
        .expect("adb");
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

/// The embedded profile's configuration, under the app's data
/// directory, as `run-as` opens it.
pub const CONFIG: &str = "interweave/config/profiles/human-android/config.yaml";

/// The person's availability choice, which overrides the profile's
/// authored `availability_mode` while it exists (ADR-0041 A 2026-10-10,
/// `interweave_profile_config::availability_overlay`).
pub const AVAILABILITY_OVERLAY: &str =
    "interweave/state/profiles/human-android/availability-overlay.json";

/// A profile's key policy as the app reads it: the schema's default,
/// `background-compatible`, where the document names none.
///
/// # Errors
/// The text is not a profile.
pub fn unlock_policy(config: &str) -> Result<KeyUnlockPolicy, String> {
    ProfileConfig::parse_yaml(config)
        .map(|c| c.runtime.android.key_unlock_policy)
        .map_err(|e| format!("not a profile ({e}):\n{config}"))
}

fn mode_text(mode: AvailabilityMode) -> &'static str {
    match mode {
        AvailabilityMode::ForegroundOnly => "foreground-only",
        AvailabilityMode::StayReachable => "stay-reachable",
    }
}

/// `config` with `runtime.android.availability_mode` set to `mode`: its
/// line replaced where there is one, else added under `android:`, or
/// under a new `android:` beneath `runtime:` -- the app's own first-run
/// profile has neither and takes the schema's defaults. Nothing else
/// changes, and the result is read back as the app parses it.
///
/// # Errors
/// More than one line sets the mode, no `runtime:` block, or a result
/// that does not parse to `mode`.
pub fn with_availability(config: &str, mode: AvailabilityMode) -> Result<String, String> {
    let set = format!("availability_mode: {}", mode_text(mode));
    let indent = |l: &str| l.len() - l.trim_start().len();
    let mut lines: Vec<String> = config.lines().map(str::to_owned).collect();
    let at: Vec<usize> = (0..lines.len())
        .filter(|&i| lines[i].trim_start().starts_with("availability_mode:"))
        .collect();
    match at.as_slice() {
        [i] => {
            let pad = " ".repeat(indent(&lines[*i]));
            lines[*i] = format!("{pad}{set}");
        }
        [] => {
            let runtime = lines
                .iter()
                .position(|l| l.trim_end() == "runtime:")
                .ok_or_else(|| format!("no runtime: block in:\n{config}"))?;
            let block_end = (runtime + 1..lines.len())
                .find(|&i| {
                    let t = lines[i].trim_start();
                    !t.is_empty() && !t.starts_with('#') && indent(&lines[i]) == 0
                })
                .unwrap_or(lines.len());
            // The document's own indent, at each level: a key indented
            // otherwise would land under the wrong parent.
            let child = |parent: usize, end: usize| {
                (parent + 1..end)
                    .map(|i| lines[i].as_str())
                    .find(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
                    .map(indent)
                    .filter(|&n| n > indent(&lines[parent]))
            };
            let step = child(runtime, block_end).unwrap_or(2);
            if let Some(i) = (runtime + 1..block_end).find(|&i| lines[i].trim() == "android:") {
                let at = child(i, block_end).unwrap_or(indent(&lines[i]) + step);
                lines.insert(i + 1, format!("{}{set}", " ".repeat(at)));
            } else {
                lines.insert(runtime + 1, format!("{}{set}", " ".repeat(2 * step)));
                lines.insert(runtime + 1, format!("{}android:", " ".repeat(step)));
            }
        }
        _ => return Err(format!("more than one availability_mode in:\n{config}")),
    }
    let out = lines.join("\n") + "\n";
    let read = ProfileConfig::parse_yaml(&out)
        .map_err(|e| format!("the changed profile does not parse ({e}):\n{out}"))?;
    if read.runtime.android.availability_mode == mode {
        Ok(out)
    } else {
        Err(format!("the changed profile reads another mode:\n{out}"))
    }
}

/// The app's profile as the phone held it before a case provisioned its
/// own, written back when this is dropped, a failing case's unwind
/// included, and read back to prove it.
///
/// A case provisions a profile derived from this one with its key
/// policy unchanged: the app reseals its identity under the policy its
/// profile names and deletes the other policy's key (ADR-0042 A
/// 2026-10-10, `custody::reseal`), so a profile with another policy
/// would destroy the person's identity setup, and no restore of the
/// file brings that back.
pub struct ProfileKept {
    found: String,
    stay_reachable_chosen: bool,
}

/// `adb shell <args>` with its exit status (the shell protocol carries
/// it, where `exec-in` and `exec-out` do not), stdin fed from `input`.
fn shell_status(args: &[&str], input: Option<&[u8]>) -> std::io::Result<(bool, Vec<u8>)> {
    use std::io::Write as _;
    use std::process::Stdio;
    let mut child = adb()
        .arg("shell")
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let (Some(bytes), Some(mut stdin)) = (input, child.stdin.take()) {
        stdin.write_all(bytes)?;
    }
    let out = child.wait_with_output()?;
    Ok((out.status.success(), out.stdout))
}

/// The app's profile file now, or `None` when it cannot be read.
fn read_config() -> Option<String> {
    match shell_status(&["run-as", PACKAGE, "cat", CONFIG], None) {
        Ok((true, bytes)) if !bytes.is_empty() => {
            Some(String::from_utf8_lossy(&bytes).into_owned())
        }
        _ => None,
    }
}

impl ProfileKept {
    /// The phone's profile now, and whether the person chose to stay
    /// reachable.
    ///
    /// # Panics
    /// If the app holds no profile to keep (a phone with an identity a
    /// person made is set up first), or the overlay cannot be asked.
    #[must_use]
    pub fn take() -> Self {
        let found = read_config().unwrap_or_else(|| {
            panic!("the app holds no profile to keep (a phone with an identity a person made)")
        });
        let (present, _) = shell_status(
            &["run-as", PACKAGE, "test", "-e", AVAILABILITY_OVERLAY],
            None,
        )
        .expect("adb");
        Self {
            found,
            stay_reachable_chosen: present,
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
    /// If the found profile does not parse.
    #[must_use]
    pub fn unlock_policy(&self) -> KeyUnlockPolicy {
        unlock_policy(&self.found).unwrap_or_else(|e| panic!("{e}"))
    }

    /// Whether the person's choice to stay reachable is on: it overrides
    /// any mode a case provisions, so a case that needs
    /// `foreground-only` cannot run on such a phone without changing the
    /// person's choice, which it does not do.
    #[must_use]
    pub fn stay_reachable_chosen(&self) -> bool {
        self.stay_reachable_chosen
    }
}

/// The phone's screen timeout raised while this lives, and the value
/// found put back when it drops. A screen that times out raises a secure
/// keyguard, and a case whose subject is a key a person's unlock would
/// open (the user-presence key: SPIKE-009 opened its window with a
/// keyguard unlock on the A40) cannot ask for that unlock mid-case, so
/// it holds this over its waits instead.
#[must_use = "the timeout is put back when this drops"]
pub struct ScreenAwake {
    /// `settings get system screen_off_timeout` before the hold; `null`
    /// when the phone had no value, which the drop deletes again.
    found: String,
}

impl ScreenAwake {
    /// Ten minutes: longer than any case's own waits.
    pub const HELD_MS: &str = "600000";

    /// Reads the timeout, then raises it.
    ///
    /// # Panics
    /// As [`shell`].
    pub fn hold() -> Self {
        let found = shell(&["settings", "get", "system", "screen_off_timeout"])
            .trim()
            .to_owned();
        shell(&[
            "settings",
            "put",
            "system",
            "screen_off_timeout",
            Self::HELD_MS,
        ]);
        Self { found }
    }
}

impl Drop for ScreenAwake {
    fn drop(&mut self) {
        // Reported, not asserted, as ProfileKept's: no second panic while
        // unwinding; the read-back is what proves the restore.
        let wrote = if self.found == "null" {
            shell_status(
                &["settings", "delete", "system", "screen_off_timeout"],
                None,
            )
        } else {
            shell_status(
                &[
                    "settings",
                    "put",
                    "system",
                    "screen_off_timeout",
                    &self.found,
                ],
                None,
            )
        };
        let now = shell_status(&["settings", "get", "system", "screen_off_timeout"], None)
            .map(|(_, out)| String::from_utf8_lossy(&out).trim().to_owned());
        if now.as_deref().ok() != Some(self.found.as_str()) {
            eprintln!(
                "the phone's screen timeout was NOT put back (write: {wrote:?}, now {now:?}); it was {}",
                self.found
            );
        }
    }
}

impl Drop for ProfileKept {
    fn drop(&mut self) {
        // Never a second panic while unwinding: reported, not asserted.
        let _ = shell_status(&["am", "force-stop", PACKAGE], None);
        // Written aside and moved over the profile, so a failed write
        // leaves the provisioned profile whole rather than a cut one; then
        // read back, since what proves the restore is the file.
        let script = format!("'cat > {CONFIG}.e2e && mv {CONFIG}.e2e {CONFIG}'");
        let wrote = shell_status(
            &["run-as", PACKAGE, "sh", "-c", &script],
            Some(self.found.as_bytes()),
        );
        if read_config().as_deref() != Some(self.found.as_str()) {
            eprintln!(
                "the phone's profile was NOT put back (write: {wrote:?}); it was:\n{}",
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
    /// The A40's (API 30) dump of 2026-11-04, the record's paths and
    /// handles as printed.
    const DESTROYING: &str = "\
ACTIVITY MANAGER SERVICES (dumpsys activity services)
  Destroying services:
  * Destroy ServiceRecord{5c5073d u0 org.interweave.human/.NetworkService}
    intent={cmp=org.interweave.human/.NetworkService}
    packageName=org.interweave.human
    processName=org.interweave.human
    dataDir=/data/user/0/org.interweave.human
    app=ProcessRecord{4815798 14224:org.interweave.human/u0a400}
    allowWhileInUsePermissionInFgs=false
    startForegroundCount=0
    recentCallingPackage=org.interweave.human
    createTime=-5s799ms startingBgTimeout=--
    lastActivity=-5s799ms restartTime=-- createdFromFg=true
    executeNesting=1 executeFg=false executingStart=-23ms
    destroying=true destroyTime=-23ms
";

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
        // A record only under "Destroying services:", as the A40 printed
        // it: not running, and not read as the running record.
        assert_eq!(
            service_record(DESTROYING, NETWORK_SERVICE).expect("parsed"),
            None
        );
        // Control: the same record in the running list is read.
        let live = DESTROYING
            .replace("  Destroying services:", "  User 0 active services:")
            .replace("* Destroy ServiceRecord{", "* ServiceRecord{");
        assert!(
            service_record(&live, NETWORK_SERVICE)
                .expect("parsed")
                .is_some()
        );
        // A running record ends where a sectioned one begins.
        // A running record ends where a sectioned one begins: the
        // destroying record's isForeground is not the bound record's.
        let foreground_after = DESTROYING.replace("startForegroundCount=0", "isForeground=true");
        let both = format!(
            "{BOUND}{}",
            foreground_after
                .lines()
                .skip(1)
                .collect::<Vec<_>>()
                .join("\n")
        );
        let r = service_record(&both, NETWORK_SERVICE)
            .expect("parsed")
            .expect("running");
        assert!(!r.foreground, "{r:?}");
        // Control: neither a record nor (nothing) is no answer at all.
        assert!(service_record("ACTIVITY MANAGER SERVICES\n", NETWORK_SERVICE).is_err());
        assert!(service_record("", NETWORK_SERVICE).is_err());
    }

    #[test]
    fn the_keyguard_is_read_where_the_a40_prints_it_and_nowhere_else() {
        // The A40's policy line, as printed on 2026-11-04.
        let line = |v: &str| {
            format!(
                "    mShowingDream=false mDreamingLockscreen=true    mStatusBar=Window{{587b901 u0 StatusBar}}    isKeyguardShowing={v}\n"
            )
        };
        assert_eq!(keyguard_showing(&line("true")), Ok(true));
        assert_eq!(keyguard_showing(&line("false")), Ok(false));
        // Control: a dump without the value is no answer, never unlocked.
        assert!(keyguard_showing("WINDOW MANAGER POLICY STATE\n").is_err());
        assert!(keyguard_showing(&line("maybe")).is_err());
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

    // The A40's layout (Samsung, API 30), its lines as dumped on
    // 2026-11-04: `fl=` is a bare hex mask. Window #0 is that dump's,
    // verbatim; the app's two are its layout with the masks set by hand,
    // one carrying `FLAG_SECURE` (0x2000) and one not, since the dump was
    // taken with no app window up.
    const A40_WINDOWS: &str = "\
WINDOW MANAGER WINDOWS (dumpsys window windows)
  Window #0 Window{c5413db u0 EdgeLightingWindow}:
    mAttrs={(0,0)(fillxfill) gr=CENTER sim={forwardNavigation} layoutInDisplayCutoutMode=shortEdges ty=2228 fmt=TRANSLUCENT
      fl=1800518
      pfl=12020010
      vsysui=600
      sfl=20000}
  Window #1 Window{d4 u0 org.interweave.human/org.interweave.human.MainActivity}:
    mAttrs={(0,0)(fillxfill) sim={adjust=resize forwardNavigation} ty=1 fmt=TRANSLUCENT
      fl=81812100
      pfl=12020040
      sfl=20000}
  Window #2 Window{e5 u0 org.interweave.human/org.interweave.human.MainActivity}:
    mAttrs={(0,0)(fillxfill) sim={adjust=resize forwardNavigation} ty=1 fmt=TRANSLUCENT
      fl=81810100
      pfl=12020040
      sfl=20000}
";

    #[test]
    fn a_bare_hex_mask_is_read_for_its_secure_bit_as_the_a40_prints_it() {
        let w = app_windows(A40_WINDOWS).expect("parsed");
        let secure: Vec<bool> = w.iter().map(|(_, s)| *s).collect();
        // The positive: 0x81812100 carries 0x2000. The control beside it:
        // 0x81810100, the A40's own value for an ordinary activity, does not.
        assert_eq!(secure, [true, false], "{w:?}");
        assert!(!secure_flag("1800518"));
        // A name is still a name, and `key=` still ends the value.
        assert!(secure_flag("NOT_FOCUSABLE SECURE"));
        assert!(!secure_flag("81810100 pfl=2000"));
    }

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

    // What the debug app writes on first run, `stand_in::document`
    // (crates/human/android-platform/src/stand_in.rs) verbatim: behind a
    // feature this crate does not turn on, so copied. No `android:`.
    const STAND_IN: &str = "schema_version: 2
profile:
  name: human-android
runtime:
  deployment: embedded-android
ipc:
  enabled: false
transport:
  listen:
    addresses: [\"/ip4/0.0.0.0/tcp/0\"]
trust:
  policy: static-allowlist
  allowed_peers: []
endpoints:
  registration_policy: configured-only
  default_direct_endpoint: human
  entries:
    - id: human
      enabled: true
      advertise: false
      allowed_client_kinds: [human-client]
      inbound: inherit_profile_trust
      outbound: inherit_profile_trust
channels:
  desired: []
";

    /// The example as a case provisions it: its placeholders filled.
    fn concrete_example() -> String {
        let peer = interweave_profile_identity::ProfileIdentity::generate()
            .transport_identity()
            .expect("a peer");
        interweave_test_support::e2e::example(
            "human-android.yaml",
            &peer,
            "/ip4/0.0.0.0/tcp/0",
            None,
        )
    }

    /// Every first-run shape a phone may hold: the example as provisioned,
    /// the debug app's stand-in, and profile-config's (`android:` with no
    /// mode); and both as a person might indent them, four spaces a level.
    fn shapes() -> [(&'static str, String); 5] {
        [
            ("example", concrete_example()),
            ("stand-in", STAND_IN.to_owned()),
            (
                "first-run",
                interweave_profile_config::provision::embedded_document("human-android"),
            ),
            (
                "stand-in, four spaces",
                STAND_IN.replace("\n  deployment:", "\n    deployment:"),
            ),
            (
                "first-run, four spaces",
                interweave_profile_config::provision::embedded_document("human-android")
                    .replace("\n  deployment:", "\n    deployment:")
                    .replace("\n  android:", "\n    android:")
                    .replace("\n    endpoint: human", "\n        endpoint: human"),
            ),
        ]
    }

    #[test]
    fn the_key_policy_is_read_as_the_app_reads_it() {
        assert!(
            EXAMPLE.contains("user-presence"),
            "the fixture's trap is there"
        );
        for (name, text) in shapes() {
            assert_eq!(
                unlock_policy(&text),
                Ok(KeyUnlockPolicy::BackgroundCompatible),
                "{name}: a key that names none is the schema's default"
            );
        }
        let presence = concrete_example().replace(
            "key_unlock_policy: background-compatible",
            "key_unlock_policy: user-presence",
        );
        assert_eq!(unlock_policy(&presence), Ok(KeyUnlockPolicy::UserPresence));
        assert!(unlock_policy("not: [a profile").is_err());
    }

    #[test]
    fn availability_is_set_on_every_shape_and_nothing_else_changes() {
        for (name, text) in shapes() {
            for mode in [
                AvailabilityMode::ForegroundOnly,
                AvailabilityMode::StayReachable,
            ] {
                let changed =
                    with_availability(&text, mode).unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(
                    ProfileConfig::parse_yaml(&changed)
                        .expect("a profile")
                        .runtime
                        .android
                        .availability_mode,
                    mode,
                    "{name}"
                );
                // The other lines are the found ones, in their order.
                let kept: Vec<&str> = changed
                    .lines()
                    .filter(|l| {
                        !l.trim_start().starts_with("availability_mode:") && l.trim() != "android:"
                    })
                    .collect();
                let was: Vec<&str> = text
                    .lines()
                    .filter(|l| {
                        !l.trim_start().starts_with("availability_mode:") && l.trim() != "android:"
                    })
                    .collect();
                assert_eq!(kept, was, "{name}");
            }
        }
        let twice = format!(
            "{}  availability_mode: stay-reachable\n",
            concrete_example()
        );
        assert!(with_availability(&twice, AvailabilityMode::ForegroundOnly).is_err());
        assert!(with_availability("x: 1\n", AvailabilityMode::StayReachable).is_err());
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

    #[test]
    fn samsungs_prompt_title_is_its_own_whole_title() {
        // The A40's header line for the prompt, verbatim.
        let a40 = format!(
            "{WINDOWS}  Window #4 Window{{fee1ccf u0 BiometricPrompt3P}}:\n    mAttrs={{fl=SECURE}}\n"
        );
        assert!(has_window(&a40, SAMSUNG_BIOMETRIC_PROMPT));
        // AOSP's title does not match Samsung's: the reason both are asked.
        assert!(!has_window(&a40, BIOMETRIC_PROMPT));
        assert!(
            !has_window(WINDOWS, SAMSUNG_BIOMETRIC_PROMPT),
            "control: absent"
        );
        // The check a case asks sees either title, and neither when absent.
        assert!(prompt_in(&a40));
        let aosp = a40.replace("u0 BiometricPrompt3P}", "u0 BiometricPrompt}");
        assert!(prompt_in(&aosp));
        assert!(!prompt_in(WINDOWS), "control: no prompt");
    }
}
