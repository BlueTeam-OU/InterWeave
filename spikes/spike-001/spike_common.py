# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001: what every driver shares, so isolation and placement are
decided once.

- ``CLAUDE``: the Claude Code binary every run uses -- ``$SPIKE_CLAUDE``,
  an explicit build, or ``claude``. Several builds live side by side and
  the default one updates itself, so a campaign names one.
- ``child_env``: a launched process gets an ALLOW-LISTED environment
  with the auto-updater off -- never the launching session's tokens,
  fabric markers or model overrides.
- ``raw_dir``: raw output goes OUTSIDE the repository, always.
- ``session_version``: the version a run's session announced to the stub
  (``initialize`` ``clientInfo``), which is the version the run measured.
"""

import json
import os
import re
import pathlib
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parent.parent
CLAUDE = os.environ.get("SPIKE_CLAUDE") or "claude"
ENV_ALLOW = ("HOME", "PATH", "LANG", "LC_ALL", "LC_CTYPE", "USER", "LOGNAME",
             "SHELL", "TMPDIR", "XDG_RUNTIME_DIR")


def child_env(extra):
    env = {k: v for k, v in os.environ.items() if k in ENV_ALLOW}
    # A build that updates itself mid-campaign changes what is measured.
    env["DISABLE_AUTOUPDATER"] = "1"
    env.update(extra)
    return env


def claude_version():
    out = subprocess.run([CLAUDE, "--version"], capture_output=True, text=True, env=child_env({}))
    return out.stdout.strip().split(" ")[0]


def raw_dir(name):
    """Where a run's RAW output goes. The raw debug log, transcript and
    screen carry host paths and account detail; only extract.py's
    distillation is committed."""
    root = pathlib.Path(os.environ.get("SPIKE_RAW_DIR") or (pathlib.Path(tempfile.gettempdir()) / "spike-001-raw"))
    resolved = root.resolve()
    if resolved == REPO or REPO in resolved.parents:
        raise SystemExit("SPIKE_RAW_DIR must be outside the repository")
    d = resolved / name
    if d.exists():
        shutil.rmtree(d)
    d.mkdir(parents=True)
    return d


def session_version(stub_log):
    """The Claude Code version the session announced in `initialize`."""
    for line in pathlib.Path(stub_log).read_text().splitlines():
        body = json.loads(line)["body"]
        if isinstance(body, dict) and body.get("method") == "initialize":
            return body["params"]["clientInfo"]["version"]
    return None


# An absolute path under /home or /run/user, not already a placeholder.
# extract.py replaces $HOME and a path that starts at /tmp; a TMPDIR under
# /run/user/<uid> it leaves whole (#696's review, F3 and F7).
HOST_PATH = re.compile(r"(?<![\w$.~-])/(?:home|run/user)/[^\s\"'/]+")


def fail_on_host_path(dest):
    """Fail the run, removing what extract.py wrote, if its committed
    files still name a host path: the check every caller of extract.py
    makes after its own replacements, whether or not it scrubs."""
    dest = pathlib.Path(dest)
    for f in ("evidence.md", "stub.jsonl"):
        p = dest / f
        leaked = HOST_PATH.search(p.read_text(errors="replace")) if p.exists() else None
        if leaked:
            shutil.rmtree(dest, ignore_errors=True)
            raise SystemExit(f"{f} still names {leaked.group(0)}...: a host path; no evidence kept")


def distil(raw, name):
    subprocess.run([sys.executable, str(HERE / "extract.py"), str(raw), str(HERE / "runs" / name)], check=True)
    fail_on_host_path(HERE / "runs" / name)
