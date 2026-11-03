// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Plan §20's network-callback Platform-tests line as an instrumented
//! case: Stage 17 met it by record and carried the case to the owner's
//! release gate, since the Wi-Fi drop takes wireless adb with it. Here
//! the drop is made by a script on the phone, started detached
//! (`setsid nohup`) so that it outlives the adb link, as
//! `device-runs/2026-10-31-pixel-9a.md` made it by hand. The script
//! writes its own markers into the phone's log around each switch, and
//! the host reads that log after adb is back.
//!
//! Asserted, from the app's own log line (`platform::told`): after the
//! Wi-Fi goes off, the runtime is told of a set with no address on a
//! local network; after it comes back, of a set with one. Both reports
//! fall between their markers. The app runs in one process throughout,
//! read before, during and after. The positive control is the report
//! after the return: a log that never carried the line would fail there,
//! not pass the drop for free.
//!
//! Device only (`#[ignore]`d). It writes the app's profile, as
//! `app_lifecycle.rs` does. Over wireless adb the link drops with the
//! Wi-Fi; the host waits for it, and on a phone that does not re-enable
//! wireless debugging on its own a person turns it back on (the wait
//! says so). Over USB nothing drops but the radio.
//!
//! What this does NOT prove: the loss callback's own line (the app
//! writes none, as the 10-31 record says); the status line a person sees
//! (`Online, limited` there, read by eye); a cellular handover, which a
//! phone with no SIM cannot make; and the dial plan after the change.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::process::Command;
use std::time::{Duration, Instant};

use interweave_android_e2e_tests::Device;
use interweave_android_e2e_tests::adb::{AdbDevice, PACKAGE};
use interweave_android_e2e_tests::platform::{self, Told};
use interweave_profile_identity::ProfileIdentity;
use interweave_test_support::e2e;

/// Where the script and what it writes live on the phone.
const DIR: &str = "/data/local/tmp/interweave-net-drop";

/// The script's log tag and its three markers.
const TAG: &str = "interweave-e2e";
const OFF: &str = "net-drop wifi-off";
const ON: &str = "net-drop wifi-on";
const DONE: &str = "net-drop done";

/// How long the Wi-Fi stays off, and how long the script watches after
/// it is back: the 10-31 record saw both reports within seconds.
const OFF_FOR: u32 = 30;
const ON_FOR: u32 = 60;

/// How long the host waits for adb and the script's end: the script's
/// own run, and a person re-enabling wireless debugging.
const BACK_PATIENCE: Duration = Duration::from_secs(600);

#[test]
#[ignore = "needs a device with the app's androidTest build: ANDROID_SERIAL and INTERWEAVE_ANDROID_INSTRUMENTATION (src/adb.rs); its Wi-Fi is switched off for 30 s"]
fn a_wifi_drop_and_return_reach_the_runtime_in_one_process() {
    let mut phone = AdbDevice::connect();
    let stranger = ProfileIdentity::generate()
        .transport_identity()
        .expect("a peer");
    phone.start(&e2e::example(
        "human-android.yaml",
        &stranger,
        "/ip4/0.0.0.0/tcp/0",
        None,
    ));
    let serial = std::env::var("ANDROID_SERIAL").ok();

    platform::shell(&["rm", "-rf", DIR]);
    platform::shell(&["mkdir", "-p", DIR]);
    let script = format!("{DIR}/run.sh");
    let local = tempfile::NamedTempFile::new().expect("a scratch file");
    std::fs::write(local.path(), script_text()).expect("the script");
    adb_ok(&["push", local.path().to_str().expect("a path"), &script]);
    // Detached: the shell adb runs it in ends with the link.
    platform::shell(&[
        "setsid",
        "nohup",
        "sh",
        &script,
        "</dev/null",
        ">/dev/null",
        "2>&1",
        "&",
    ]);

    let deadline = Instant::now() + BACK_PATIENCE;
    let mut asked = false;
    loop {
        if done() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the script's end not read in {BACK_PATIENCE:?}: adb not back, or the script stopped"
        );
        if let Some(s) = serial.as_deref().filter(|s| s.contains(':')) {
            // Wireless: ask adb to reach the phone again.
            let _ = Command::new("adb").args(["connect", s]).output();
            if !asked {
                eprintln!(
                    "network_callback: if wireless debugging does not come back on its own after the Wi-Fi returns, turn it on (the same port)"
                );
                asked = true;
            }
        }
        std::thread::sleep(Duration::from_secs(5));
    }

    let log = String::from_utf8_lossy(&pull(&format!("{DIR}/logcat.txt"))).into_owned();
    let pids: Vec<String> = ["before", "during", "after"]
        .iter()
        .map(|w| {
            String::from_utf8_lossy(&pull(&format!("{DIR}/pid-{w}")))
                .trim()
                .to_owned()
        })
        .collect();
    platform::shell(&["rm", "-rf", DIR]);

    assert!(
        !pids[0].is_empty() && pids.iter().all(|p| *p == pids[0]),
        "one process throughout (before, during, after): {pids:?}"
    );
    let (drop, back) = phases(&log).unwrap_or_else(|e| panic!("{e}\n{log}"));
    assert!(
        drop.iter().any(|t| t.on_local() == 0),
        "the drop told: no address on a local network after {OFF}: {drop:?}\n{log}"
    );
    assert!(
        back.iter().any(|t| t.on_local() >= 1),
        "control, the return told: an address on a local network after {ON}: {back:?}\n{log}"
    );
    eprintln!(
        "network_callback: drop {drop:?}, return {back:?}, process {}",
        pids[0]
    );
    phone.stop();
}

