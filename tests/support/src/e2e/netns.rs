// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A second network on one host: plan section 21's decided (7). Two
//! network namespaces, A and B, each behind its own NAT, the relay in a
//! third, all inside one unprivileged user namespace, so a test needs no
//! capability in the host's namespaces ([`in_second_network`]).
//!
//! ```text
//!   A 10.1.0.2 -- NAT-A -- 203.0.113.2 \
//!                                       core -- 203.0.113.10 R (the relay)
//!   B 10.2.0.2 -- NAT-B -- 203.0.113.6 /
//! ```
//!
//! Each NAT masquerades what leaves its private side and forwards
//! nothing new into it, so neither daemon can dial the other's listen
//! address, while both reach the relay -- symmetric, which one NAT
//! between a namespace and the host would not be. The core routes each
//! private range to its NAT, so a dial into one is carried as far as
//! that NAT and refused THERE, by its forward chain's last rule, which
//! counts what it drops ([`inward_drops`]): the refusal a test asserts
//! is the NAT's, not an unroutable address's.
//!
//! # The overlay: two hosts on one private link
//!
//! The other topology ([`in_overlay`], ADR-0055 decision 5b): A and B
//! alone, joined by one point-to-point link on a carrier-grade prefix,
//! as an overlay a deployment provides would join two hosts -- no NAT,
//! no router, nothing between them.
//!
//! ```text
//!   A 100.64.0.1/30 (w0) ---- (w1) 100.64.0.2/30 B
//! ```
//!
//! [`overlay_down`] deletes the link, both ends with it, as an overlay
//! interface taken down goes; [`overlay_up`] makes it again, the same
//! addresses on the same names.
//!
//! # Why the namespace has a root of its own
//!
//! An unprivileged user namespace cannot map the host's root, so `/` and
//! `/tmp` read there as owned by the overflow uid, and the daemon's
//! ancestor rule (ADR-0028 A 2026-10-08, its trust boundary `/` on a
//! Linux host) refuses every profile directory beneath them -- rightly.
//! So the test runs under a `pivot_root` into a tmpfs the namespace owns,
//! the system directories and this repository bind-mounted into it:
//! every ancestor a daemon judges is then the namespace's own, as on a
//! host it is root's or the user's. Bind mounts keep their owners; the
//! daemon never walks above its own directories into them.
//!
//! # How a test uses it
//!
//! [`in_second_network`] (or [`in_overlay`]) at the top of the test: the
//! outer copy re-runs the test's own binary inside the namespaces, asserts
//! it passed, and returns `false`; the inner copy gets `true` and runs the
//! body. Where user namespaces, `iproute2`, util-linux's `unshare` (with
//! `--kill-child`) and `setpriv` (with `--pdeathsig`), or -- for the NATs'
//! topology -- `nft` are unavailable the test FAILS, never skips (decided
//! (7), ADR-0053 rule 9): a skip is a green check that asserted nothing.

use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use interweave_transport_api::TransportIdentity;

use super::relay::Relay;

/// How long the inner copy may run before it is killed, every process it
/// started with it: its own waits are bounded by `PATIENCE` each, so
/// this is reached only by a hang.
const INNER_DEADLINE: Duration = Duration::from_secs(600);

/// Where the outer copy makes the new root's mount point, when set: the
/// system's temporary directory otherwise.
pub const ROOT_PARENT: &str = "INTERWEAVE_E2E_SECOND_NETWORK_ROOT_PARENT";

/// Set in the inner copy; its value is unused.
pub const NETNS_ENV: &str = "INTERWEAVE_E2E_SECOND_NETWORK";

/// A's address in the overlay, on its end of the link.
pub const OVERLAY_A: Ipv4Addr = Ipv4Addr::new(100, 64, 0, 1);
/// B's address in the overlay, on its end of the link.
pub const OVERLAY_B: Ipv4Addr = Ipv4Addr::new(100, 64, 0, 2);
/// A's address, behind NAT-A.
pub const A: Ipv4Addr = Ipv4Addr::new(10, 1, 0, 2);
/// B's address, behind NAT-B.
pub const B: Ipv4Addr = Ipv4Addr::new(10, 2, 0, 2);
/// NAT-A's public address: what the relay sees of A.
pub const NAT_A: Ipv4Addr = Ipv4Addr::new(203, 0, 113, 2);
/// NAT-B's public address: what the relay sees of B.
pub const NAT_B: Ipv4Addr = Ipv4Addr::new(203, 0, 113, 6);
/// The relay's address, on the core.
pub const RELAY: Ipv4Addr = Ipv4Addr::new(203, 0, 113, 10);

