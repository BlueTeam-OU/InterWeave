# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S22 (plan §22): the organisation path for ``--channels``
without the development flag, measured on the marketplace
``packaging/claude-plugin/build.sh`` assembles from the release build.

    python3 s22_org_channels.py <run>

Every run installs ``interweave@interweave`` from that local marketplace
into its own scratch CLAUDE_CONFIG_DIR and starts one interactive session
with ``--channels plugin:interweave@interweave`` and NO development flag.
The runs differ only in what the organisation's managed settings say:

  a-org-allow       managed-settings.d/interweave.json: channelsEnabled true,
                    allowedChannelPlugins naming {interweave, interweave};
                    two daemons, a peer sends, the prompt asks for a reply
  b-no-setting      no managed settings (the control: S19's c)
  c-enabled-false   as a, with channelsEnabled false
  d-org-other       as a, with allowedChannelPlugins naming another plugin
  e-managed-file    as a's settings, in managed-settings.json instead

Where the file is: Claude Code 2.1.286 reads an organisation's managed
settings on Linux from ``/etc/claude-code/managed-settings.json`` and
every ``*.json`` in ``/etc/claude-code/managed-settings.d/`` (read from
the binary, S22 in the README). This login cannot write ``/etc``, so the
session runs under bubblewrap with a private ``/etc``: every entry of the
host's ``/etc`` bound read-only at its own path, and a scratch directory
bound read-only at ``/etc/claude-code``. The binary reads the same path a
host-wide file would be at; nothing outside the session's mount
namespace sees it. The setup commands (the marketplace add and the
install) run outside it, as an operator's would before the policy.

Runs b to e answer registration, which the debug log names; nothing is
typed and no daemon runs (the bridge answers initialize with its channel
capability with the daemon away). Run a is S16's host run without the
development flag: two daemons, the far peer a second bridge on B, the
prompt typed once the peer's message is accepted, and the run passes
when the session's reply reaches the peer. No screen is answered: a run
that meets one records it and fails.

The nested session authenticates with this login's long-term token, as
s16_run.py and s19_channels.py do, passed as one named variable and
recorded by name only. Raw output goes outside the repository
(spike_common.raw_dir); runs/s22-<run>/evidence.md is extract.py's
distillation, kept only once neither it nor any raw file holds the
token; a raw directory that holds it is removed too. EVIDENCE ONLY.
"""

import json
import os
import pathlib
import pty
import re
import secrets
import select
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import traceback

from spike_common import CLAUDE, REPO, child_env, raw_dir
from s16_run import PROFILE, STRANGER, Home, Peer, free_port, private_ipv4
from s19_channels import ANSI, long_term_token, seed_config

HERE = pathlib.Path(__file__).resolve().parent
MARKETPLACE = "interweave"
PLUGIN = "interweave"
FLAGS = ["--channels", f"plugin:{PLUGIN}@{MARKETPLACE}"]
TOOL = "mcp__plugin_interweave_interweave__"
ALLOW = {"channelsEnabled": True, "allowedChannelPlugins": [{"plugin": PLUGIN, "marketplace": MARKETPLACE}]}
DROP_IN = "managed-settings.d/interweave.json"
RUNS = {
    "a-org-allow": {"file": DROP_IN, "settings": ALLOW, "traffic": True},
    "b-no-setting": {"file": None, "settings": None, "traffic": False},
    "c-enabled-false": {"file": DROP_IN, "settings": {**ALLOW, "channelsEnabled": False}, "traffic": False},
    "d-org-other": {"file": DROP_IN, "traffic": False,
                    "settings": {**ALLOW, "allowedChannelPlugins": [{"plugin": "other", "marketplace": MARKETPLACE}]}},
    "e-managed-file": {"file": "managed-settings.json", "settings": ALLOW, "traffic": False},
}
SETTLE_S = 45
# extract.py's own pattern, which scrub has to see the paths through.
UUID = re.compile(r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b")


def private_etc(policy_dir):
    """bubblewrap arguments: the host's root, with /etc replaced by one
    whose every entry is the host's, read-only, and whose claude-code is
    policy_dir, read-only."""
    args = ["bwrap", "--dev-bind", "/", "/", "--tmpfs", "/etc"]
    for entry in sorted(os.listdir("/etc")):
        path = f"/etc/{entry}"
        if entry == "claude-code":
            continue
        args += ["--symlink", os.readlink(path), path] if os.path.islink(path) else ["--ro-bind", path, path]
    return args + ["--ro-bind", str(policy_dir), "/etc/claude-code", "--"]


def cli(config, *args):
    env = child_env({"CLAUDE_CONFIG_DIR": str(config)})
    proc = subprocess.run([CLAUDE, *args], capture_output=True, text=True, env=env)
    return {"command": ["claude", *args], "exit": proc.returncode, "output": (proc.stdout + proc.stderr).strip()}


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in RUNS:
        raise SystemExit(f"usage: {sys.argv[0]} {{{'|'.join(RUNS)}}}")
    if os.environ.get("SPIKE_CLAUDE"):
        # The measured binary is the one on PATH; a run against another is
        # not the one the record names.
        raise SystemExit("unset SPIKE_CLAUDE: S22 measures the claude on PATH")
    name = sys.argv[1]
    spec = RUNS[name]
    run = "s22-" + name

    subprocess.run(["cargo", "build", "-q", "--release", "-p", "interweave-claude-channel"], check=True, cwd=REPO)
    if spec["traffic"]:
        subprocess.run(["cargo", "build", "-q", "-p", "interweave-transport-daemon", "-p", "interweave-transportctl"],
                       check=True, cwd=REPO)
    bridge = REPO / "target" / "release" / "claude-channel"
    daemon, ctl = REPO / "target" / "debug" / "transport-daemon", REPO / "target" / "debug" / "transportctl"

    out = raw_dir(run)
    scratch = pathlib.Path(tempfile.mkdtemp(prefix="spike001s22-")).resolve()
    if scratch == REPO or REPO in scratch.parents:
        # TMPDIR decides where mkdtemp lands; the daemons' keys and the
        # session's config must never be under the working copy.
        scratch.rmdir()
        raise SystemExit("TMPDIR must be outside the repository")
    config, workdir, built, policy = (scratch / d for d in ("config", "work", "marketplace", "policy"))
    for d in (config, workdir, policy):
        d.mkdir()
    managed = None
    if spec["file"]:
        path = policy / spec["file"]
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(spec["settings"], indent=2) + "\n")
        managed = {"path": f"/etc/claude-code/{spec['file']}", "content": json.loads(path.read_text())}
    subprocess.run(["bash", str(REPO / "packaging" / "claude-plugin" / "build.sh"), str(built), str(bridge)],
                   check=True, capture_output=True)

    # Every process started from here is stopped, and the scratch tree --
    # the daemons' private keys included -- removed, however the run ends.
    live = []
    record = {"run": run, "managed_settings": managed}
    passed = False
    raw_tainted = []
    try:
        session_env = {}
        peer = None
        if spec["traffic"]:
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
            # The session's environment is the one its servers inherit:
            # the installed bridge finds daemon A through it.
            session_env = a.env()

        config_args = ["--config", f"profile={PROFILE}", "--config", "endpoint=claude"] if spec["traffic"] else []
        record["setup"] = [cli(config, "plugin", "marketplace", "add", str(built)),
                           cli(config, "plugin", "install", f"{PLUGIN}@{MARKETPLACE}", *config_args)]
        seed_config(config, workdir)

        debug = out / "debug.txt"
        allowed = ["--allowedTools", TOOL + "reply", TOOL + "status"] if spec["traffic"] else []
        cmd = [CLAUDE, *allowed, "--debug-file", str(debug), *FLAGS]
        record["command"] = ["claude", *allowed, "--debug-file", "$RAW/debug.txt", *FLAGS]
        record["sandbox"] = "bwrap: the host's /etc entries read-only, /etc/claude-code a scratch directory read-only"

        screen = bytearray()
        pid, fd = pty.fork()
        if pid == 0:
            # The child execs or dies here: it must never return into the
            # parent's code, whose finally would delete the live run's tree.
            try:
                os.chdir(workdir)
                env = child_env({"TERM": "xterm-256color", "COLUMNS": "120", "LINES": "40",
                                 "CLAUDE_CONFIG_DIR": str(config), **session_env, **long_term_token()})
                argv = private_etc(policy) + cmd
                os.execvpe(argv[0], argv, env)
            except BaseException:
                traceback.print_exc()
            finally:
                os._exit(127)
        live.append(pid)

        def pump(seconds):
            end = time.time() + seconds
            while time.time() < end:
                r, _, _ = select.select([fd], [], [], 0.1)
                if r:
                    try:
                        chunk = os.read(fd, 65536)
                    except OSError:
                        return False
                    if not chunk:
                        return False
                    screen.extend(chunk)
            return True

        def text():
            return ANSI.sub(b"", bytes(screen)).decode("utf-8", "replace")

        def decision():
            log = debug.read_text(errors="replace") if debug.exists() else ""
            for line in log.splitlines():
                if 'MCP server "plugin:interweave:interweave"' in line and "Channel notifications" in line:
                    return line
            return None

        started = time.time()
        steps = []
        alive = True
        while alive and time.time() - started < SETTLE_S and decision() is None:
            alive = pump(0.5)
        line = decision()
        registered = line is not None and "Channel notifications registered" in line
        steps.append({"channel_decision_at_s": round(time.time() - started, 1) if line else None,
                      "registered": registered})

        if spec["traffic"] and registered:
            nonce = secrets.token_hex(4)
            message = f"S22 {nonce}: hello from the peer on daemon B"
            prompt = (HERE / "prompt-s16.txt").read_text().strip().replace("<TOOL>", TOOL + "reply")
            record.update({"nonce": nonce, "message": message, "prompt": prompt})
            sent, deadline = None, time.time() + 120
            while time.time() < deadline:
                answer, error = peer.tool("send", {"peer": peers["a"], "endpoint": "claude", "content": message})
                if not error:
                    sent = {"answer": answer, "at_s": round(time.time() - started, 1)}
                    break
                pump(1)
            steps.append({"peer_send": sent})
            pump(8)
            os.write(fd, prompt.encode())
            pump(0.5)
            os.write(fd, b"\r")
            replied, deadline = False, time.time() + 120
            while time.time() < deadline and not replied:
                pump(2)
                peer.drain(0.5)
                replied = peer.received("ack")
            steps.append({"peer_received_reply": replied, "at_s": round(time.time() - started, 1)})
            # The reply arrives mid-turn: let the turn finish before leaving.
            pump(30)
            passed = bool(sent) and replied
        else:
            pump(3)
            passed = line is not None
        record["steps"] = steps

        # SIGINT is the shutdown SPIKE-001 measured; SIGTERM, then SIGKILL.
        status = None
        for sig, grace in ((signal.SIGINT, 5), (signal.SIGTERM, 5), (signal.SIGKILL, 1)):
            done, status = os.waitpid(pid, os.WNOHANG)
            if done:
                break
            os.kill(pid, sig)
            pump(grace)
        else:
            _, status = os.waitpid(pid, 0)
        live.remove(pid)
        os.close(fd)
        record["wait_status"] = status
        (out / "screen.txt").write_text(text())

        log = debug.read_text(errors="replace") if debug.exists() else ""
        record["versions_in_debug_log"] = sorted(set(re.findall(r"\b2\.\d+\.\d+\b", log)))
        record["channel_decision"] = line
        record["env_passed"] = sorted(child_env({"TERM": "", "COLUMNS": "", "LINES": "", "CLAUDE_CONFIG_DIR": "",
                                                 **{k: "" for k in session_env}, "CLAUDE_CODE_OAUTH_TOKEN": ""}))
        transcripts = sorted((config / "projects").glob("*/*.jsonl"))
        if transcripts:
            shutil.copy(transcripts[-1], out / "session.jsonl")
        if peer is not None:
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
                try:
                    os.kill(thing, signal.SIGKILL)
                    os.waitpid(thing, 0)
                except (ProcessLookupError, ChildProcessError):
                    pass
            elif isinstance(thing, Peer):
                thing.stop()
            elif thing.poll() is None:
                thing.kill()
                thing.wait()
        shutil.rmtree(scratch, ignore_errors=True)
        # Checked however the run ended, before anything is distilled into
        # the working copy: a raw file Claude Code wrote the token into is
        # not kept either.
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
        # A partial distillation was never checked for the token.
        shutil.rmtree(dest, ignore_errors=True)
        raise
    if tainted:
        shutil.rmtree(dest)
        shutil.rmtree(out)
        raise SystemExit(f"the token appears in {', '.join(tainted)}: no evidence kept, raw output removed")
    print(f"{run}: decision={record.get('channel_decision')!r} passed={passed} raw={out}")
    return 0 if passed else 1


def scrub(evidence, scratch, out):
    """extract.py replaces the home directory; the run's own scratch and
    raw directories, in either spelling the TUI and the CLI print, become
    placeholders, so the evidence names no host path. extract.py has also
    rewritten every UUID to <uuid> by then, and a TMPDIR under a session
    scratchpad has one, so each spelling is matched as extract.py left it.
    A run whose evidence still names its scratch or raw directory fails."""
    home = str(pathlib.Path.home())
    text = evidence.read_text()
    for path, name in ((str(scratch), "$SCRATCH"), (str(out), "$RAW")):
        for spelling in (path, path.replace(home, "$HOME", 1), path.replace(home, "~", 1)):
            text = text.replace(UUID.sub("<uuid>", spelling), name).replace(spelling, name)
    evidence.write_text(text)
    for leftover in (scratch.name, out.parent.name):
        if leftover in text:
            raise SystemExit(f"{evidence.name} still names {leftover}: a host path the scrub missed")


def token_in(dirs):
    """The names of the files under dirs holding the long-term token; only
    whether it was found is ever reported, never where in the text."""
    token = long_term_token()["CLAUDE_CODE_OAUTH_TOKEN"]
    files = [p for d in dirs if d.exists() for p in sorted(d.rglob("*")) if p.is_file()]
    return [p.name for p in files if token in p.read_text(errors="replace")]


def purge_if_token(out):
    """The raw directory is outside the repository but still on this host:
    if the token reached it, none of it is kept."""
    tainted = token_in([out])
    if tainted:
        shutil.rmtree(out, ignore_errors=True)
    return tainted


if __name__ == "__main__":
    raise SystemExit(main())
