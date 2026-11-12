# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S33 (2): S32 (b)'s deciding cells on the fleet's pin, with
the real claude-channel bridge across two daemons in place of the stub
(the README's "Not established by S32 (b)").

    python3 s33_reply.py <told>-<message>-<arrival>-<N>
    python3 s33_reply.py table     # the committed runs, per cell

  <told>     ``prompt`` (the answer sentence in the first user turn) or
             ``plugin`` (only in the server's instructions)
  <message>  S32 (b)'s ``greet``, ``question`` or ``task``; S33 ran
             ``question`` only
  <arrival>  ``idle`` (the message starts a turn of its own), ``tool``
             (it joins a turn making ``status`` calls) or ``text`` (it
             comes during a turn writing text alone); S33 ran ``idle``
             and ``tool``

s34_reply.py runs the same code as the series ``s34`` (S32 (b)'s other
cells), its runs written as ``runs/s34-reply-*`` and tabled apart.

The session is S32 (b)'s, run by its own code: ``s32_reply.spec_of``
words the first turn, ``s32_reply.run_session`` starts the print
session, writes the turn, fires the event at the same signal and settles
by the same rule, and its readers turn the record into the outcome. What
differs:

- **The bridge is the release ``claude-channel``**, installed as
  ``interweave@interweave`` with S28's ``--config`` (profile,
  ``endpoint=claude``, ``log_level=debug``), behind s33_proxy.py, which
  records its exchange in the stub's record shape. For ``plugin`` the
  proxy appends the sentence to the bridge's ``initialize``
  instructions; for ``tool`` it holds each ``status`` answer 3 s, as the
  stub did.
- **The message comes from a peer**: S16's two daemons on this host's
  private address, the far peer a second bridge on daemon B, which sends
  the cell's S32 message to endpoint ``claude`` when run_session's
  trigger appears. A reply is S32's (a ``reply`` call with the event's
  token, not refused) that the bridge answered without ``isError`` and
  that the peer received.