/// One of the namespaces a test places something in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Net {
    /// Behind NAT-A; in the overlay, A's end of the link.
    A,
    /// Behind NAT-B; in the overlay, B's end of the link.
    B,
    /// The relay's, on the core.
    Relay,
    /// NAT-A itself, between A and the core.
    NatA,
    /// NAT-B itself, between B and the core.
    NatB,
}

impl Net {
    /// The namespace's file, as `ip netns` keeps it.
    #[must_use]
    pub fn path(self) -> PathBuf {
        let name = match self {
            Self::A => "A",
            Self::B => "B",
            Self::Relay => "R",
            Self::NatA => "natA",
            Self::NatB => "natB",
        };
        Path::new("/run/netns").join(name)
    }
}

/// What both topologies' shells begin with: `$1` is the new root.
/// Every step is `set -e`: a namespace that half-built fails the run.
const PRELUDE: &str = r#"
set -e
NR="$1"; shift
# The network first: `ip -n` remounts /sys as it switches, which the
# pivoted root refuses. /run, holding the namespaces, is carried over.
mount -t tmpfs -o mode=0755 none /run; mkdir /run/netns
"#;

/// The NATs' topology, built inside the inner copy before it execs the
/// test.
const NAT_NETWORK: &str = r#"
for n in core natA natB A B R; do ip netns add $n; ip -n $n link set lo up; done
link() { ip link add "$1" type veth peer name "$3"; ip link set "$1" netns "$2"; ip link set "$3" netns "$4"; ip -n "$2" link set "$1" up; ip -n "$4" link set "$3" up; }
link a0 A a1 natA; ip -n A addr add 10.1.0.2/24 dev a0; ip -n natA addr add 10.1.0.1/24 dev a1; ip -n A route add default via 10.1.0.1
link b0 B b1 natB; ip -n B addr add 10.2.0.2/24 dev b0; ip -n natB addr add 10.2.0.1/24 dev b1; ip -n B route add default via 10.2.0.1
link na natA ca core; ip -n natA addr add 203.0.113.2/30 dev na; ip -n core addr add 203.0.113.1/30 dev ca; ip -n natA route add default via 203.0.113.1
link nb natB cb core; ip -n natB addr add 203.0.113.6/30 dev nb; ip -n core addr add 203.0.113.5/30 dev cb; ip -n natB route add default via 203.0.113.5
link r0 R cr core; ip -n R addr add 203.0.113.10/30 dev r0; ip -n core addr add 203.0.113.9/30 dev cr; ip -n R route add default via 203.0.113.9
for n in core natA natB; do nsenter --net=/run/netns/$n sh -c 'echo 1 > /proc/sys/net/ipv4/ip_forward'; done
nat() { nsenter --net=/run/netns/$1 nft -f - <<NFT
table ip nat {
  chain post {
    type nat hook postrouting priority 100
    oifname "$2" masquerade
  }
}
table ip filter {
  chain forwarding {
    type filter hook forward priority 0; policy drop
    ct state established,related accept
    iifname "$3" accept
    counter drop
  }
}
NFT
}
nat natA na a1
nat natB nb b1
# The core routes each private range to its NAT, so a dial into one gets
# as far as that NAT's forward chain: the NAT, and nothing upstream of
# it, is what refuses it (counted, `inward_drops`).
ip -n core route add 10.1.0.0/24 via 203.0.113.2
ip -n core route add 10.2.0.0/24 via 203.0.113.6
"#;

/// The overlay's two namespaces; [`OVERLAY_UP`] then makes the link.
const OVERLAY_NETWORK: &str = r"
for n in A B; do ip netns add $n; ip -n $n link set lo up; done
";

/// The overlay's link, made: by the setup, and by [`overlay_up`] from the
/// pivoted root, where `ip -n` is refused -- so every step in a namespace
/// is `nsenter`'s.
const OVERLAY_UP: &str = r"
ip link add w0 type veth peer name w1
ip link set w0 netns A; ip link set w1 netns B
nsenter --net=/run/netns/A sh -c 'ip addr add 100.64.0.1/30 dev w0 && ip link set w0 up'
nsenter --net=/run/netns/B sh -c 'ip addr add 100.64.0.2/30 dev w1 && ip link set w1 up'
";

/// The overlay's link, deleted: a veth goes with both its ends.
const OVERLAY_DOWN: &str = "nsenter --net=/run/netns/A ip link del w0";

/// The namespace's own root, made and pivoted into, after either
/// topology.
const NEW_ROOT: &str = r#"
# Then the root of the namespace's own.
mount -t tmpfs -o mode=0755 none "$NR"
for d in usr etc proc sys dev run tmp scratch; do mkdir -p "$NR/$d"; done
for l in bin sbin lib lib64; do
  if [ -L "/$l" ]; then ln -s "$(readlink "/$l")" "$NR/$l"
  elif [ -d "/$l" ]; then mkdir -p "$NR/$l"; mount --rbind "/$l" "$NR/$l"; fi
