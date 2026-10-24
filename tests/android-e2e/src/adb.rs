// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The Android side on a real device, driven over `adb` (architect-cto's
//! DECISION of 2026-10-10): the case runs in the app's instrumentation,
//! in the app's own process, over the binding of the runtime the app's
//! service started; this host drives the lifecycle and reads the result
//! back from the instrumentation's status (the seam agreed with
//! rust-ui-dev, 01a12771).
//!
//! What a device run is, so its evidence is read for what it is:
//! - **Every case is a fresh process.** `am instrument` starts the app's
//!   process for the run and the process ends with it, so the runtime
//!   starts for each case that needs one -- alone for
//!   [`cases::NEED_A_RUNTIME`], the app's service in full for
//!   [`cases::NEED_THE_CLIENT`].
//!   [`restart`](AdbDevice::restart) is therefore the next case, and a
//!   stop is the platform's force-stop; the app's graceful stop is the
//!   app's own platform test, not this.
//! - **The network is adb's tunnel.** The relay and the desktop listen on
//!   this host's loopback, and [`reach`](AdbDevice::reach) reverses each
//!   port, so the phone's sockets reach the host over the USB/TCP link
//!   adb holds, at the same loopback address -- real sockets on the
//!   phone's runtime, not a radio network, no NAT. A case that reverses
//!   nothing reaches the desktop over the phone's Wi-Fi instead
//!   ([`crate::wifi`]); adb then carries the instrumentation alone.
//! - **The identity must outlive the process.** A profile's `PeerId` is
//!   known before the first start and kept across restarts;
//!   [`AdbDevice::connect`] refuses a build whose identity changes across
//!   a force-stop rather than letting a case read that as a network fault.
//!
//! `ANDROID_SERIAL` selects the device as for `adb` itself;
//! `INTERWEAVE_ANDROID_INSTRUMENTATION` names the instrumentation
//! component (`<test package>/<runner>`), which the app's androidTest
//! build defines.

use std::fmt::Write as _;
use std::process::{Command, Output};

use interweave_android_e2e_cases::{cases, keys};
use interweave_transport_api::TransportIdentity;
use serde_json::{Map, Value};

use crate::{CaseRun, Device};

/// The app under test.
pub const PACKAGE: &str = "org.interweave.human";

/// The environment variable naming the instrumentation component.
pub const INSTRUMENTATION_ENV: &str = "INTERWEAVE_ANDROID_INSTRUMENTATION";

/// The prefix the instrumentation gives each result key in its status.
pub const STATUS_PREFIX: &str = "interweave.";

/// The Android side on a device.
pub struct AdbDevice {
    instrumentation: String,
    peer: TransportIdentity,
    reversed: Vec<u16>,
}

impl AdbDevice {
    /// The device `adb` selects, its app's identity read and checked to
    /// survive a force-stop.
    ///
    /// # Panics
    /// With no instrumentation named, no device, a failing `identity`
    /// case, or an identity that changed across the force-stop.
    #[must_use]
    pub fn connect() -> Self {
        let instrumentation = std::env::var(INSTRUMENTATION_ENV).unwrap_or_else(|_| {
            panic!("{INSTRUMENTATION_ENV} names no instrumentation: a device run needs the app's androidTest build installed")
        });
        let identity = |instrumentation: &str| {
            let out = parse(&instrument(
                instrumentation,
                cases::IDENTITY,
                &Value::Object(Map::new()),
            ));
            assert_eq!(
                out.get(keys::RESULT).and_then(Value::as_str),
                Some(keys::PASS),
                "the identity case: {out:?}\n{}",
                logcat()
            );
            out.get(keys::PEER)
                .and_then(Value::as_str)
                .and_then(|p| TransportIdentity::parse(p).ok())
                .unwrap_or_else(|| panic!("the identity case answered no PeerId: {out:?}"))
        };
        let peer = identity(&instrumentation);
        force_stop();
        assert_eq!(
            identity(&instrumentation),
            peer,
            "the app's PeerId changed across a force-stop: this build has no persistent identity, and no case can name it in advance"
        );
        Self {
            instrumentation,
            peer,
            reversed: Vec::new(),
        }
    }
}