The build is the ``claude`` on PATH, the fleet's pin. EVIDENCE ONLY.
"""

import datetime
import json
import os
import pathlib
import re
import secrets
import shutil
import subprocess
import sys
import tempfile
import threading
import time

import s32_reply
from s28_era import tree_stamp
from spike_common import REPO, child_env, claude_version, raw_dir
from s16_run import PROFILE, STRANGER, Home, Peer, free_port, private_ipv4
from s19_channels import seed_config
from s22_org_channels import ALLOW, DROP_IN, MARKETPLACE, PLUGIN, cli, purge_if_token, scrub, token_in
from s28_print import TRAFFIC
from s29_print import elide
from s31_print import dashed

HERE = pathlib.Path(__file__).resolve().parent
TOLD = ("prompt", "plugin")
MESSAGES = tuple(s32_reply.MESSAGES)
ARRIVALS = s32_reply.ARRIVALS


def spec_of(name):
    m = re.fullmatch(rf"({'|'.join(TOLD)})-({'|'.join(MESSAGES)})-({'|'.join(ARRIVALS)})-([1-9]\d?)", name)
    return s32_reply.spec_of(name) if m else None


def committed_runs(series):
    for path in sorted((HERE / "runs").glob(f"{series}-reply-*/evidence.md")):
        m = re.search(r"## Run\n\n```json\n(.*?)\n```", path.read_text(), re.S)
        if m:
            yield json.loads(m.group(1))


def table(series):
    cells = {}
    for r in committed_runs(series):
        if not r.get("passed"):
            continue
        c = cells.setdefault(r["cell"], {"runs": 0, "replies": 0, "after": [], "joined": 0, "tokens": 0})
        c["runs"] += 1
        c["replies"] += bool(r["outcome"]["replied"])
        c["joined"] += r["outcome"]["joined_running_turn"]
        if r["outcome"]["reply_after_s"] is not None:
            c["after"].append(r["outcome"]["reply_after_s"])
        for res in r["results"]:
            u = res.get("usage") or {}
            c["tokens"] += sum(u.get(k) or 0 for k in ("input_tokens", "cache_creation_input_tokens",
                                                        "cache_read_input_tokens", "output_tokens"))
    print("| Cell | Runs | Joined a running turn | Replies | Median s to reply | Tokens |")
    print("|---|---|---|---|---|---|")
    for cell in sorted(cells):
        c = cells[cell]
        after = sorted(c["after"])
        n = len(after)
        median = None if not n else after[n // 2] if n % 2 else (after[n // 2 - 1] + after[n // 2]) / 2
        print(f"| `{cell}` | {c['runs']} | {c['joined']} | {c['replies']} | "
              f"{'-' if median is None else f'{median:.1f}'} | {c['tokens']:,} |")
    print(f"\ntokens: {sum(c['tokens'] for c in cells.values()):,}")


def main(series="s33") -> int:
    if sys.argv[1:] == ["table"]:
        table(series)
        return 0
    if len(sys.argv) != 2 or spec_of(sys.argv[1]) is None:
        raise SystemExit(f"usage: {sys.argv[0]} "
                         f"<{'|'.join(TOLD)}>-<{'|'.join(MESSAGES)}>-<{'|'.join(ARRIVALS)}>-<N>")
    if os.environ.get("SPIKE_CLAUDE"):
        raise SystemExit("unset SPIKE_CLAUDE: S33 measures the claude on PATH")
    name = sys.argv[1]
    spec = spec_of(name)
    run = f"{series}-reply-" + name

    subprocess.run(["cargo", "build", "-q", "-j", "2", "--release", "-p", "interweave-claude-channel"], check=True,
                   cwd=REPO)
    subprocess.run(["cargo", "build", "-q", "-j", "2", "-p", "interweave-transport-daemon", "-p",
                    "interweave-transportctl"], check=True, cwd=REPO)
    bridge = REPO / "target" / "release" / "claude-channel"
    daemon, ctl = REPO / "target" / "debug" / "transport-daemon", REPO / "target" / "debug" / "transportctl"
    stamp = tree_stamp()

    out = raw_dir(run)
    scratch = pathlib.Path(tempfile.mkdtemp(prefix="s33-")).resolve()
    if scratch == REPO or REPO in scratch.parents:
        scratch.rmdir()
        raise SystemExit("TMPDIR must be outside the repository")

    nonce = secrets.token_hex(4)
    message = s32_reply.MESSAGES[spec["message"]].format(nonce=nonce)
    record = {"run": run, **stamp, "claude_version": claude_version(), "nonce": nonce,
              "cell": name.rsplit("-", 1)[0], "told": spec["told"], "message_kind": spec["message"],
              "arrival": spec["arrival"], "first_turn": spec["first"], "message": message,
              "instructions_added": spec["extra_instructions"], "bridge": "release claude-channel at the tree",
              "proxy_tool_delay_ms": s32_reply.TOOL_DELAY_MS if spec["arrival"] == "tool" else 0}
    passed, live = False, []
    try:
        config, workdir, built, policy = (scratch / d for d in ("config", "work", "marketplace", "policy"))
        for d in (config, workdir, policy):
            d.mkdir()
        path = policy / DROP_IN
        path.parent.mkdir(parents=True)
        path.write_text(json.dumps(ALLOW, indent=2) + "\n")
        record["managed_settings"] = {"path": f"/etc/claude-code/{DROP_IN}", "content": ALLOW}

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
        peer = Peer(bridge, b, out / "peer.jsonl")
        live.append(peer)

        # The plugin's binary: the proxy in front of the release bridge,
        # daemon A's XDG tree in the bridge's environment only, so the
        # session's own environment is S32 (b)'s.
        env = {**a.env(), "SPIKE_TOOL_DELAY_MS": record["proxy_tool_delay_ms"]}
        if spec["extra_instructions"]:
            env["SPIKE_EXTRA_INSTRUCTIONS"] = spec["extra_instructions"]
        assigns = " ".join(f"{k}='{v}'" for k, v in env.items())
        bridge_log = scratch / "bridge-stderr.log"
        wrapper = scratch / "claude-channel-wrapper"
        wrapper.write_text(f"#!/bin/bash\nexec env {assigns} python3 -I '{HERE / 's33_proxy.py'}' "
                           f"'{scratch / 'stub.jsonl'}' '{bridge}' \"$@\" 2> >(while IFS= read -r l; do "
                           f"printf '%s %s\\n' \"$(date +%s.%N)\" \"$l\"; done >> '{bridge_log}')\n")
        wrapper.chmod(0o755)
        subprocess.run(["bash", str(REPO / "packaging" / "claude-plugin" / "build.sh"), str(built), str(wrapper)],
                       check=True, capture_output=True)
        record["setup"] = [cli(config, "plugin", "marketplace", "add", str(built)),
                           cli(config, "plugin", "install", f"{PLUGIN}@{MARKETPLACE}", *TRAFFIC)]
        seed_config(config, workdir)
        record["sandbox"] = "bwrap: the host's /etc entries read-only, /etc/claude-code a scratch directory read-only"

        # run_session fires by creating <scratch>/trigger.1, the stub's
        # trigger; here the peer sends when it appears.
        sends, done = [], threading.Event()

        def sender():
            trigger = pathlib.Path(f"{scratch / 'trigger'}.1")
            while not done.is_set():
                if trigger.exists():
                    t = time.time()
                    answer, error = peer.tool("send", {"peer": peers["a"], "endpoint": "claude", "content": message})
                    sends.append({"wall": t, "answer": answer, "error": error})
                    return
                time.sleep(0.02)

        thread = threading.Thread(target=sender, daemon=True)
        thread.start()
        try:
            ok, _ = s32_reply.run_session(spec, scratch, out, config, workdir, policy, record)
        finally:
            done.set()
            thread.join(35)
        # run_session's origin, to the millisecond it records.
        started = datetime.datetime.strptime(record["started_at"], "%Y-%m-%dT%H:%M:%S.%fZ") \
            .replace(tzinfo=datetime.timezone.utc).timestamp()
        record["peer_sends"] = [{"at_s": round(s["wall"] - started, 3), "answer": s["answer"], "error": s["error"]}
                                for s in sends]
        peer.drain(2)
        received = [v["params"]["content"] for v in peer.lines if v.get("method") == "notifications/claude/channel"]
        record["peer_received"] = received

        o = record["outcome"]
        stub = record["stub"]
        _, token = s32_reply.event_of(stub)
        calls = {e["body"].get("id"): e for e in stub if e["dir"] == "in" and isinstance(e["body"], dict)
                 and e["body"].get("method") == "tools/call"
                 and (e["body"].get("params") or {}).get("name") == "reply"
                 and ((e["body"]["params"].get("arguments") or {}).get("reply_token") == token)}
        answers = [e["body"] for e in stub if e["dir"] == "out" and isinstance(e["body"], dict)
                   and e["body"].get("id") in calls and "method" not in e["body"]]
        bridge_ok = [r for r in answers if "error" not in r and not (r.get("result") or {}).get("isError")]
        o["s32_replied"] = o["replied"]
        o["bridge_answers"] = answers
        o["peer_received_reply"] = bool(o["reply_content"] is not None and o["reply_content"] in received)
        o["replied"] = bool(o["s32_replied"] and bridge_ok and o["peer_received_reply"])
        if not o["replied"]:
            o["reply_after_s"] = None
        o["joined_running_turn"] = any(e.get("detail") == "queued_command" for e in record.get("event_entries", []))
        record["bridge_stderr"] = [
            f"{round(float(l.split(' ', 1)[0]) - started, 1)} s: {l.split(' ', 1)[1][:200]}"
            for l in (bridge_log.read_text(errors="replace").splitlines() if bridge_log.exists() else [])
            if " " in l][:60]
        record["env_passed"] = sorted(child_env({"CLAUDE_CONFIG_DIR": "", "CLAUDE_CODE_OAUTH_TOKEN": ""}))
        passed = ok and bool(sends) and not sends[0]["error"] and record["shutdown"]["group_empty"] and \
            all(r["subtype"] == "success" for r in record["results"])

        peer.stop()
        live.remove(peer)
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
        raw_tainted = purge_if_token(out)
    if raw_tainted:
        raise SystemExit(f"the token appears in {', '.join(raw_tainted)}: raw output removed, no evidence written")

    record["passed"] = passed
    (out / "run.json").write_text(json.dumps(elide(record), indent=2) + "\n")
    dest = HERE / "runs" / run
    try:
        subprocess.run([sys.executable, str(HERE / "extract.py"), str(out), str(dest)], check=True)
        scrub(dest / "evidence.md", scratch, out)
        if dashed(scratch.name) in (dest / "evidence.md").read_text():
            raise SystemExit("evidence.md still names the scratch directory in a project directory's spelling")
        tainted = token_in([out, dest])
    except BaseException:
        shutil.rmtree(dest, ignore_errors=True)
        raise
    if tainted:
        shutil.rmtree(dest)
        shutil.rmtree(out)
        raise SystemExit(f"the token appears in {', '.join(tainted)}: no evidence kept, raw output removed")
    o = record["outcome"]
    print(f"{run}: replied={o['replied']} after={o['reply_after_s']} joined={o['joined_running_turn']} "
          f"event_turns={o['event_turns']} calls={[c['name'] for c in o['calls_after_event']]} passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