done
for d in usr etc dev proc sys run; do mount --rbind "/$d" "$NR/$d"; done
mkdir -p "$NR/var"; ln -s /run "$NR/var/run"
chmod 1777 "$NR/tmp"; chmod 0700 "$NR/scratch"
for w in "$@"; do mkdir -p "$NR$w"; mount --rbind "$w" "$NR$w"; done
# Private before the old root goes: `ip netns` made /run/netns shared,
# and the old root's unmount would otherwise reach the copy here.
mount --make-rprivate "$NR"
mkdir "$NR/old"; cd "$NR"; pivot_root . old; cd /; umount -l /old; rmdir /old
"#;

/// Run the calling test inside the second network. `true` in the inner
/// copy, which runs the body; `false` in the outer one, which has
/// asserted that the inner one passed.
///
/// # Panics
/// In the outer copy, when the namespaces cannot be made or the inner
/// copy did not pass -- never a skip.
#[must_use]
pub fn in_second_network(test: &str) -> bool {
    inside(NAT_NETWORK, test)
}

/// [`in_second_network`], in the overlay's topology instead: A and B on
/// one private link and nothing else.
///
/// # Panics
/// As [`in_second_network`].
#[must_use]
pub fn in_overlay(test: &str) -> bool {
    inside(&format!("{OVERLAY_NETWORK}{OVERLAY_UP}"), test)
}

/// Delete the overlay's link, as an overlay interface taken down goes:
/// both ends' addresses leave their namespaces.
///
/// # Panics
/// Outside the overlay, or when the link is already down.
pub fn overlay_down() {
    shell("deleting the overlay's link", OVERLAY_DOWN);
}

/// Make the overlay's link again, the same addresses on the same names.
///
/// # Panics
/// Outside the overlay, or when the link is already up.
pub fn overlay_up() {
    shell(
        "making the overlay's link",
        &format!("set -e\n{OVERLAY_UP}"),
    );
}

fn shell(what: &str, script: &str) {
    let out = Command::new("sh")
        .args(["-c", script])
        .output()
        .unwrap_or_else(|e| panic!("{what}: {e}"));
    assert!(
        out.status.success(),
        "{what}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The outer copy's run of the inner one in the namespaces `network`
/// builds.
fn inside(network: &str, test: &str) -> bool {
    if std::env::var_os(NETNS_ENV).is_some() {
        return true;
    }
    let exe = std::env::current_exe().expect("the test binary's path");
    // What the inner copy needs of this host's tree, mounted at the same
    // paths: the repository (its examples) and the target directory the
    // binaries are in, wherever CARGO_TARGET_DIR put it.
    let target = exe
        .ancestors()
        .nth(3)
        .expect("<target>/<profile>/deps/<test>")
        .to_path_buf();
    let repo = crate::repo_root();
    // Under ROOT_PARENT when set: a caller that kills this copy -- whose
    // TempDir then never drops -- removes what it left.
    let parent = std::env::var_os(ROOT_PARENT).map_or_else(std::env::temp_dir, PathBuf::from);
    let root = tempfile::Builder::new()
        .prefix("iw-second-network-")
        .tempdir_in(parent)
        .expect("the new root's mount point");
    let script = format!(
        "{PRELUDE}{network}{NEW_ROOT}\nexport {NETNS_ENV}=1 TMPDIR=/scratch\n\
         exec \"$IW_TEST_EXE\" --exact \"$IW_TEST_NAME\" --include-ignored --nocapture --test-threads 1"
    );
    // Every process the inner copy starts dies with this test, whichever
    // way it ends. `setpriv --pdeathsig KILL` has the kernel kill `unshare`
    // when the thread that spawned it -- this one, which waits below --
    // goes, the test process killed included; a pid namespace with
    // `--kill-child` then takes the inner copy and its daemons with
    // `unshare`, and the deadline below kills `unshare` on a hang.
    // The signal follows the spawning THREAD: spawn from a thread that
    // outlives the child, as this one does, or the tie is void. The
    // mount point above is still left when the test is SIGKILLed, since no
    // drop runs: a caller that kills it gives it a parent (ROOT_PARENT). `second_network.rs`'s run with the outer copy SIGKILLed is
    // the check (`the_inner_copy_dies_with_the_test`).
    let child = Command::new("setpriv")
        .args([
            "--pdeathsig",
            "KILL",
            "unshare",
            "-rnm",
            "--pid",
            "--fork",
            "--kill-child",
            "--propagation",
            "private",
            "sh",
            "-c",
            &script,
            "sh",
        ])
        .arg(root.path())
        .arg(&repo)
        .arg(&target)
        .env("IW_TEST_EXE", &exe)
        .env("IW_TEST_NAME", test)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| {
            panic!(
                "cannot run `setpriv --pdeathsig KILL unshare -rnm` ({e}): this test needs \
                 util-linux's setpriv and unshare, an unprivileged user namespace, iproute2 \
                 and, for the NATs' topology, nft (plan section 21, decided (7)); it fails \
                 rather than skips where they are unavailable"
            )
        });
    let pid = child.id();
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = done.send(child.wait_with_output());
    });
    let Ok(output) = finished.recv_timeout(INNER_DEADLINE) else {
        let _ = Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .status();
        panic!("the test inside the second network ran past {INNER_DEADLINE:?}: killed");
    };
    let output = output.expect("the inner copy's output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "the test inside the second network did not pass ({}). If the namespaces could not \
         be made, this host cannot ask the question (decided (7)).\n--- stdout ---\n{stdout}\n\
         --- stderr ---\n{stderr}",
        output.status
    );
    false
}