/// How many of the app's own log lines a failure carries.
const APP_LOG_LINES: usize = 1000;

/// How many of the platform's lines about the app a failure carries.
const PLATFORM_LINES: usize = 50;

/// The device's log since the last clear, as a failure needs it: the
/// lines of the app's processes, then `ActivityManager`'s lines naming
/// the app -- why its process started or died. Not the device's last
/// lines: a phone writes thousands a minute, and a run on a Pixel 9a
/// carried none of the app's in its last 300.
///
/// BY PROCESS, from `ActivityManager`'s `Start proc` lines, and filtered
/// here: on the 2026-10-20 Pixel run `logcat --uid` answered nothing
/// while the app's processes were logging
/// (`tests/android-e2e/runs/2026-10-20-pixel-9a-api37.md`, finding 1),
/// and logcat documents the option as of use to the root, log and system
/// users, which adb's shell is not.
fn logcat() -> String {
    let manager = text(&["logcat", "-d", "-b", "system", "-s", "ActivityManager"]);
    let app = match &manager {
        Err(why) => why.clone(),
        Ok(manager) => {
            let pids = app_pids(manager);
            if pids.is_empty() {
                format!("(no process of {PACKAGE} started since the log was cleared)")
            } else {
                text(&["logcat", "-d", "-v", "threadtime"])
                    .map_or_else(|why| why, |log| lines_of(&pids, &log))
            }
        }
    };
    let platform = manager.map_or_else(|why| why, |log| about_the_app(&log));
    format!("--- the app's log ---\n{app}\n--- ActivityManager on the app ---\n{platform}")
}

/// The pids of the app's processes, from `ActivityManager`'s lines
/// `Start proc <pid>:<PACKAGE>/<user>` (an instrumentation runs in the
/// app's own process, so this is its pid too).
fn app_pids(manager: &str) -> Vec<String> {
    let process = format!(":{PACKAGE}/");
    manager
        .lines()
        .filter_map(|line| {
            let (pid, rest) = line.split_once("Start proc ")?.1.split_once(':')?;
            rest.starts_with(&process[1..])
                .then(|| pid.trim().to_owned())
                .filter(|pid| pid.bytes().all(|b| b.is_ascii_digit()) && !pid.is_empty())
        })
        .collect()
}

/// The last [`APP_LOG_LINES`] of a `threadtime` log whose pid, the third
/// field, is one of `pids`, after a line saying how many earlier ones
/// were cut, when any were: a reader must not take a cut log's first line
/// for the run's first (`a_cut_app_log_says_so`).
fn lines_of(pids: &[String], log: &str) -> String {
    let lines: Vec<&str> = log
        .lines()
        .filter(|line| {
            line.split_whitespace()
                .nth(2)
                .is_some_and(|pid| pids.iter().any(|p| p == pid))
        })
        .collect();
    let cut = lines.len().saturating_sub(APP_LOG_LINES);
    let kept = lines[cut..].join("\n");
    if cut == 0 {
        kept
    } else {
        format!("({cut} {CUT_LINES})\n{kept}")
    }
}

/// What [`lines_of`] says when it cut the front of the app's log.
pub const CUT_LINES: &str = "earlier lines of the app's log cut";

/// `adb args`'s stdout, or why there is none: a failure report that
/// cannot read the log says why, never an empty section or a guess.
fn text(args: &[&str]) -> Result<String, String> {
    read_text(args, adb(args))
}

/// [`text`] over an `adb` run's result: adb exits nonzero for an
/// unauthorized or absent device and a refused option, with the reason
/// on stderr (`a_failed_read_says_why`).
fn read_text(args: &[&str], run: std::io::Result<Output>) -> Result<String, String> {
    match run {
        Err(e) => Err(format!("(adb {}: {e})", args.join(" "))),
        Ok(out) if !out.status.success() => Err(format!(
            "(adb {} ended with {}: {})",
            args.join(" "),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim_end()
        )),
        Ok(out) => Ok(String::from_utf8_lossy(&out.stdout).into_owned()),
    }
}

