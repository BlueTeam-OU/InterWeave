# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S30 d: the broadcast drop count at the daemon beside what
the bridge was notified of, in the shape of #376's 200-broadcast case
(``tests/desktop-e2e/tests/claude_channel.rs``,
``a_broadcast_burst_reaches_the_model_within_the_ingress_bucket``).

    python3 s30_drops.py <run>

No Claude Code: what is measured is the daemon and the bridge. S16's two
daemons on this host's private address; on A the release bridge in push
mode, on B a second release bridge, both spoken to over their stdio as
s16_run.py's ``Peer``. Both join ``general``; B broadcasts until A's
bridge is notified (the mesh), the driver waits 20 s for B's bucket at A
to refill, as the test does, then reads, in this order:

- ``transportctl status --json`` on A and on B, through each daemon's
  admin socket: ``admin.status``, whose ``ipc.events_dropped_total``
  (``LOCAL-IPC.md`` A 2026-11-04, #554) is the daemon's count of copies
  its session queues dropped;
- A's bridge ``status`` tool: its session's ``broadcast_loss``.

Then B's bridge calls ``broadcast`` 200 times back to back, each call
answered before the next is made; the driver reads A's bridge's stdout
for 10 s after the last answer, and reads both again. A's notified casts
are counted from its ``notifications/claude/channel`` lines. Every
figure the README cites is in ``run.json``; the bridges' records stay in
the raw directory. EVIDENCE ONLY.
"""

import json
import re
import secrets
import shutil
import subprocess
import sys
import tempfile
import time
import pathlib

from spike_common import REPO, raw_dir
from s16_run import STRANGER, Home, Peer, free_port, private_ipv4

HERE = pathlib.Path(__file__).resolve().parent
COUNT = 200
REFILL_S = 20
QUIET_S = 10
RUNS = ("d-daemon-drops",)


def admin_status(home, ctl):
    out = home.ctl(ctl, "status", "--json")
    if out.returncode != 0:
        return {"error": out.stderr.strip()[:300]}
    status = json.loads(out.stdout)
    return {"ipc": status.get("ipc"), "health": status.get("health")}


def loss(bridge):
    text, error = bridge.tool("status", {})
    if error:
        return {"error": text[:300]}
    return json.loads(text).get("broadcast_loss")


def casts(bridge, nonce):
    pattern = re.compile(rf"S30 {nonce} cast (\d+)$")
    out = []
    for v in bridge.lines:
        if v.get("method") == "notifications/claude/channel":
            m = pattern.search(v["params"]["content"])
            if m:
                out.append(int(m.group(1)))
    return out


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in RUNS:
        raise SystemExit(f"usage: {sys.argv[0]} {{{'|'.join(RUNS)}}}")
    run = "s30-" + sys.argv[1]
    subprocess.run(["cargo", "build", "-q", "-j", "2", "--release", "-p", "interweave-claude-channel"], check=True,
                   cwd=REPO)
    subprocess.run(["cargo", "build", "-q", "-j", "2", "-p", "interweave-transport-daemon", "-p",
                    "interweave-transportctl"], check=True, cwd=REPO)
    bridge = REPO / "target" / "release" / "claude-channel"
    daemon, ctl = REPO / "target" / "debug" / "transport-daemon", REPO / "target" / "debug" / "transportctl"
    head = subprocess.run(["git", "rev-parse", "--short=9", "HEAD"], capture_output=True, text=True,
                          cwd=REPO).stdout.strip()

    out = raw_dir(run)
    scratch = pathlib.Path(tempfile.mkdtemp(prefix="s30d-")).resolve()
    nonce = secrets.token_hex(4)
    started = time.time()
    record = {"run": run, "tree": head, "nonce": nonce, "count": COUNT, "refill_wait_s": REFILL_S,
              "quiet_after_s": QUIET_S,
              "started_at": time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime(started)) + "Z"}
    live = []
    passed = False
    try:
        ip = private_ipv4()
        a, b = Home(scratch / "a"), Home(scratch / "b")
        a_port, b_port = free_port(ip), free_port(ip)
        peers = {}
        for label, home, port in (("a", a, a_port), ("b", b, b_port)):
            home.write_config(STRANGER, f"/ip4/{ip}/tcp/{port}", None)
            proc = home.start(daemon, out / f"daemon-{label}-keygen.log", "--create-identity")
            live.append(proc)
            peers[label] = home.serving(ctl)["peer"]
            home.stop(ctl, proc)
            live.remove(proc)
        route = lambda port, peer_id: f"/ip4/{ip}/tcp/{port}/p2p/{peer_id}"
        a.write_config(peers["b"], f"/ip4/{ip}/tcp/{a_port}", route(b_port, peers["b"]))
        b.write_config(peers["a"], f"/ip4/{ip}/tcp/{b_port}", route(a_port, peers["a"]))
        a_proc, b_proc = a.start(daemon, out / "daemon-a.log"), b.start(daemon, out / "daemon-b.log")
        live += [a_proc, b_proc]
        a.serving(ctl)
        b.serving(ctl)
        at_a = Peer(bridge, a, out / "bridge-a.jsonl")
        live.append(at_a)
        at_b = Peer(bridge, b, out / "bridge-b.jsonl")
        live.append(at_b)
        steps = {}
        for name, br in (("a", at_a), ("b", at_b)):
            deadline = time.time() + 60
            while True:
                text, error = br.tool("join", {"channel": "general"})
                if not error or time.time() > deadline:
                    break
                time.sleep(1)
            steps[f"join_{name}"] = {"answer": text[:200], "error": error}

        # The mesh: until it forms a publish reaches no one.
        mesh, mesh_formed, deadline = 0, False, time.time() + 120
        while not mesh_formed and time.time() < deadline:
            mesh += 1
            at_b.tool("broadcast", {"channel": "general", "content": f"S30 {nonce} mesh {mesh}"})
            at_a.drain(0.5)
            mesh_formed = any(v.get("method") == "notifications/claude/channel" for v in at_a.lines)
        steps["mesh_publishes"] = mesh
        steps["mesh_formed"] = mesh_formed
        steps["mesh_seen_at_s"] = round(time.time() - started, 1) if mesh_formed else None
        time.sleep(REFILL_S)
        at_a.drain(0.5)
        before = {"a_admin": admin_status(a, ctl), "b_admin": admin_status(b, ctl), "a_bridge_loss": loss(at_a)}

        first = time.time()
        answers = []
        for n in range(1, COUNT + 1):
            text, error = at_b.tool("broadcast", {"channel": "general", "content": f"S30 {nonce} cast {n}"})
            answers.append(None if not error else text[:200])
        steps["publish_span_s"] = round(time.time() - first, 3)
        steps["publish_errors"] = [(n, e) for n, e in enumerate(answers, 1) if e][:20]
        at_a.drain(QUIET_S)
        after = {"a_admin": admin_status(a, ctl), "b_admin": admin_status(b, ctl), "a_bridge_loss": loss(at_a)}
        seen = casts(at_a, nonce)
        record.update({"steps": steps, "before": before, "after": after, "notified": len(seen),
                       "notified_distinct": len(set(seen)), "notified_order_held": seen == sorted(seen),
                       "notified_numbers": seen})

        def count(state, side):
            return ((state[side] or {}).get("ipc") or {}).get("events_dropped_total")

        dropped_a = (count(after, "a_admin") or 0) - (count(before, "a_admin") or 0)
        refused = ((after["a_bridge_loss"] or {}).get("ingress_refused") or 0) - \
            ((before["a_bridge_loss"] or {}).get("ingress_refused") or 0)
        record["sums"] = {"daemon_a_events_dropped_delta": dropped_a,
                          "events_dropped_total_present": count(after, "a_admin") is not None,
                          "bridge_a_ingress_refused_delta": refused,
                          "notified_plus_daemon_dropped": len(seen) + dropped_a,
                          "notified_plus_daemon_dropped_plus_ingress_refused": len(seen) + dropped_a + refused}
        # A run whose broadcasts could reach no one measures nothing (#580 F2).
        passed = (mesh_formed and not steps["join_a"]["error"] and not steps["join_b"]["error"]
                  and not steps["publish_errors"])
        at_a.stop()
        at_b.stop()
        live.remove(at_a)
        live.remove(at_b)
        a.stop(ctl, a_proc)
        b.stop(ctl, b_proc)
        live.remove(a_proc)
        live.remove(b_proc)
    finally:
        for thing in reversed(live):
            if isinstance(thing, Peer):
                thing.stop()
            elif thing.poll() is None:
                thing.kill()
                thing.wait()
        shutil.rmtree(scratch, ignore_errors=True)

    record["passed"] = passed
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    dest = HERE / "runs" / run
    subprocess.run([sys.executable, str(HERE / "extract.py"), str(out), str(dest)], check=True)
    print(f"{run}: sums={json.dumps(record.get('sums'))} notified={record.get('notified')} passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
