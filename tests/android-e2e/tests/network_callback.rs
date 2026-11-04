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
//! Device only (`#[ignore]`d). It provisions the phone's own profile as
//! `stay-reachable`, its key policy and peers unchanged, and puts the
//! profile back as found when it ends (`platform::ProfileKept`); it
//! needs a `background-compatible` key, which a start does not wait on a
//! person for. Over USB nothing drops but the radio. Over wireless adb
//! the link drops with the Wi-Fi, and two things follow, both measured
//! on the A40 (API 30) on 2026-11-04:
//! - With USB debugging off, losing the Wi-Fi turns adb off entirely,
//!   and adbd's stop ends every process it started, the detached script
//!   too: its log ended a second after the switch and the Wi-Fi stayed
//!   off. So a wireless run refuses to start unless USB debugging is on
//!   (`adb_enabled`), before anything is changed; with it on, the
//!   script outlives the link (a detached heartbeat survived an
//!   `adb disconnect` on the same phone).
//! - Wireless debugging comes back off, and on a new port when it is
//!   switched on. The script switches it back on (`adb_wifi_enabled`)
//!   after the Wi-Fi returns; the host finds the new port among the
//!   phone's ephemeral ports (`32768..=60999`, the phone's
//!   `ip_local_port_range`), connects there, and follows the phone to it
//!   (`platform::moved_to`), the profile's restore included. A phone
//!   that does not take the switch is turned on by a person (the wait
//!   says so), with no port to type.
//!
//! What this does NOT prove: the loss callback's own line (the app
//! writes none, as the 10-31 record says); the status line a person sees
//! (`Online, limited` there, read by eye); a cellular handover, which a
//! phone with no SIM cannot make; and the dial plan after the change.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::net::{SocketAddr, TcpStream};
use std::process::Command;
use std::time::{Duration, Instant};

use interweave_android_e2e_tests::Device;
use interweave_android_e2e_tests::adb::{AdbDevice, PACKAGE};
use interweave_android_e2e_tests::platform::{self, ProfileKept, Told};
use interweave_profile_config::runtime::{AvailabilityMode, KeyUnlockPolicy};

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

/// Where a wireless-debugging port is taken from: the phone's
/// `ip_local_port_range` (the A40's, read 2026-11-04; Linux's default).
const PORTS: std::ops::RangeInclusive<u16> = 32768..=60999;

/// How long the host waits for adb and the script's end: the script's
/// own run, and a person re-enabling wireless debugging.
const BACK_PATIENCE: Duration = Duration::from_secs(600);

#[test]
#[ignore = "needs a device with the app's androidTest build: ANDROID_SERIAL and INTERWEAVE_ANDROID_INSTRUMENTATION (src/adb.rs); its Wi-Fi is switched off for 30 s"]
fn a_wifi_drop_and_return_reach_the_runtime_in_one_process() {
    let serial = std::env::var("ANDROID_SERIAL").ok();
    let wireless = serial.as_deref().and_then(|s| s.rsplit_once(':'));
    if wireless.is_some() {
        let usb = platform::shell(&["settings", "get", "global", "adb_enabled"]);
        assert_eq!(
            usb.trim(),
            "1",
            "over wireless adb this case needs USB debugging on (Developer options): without it the Wi-Fi drop turns adb off and ends the script that turns the Wi-Fi back on"
        );
    }
    let kept = ProfileKept::take();
    assert_eq!(
        kept.unlock_policy(),
        KeyUnlockPolicy::BackgroundCompatible,
        "the case needs a phone whose key does not wait for the person"
    );
    let mut phone = AdbDevice::connect();
    phone.start(
        &platform::with_availability(kept.found(), AvailabilityMode::StayReachable)
            .unwrap_or_else(|e| panic!("{e}")),
    );
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
        if let Some((host, _)) = wireless {
            // Wireless: the phone where it answers now, its old port first.
            let now = platform::moved().or(serial.clone());
            if !now.as_deref().is_some_and(connected)
                && let Some(found) = find_adb(host)
            {
                eprintln!("network_callback: the phone answers at {found} now");
                platform::moved_to(&found);
            }
            if !asked {
                eprintln!(
                    "PERSON: if wireless debugging is still off a minute after the Wi-Fi returns, turn it on (Developer options); no port to tell"
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
settings put global adb_wifi_enabled 1
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

/// Whether adb reaches the phone at `serial`: connected, and a device.
fn connected(serial: &str) -> bool {
    let _ = Command::new("adb").args(["connect", serial]).output();
    Command::new("adb")
        .args(["-s", serial, "get-state"])
        .output()
        .is_ok_and(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "device")
}

/// The serial wireless debugging answers at on `host` now: each port of
/// [`PORTS`] that accepts a connection, tried with `adb connect`. The
/// ports are tried 512 at a time; a closed one refuses at once.
fn find_adb(host: &str) -> Option<String> {
    let ports: Vec<u16> = PORTS.collect();
    let open: Vec<u16> = ports
        .chunks(512)
        .flat_map(|chunk| {
            let probes: Vec<_> = chunk
                .iter()
                .map(|&port| {
                    let host = host.to_owned();
                    std::thread::spawn(move || {
                        let addr: SocketAddr = format!("{host}:{port}").parse().ok()?;
                        TcpStream::connect_timeout(&addr, Duration::from_millis(800))
                            .ok()
                            .map(|_| port)
                    })
                })
                .collect();
            probes
                .into_iter()
                .filter_map(|p| p.join().ok().flatten())
                .collect::<Vec<_>>()
        })
        .collect();
    open.into_iter()
        .map(|port| format!("{host}:{port}"))
        .find(|s| connected(s))
}

fn done() -> bool {
    platform::adb()
        .args(["shell", "cat", &format!("{DIR}/done")])
        .output()
        .is_ok_and(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "done")
}

fn pull(path: &str) -> Vec<u8> {
    let out = platform::adb()
        .args(["exec-out", "cat", path])
        .output()
        .expect("adb");
    assert!(out.status.success(), "reading {path}");
    out.stdout
}

fn adb_ok(args: &[&str]) {
    let out = platform::adb().args(args).output().expect("adb");
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