/// The last of `ActivityManager`'s lines that name the app.
fn about_the_app(log: &str) -> String {
    let lines: Vec<&str> = log.lines().filter(|l| l.contains(PACKAGE)).collect();
    lines[lines.len().saturating_sub(PLATFORM_LINES)..].join("\n")
}

impl Device for AdbDevice {
    fn peer(&self) -> TransportIdentity {
        self.peer.clone()
    }

    fn reach(&mut self, port: u16) {
        let spec = format!("tcp:{port}");
        run_ok(&["reverse", &spec, &spec]);
        self.reversed.push(port);
    }

    /// Provisions `config`, after clearing the device's log: what a case
    /// reads from logcat is then this run's, never an earlier one's.
    fn start(&mut self, config: &str) {
        run_ok(&["logcat", "-c"]);
        let out = parse(&instrument(
            &self.instrumentation,
            cases::PROVISION,
            &serde_json::json!({ "config": config }),
        ));
        assert_eq!(
            out.get(keys::RESULT).and_then(Value::as_str),
            Some(keys::PASS),
            "provisioning the app: {out:?}\n{}",
            logcat()
        );
    }

    fn stop(&mut self) {
        force_stop();
    }

    fn fresh_log(&mut self) {
        run_ok(&["logcat", "-c"]);
    }

    fn kill(&mut self) {
        force_stop();
    }

    /// Nothing to do: the next case's instrumentation starts the process.
    fn restart(&mut self) {}

    fn log(&self) -> String {
        logcat()
    }

