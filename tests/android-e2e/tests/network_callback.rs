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
//! person for. Over USB the link stays up, but the Wi-Fi cycle can still
//! take wireless debugging's listener with it (below). Over wireless adb
//! the link drops with the Wi-Fi, and two things follow, both measured
//! on the A40 (API 30) on 2026-11-04. A run is wireless when adb lists
//! the phone with no `usb:` path (`attachment`), an mDNS serial with no
//! `:` in it included.
//! - With USB debugging off, losing the Wi-Fi turns adb off entirely,
//!   and adbd's stop ends every process it started, the detached script
//!   too: its log ended a second after the switch and the Wi-Fi stayed
//!   off. So a wireless run refuses to start unless USB debugging is on
//!   (`adb_enabled`), before anything is changed; with it on, the
//!   script outlives the link (a detached heartbeat survived an
//!   `adb disconnect` on the same phone).
//! - Wireless debugging comes back off, and on a new port when it is
//!   switched on. The host reads `adb_wifi_enabled` before anything is
//!   changed, the script writes that value back after the Wi-Fi returns
//!   (so a USB run on a phone with wireless debugging off leaves it off),
//!   and the host puts it back again however the case ends
//!   (`AdbWifiKept`). The host finds the new port among the phone's
//!   ephemeral ports (`32768..=60999`, the phone's
//!   `ip_local_port_range`), connects there, and follows the phone to it
//!   (`platform::moved_to`), the profile's restore included. On the A40
//!   the write did not bring the port back (the 2026-11-05 run record):
//!   a person switches it on (the wait says so), with no port to type.
//!
//! What this does NOT prove: the loss callback's own line (the app
//! writes none, as the 10-31 record says); the status line a person sees
//! (`Online, limited` there, read by eye); a cellular handover, which a
//! phone with no SIM cannot make; and the dial plan after the change.

#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]

use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
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

/// How long the script waits for the app's process after starting it.
const START_FOR: u32 = 30;

/// Where a wireless-debugging port is taken from: the phone's
/// `ip_local_port_range` (the A40's, read 2026-11-04; Linux's default).
const PORTS: std::ops::RangeInclusive<u16> = 32768..=60999;

/// How long the host waits for adb and the script's end: the script's
/// own run, and a person re-enabling wireless debugging.
const BACK_PATIENCE: Duration = Duration::from_secs(600);

