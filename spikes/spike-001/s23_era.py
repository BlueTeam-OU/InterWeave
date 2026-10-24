# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S23 (plan §19, carried): does a channel server that
negotiates the 2026-07-28 protocol revision register as a channel?

    python3 s23_era.py <run>

Each run is one interactive session on the claude on PATH, the
development flag naming ``server:spike001``, against s23_stub.py as a
bare ``--mcp-config`` server. The runs differ only in the stub's answer
to ``server/discover`` and the client's negotiation setting:

  a-modern          the stub offers 2026-07-28; negotiation unset
  b-legacy          the stub answers -32601, as the bridge does (control)
  c-modern-pinned   the stub offers 2026-07-28; MCP_PROTOCOL_NEGOTIATION=legacy
  d-modern-no-resulttype  as a, the stub's modern results without resultType

The session gets its own scratch CLAUDE_CONFIG_DIR with the folder
already trusted (s19_channels.seed_config), so the only screen is the
development warning, which is accepted. Nothing is typed after it: an
idle session answers a channel event on its own (fact 9), so whether the
event reached the model is in the transcript. The session is stopped with
SIGINT, as SPIKE-001 measured, and every process of it has exited before
anything is read.

The nested session authenticates with this login's long-term token as
S22 does; raw output goes outside the repository, the evidence is
extract.py's distillation and is kept only when no file holds the token.
EVIDENCE ONLY.
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

from spike_common import CLAUDE, REPO, child_env, claude_version, raw_dir
from s19_channels import ANSI, long_term_token, seed_config
from s22_org_channels import purge_if_token, scrub, stop_group, token_in

