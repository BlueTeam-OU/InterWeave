# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S25, case 5: does the remedy the bridge's refusal names,
``claude plugin configure interweave@interweave``, reach the plugin's
settings?

    python3 s25_configure.py

No session, no model and no credential, as S20: one scratch
CLAUDE_CONFIG_DIR, the release bridge installed as
``interweave@interweave`` with no ``--config``, its ``bin/claude-channel``
a wrapper logging the argv Claude Code gives it. ``claude mcp get``
starts the server to check it, so the wrapper's log is the argv the next
start gets. Three steps, each followed by the user settings'
``pluginConfigs`` and that argv:

  bare-no-tty   the command as the refusal writes it, stdin not a terminal
  bare-tty      the same under a pseudo-terminal, as a person types it;
                given 15 s, then stopped if still running
  values-stdin  the command with ``--values-stdin`` and a JSON object
                (control: a write the CLI itself offers)

EVIDENCE ONLY.
"""

import json
import os
import pathlib
import pty
import re
import select
import shutil
import subprocess
import sys
import tempfile
import time

from spike_common import CLAUDE, REPO, child_env, claude_version
from s22_org_channels import MARKETPLACE, PLUGIN, cli
from s25_print import write_wrapper

HERE = pathlib.Path(__file__).resolve().parent
RUN = "s25-i-plugin-configure"
SERVER = f"plugin:{PLUGIN}:{MARKETPLACE}"
ANSI = re.compile(r"\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b[()][0-9A-Za-z]|\r")
VALUES = {"profile": "work", "endpoint": "bot"}


def under_pty(args, env, cwd, seconds=15):
    """Run args on a pseudo-terminal, typing nothing; the screen with its
    escape sequences removed, and how it ended."""
    pid, fd = pty.fork()
    if pid == 0:
        os.chdir(cwd)
        os.execvpe(args[0], args, env)
    screen, status, end = b"", None, time.time() + seconds
    while time.time() < end:
        ready, _, _ = select.select([fd], [], [], 0.2)
        if ready:
            try:
                chunk = os.read(fd, 4096)
            except OSError:
                chunk = b""
            screen += chunk
        done, st = os.waitpid(pid, os.WNOHANG)
        if done:
            status = os.waitstatus_to_exitcode(st)
            break
    if status is None:
        os.kill(pid, 9)
        os.waitpid(pid, 0)
    os.close(fd)
    return {"screen": ANSI.sub("", screen.decode(errors="replace")).strip(),
            "exit": status if status is not None else f"still running after {seconds} s, stopped"}


def main() -> int:
    subprocess.run(["cargo", "build", "-q", "-j", "2", "--release", "-p", "interweave-claude-channel"], check=True,
                   cwd=REPO)
    bridge = REPO / "target" / "release" / "claude-channel"
    scratch = pathlib.Path(tempfile.mkdtemp(prefix="spike001s25i-")).resolve()
    if scratch == REPO or REPO in scratch.parents:
        scratch.rmdir()
        raise SystemExit("TMPDIR must be outside the repository")
    record = {"run": RUN, "claude_version": claude_version()}
    try:
        config, workdir, built, argv_log = (scratch / d for d in ("config", "work", "marketplace", "argv.log"))
        config.mkdir()
        workdir.mkdir()
        wrapper = scratch / "claude-channel-wrapper"
        write_wrapper(wrapper, argv_log, bridge)
        subprocess.run(["bash", str(REPO / "packaging" / "claude-plugin" / "build.sh"), str(built), str(wrapper)],
                       check=True, capture_output=True)
        env = child_env({"CLAUDE_CONFIG_DIR": str(config)})

        def state():
            settings = json.loads((config / "settings.json").read_text())
            before = len(argv_log.read_text().splitlines()) if argv_log.exists() else 0
            got = subprocess.run([CLAUDE, "mcp", "get", SERVER], capture_output=True, text=True, env=env,
                                 cwd=workdir)
            lines = argv_log.read_text().splitlines() if argv_log.exists() else []
            status = [l.strip() for l in got.stdout.splitlines() if l.strip().startswith("Status")]
            return {"pluginConfigs": settings.get("pluginConfigs"), "mcp_get_status": status,
                    "bridge_argv": lines[before:]}

        record["setup"] = [cli(config, "plugin", "marketplace", "add", str(built)),
                           cli(config, "plugin", "install", f"{PLUGIN}@{MARKETPLACE}")]
        record["after_install"] = state()
        steps = {}
        command = [CLAUDE, "plugin", "configure", f"{PLUGIN}@{MARKETPLACE}"]
        got = subprocess.run(command, capture_output=True, text=True, env=env, cwd=workdir,
                             stdin=subprocess.DEVNULL)
        steps["bare-no-tty"] = {"command": ["claude", *command[1:]], "stdin": "/dev/null", "exit": got.returncode,
                                "output": (got.stdout + got.stderr).strip(), **state()}
        steps["bare-tty"] = {"command": ["claude", *command[1:]], "stdin": "a pseudo-terminal, nothing typed",
                             **under_pty(command, env, workdir), **state()}
        got = subprocess.run([*command, "--values-stdin"], capture_output=True, text=True, env=env, cwd=workdir,
                             input=json.dumps(VALUES))
        steps["values-stdin"] = {"command": ["claude", *command[1:], "--values-stdin"], "stdin": json.dumps(VALUES),
                                 "exit": got.returncode, "output": (got.stdout + got.stderr).strip(), **state()}
        record["steps"] = steps
        defaults = record["after_install"]["bridge_argv"]
        record["passed"] = (bool(defaults) and all(steps[s]["bridge_argv"] for s in steps)
                            and "--profile work --endpoint bot" in " ".join(steps["values-stdin"]["bridge_argv"]))
    finally:
        shutil.rmtree(scratch, ignore_errors=True)

    text = json.dumps(record, indent=2, ensure_ascii=False)
    for path, name in ((str(scratch), "$SCRATCH"), (str(REPO) + "/", ""), (str(pathlib.Path.home()), "$HOME")):
        text = text.replace(path, name)
    # Checked before anything is written: a failed check leaves no evidence in the tree.
    if scratch.name in text:
        raise SystemExit("the evidence still names the scratch directory")
    dest = HERE / "runs" / RUN
    dest.mkdir(parents=True, exist_ok=True)
    (dest / "evidence.md").write_text(f"# {RUN}\n\nNo session, no model, no credential.\n\n## Run\n\n```json\n"
                                      f"{text}\n```\n")
    print(f"{RUN}: passed={record['passed']}")
    return 0 if record["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