#[test]
#[ignore = "needs a device with the app's androidTest build: ANDROID_SERIAL and INTERWEAVE_ANDROID_INSTRUMENTATION (src/adb.rs); its Wi-Fi is switched off for 30 s"]
fn a_wifi_drop_and_return_reach_the_runtime_in_one_process() {
    let (serial, link) = attachment(
        &adb_text(&["devices", "-l"]),
        std::env::var("ANDROID_SERIAL").ok().as_deref(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    // The host the phone is followed to when wireless debugging moves,
    // learned before anything is changed: from an `ip:port` serial, or
    // for an mDNS serial from adb's own list of discovered services.
    let wireless = match link {
        Link::Usb => None,
        Link::Network => Some(
            host_of(&serial)
                .or_else(|| mdns_host(&adb_text(&["mdns", "services"]), &serial))
                .unwrap_or_else(|| {
                    panic!("over wireless adb at {serial}: no address to follow the phone to, neither in the serial nor in `adb mdns services`")
                }),
        ),
    };
    if wireless.is_some() {
        let usb = platform::shell(&["settings", "get", "global", "adb_enabled"]);
        assert_eq!(
            usb.trim(),
            "1",
            "over wireless adb this case needs USB debugging on (Developer options): without it the Wi-Fi drop turns adb off and ends the script that turns the Wi-Fi back on"
        );
    }
    let adb_wifi = AdbWifiKept::take(wireless.is_some());
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
    // The activity is brought to the front over an unlocked screen, so
    // the run starts from the state a person leaves the app in.
    platform::until_unlocked();
    platform::shell(&["rm", "-rf", DIR]);
    platform::shell(&["mkdir", "-p", DIR]);
    let script = format!("{DIR}/run.sh");
    let local = tempfile::NamedTempFile::new().expect("a scratch file");
    std::fs::write(local.path(), script_text(adb_wifi.found())).expect("the script");
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
        if let Some(host) = &wireless {
            // Wireless: the phone where it answers now, its old port first.
            let now = platform::moved().unwrap_or_else(|| serial.clone());
            if !connected(&now)
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
/// host does meanwhile. Nothing in it waits on the screen: an
/// `am start -W` behind a keyguard that came back after the host's
/// unlock check never returns (the A40, 2026-11-04: the Wi-Fi went off
/// 38 minutes late, after the host had given up), so the start is not
/// waited on and the process is polled for, at most `START_FOR`
/// seconds; a process that never came is an empty `pid-before`, which
/// the host refuses.
fn script_text(adb_wifi: &str) -> String {
    let adb_wifi = put_back(ADB_WIFI, adb_wifi);
    format!(
        "\
out={DIR}
logcat -c
logcat -v time > $out/logcat.txt &
lp=$!
am start -n {PACKAGE}/.MainActivity >/dev/null
n=0
until pidof {PACKAGE} >/dev/null || [ $n -ge {START_FOR} ]; do sleep 1; n=$((n+1)); done
sleep 5
pidof {PACKAGE} > $out/pid-before
log -t {TAG} '{OFF}'
cmd wifi set-wifi-enabled disabled || svc wifi disable
sleep {OFF_FOR}
pidof {PACKAGE} > $out/pid-during
log -t {TAG} '{ON}'
cmd wifi set-wifi-enabled enabled || svc wifi enable
sleep {ON_FOR}
{adb_wifi}
pidof {PACKAGE} > $out/pid-after
log -t {TAG} '{DONE}'
sleep 1
kill $lp
echo done > $out/done
"
    )
}

/// How adb reaches the phone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Link {
    Usb,
    /// Wireless debugging, or `adb tcpip`: an `ip:port` serial or an
    /// mDNS one, which carries no `:`.
    Network,
}

/// The phone's serial and link, read from `adb devices -l` rather than
/// from the serial's shape: an mDNS serial
/// (`adb-<serialno>-<id>._adb-tls-connect._tcp`) has no `:` and is not
/// USB. A device is on USB when its line names a `usb:` path. With no
/// `serial` asked for, the one device listed, as adb would pick it.
fn attachment(devices: &str, serial: Option<&str>) -> Result<(String, Link), String> {
    let lines: Vec<Vec<&str>> = devices
        .lines()
        .filter(|l| !l.starts_with("List of devices") && !l.starts_with('*'))
        .map(|l| l.split_whitespace().collect::<Vec<_>>())
        .filter(|f| f.len() >= 2)
        .collect();
    let entry = match serial {
        Some(serial) => lines.iter().find(|f| f[0] == serial),
        None if lines.len() == 1 => lines.first(),
        None => None,
    }
    .ok_or_else(|| format!("no device {serial:?} in `adb devices -l`:\n{devices}"))?;
    let link = if entry[2..].iter().any(|f| f.starts_with("usb:")) {
        Link::Usb
    } else {
        Link::Network
    };
    Ok((entry[0].to_owned(), link))
}

/// The host of an `ip:port` serial (`[v6]:port` too); `None` for a
/// serial with no port, an mDNS one.
fn host_of(serial: &str) -> Option<String> {
    let (host, port) = serial.rsplit_once(':')?;
    port.parse::<u16>().ok()?;
    Some(host.to_owned())
}

/// The host adb discovered an mDNS `serial` at, from `adb mdns
/// services`: one line per service, its instance name first, its type
/// next and its `ip:port` last. adb names an mDNS device
/// `<instance>.<type>`, or the instance alone.
fn mdns_host(services: &str, serial: &str) -> Option<String> {
    services.lines().find_map(|l| {
        let f: Vec<&str> = l.split_whitespace().collect();
        let (instance, kind, addr) = (f.first()?, f.get(1)?, f.last()?);
        let named =
            *instance == serial || serial == format!("{instance}.{}", kind.trim_end_matches('.'));
        named.then(|| host_of(addr)).flatten()
    })
}

/// What `adb <args>` printed, with no serial chosen: a listing of the
/// server's own.
fn adb_text(args: &[&str]) -> String {
    let out = Command::new("adb").args(args).output().expect("adb");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn an_mdns_serial_is_a_network_link_and_its_host_is_learned_from_adb() {
    let mdns = "adb-R58M41ABCDE-Xy1zQw._adb-tls-connect._tcp";
    let devices = format!(
        "List of devices attached\n\
         0123456789ABCDEF       device usb:1-1.2 product:a40eea model:SM_A405FN device:a40 transport_id:1\n\
         192.168.1.11:44779     device product:a40eea model:SM_A405FN device:a40 transport_id:2\n\
         {mdns} device product:a40eea model:SM_A405FN device:a40 transport_id:3\n"
    );
    // The case this exists for: no ':' and still not USB.
    assert_eq!(
        attachment(&devices, Some(mdns)),
        Ok((mdns.to_owned(), Link::Network))
    );
    // Controls: the USB line is USB, and an ip:port serial is a network.
    assert_eq!(
        attachment(&devices, Some("0123456789ABCDEF")),
        Ok(("0123456789ABCDEF".to_owned(), Link::Usb))
    );
    assert_eq!(
        attachment(&devices, Some("192.168.1.11:44779")),
        Ok(("192.168.1.11:44779".to_owned(), Link::Network))
    );
    // Unlisted, or none asked for among several, is an error, never USB.
    assert!(attachment(&devices, Some("gone")).is_err());
    assert!(attachment(&devices, None).is_err());
    let one = format!("List of devices attached\n{mdns} device transport_id:3\n");
    assert_eq!(attachment(&one, None), Ok((mdns.to_owned(), Link::Network)));

    assert_eq!(
        host_of("192.168.1.11:44779").as_deref(),
        Some("192.168.1.11")
    );
    assert_eq!(host_of("[fe80::1%3]:5555").as_deref(), Some("[fe80::1%3]"));
    assert_eq!(host_of(mdns), None);
    let services = "List of discovered mdns services\n\
        adb-OTHER-aaaa\t_adb-tls-connect._tcp\t192.168.1.54:40001\n\
        adb-R58M41ABCDE-Xy1zQw\t_adb-tls-pairing._tcp\t192.168.1.11:37000\n\
        adb-R58M41ABCDE-Xy1zQw\t_adb-tls-connect._tcp\t192.168.1.11:44779\n";
    assert_eq!(mdns_host(services, mdns).as_deref(), Some("192.168.1.11"));
    assert_eq!(
        mdns_host(services, "adb-R58M41ABCDE-Xy1zQw").as_deref(),
        Some("192.168.1.11")
    );
    assert_eq!(
        mdns_host(services, "adb-ABSENT-bbbb._adb-tls-connect._tcp"),
        None
    );
}

/// The global setting that switches wireless debugging.
const ADB_WIFI: &str = "adb_wifi_enabled";

/// The shell command that puts a global setting back to `found`, as
/// `settings get` printed it: `null` was no value, so it is deleted.
fn put_back(name: &str, found: &str) -> String {
    if found == "null" {
        format!("settings delete global {name}")
    } else {
        format!("settings put global {name} {found}")
    }
}

/// `adb_wifi_enabled` as found before the case, written back when this
/// drops, the read-back said on stderr when it differs (as
/// `ProfileKept`). The setting is not the listener: on the A40 it read 1
/// with no port open (the 2026-11-05 run record). A wireless run reaches
/// the phone again or fails, so a listener that did not return shows;
/// a USB run never looks, so when it found wireless debugging on, the
/// drop asks a person to check it.
struct AdbWifiKept {
    found: String,
    wireless: bool,
}

impl AdbWifiKept {
    fn take(wireless: bool) -> Self {
        let found = platform::shell(&["settings", "get", "global", ADB_WIFI])
            .trim()
            .to_owned();
        Self { found, wireless }
    }

    fn found(&self) -> &str {
        &self.found
    }
}

impl Drop for AdbWifiKept {
    fn drop(&mut self) {
        let wrote = platform::adb()
            .args(["shell", &put_back(ADB_WIFI, &self.found)])
            .output();
        let now = platform::adb()
            .args(["shell", "settings", "get", "global", ADB_WIFI])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());
        if now.as_deref().ok() != Some(self.found.as_str()) {
            eprintln!(
                "the phone's {ADB_WIFI} was NOT put back (write: {:?}, now {now:?}); it was {}",
                wrote.map(|o| o.status),
                self.found
            );
        }
        if self.found == "1" && !self.wireless {
            eprintln!(
                "PERSON: wireless debugging was on before the Wi-Fi drop; check it is still on (Developer options): the setting reads back, the listener may not have"
            );
        }
    }
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
/// [`PORTS`] that accepts a connection, tried with `adb connect`.
fn find_adb(host: &str) -> Option<String> {
    open_ports(host, PORTS)?
        .into_iter()
        .map(|port| format!("{host}:{port}"))
        .find(|s| connected(s))
}

/// The ports of `ports` that accept a connection at any address `host`
/// resolves to now, in order; `None` when it resolves to none. `host` is
/// resolved once, so a serial naming the phone by hostname is followed
/// too (a `SocketAddr` parses an IP literal only), and every address is
/// tried, since a name may resolve to an address family adbd does not
/// listen on. The probes run 512 at a time; a closed port refuses at once.
fn open_ports(host: &str, ports: std::ops::RangeInclusive<u16>) -> Option<Vec<u16>> {
    let probes = probe_addrs(&resolve(host)?, ports);
    let mut open: Vec<u16> = probes
        .chunks(512)
        .flat_map(|chunk| {
            let threads: Vec<_> = chunk
                .iter()
                .map(|&addr| {
                    std::thread::spawn(move || {
                        TcpStream::connect_timeout(&addr, Duration::from_millis(800))
                            .ok()
                            .map(|_| addr.port())
                    })
                })
                .collect();
            threads
                .into_iter()
                .filter_map(|p| p.join().ok().flatten())
                .collect::<Vec<_>>()
        })
        .collect();
    open.dedup();
    Some(open)
}

/// Each address of `addrs` at each port of `ports`, port by port, so
/// that a port open at two addresses is found twice in a row. The port
/// is set on the resolved address rather than the address rebuilt from
/// its IP, so an IPv6 link-local serial keeps its scope id.
fn probe_addrs(addrs: &[SocketAddr], ports: std::ops::RangeInclusive<u16>) -> Vec<SocketAddr> {
    ports
        .flat_map(|port| {
            addrs.iter().map(move |&addr| {
                let mut addr = addr;
                addr.set_port(port);
                addr
            })
        })
        .collect()
}

/// The addresses `host` -- an IP literal, bracketed for IPv6 as adb
/// prints it, or a name -- resolves to now, at port 0; `None`, said on
/// stderr, when it resolves to none.
fn resolve(host: &str) -> Option<Vec<SocketAddr>> {
    match format!("{host}:0").to_socket_addrs() {
        Ok(addrs) => Some(addrs.collect::<Vec<_>>()).filter(|a| !a.is_empty()),
        Err(e) => {
            eprintln!("network_callback: {host} does not resolve now: {e}");
            None
        }
    }
}

#[test]
fn a_port_open_on_a_serials_host_is_found_by_name_as_well_as_by_literal() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a listener");
    let port = listener.local_addr().expect("its address").port();
    // The case this exists for: a name, which a SocketAddr does not parse,
    // and which may resolve to ::1 before the address that listens.
    assert!(format!("localhost:{port}").parse::<SocketAddr>().is_err());
    assert_eq!(open_ports("localhost", port..=port), Some(vec![port]));
    // Controls: the literal adb prints is found, and a closed port is not.
    assert_eq!(open_ports("127.0.0.1", port..=port), Some(vec![port]));
    drop(listener);
    assert_eq!(open_ports("127.0.0.1", port..=port), Some(vec![]));
    assert_eq!(open_ports("no such host.invalid", port..=port), None);
}

#[test]
fn a_link_local_serials_scope_survives_resolution() {
    let addrs = resolve("[fe80::1%3]").expect("a literal resolves");
    let probes = probe_addrs(&addrs, 5555..=5556);
    assert!(
        matches!(probes.as_slice(),
            [SocketAddr::V6(a), SocketAddr::V6(b)]
                if a.scope_id() == 3 && b.scope_id() == 3 && a.port() == 5555 && b.port() == 5556),
        "{probes:?}"
    );
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

#[test]
fn the_script_waits_on_no_screen_and_polls_for_the_process() {
    let script = script_text("1");
    let start = script
        .lines()
        .find(|l| l.starts_with("am start"))
        .expect("the script starts the app");
    assert!(
        !start.split_whitespace().any(|w| w == "-W"),
        "an `am start -W` behind a keyguard never returns: {start}"
    );
    // Control: the start is followed by the bounded poll for the process.
    let poll = format!("until pidof {PACKAGE} >/dev/null || [ $n -ge {START_FOR} ]");
    assert!(script.contains(&poll), "no bounded poll in:\n{script}");
    assert!(script.find(start) < script.find(&poll));
}

#[test]
fn the_script_writes_back_the_adb_wifi_value_the_host_found() {
    // A USB run on a phone with wireless debugging off leaves it off.
    let off = script_text("0");
    assert!(
        off.contains("settings put global adb_wifi_enabled 0\n"),
        "{off}"
    );
    assert!(!off.contains("adb_wifi_enabled 1"), "{off}");
    // Control: a phone that had it on gets it back on.
    assert!(script_text("1").contains("settings put global adb_wifi_enabled 1\n"));
    // No value found is no value left.
    let none = script_text("null");
    assert!(
        none.contains("settings delete global adb_wifi_enabled\n"),
        "{none}"
    );
    assert!(
        !none.contains("settings put global adb_wifi_enabled"),
        "{none}"
    );
}