    /// Read through `run-as`, as the app: what a case wrote is under the
    /// app's private data directory, which `run-as` opens to the shell
    /// for a debuggable build -- the androidTest build's app is one.
    fn pull(&self, path: &str) -> Vec<u8> {
        let out = adb(&["exec-out", "run-as", PACKAGE, "cat", path]).expect("adb");
        assert!(
            out.status.success() && out.stderr.is_empty(),
            "reading {path} as the app: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        out.stdout
    }

    fn run_case(&self, case: &str, args: &Value) -> CaseRun {
        let (instrumentation, name, args) =
            (self.instrumentation.clone(), case.to_owned(), args.clone());
        CaseRun::on_thread(case, move || {
            Value::Object(parse(&instrument(&instrumentation, &name, &args))).to_string()
        })
    }
}

impl Drop for AdbDevice {
    fn drop(&mut self) {
        // A test leaves nothing behind it did not find: the reversals go,
        // and the app does not keep running a case's profile.
        for port in &self.reversed {
            let _ = adb(&["reverse", "--remove", &format!("tcp:{port}")]);
        }
        force_stop();
    }
}

fn adb(args: &[&str]) -> std::io::Result<Output> {
    Command::new("adb").args(args).output()
}

fn run_ok(args: &[&str]) {
    let out = adb(args).expect("adb");
    assert!(
        out.status.success(),
        "adb {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn force_stop() {
    let _ = adb(&["shell", "am", "force-stop", PACKAGE]);
}

/// `am instrument`'s output for `case` with `args`.
fn instrument(instrumentation: &str, case: &str, args: &Value) -> String {
    let command = instrument_argv(instrumentation, case, args);
    let command: Vec<&str> = command.iter().map(String::as_str).collect();
    match adb(&command) {
        Ok(out) => outcome(&out),
        Err(e) => format!("INSTRUMENTATION_FAILED: adb: {e}"),
    }
}

/// What a run printed, and -- when `adb` itself failed (no device, more
/// than one, an unauthorized one), which it says on stderr with nothing
/// on stdout -- its exit and what it said, so a result-less run's detail
/// names the cause.
fn outcome(out: &Output) -> String {
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    if !out.status.success() || !out.stderr.is_empty() {
        let _ = write!(
            text,
            "\nINSTRUMENTATION_FAILED: adb ended with {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim_end()
        );
    }
    text
}

/// The `adb` arguments that run `case`: the arguments travel base64, so
/// no shell between here and the runner can split or expand them.
#[must_use]
pub fn instrument_argv(instrumentation: &str, case: &str, args: &Value) -> Vec<String> {
    [
        "shell",
        "am",
        "instrument",
        "-w",
        "-r",
        "-e",
        "case",
        case,
        "-e",
        "args",
        &base64(args.to_string().as_bytes()),
        instrumentation,
    ]
    .map(str::to_owned)
    .to_vec()
}

/// The result a run's raw output (`am instrument -r`) carries: each
/// `INSTRUMENTATION_STATUS: interweave.<key>=<value>` as `<key>`, a
/// value running on over the lines up to the next `INSTRUMENTATION_` line.
/// A run that reported no result -- a crash, a missing runner -- is a
/// failure whose detail is the whole output, so nothing about why is
/// lost.
#[must_use]
pub fn parse(output: &str) -> Map<String, Value> {
    let mut out = Map::new();
    let mut current: Option<(String, String)> = None;
    let flush = |current: &mut Option<(String, String)>, out: &mut Map<String, Value>| {
        if let Some((key, value)) = current.take() {
            out.insert(key, value.into());
        }
    };
    for line in output.lines() {
        if line.starts_with("INSTRUMENTATION_") {
            flush(&mut current, &mut out);
            if let Some(rest) = line.strip_prefix("INSTRUMENTATION_STATUS: ")
                && let Some((key, value)) = rest.split_once('=')
                && let Some(key) = key.strip_prefix(STATUS_PREFIX)
            {
                current = Some((key.to_owned(), value.to_owned()));
            }
        } else if let Some((_, value)) = current.as_mut() {
            value.push('\n');
            value.push_str(line);
        }
    }
    flush(&mut current, &mut out);
    if !out.contains_key(keys::RESULT) {
        out.insert(keys::RESULT.to_owned(), keys::FAIL.into());
        out.insert(
            keys::DETAIL.to_owned(),
            format!("the instrumentation reported no result:\n{output}").into(),
        );
    }
    out
}

/// RFC 4648 base64, padded: what `android.util.Base64.decode(_, DEFAULT)`
/// reads back.
#[must_use]
pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 0x3f) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The app's lines are filtered by pid and capped at the last
    /// [`APP_LOG_LINES`]; past the cap the dump says how many it cut, so
    /// its first line is not taken for the run's first. The control: at
    /// the cap exactly, nothing is said.
    #[test]
    fn a_cut_app_log_says_so() {
        let line = |pid: &str, n: usize| format!("10-21 20:00:00.000 {pid} {pid} D tag: line {n}");
        let log = |count: usize| {
            (0..count)
                .flat_map(|n| [line("4242", n), line("7", n)])
                .collect::<Vec<_>>()
                .join("\n")
        };
        let pids = ["4242".to_owned()];
        let whole = lines_of(&pids, &log(APP_LOG_LINES));
        assert!(!whole.contains(CUT_LINES), "nothing cut at the cap");
        assert_eq!(whole.lines().count(), APP_LOG_LINES);
        assert!(
            whole.lines().all(|l| l.contains(" 4242 ")),
            "the app's pid only"
        );
        let cut = lines_of(&pids, &log(APP_LOG_LINES + 3));
        assert_eq!(
            cut.lines().next(),
            Some(format!("(3 {CUT_LINES})").as_str())
        );
        assert!(
            cut.lines().nth(1).is_some_and(|l| l.ends_with("line 3")),
            "{cut}"
        );
    }