HERE = pathlib.Path(__file__).resolve().parent
RUNS = {
    "a-modern": {"era": "modern", "negotiation": None},
    "b-legacy": {"era": "legacy", "negotiation": None},
    "c-modern-pinned": {"era": "modern", "negotiation": "legacy"},
    "d-modern-no-resulttype": {"era": "modern", "negotiation": None, "omit_resulttype": True},
}
SETTLE_S = 45
AFTER_DECISION_S = 25


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in RUNS:
        raise SystemExit(f"usage: {sys.argv[0]} {{{'|'.join(RUNS)}}}")
    if os.environ.get("SPIKE_CLAUDE"):
        raise SystemExit("unset SPIKE_CLAUDE: S23 measures the claude on PATH")
    name = sys.argv[1]
    spec = RUNS[name]
    run = "s23-" + name
    out = raw_dir(run)
    scratch = pathlib.Path(tempfile.mkdtemp(prefix="spike001s23-")).resolve()
    if scratch == REPO or REPO in scratch.parents:
        scratch.rmdir()
        raise SystemExit("TMPDIR must be outside the repository")
    nonce = secrets.token_hex(4)
    record = {"run": run, "claude_binary_version": claude_version(), "stub_era": spec["era"],
              "negotiation": spec["negotiation"], "nonce": nonce}
    passed = False
    pid = None
    try:
        config, workdir = scratch / "config", scratch / "work"
        config.mkdir()
        workdir.mkdir()
        seed_config(config, workdir)
        mcp = {"mcpServers": {"spike001": {"command": sys.executable, "args": [str(HERE / "s23_stub.py")], "env": {
            "SPIKE_LOG": str(out / "stub.jsonl"), "SPIKE_ERA": spec["era"], "SPIKE_NONCE": nonce,
            **({"SPIKE_OMIT_RESULTTYPE": "1"} if spec.get("omit_resulttype") else {})}}}}
        (workdir / "mcp.json").write_text(json.dumps(mcp, indent=2))
        debug = out / "debug.txt"
        cmd = [CLAUDE, "--mcp-config", "mcp.json", "--strict-mcp-config", "--debug-file", str(debug),
               "--dangerously-load-development-channels", "server:spike001"]
        record["command"] = ["claude", "--mcp-config", "mcp.json", "--strict-mcp-config",
                             "--debug-file", "$RAW/debug.txt",
                             "--dangerously-load-development-channels", "server:spike001"]
        extra = {"MCP_PROTOCOL_NEGOTIATION": spec["negotiation"]} if spec["negotiation"] else {}

        screen = bytearray()
        pid, fd = pty.fork()
        if pid == 0:
            try:
                os.chdir(workdir)
                env = child_env({"TERM": "xterm-256color", "COLUMNS": "120", "LINES": "40",
                                 "CLAUDE_CONFIG_DIR": str(config), **extra, **long_term_token()})
                os.execvpe(cmd[0], cmd, env)
            except BaseException:
                traceback.print_exc()
            finally:
                os._exit(127)

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

        def flat():
            return "".join(ANSI.sub(b"", bytes(screen)).decode("utf-8", "replace").split())

        def decision():
            log = debug.read_text(errors="replace") if debug.exists() else ""
            for line in log.splitlines():
                if 'MCP server "spike001"' in line and "Channel notifications" in line:
                    return line
            return None

        started = time.time()
        steps = []
        # The development warning: accepted, as fact 5's runs did.
        while time.time() - started < 60 and "localdevelopment" not in flat():
            if not pump(0.5):
                break
        warned = "localdevelopment" in flat()
        steps.append({"development_warning": warned, "at_s": round(time.time() - started, 1)})
        if warned:
            pump(1)
            os.write(fd, b"\r")
        while time.time() - started < SETTLE_S and decision() is None:
            if not pump(0.5):
                break
        line = decision()
        steps.append({"channel_decision_at_s": round(time.time() - started, 1) if line else None})
        pump(AFTER_DECISION_S)
        bare = flat()
        notice = re.search(r"Channelmessagesfrom\"spike001\"areunavailable[^.]*", bare)
        record["screen_era_notice"] = notice.group(0) if notice else None
        record["steps"] = steps
        record["channel_decision"] = line

        # SIGINT to Claude Code itself, the shutdown SPIKE-001 measured, so it
        # stops its server; then the session's whole process group (pty.fork
        # made Claude Code its leader) is escalated until it is empty, and
        # nothing is read before that.
        os.kill(pid, signal.SIGINT)
        end = time.time() + 5
        while time.time() < end and not os.waitpid(pid, os.WNOHANG)[0]:
            pump(0.2)
        record["shutdown"] = stop_group(pid, pump, steps=(("group", signal.SIGTERM, 5),
                                                           ("group", signal.SIGKILL, 5)))
        record["shutdown"]["signals_sent"].insert(0, "SIGINT to Claude Code")
        pid = None
        os.close(fd)
        (out / "screen.txt").write_text(ANSI.sub(b"", bytes(screen)).decode("utf-8", "replace"))
        log = debug.read_text(errors="replace") if debug.exists() else ""
        record["versions_in_debug_log"] = sorted(set(re.findall(r"\b2\.\d+\.\d+\b", log)))
        record["env_passed"] = sorted(child_env({"TERM": "", "COLUMNS": "", "LINES": "", "CLAUDE_CONFIG_DIR": "",
                                                 **extra, "CLAUDE_CODE_OAUTH_TOKEN": ""}))
        transcripts = sorted((config / "projects").glob("*/*.jsonl"))
        if transcripts:
            shutil.copy(transcripts[-1], out / "session.jsonl")
        delivered = (out / "session.jsonl").exists() and f"S23 {nonce}" in (out / "session.jsonl").read_text()
        record["event_reached_conversation"] = delivered
        passed = line is not None and warned and record["shutdown"]["group_empty"]
    finally:
        if pid is not None:
            try:
                os.killpg(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            try:
                os.waitpid(pid, 0)
            except ChildProcessError:
                pass
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
        scrub(dest / "stub.jsonl", scratch, out)
        tainted = token_in([out, dest])
    except BaseException:
        shutil.rmtree(dest, ignore_errors=True)
        raise
    if tainted:
        shutil.rmtree(dest)
        shutil.rmtree(out)
        raise SystemExit(f"the token appears in {', '.join(tainted)}: no evidence kept, raw output removed")
    print(f"{run}: decision={record['channel_decision']!r} delivered={record['event_reached_conversation']} "
          f"notice={record['screen_era_notice']!r} passed={passed} raw={out}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
