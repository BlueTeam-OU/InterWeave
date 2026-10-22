# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's carry into Stage 19 (plan §22): which way of loading the
packaged bridge registers it as a channel, measured on the marketplace
``packaging/claude-plugin/build.sh`` assembles from the release build.

    python3 s19_channels.py <run>

Runs, one session each, every one on the same built marketplace:
  a-dev-plugin-dir     --plugin-dir, --dangerously-load-development-channels plugin:interweave@inline
  b-dev-marketplace    installed from the local marketplace, the development flag on plugin:interweave@interweave
  c-channels           installed from the local marketplace, --channels plugin:interweave@interweave, no development flag
  d-no-flag            installed from the local marketplace, no channel flag (the control)

The question is registration, which the debug log answers ("Channel
notifications registered" or "... skipped: ..."), so nothing is typed
but the development warning's answer -- the session is ended by signal,
not by ``/exit`` -- and no daemon runs: the bridge answers initialize with its channel
capability with the daemon away (it stays a functioning MCP server
without one). Each run has its own CLAUDE_CONFIG_DIR, pre-seeded so that
onboarding and folder trust show no screen; what remains on screen is
what the channel flags themselves show, and each screen the driver
answers is recorded. A menu is answered by the number of the option the
run names, never by position, so a reordered screen is a failed expect,
not a different choice.

The nested session authenticates with this login's long-term token,
passed as one named variable and recorded by name only (the owner's
ruling of 2026-10-06, as s16_run.py; architect-cto's decision of
2026-10-20 for these four runs, and the owner's word for this driver the
same day). Raw output goes outside the repository
(spike_common.raw_dir); runs/s19-<run>/evidence.md is the redacted
distillation, written only after every raw file and the distillation
itself are checked not to contain the token. EVIDENCE ONLY.
"""

import json
import os
import pathlib
import pty
import re
import select
import shutil
import signal
import subprocess
import sys
import tempfile
import time

from spike_common import CLAUDE, REPO, child_env, raw_dir

# Every escape the TUI sends, private-parameter CSI (ESC[<u, ESC[>4m) and
# ESC7/ESC8 included, then any other C0 control but newline, CR and tab:
# what is left is the text the screen showed.
ANSI = re.compile(rb"\x1b\[[0-9;?<=>]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b[()][0-9A-Za-z]|\x1b[=>78]"
                  rb"|[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]")
MARKETPLACE = "interweave"
PLUGIN = "interweave"
DEV = "--dangerously-load-development-channels"
RUNS = {
    "a-dev-plugin-dir": {"install": False, "flags": [DEV, f"plugin:{PLUGIN}@inline"]},
    "b-dev-marketplace": {"install": True, "flags": [DEV, f"plugin:{PLUGIN}@{MARKETPLACE}"]},
    "c-channels": {"install": True, "flags": ["--channels", f"plugin:{PLUGIN}@{MARKETPLACE}"]},
    "d-no-flag": {"install": True, "flags": []},
}
# The development-channel warning's accepting option (SPIKE-001 fact 5).
DEV_WARNING = "Loading development channels"
DEV_ACCEPT = "I am using this for local development"
# How long a session is given to connect its servers and decide on channels.
SETTLE_S = 45
CHANNEL_LINE = re.compile(r"channel", re.IGNORECASE)


def long_term_token():
    # As s16_run.py: the value is read here and goes into the session's
    # environment and nowhere else -- the setup commands run without it.
    path = pathlib.Path.home() / ".config" / "agent-fabric" / "secrets.env"
    if not path.exists():
        raise SystemExit("no agent-fabric secrets file: run fabric-secrets sync")
    for line in path.read_text().splitlines():
        m = re.match(r"\s*(?:export\s+)?CLAUDE_CODE_OAUTH_TOKEN=(['\"]?)(.+)\1\s*$", line)
        if m:
            return {"CLAUDE_CODE_OAUTH_TOKEN": m.group(2)}
    raise SystemExit("the secrets file holds no CLAUDE_CODE_OAUTH_TOKEN")


def seed_config(config, workdir):
    """No onboarding screen and the scratch folder already trusted: the only
    screens left are the channel flags' own."""
    state = config / ".claude.json"
    data = json.loads(state.read_text()) if state.exists() else {}
    data["hasCompletedOnboarding"] = True
    data.setdefault("projects", {})[str(workdir)] = {"hasTrustDialogAccepted": True}
    state.write_text(json.dumps(data, indent=2))


def cli(config, *args):
    env = child_env({"CLAUDE_CONFIG_DIR": str(config)})
    proc = subprocess.run([CLAUDE, *args], capture_output=True, text=True, env=env)
    return {"command": ["claude", *args], "exit": proc.returncode,
            "output": (proc.stdout + proc.stderr).strip()}


def option_number(screen, label):
    # Matched with every space removed: the TUI lays a line out with
    # cursor moves, so "1. I am using" reaches the pty as "1.Iamusing".
    m = re.search(r"(\d)\." + re.escape("".join(label.split())), "".join(screen.split()))
    return m.group(1) if m else None


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in RUNS:
        raise SystemExit(f"usage: {sys.argv[0]} {{{'|'.join(RUNS)}}}")
    name = sys.argv[1]
    spec = RUNS[name]
    out = raw_dir("s19-" + name)
    scratch = pathlib.Path(tempfile.mkdtemp(prefix="spike001s19-"))
    config, workdir, built = scratch / "config", scratch / "work", scratch / "marketplace"
    config.mkdir()
    workdir.mkdir()
    subprocess.run(["bash", str(REPO / "packaging" / "claude-plugin" / "build.sh"), str(built)],
                   check=True, capture_output=True)

    setup = []
    if spec["install"]:
        setup.append(cli(config, "plugin", "marketplace", "add", str(built)))
        setup.append(cli(config, "plugin", "install", f"{PLUGIN}@{MARKETPLACE}"))
        source = []
    else:
        source = ["--plugin-dir", str(built / PLUGIN)]
    # No other claude command runs: the version is the one the session's
    # own debug log names, and the install's output says where it went.
    seed_config(config, workdir)

    debug = out / "debug.txt"
    cmd = [CLAUDE, *source, "--debug-file", str(debug), *spec["flags"]]

    screen = bytearray()
    pid, fd = pty.fork()
    if pid == 0:
        os.chdir(workdir)
        env = child_env({"TERM": "xterm-256color", "COLUMNS": "120", "LINES": "40",
                         "CLAUDE_CONFIG_DIR": str(config), **long_term_token()})
        os.execvpe(cmd[0], cmd, env)

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

    def squeezed():
        # The TUI positions text with cursor moves rather than spaces.
        return "".join(text().split())

    def channel_decided():
        return debug.exists() and "Channelnotifications" in "".join(debug.read_text(errors="replace").split())

    started = time.time()
    answered = []
    alive = True
    while alive and time.time() - started < SETTLE_S and not channel_decided():
        alive = pump(0.5)
        if "".join(DEV_WARNING.split()) in squeezed() and not any(a["screen"] == "dev-warning" for a in answered):
            pump(1)
            number = option_number(text(), DEV_ACCEPT)
            # The screen as it was answered, from its heading on, so the
            # evidence shows what the warning named.
            shown = text()
            shown = shown[shown.rfind("WARNING"):] if "WARNING" in shown else shown[-1500:]
            answered.append({"screen": "dev-warning", "option": number, "at_s": round(time.time() - started, 1),
                             "text": [l.strip() for l in shown.splitlines() if l.strip()][:20]})
            if number is None:
                break
            os.write(fd, number.encode())
            pump(0.5)
    decided = channel_decided()
    pump(3)
    # SIGINT is the shutdown SPIKE-001 measured; SIGTERM, then SIGKILL, if
    # the session outlives it.
    status = None
    for sig, grace in ((signal.SIGINT, 5), (signal.SIGTERM, 5), (signal.SIGKILL, 1)):
        done, status = os.waitpid(pid, os.WNOHANG)
        if done:
            break
        os.kill(pid, sig)
        pump(grace)
    else:
        _, status = os.waitpid(pid, 0)
    os.close(fd)

    (out / "screen.txt").write_text(text())
    debug_text = debug.read_text(errors="replace") if debug.exists() else ""
    record = {
        "run": name,
        "versions_in_debug_log": sorted(set(re.findall(r"\b2\.\d+\.\d+\b", debug_text))),
        "command": ["claude", *(["--plugin-dir", "$MARKETPLACE/interweave"] if source else []),
                    "--debug-file", "$RAW/debug.txt", *spec["flags"]],
        "setup": setup,
        "screens_answered": answered,
        "channel_decided_in_s": round(time.time() - started, 1) if decided else None,
        "wait_status": status,
        "env_passed": sorted(child_env({"TERM": "", "COLUMNS": "", "LINES": "",
                                        "CLAUDE_CONFIG_DIR": "", "CLAUDE_CODE_OAUTH_TOKEN": ""})),
        "channel_lines": [l for l in debug_text.splitlines() if CHANNEL_LINE.search(l)],
        "screen_tail": text()[-4000:],
    }
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    shutil.rmtree(scratch, ignore_errors=True)
    distil(out, name, str(scratch), str(built))
    print(f"s19-{name}: versions={record['versions_in_debug_log']} decided={decided} "
          f"answered={[a['option'] for a in answered]} raw={out}")
    return 0


def distil(out, name, *scratch_paths):
    """runs/s19-<run>/evidence.md: the run record with every host path
    replaced by a placeholder and debug-log timestamps dropped."""
    record = json.loads((out / "run.json").read_text())
    home = str(pathlib.Path.home())
    # The TUI prints a path under the home directory in its "~/" form.
    tilde = [p.replace(home, "~", 1) for p in scratch_paths if p.startswith(home)]
    subs = ([(p, "$SCRATCH") for p in [*scratch_paths, *tilde]]
            + [(str(out), "$RAW"), (home, "$HOME")])

    def redact(s):
        for old, new in subs:
            s = s.replace(old, new)
        return re.sub(r"^\S*\d{4}-\d\d-\d\dT[\d:.]+Z\s*", "", s)

    lines = [f"# s19-{name}", "",
             "Claude Code version, as the session's debug log names it: "
             f"{', '.join(record['versions_in_debug_log']) or 'none'}.", "",
             "## Command", "", "```text", " ".join(record["command"]), "```", "",
             "## Setup (no session)", ""]
    for step in record["setup"]:
        lines += ["```text", "$ " + " ".join(redact(a) for a in step["command"]) + f"   # exit {step['exit']}",
                  redact(step["output"]), "```", ""]
    lines += ["## Screens answered", ""]
    for a in record["screens_answered"]:
        lines += [f"{a['screen']}, answered with option {a['option']} at {a['at_s']} s:", "", "```text",
                  *[redact(l) for l in a["text"]], "```", ""]
    lines += ["(none)", ""] if not record["screens_answered"] else []
    lines += [
              f"Channel decision logged after: {record['channel_decided_in_s']} s. "
              f"Session wait status: {record['wait_status']}.", "",
              "Environment passed (names only): " + ", ".join(record["env_passed"]) + ".", "",
              "## Debug-log lines naming a channel", "", "```text"]
    lines += [redact(l) for l in record["channel_lines"]] or ["(none)"]
    lines += ["```", "", "## Screen, last lines", "", "```text"]
    tail = [l.rstrip() for l in redact(record["screen_tail"]).splitlines() if l.strip()]
    lines += tail[-40:] + ["```", ""]
    evidence = "\n".join(lines)
    refuse_if_token(out, evidence)
    dest = REPO / "spikes" / "spike-001" / "runs" / f"s19-{name}"
    dest.mkdir(parents=True, exist_ok=True)
    (dest / "evidence.md").write_text(evidence)


def refuse_if_token(out, evidence):
    """Nothing enters the repository if the token reached any raw file or
    the distillation; the raw directory stays outside it either way. Only
    whether it was found is reported, never where in the text."""
    token = long_term_token()["CLAUDE_CODE_OAUTH_TOKEN"]
    tainted = [p.name for p in sorted(out.iterdir()) if p.is_file() and token in p.read_text(errors="replace")]
    if token in evidence:
        tainted.append("evidence.md")
    if tainted:
        raise SystemExit(f"the token appears in {', '.join(tainted)}: no evidence written; raw output kept at {out}")


if __name__ == "__main__":
    raise SystemExit(main())
