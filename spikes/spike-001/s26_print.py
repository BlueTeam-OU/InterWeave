# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S26: whether a channel's reply leaves a print session in
each permission mode, with no --allowedTools, and with a host that
answers the permission prompt.

    python3 s26_print.py <run>

Every run is S25's c -- the release bridge installed as
``interweave@interweave`` from the local marketplace, under bubblewrap
with a private ``/etc`` holding S22's allow entry, ``--channels`` naming
the plugin, S16's two daemons with the far peer a second bridge on
daemon B, the peer sending once the first turn's result is out, stdin
open -- with NO --allowedTools. The runs differ only in the permission
flags:

  a-accept-edits    --permission-mode acceptEdits
  b-bypass          --permission-mode bypassPermissions
  c-dont-ask        --permission-mode dontAsk
  d-plan            --permission-mode plan
  e-host-allow      --permission-mode manual --permission-prompts host
                    --permission-prompt-tool stdio; the driver answers
                    every can_use_tool request allow
  f-host-deny       as e, the driver answering deny
  g-host-no-tool    --permission-mode manual --permission-prompts host,
                    no --permission-prompt-tool; the driver would answer
                    allow (S25's e with the flag written out)
  h-prompts-none    --permission-mode manual --permission-prompts none
                    (control for e to g)

The driver is the SDK host the help text names: a can_use_tool
control_request on stdout is answered on stdin with the SDK's
control_response, ``{"behavior": "allow", "updatedInput": <input>}`` or
``{"behavior": "deny", "message": ...}``. Every request and answer is
recorded. Whether the peer receives ``ack`` is what each run measures,
either way. The token, the raw directory and the evidence are handled
as in S22. EVIDENCE ONLY.
"""

import json
import os
import pathlib
import secrets
import shutil
import subprocess
import sys
import tempfile
import threading
import time

from spike_common import CLAUDE, REPO, child_env, claude_version, raw_dir
from s16_run import PROFILE, STRANGER, Home, Peer, free_port, private_ipv4
from s19_channels import long_term_token, seed_config
from s22_org_channels import (ALLOW, DROP_IN, MARKETPLACE, PLUGIN, TOOL, cli, private_etc, purge_if_token,
                              scrub, stop_group, token_in)
from s24_print import CHANNELS, SERVER, Session
from s25_print import ANSWER_S, FIRST_TURN, MENTIONS, timeline

HERE = pathlib.Path(__file__).resolve().parent
HOST = ["--permission-prompts", "host", "--permission-prompt-tool", "stdio"]
RUNS = {
    "a-accept-edits": {"flags": ["--permission-mode", "acceptEdits"], "answer": None},
    "b-bypass": {"flags": ["--permission-mode", "bypassPermissions"], "answer": None},
    "c-dont-ask": {"flags": ["--permission-mode", "dontAsk"], "answer": None},
    "d-plan": {"flags": ["--permission-mode", "plan"], "answer": None},
    "e-host-allow": {"flags": ["--permission-mode", "manual", *HOST], "answer": "allow"},
    "f-host-deny": {"flags": ["--permission-mode", "manual", *HOST], "answer": "deny"},
    "g-host-no-tool": {"flags": ["--permission-mode", "manual", "--permission-prompts", "host"], "answer": "allow"},
    "h-prompts-none": {"flags": ["--permission-mode", "manual", "--permission-prompts", "none"], "answer": None},
}
DENY_MESSAGE = "S26: the host denies this tool."


class HostSession(Session):
    """S24's session reader, which also answers every can_use_tool
    control_request as the SDK host would, and records what it was asked
    and what it answered. ``answer`` None answers nothing."""

    def __init__(self, proc, started, answer):
        self.answer, self.asked, self.lock = answer, [], threading.Lock()
        super().__init__(proc, started)

    def _read(self):
        for line in self.proc.stdout:
            try:
                value = json.loads(line)
            except ValueError:
                value = {"unparsed": line.rstrip("\n")}
            at = round(time.time() - self.started, 1)
            self.lines.append((at, value))
            request = value.get("request") or {}
            if value.get("type") == "control_request" and request.get("subtype") == "can_use_tool":
                self._answer(at, value["request_id"], request)

    def _answer(self, at, request_id, request):
        entry = {"at_s": at, "tool_name": request.get("tool_name"), "input_keys": sorted(request.get("input") or {}),
                 "decision_reason_type": request.get("decision_reason_type"), "answered": None}
        if self.answer == "allow":
            decision = {"behavior": "allow", "updatedInput": request.get("input") or {}}
        elif self.answer == "deny":
            decision = {"behavior": "deny", "message": DENY_MESSAGE}
        else:
            decision = None
        if decision is not None:
            reply = {"type": "control_response",
                     "response": {"subtype": "success", "request_id": request_id, "response": decision}}
            try:
                with self.lock:
                    self.proc.stdin.write(json.dumps(reply) + "\n")
                    self.proc.stdin.flush()
                entry["answered"] = decision["behavior"]
            except (BrokenPipeError, ValueError):
                entry["answered"] = "stdin closed"
        self.asked.append(entry)

    def write(self, value):
        with self.lock:
            super().write(value)


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in RUNS:
        raise SystemExit(f"usage: {sys.argv[0]} {{{'|'.join(RUNS)}}}")
    if os.environ.get("SPIKE_CLAUDE"):
        raise SystemExit("unset SPIKE_CLAUDE: S26 measures the claude on PATH")
    name = sys.argv[1]
    spec = RUNS[name]
    run = "s26-" + name

    subprocess.run(["cargo", "build", "-q", "-j", "2", "--release", "-p", "interweave-claude-channel"], check=True,
                   cwd=REPO)
    subprocess.run(["cargo", "build", "-q", "-j", "2", "-p", "interweave-transport-daemon", "-p",
                    "interweave-transportctl"], check=True, cwd=REPO)
    bridge = REPO / "target" / "release" / "claude-channel"
    daemon, ctl = REPO / "target" / "debug" / "transport-daemon", REPO / "target" / "debug" / "transportctl"

    out = raw_dir(run)
    scratch = pathlib.Path(tempfile.mkdtemp(prefix="spike001s26-")).resolve()
    if scratch == REPO or REPO in scratch.parents:
        scratch.rmdir()
        raise SystemExit("TMPDIR must be outside the repository")

    live = []
    record = {"run": run, "claude_version": claude_version()}
    passed = False
    try:
        config, workdir, built, policy = (scratch / d for d in ("config", "work", "marketplace", "policy"))
        for d in (config, workdir, policy):
            d.mkdir()
        path = policy / DROP_IN
        path.parent.mkdir(parents=True)
        path.write_text(json.dumps(ALLOW, indent=2) + "\n")
        record["managed_settings"] = {"path": f"/etc/claude-code/{DROP_IN}", "content": ALLOW}
        subprocess.run(["bash", str(REPO / "packaging" / "claude-plugin" / "build.sh"), str(built), str(bridge)],
                       check=True, capture_output=True)

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

        record["setup"] = [cli(config, "plugin", "marketplace", "add", str(built)),
                           cli(config, "plugin", "install", f"{PLUGIN}@{MARKETPLACE}", "--config",
                               f"profile={PROFILE}", "--config", "endpoint=claude")]
        seed_config(config, workdir)

        debug = out / "debug.txt"
        io = ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose",
              "--include-partial-messages"]
        cmd = [CLAUDE, *io, *spec["flags"], "--debug-file", str(debug), *CHANNELS]
        record["command"] = ["claude", *io, *spec["flags"], "--debug-file", "$RAW/debug.txt", *CHANNELS]
        record["host_answers"] = spec["answer"]
        record["sandbox"] = "bwrap: the host's /etc entries read-only, /etc/claude-code a scratch directory read-only"
        env = child_env({"CLAUDE_CONFIG_DIR": str(config), **a.env(), **long_term_token()})
        started = time.time()
        record["started_at"] = time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime(started)) + f".{int(started % 1 * 1000):03d}Z"
        proc = subprocess.Popen(private_etc(policy) + cmd, cwd=workdir, env=env, stdin=subprocess.PIPE,
                                stdout=subprocess.PIPE, stderr=open(out / "stderr.txt", "w"), text=True,
                                start_new_session=True)
        live.append(proc.pid)
        session = HostSession(proc, started, spec["answer"])
        steps = []
        # As S24: nothing is written before the server has had time to
        # connect, so the first turn is not the one that connects it.
        time.sleep(5)
        session.write({"type": "user", "message": {"role": "user", "content": FIRST_TURN}})
        steps.append({"first_turn_written_at_s": round(time.time() - started, 1)})
        first = session.wait_for(lambda v: v.get("type") == "result", 120)
        steps.append({"first_turn_result_at_s": first[1] if first else None})

        sent = None
        if first:
            nonce = secrets.token_hex(4)
            message = f"S26 {nonce}: hello from the peer on daemon B"
            record.update({"nonce": nonce, "message": message})
            deadline = time.time() + 60
            while time.time() < deadline and sent is None:
                answer, error = peer.tool("send", {"peer": peers["a"], "endpoint": "claude", "content": message})
                if not error:
                    sent = {"answer": answer, "at_s": round(time.time() - started, 1)}
                else:
                    time.sleep(1)
            steps.append({"peer_send": sent})

        replied, exited, deadline = None, None, time.time() + ANSWER_S
        while sent and time.time() < deadline and replied is None:
            peer.drain(1)
            if peer.received("ack"):
                replied = round(time.time() - started, 1)
            if exited is None and proc.poll() is not None:
                exited = round(time.time() - started, 1)
                deadline = min(deadline, time.time() + 10)
        # The tag's turn may still be running when ack arrives; its result
        # carries the permission denials, so wait for it before closing.
        if sent:
            session.wait_for(lambda v: v.get("type") == "result", 30, since=first[0] + 1)
        steps.append({"peer_received_reply_at_s": replied, "session_exited_by_s": exited})
        record["steps"] = steps
        record["peer_received_reply"] = replied is not None
        passed = bool(sent)

        with session.lock:
            if not proc.stdin.closed:
                proc.stdin.close()
        try:
            proc.wait(timeout=30)
            record["exit_after_stdin_close"] = proc.returncode
        except subprocess.TimeoutExpired:
            record["exit_after_stdin_close"] = None
        record["shutdown"] = stop_group(proc.pid, lambda s: time.sleep(s))
        passed = passed and record["shutdown"]["group_empty"]
        live.remove(proc.pid)
        session.thread.join(5)
        (out / "transcript.txt").write_text("".join(json.dumps(v) + "\n" for _, v in session.lines))
        record["timeline"] = timeline(session.lines)
        record["permission_modes"] = sorted({v.get("permissionMode") for _, v in session.lines
                                             if v.get("type") == "system" and v.get("subtype") == "init"} - {None})
        record["permission_requests"] = session.asked
        record["permission_denials"] = [d for _, v in session.lines if v.get("type") == "result"
                                        for d in (v.get("permission_denials") or [])]
        record["permission_denied_messages"] = [v.get("message") for _, v in session.lines
                                                if v.get("type") == "system"
                                                and v.get("subtype") == "permission_denied"]

        log = debug.read_text(errors="replace") if debug.exists() else ""
        record["channel_lines"] = [l[25:] for l in log.splitlines() if "Channel" in l and SERVER in l]
        stderr = (out / "stderr.txt").read_text(errors="replace")
        record["stderr_head"] = stderr.splitlines()[:10]
        record["channel_mentions"] = {where: [l[:200] for l in text.splitlines() if MENTIONS.search(l)][:10]
                                      for where, text in (("debug", log), ("stderr", stderr))}
        record["env_passed"] = sorted(child_env({"CLAUDE_CONFIG_DIR": "", **{k: "" for k in a.env()},
                                                 "CLAUDE_CODE_OAUTH_TOKEN": ""}))
        transcripts = sorted((config / "projects").glob("*/*.jsonl"))
        if transcripts:
            shutil.copy(transcripts[-1], out / "session.jsonl")
        peer.drain(1)
        peer.stop()
        live.remove(peer)
        a.stop(ctl, a_proc)
        b.stop(ctl, b_proc)
        live.remove(a_proc)
        live.remove(b_proc)
    finally:
        for thing in reversed(live):
            if isinstance(thing, int):
                stop_group(thing, lambda s: time.sleep(s), steps=(("group", 9, 5),))
            elif isinstance(thing, Peer):
                thing.stop()
            elif thing.poll() is None:
                thing.kill()
                thing.wait()
        shutil.rmtree(scratch, ignore_errors=True)
        raw_tainted = purge_if_token(out)
    if raw_tainted:
        raise SystemExit(f"the token appears in {', '.join(raw_tainted)}: raw output removed, no evidence written")

    record["passed"] = passed
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    dest = HERE / "runs" / run
    try:
        subprocess.run([sys.executable, str(HERE / "extract.py"), str(out), str(dest)], check=True)
        scrub(dest / "evidence.md", scratch, out)
        tainted = token_in([out, dest])
    except BaseException:
        shutil.rmtree(dest, ignore_errors=True)
        raise
    if tainted:
        shutil.rmtree(dest)
        shutil.rmtree(out)
        raise SystemExit(f"the token appears in {', '.join(tainted)}: no evidence kept, raw output removed")
    print(f"{run}: steps={json.dumps(record['steps'])} modes={record['permission_modes']} "
          f"requests={json.dumps(record['permission_requests'])} replied={record['peer_received_reply']} "
          f"passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