/// The phone's script: the app in front, then the Wi-Fi off and on with
/// a marker before each switch, the app's process read at each step,
/// and the log kept throughout. It turns the Wi-Fi back on whatever the
/// host does meanwhile.
fn script_text() -> String {
    format!(
        "\
out={DIR}
logcat -c
logcat -v time > $out/logcat.txt &
lp=$!
am start -W -n {PACKAGE}/.MainActivity >/dev/null
sleep 5
pidof {PACKAGE} > $out/pid-before
log -t {TAG} '{OFF}'
cmd wifi set-wifi-enabled disabled || svc wifi disable
sleep {OFF_FOR}
pidof {PACKAGE} > $out/pid-during
log -t {TAG} '{ON}'
cmd wifi set-wifi-enabled enabled || svc wifi enable
sleep {ON_FOR}
pidof {PACKAGE} > $out/pid-after
log -t {TAG} '{DONE}'
sleep 1
kill $lp
echo done > $out/done
"
    )
}

/// The reports between the off marker and the on marker, and between the
/// on marker and the end; an error when a marker is missing, so a log cut
/// short never reads as no report.
fn phases(log: &str) -> Result<(Vec<Told>, Vec<Told>), String> {
    let at = |m: &str| {
        log.lines()
            .position(|l| l.contains(TAG) && l.contains(m))
            .ok_or_else(|| format!("no marker {m:?} in the phone's log"))
    };
    let (off, on, end) = (at(OFF)?, at(ON)?, at(DONE)?);
    let between = |a: usize, b: usize| -> Vec<Told> {
        log.lines()
            .skip(a + 1)
            .take(b.saturating_sub(a + 1))
            .filter_map(platform::told)
            .collect()
    };
    Ok((between(off, on), between(on, end)))
}

fn done() -> bool {
    Command::new("adb")
        .args(["shell", "cat", &format!("{DIR}/done")])
        .output()
        .is_ok_and(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "done")
}

fn pull(path: &str) -> Vec<u8> {
    let out = Command::new("adb")
        .args(["exec-out", "cat", path])
        .output()
        .expect("adb");
    assert!(out.status.success(), "reading {path}");
    out.stdout
}

fn adb_ok(args: &[&str]) {
    let out = Command::new("adb").args(args).output().expect("adb");
    assert!(
        out.status.success(),
        "adb {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn the_phases_are_read_between_the_markers_and_a_missing_one_is_an_error() {
    let line = |n: u32, off: u32| {
        format!(
            "I/interweave: the runtime was told the networks: {n} addresses ({off} on no local network), 2 resolvers, strict Private DNS off"
        )
    };
    let log = [
        line(2, 0),
        format!("I/{TAG}: {OFF}"),
        line(0, 0),
        format!("I/{TAG}: {ON}"),
        line(2, 0),
        format!("I/{TAG}: {DONE}"),
        line(9, 0),
    ]
    .join("\n");
    let (drop, back) = phases(&log).expect("all three markers");
    assert_eq!(
        drop,
        vec![Told {
            addresses: 0,
            off_local: 0
        }]
    );
    assert_eq!(
        back,
        vec![Told {
            addresses: 2,
            off_local: 0
        }]
    );
    let cut = log.replace(DONE, "cut");
    assert!(phases(&cut).is_err(), "a log cut short is no reading");
}