    /// A nonzero exit is the reason, stderr included; the control, a
    /// success, is its stdout.
    #[test]
    fn a_failed_read_says_why() {
        use std::os::unix::process::ExitStatusExt;
        let run = |code: i32, stdout: &str, stderr: &str| {
            Ok(Output {
                status: std::process::ExitStatus::from_raw(code << 8),
                stdout: stdout.into(),
                stderr: stderr.into(),
            })
        };
        let why = read_text(&["logcat", "-d"], run(1, "", "error: device unauthorized"))
            .expect_err("a nonzero exit is no log");
        assert!(why.contains("adb logcat -d"), "{why}");
        assert!(why.contains("device unauthorized"), "{why}");
        assert_eq!(
            read_text(&["logcat"], run(0, "the log", "")),
            Ok("the log".to_owned())
        );
    }

    /// The app's lines are found by its processes' pids, as
    /// `ActivityManager` names them, and only its: another app's process,
    /// a package whose name merely starts with the app's, and a line of
    /// another pid that contains the app's pid elsewhere are left out.
    #[test]
    fn the_app_log_is_the_lines_of_the_processes_started_for_it() {
        let manager = format!(
            "10-20 18:05:36.4  1492  1548 I ActivityManager: Start proc 30252:{PACKAGE}/u0a271 for added application {PACKAGE}\n\
             10-20 18:05:36.5  1492  1548 I ActivityManager: Start proc 777:com.other/u0a1 for service\n\
             10-20 18:05:36.6  1492  1548 I ActivityManager: Start proc 778:{PACKAGE}e/u0a9 for service\n\
             10-20 18:05:43.0  1492  1548 I ActivityManager: Start proc 30361:{PACKAGE}/u0a271 for added application {PACKAGE}\n"
        );
        let pids = app_pids(&manager);
        assert_eq!(pids, ["30252", "30361"]);
        let log = "10-20 18:05:43.388 30361 30378 D interweave: peer connected path=relayed\n\
                   10-20 18:05:43.390   777   780 I other: pid 30361 is not mine\n\
                   10-20 18:05:43.391   778   781 I humane: a look-alike\n\
                   10-20 18:05:36.900 30252 30252 D interweave: started\n";
        let kept = lines_of(&pids, log);
        assert_eq!(
            kept.lines().collect::<Vec<_>>(),
            [
                "10-20 18:05:43.388 30361 30378 D interweave: peer connected path=relayed",
                "10-20 18:05:36.900 30252 30252 D interweave: started"
            ]
        );
        assert!(app_pids("").is_empty());
    }

    #[test]
    fn the_app_log_keeps_its_last_lines() {
        let pids = ["1".to_owned()];
        let log: Vec<String> = (0..APP_LOG_LINES + 3)
            .map(|n| format!("10-20 00:00:00.0 1 1 D interweave: line {n}"))
            .collect();
        let kept = lines_of(&pids, &log.join("\n"));
        // The last APP_LOG_LINES, after the line saying three were cut.
        assert_eq!(kept.lines().count(), APP_LOG_LINES + 1);
        assert!(kept.starts_with(&format!("(3 {CUT_LINES})")), "{kept}");
        assert!(kept.ends_with(&log[APP_LOG_LINES + 2]), "{kept}");
    }

    #[test]
    fn the_platform_lines_are_the_last_naming_the_app() {
        let mut log: Vec<String> = (0..PLATFORM_LINES + 5)
            .map(|n| format!("I ActivityManager: Start proc {n}:{PACKAGE}/u0a271"))
            .collect();
        log.insert(
            3,
            "I ActivityManager: Start proc 9:com.other/u0a1".to_owned(),
        );
        let kept = about_the_app(&log.join("\n"));
        assert_eq!(kept.lines().count(), PLATFORM_LINES);
        assert!(!kept.contains("com.other"));
        assert!(kept.ends_with(&log[PLATFORM_LINES + 5]), "{kept}");
    }