/// `f` on a thread of its own that sits in `net`, its result returned:
/// what that thread opens -- sockets, a runtime's I/O -- is `net`'s.
///
/// # Panics
/// Outside the second network, or when the move is refused.
pub fn in_net<T: Send>(net: Net, f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                enter(net);
                f()
            })
            .join()
            .expect("the thread in the namespace")
    })
}

/// Move the calling thread into `net`.
fn enter(net: Net) {
    let file = std::fs::File::open(net.path())
        .unwrap_or_else(|e| panic!("{}: {e} (inside the second network?)", net.path().display()));
    rustix::thread::move_into_link_name_space(
        std::os::fd::AsFd::as_fd(&file),
        Some(rustix::thread::LinkNameSpaceType::Network),
    )
    .unwrap_or_else(|e| panic!("entering {}: {e}", net.path().display()));
}

/// How many packets `nat`'s forward chain has dropped coming in: new
/// connections into its private side. Read with `nft` in `nat`.
///
/// # Panics
/// When `nat` is not a NAT, or `nft` cannot be read.
#[must_use]
pub fn inward_drops(nat: Net) -> u64 {
    assert!(matches!(nat, Net::NatA | Net::NatB), "{nat:?} is not a NAT");
    let out = Command::new("nsenter")
        .arg(format!("--net={}", nat.path().display()))
        .args(["--", "nft", "list", "chain", "ip", "filter", "forwarding"])
        .output()
        .expect("nft runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .find(|l| l.contains("counter") && l.trim_end().ends_with("drop"))
        .and_then(|l| l.split_whitespace().skip_while(|w| *w != "packets").nth(1))
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no drop counter in:\n{text}"))
}

/// Whether a TCP connection from `net` to `to` is made within `patience`:
/// the probe of what a NAT lets through.
#[must_use]
pub fn tcp_reaches(net: Net, to: SocketAddrV4, patience: Duration) -> bool {
    in_net(net, || {
        std::net::TcpStream::connect_timeout(&std::net::SocketAddr::V4(to), patience).is_ok()
    })
}

/// The test relay ([`Relay`]) serving from the relay's namespace, on
/// [`RELAY`]: a thread of its own in that namespace drives it, so its
/// listener and every connection it accepts are the namespace's.
pub struct NetRelay {
    relay: Option<Relay>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl NetRelay {
    /// Serve `clients` from the relay's namespace.
    ///
    /// # Panics
    /// Outside the second network.
    #[must_use]
    pub fn start(clients: &[&TransportIdentity]) -> Self {
        let clients: Vec<TransportIdentity> = clients.iter().map(|c| (*c).clone()).collect();
        let (ready, relay) = std::sync::mpsc::channel();
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let thread = std::thread::spawn(move || {
            enter(Net::Relay);
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("the relay's runtime");
            runtime.block_on(async move {
                let clients: Vec<&TransportIdentity> = clients.iter().collect();
                let relay = Relay::start_on(&clients, RELAY).await;
                let _ = ready.send(relay);
                let _ = stopped.await;
            });
        });
        let relay = relay.recv().expect("the relay started");
        Self {
            relay: Some(relay),
            stop: Some(stop),
            thread: Some(thread),
        }
    }

    /// The relay.
    ///
    /// # Panics
    /// Never: it is taken only on drop.
    #[must_use]
    pub fn relay(&self) -> &Relay {
        self.relay.as_ref().expect("running")
    }
}

impl Drop for NetRelay {
    fn drop(&mut self) {
        drop(self.relay.take());
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