    #[test]
    fn base64_matches_rfc_4648s_vectors() {
        for (plain, encoded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(plain.as_bytes()), encoded, "{plain:?}");
        }
        // Every byte value, so the alphabet's last two and the high bits
        // are read: 0xfb 0xff -> "+/8=".
        assert_eq!(base64(&[0xfb, 0xff]), "+/8=");
    }

    #[test]
    fn the_case_and_its_arguments_reach_the_runner_as_single_words() {
        let hostile = serde_json::json!({ "config": "a b\n'c' $HOME \"d\"" });
        let argv = instrument_argv("org.interweave.human.test/Runner", "provision", &hostile);
        assert_eq!(
            &argv[..8],
            [
                "shell",
                "am",
                "instrument",
                "-w",
                "-r",
                "-e",
                "case",
                "provision"
            ]
        );
        assert_eq!(argv[8], "-e");
        assert_eq!(argv[9], "args");
        assert!(
            argv[10]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b)),
            "nothing a shell would split or expand: {}",
            argv[10]
        );
        assert_eq!(argv[10], base64(hostile.to_string().as_bytes()));
        assert_eq!(argv[11], "org.interweave.human.test/Runner");
        assert_eq!(argv.len(), 12);
    }

    #[test]
    fn a_status_is_read_by_its_interweave_keys_and_a_value_may_run_on() {
        let raw = "\
INSTRUMENTATION_STATUS: interweave.case=paths
INSTRUMENTATION_STATUS: interweave.result=fail
INSTRUMENTATION_STATUS: interweave.detail=no route to 12D3Koo within the deadline
second line of the detail
INSTRUMENTATION_STATUS: class=org.interweave.human.test.E2eCase
INSTRUMENTATION_STATUS_CODE: 0
INSTRUMENTATION_RESULT: stream=
INSTRUMENTATION_CODE: -1
";
        let out = parse(raw);
        assert_eq!(out[keys::CASE], "paths");
        assert_eq!(out[keys::RESULT], keys::FAIL);
        assert_eq!(
            out[keys::DETAIL],
            "no route to 12D3Koo within the deadline\nsecond line of the detail"
        );
        assert!(
            !out.contains_key("class"),
            "only interweave.* keys: {out:?}"
        );
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn a_pass_is_read_as_one() {
        let out = parse(
            "INSTRUMENTATION_STATUS: interweave.result=pass\nINSTRUMENTATION_STATUS: interweave.path=relayed\nINSTRUMENTATION_CODE: -1\n",
        );
        assert_eq!(out[keys::RESULT], keys::PASS);
        assert_eq!(out[keys::PATH], "relayed");
    }

    #[test]
    fn when_adb_itself_fails_the_detail_carries_its_exit_and_what_it_said() {
        use std::os::unix::process::ExitStatusExt as _;
        let out = Output {
            // Exit code 1, as a wait status.
            status: std::process::ExitStatus::from_raw(1 << 8),
            stdout: Vec::new(),
            stderr: b"adb: no devices/emulators found\n".to_vec(),
        };
        let result = parse(&outcome(&out));
        assert_eq!(result[keys::RESULT], keys::FAIL);
        let detail = result[keys::DETAIL].as_str().expect("a detail");
        assert!(
            detail.contains("no devices/emulators found") && detail.contains("exit status: 1"),
            "{detail}"
        );
        // A run that succeeded and said nothing on stderr is passed
        // through as it printed.
        let ok = Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: b"INSTRUMENTATION_STATUS: interweave.result=pass\n".to_vec(),
            stderr: Vec::new(),
        };
        assert_eq!(
            outcome(&ok),
            "INSTRUMENTATION_STATUS: interweave.result=pass\n"
        );
    }

    #[test]
    fn a_run_that_reported_no_result_fails_with_all_it_printed() {
        let raw = "INSTRUMENTATION_RESULT: shortMsg=Process crashed.\nINSTRUMENTATION_CODE: 0\n";
        let out = parse(raw);
        assert_eq!(out[keys::RESULT], keys::FAIL);
        let detail = out[keys::DETAIL].as_str().expect("a detail");
        assert!(detail.contains("Process crashed."), "{detail}");
    }
}
