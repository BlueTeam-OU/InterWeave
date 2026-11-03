# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S27: the organisation-policy cases S22 left not measured.

    python3 s27_policy.py <run>

Every run is one print session on the release bridge installed as
``interweave@interweave`` from the local marketplace (S22's install),
with no daemon and no traffic. It answers one question, the stream-json
control request ``channel_enable`` (S24's g), whose answer carries the
gate's verdict in words, since a print session logs no skipped channel
(fact 42):

    claude -p --input-format stream-json --output-format stream-json --verbose
           --channels plugin:interweave@interweave

The runs differ in where the managed settings are, what they hold, and
the login the session has:

  a-container-etc      a podman container of the run (Fedora 43): its own
                       /etc/claude-code/managed-settings.json, written by the
                       container's root, S22's allow entry; the session runs
                       as this login's uid inside it
  b-container-none     as a, no managed settings (control)
  c-bwrap-etc          S22's private /etc on the host (bubblewrap, a scratch
                       directory read-only at /etc/claude-code), the same file
                       at managed-settings.json (S22's e, in print)
  d-enabled-false      as c, with "channelsEnabled": false; this login's
                       claude.ai token (S22's c, in print)
  e-key-enabled-false  as d, the session given a PLACEHOLDER API key, which
                       authenticates nothing, in place of the token: a login
                       that is not claude.ai, as far as the gate reads one
  f-key-enabled-true   as e, with S22's allow entry (control for e)
  g-key-no-enabled     as e, with allowedChannelPlugins alone, no
                       channelsEnabled

No user turn is written: the gate answers the control request with no
model call, which is what lets the placeholder key reach it. The session
is stopped by closing its stdin, then as S22 stops one; a container is
then removed. The token, the raw directory and the evidence are handled
as in S22. EVIDENCE ONLY.
"""

import json
import os
import pathlib
import re
import secrets
import shutil
import subprocess
import sys
import tempfile
import time

from spike_common import CLAUDE, REPO, child_env, claude_version, raw_dir
from s19_channels import long_term_token, seed_config
from s22_org_channels import (ALLOW, MARKETPLACE, PLUGIN, cli, private_etc, purge_if_token, scrub, stop_group,
                              token_in)
from s24_print import CHANNELS, SERVER, Session

HERE = pathlib.Path(__file__).resolve().parent
IMAGE = "registry.fedoraproject.org/fedora:43"
FILE = "managed-settings.json"
# Not a credential: a string in the shape of an API key that no service
# issued, so a session holding it is an API-key login that can make no
# model call. It is committed in the evidence on purpose.
PLACEHOLDER_KEY = "sk-ant-api03-spike001-placeholder-not-a-key"
OFF = {**ALLOW, "channelsEnabled": False}
LIST_ONLY = {"allowedChannelPlugins": ALLOW["allowedChannelPlugins"]}
RUNS = {
    "a-container-etc": {"where": "container", "settings": ALLOW, "login": "token"},
    "b-container-none": {"where": "container", "settings": None, "login": "token"},
    "c-bwrap-etc": {"where": "bwrap", "settings": ALLOW, "login": "token"},
    "d-enabled-false": {"where": "bwrap", "settings": OFF, "login": "token"},
    "e-key-enabled-false": {"where": "bwrap", "settings": OFF, "login": "key"},
    "f-key-enabled-true": {"where": "bwrap", "settings": ALLOW, "login": "key"},
    "g-key-no-enabled": {"where": "bwrap", "settings": LIST_ONLY, "login": "key"},
}
# What the session says about the organisation's settings, wherever it
# says it: the server-managed fetch, the MDM tier, the policy file.
POLICY_LINES = re.compile(r"remote managed|remote settings|managed settings|managed-settings|policySettings|mdm",
                          re.IGNORECASE)


def podman(*args, **kwargs):
    return subprocess.run(["podman", *args], capture_output=True, text=True, **kwargs)


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in RUNS:
        raise SystemExit(f"usage: {sys.argv[0]} {{{'|'.join(RUNS)}}}")
    if os.environ.get("SPIKE_CLAUDE"):
        raise SystemExit("unset SPIKE_CLAUDE: S27 measures the claude on PATH")
    name = sys.argv[1]
    spec = RUNS[name]
    run = "s27-" + name

    subprocess.run(["cargo", "build", "-q", "-j", "2", "--release", "-p", "interweave-claude-channel"], check=True,
                   cwd=REPO)
    bridge = REPO / "target" / "release" / "claude-channel"
    # The container is given the binary itself, not the launcher on PATH.
    binary = pathlib.Path(shutil.which(CLAUDE)).resolve()

    out = raw_dir(run)
    scratch = pathlib.Path(tempfile.mkdtemp(prefix="spike001s27-")).resolve()
    if scratch == REPO or REPO in scratch.parents:
        scratch.rmdir()
        raise SystemExit("TMPDIR must be outside the repository")

    container = f"spike001-{run}-{secrets.token_hex(3)}"
    record = {"run": run, "claude_version": claude_version(), "where": spec["where"], "login": spec["login"]}
    proc, started_container, passed = None, False, False
    try:
        config, workdir, built, policy, home, xdg = (scratch / d for d in ("config", "work", "marketplace", "policy",
                                                                            "home", "xdg"))
        for d in (config, workdir, policy, home, xdg):
            d.mkdir()
        xdg.chmod(0o700)
        subprocess.run(["bash", str(REPO / "packaging" / "claude-plugin" / "build.sh"), str(built), str(bridge)],
                       check=True, capture_output=True)
        record["setup"] = [cli(config, "plugin", "marketplace", "add", str(built)),
                           cli(config, "plugin", "install", f"{PLUGIN}@{MARKETPLACE}")]
        seed_config(config, workdir)

        debug = out / "debug.txt"
        io = ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose"]
        args = [*io, "--debug-file", str(debug), *CHANNELS]
        record["command"] = ["claude", *io, "--debug-file", "$RAW/debug.txt", *CHANNELS]
        login = long_term_token() if spec["login"] == "token" else {"ANTHROPIC_API_KEY": PLACEHOLDER_KEY}
        if spec["login"] == "key":
            record["api_key"] = PLACEHOLDER_KEY
        env = child_env({"CLAUDE_CONFIG_DIR": str(config), **login})
        record["managed_settings"] = ({"path": f"/etc/claude-code/{FILE}", "content": spec["settings"]}
                                      if spec["settings"] else None)

        if spec["where"] == "container":
            # Pulled first, so the digest recorded is the image that runs:
            # an inspect of an image not yet local answers nothing.
            pulled = podman("pull", "-q", IMAGE)
            image = podman("image", "inspect", "--format", "{{.Digest}}", IMAGE)
            digest = image.stdout.strip()
            if pulled.returncode or image.returncode or not digest.startswith("sha256:"):
                raise SystemExit(f"no digest for {IMAGE}: {(pulled.stderr + image.stderr).strip()}")
            record["image"] = {"name": IMAGE, "digest": digest}
            # keep-id: this login's uid is the same inside, so the scratch
            # tree it owns is writable there; uid 0 inside is not root here.
            mounts = []
            for path, mode in ((binary, "ro"), (scratch, "rw"), (out, "rw")):
                mounts += ["-v", f"{path}:{path}:{mode}"]
            started_container = True
            # label=disable: under an enforcing SELinux host the container
            # may not execute the bind-mounted binary ("Permission denied"),
            # and relabelling it (:z) would change the host's own file.
            created = podman("run", "-d", "--rm", "--name", container, "--userns=keep-id",
                             "--security-opt", "label=disable", *mounts, IMAGE, "sleep", "infinity")
            record["container_run"] = {"exit": created.returncode, "stderr": created.stderr.strip()[-300:]}
            if created.returncode:
                raise SystemExit(f"podman run failed: {created.stderr.strip()}")
            if spec["settings"]:
                wrote = podman("exec", "-i", "--user", "0", container, "sh", "-c",
                               f"mkdir -p /etc/claude-code && cat > /etc/claude-code/{FILE} && "
                               f"chmod 644 /etc/claude-code/{FILE}",
                               input=json.dumps(spec["settings"], indent=2) + "\n")
                record["write_as_root"] = {"exit": wrote.returncode, "stderr": wrote.stderr.strip()}
            # What the session will read, as the container sees it: whether
            # /etc/claude-code is a mount of its own (S22's was) or a
            # directory of the container's own /etc.
            seen = podman("exec", container, "sh", "-c",
                          "id -u; stat -c '%u %a %F %n' /etc /etc/claude-code /etc/claude-code/* 2>&1; "
                          "grep -c ' /etc/claude-code ' /proc/self/mountinfo; "
                          "grep ' /etc ' /proc/self/mountinfo | wc -l")
            record["container_etc"] = seen.stdout.strip().splitlines()
            names = ["CLAUDE_CONFIG_DIR", *login, "DISABLE_AUTOUPDATER", "LANG", "PATH"]
            # What the session inside receives: the named variables and the
            # two set here, not the podman client's environment.
            passed_names = sorted({*names, "HOME", "XDG_RUNTIME_DIR"} - set(login)) + sorted(login)
            cmd = ["podman", "exec", "-i", "--user", str(os.getuid()), "-w", str(workdir), "-e", f"HOME={home}",
                   # A container has no login session, and without a runtime
                   # directory the bridge serves its refusal, which declares
                   # no channel (measured on the first attempt of a).
                   "-e", f"XDG_RUNTIME_DIR={xdg}",
                   *[x for k in names for x in ("-e", k)], container, str(binary), *args]
            record["sandbox"] = (f"podman container of {IMAGE}: its own /etc, the policy file written by the "
                                 "container's root; the session as this login's uid (keep-id)")
            env = {**env, "PATH": "/usr/local/bin:/usr/bin:/bin"}
        else:
            if spec["settings"]:
                (policy / FILE).write_text(json.dumps(spec["settings"], indent=2) + "\n")
            cmd = private_etc(policy) + [CLAUDE, *args]
            record["sandbox"] = ("bwrap: the host's /etc entries read-only, /etc/claude-code a scratch directory "
                                 "read-only")

        started = time.time()
        record["started_at"] = time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime(started)) + f".{int(started % 1 * 1000):03d}Z"
        proc = subprocess.Popen(cmd, cwd=workdir, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                stderr=open(out / "stderr.txt", "w"), text=True, start_new_session=True)
        session = Session(proc, started)
        steps = []
        # As S24: a control request before the server connects is answered
        # "not connected", and is retried.
        time.sleep(5)
        answer, deadline, tries = None, time.time() + 40, 0
        while time.time() < deadline:
            tries += 1
            rid = secrets.token_hex(6)
            session.write({"type": "control_request", "request_id": rid,
                           "request": {"subtype": "channel_enable", "serverName": SERVER}})
            got = session.wait_for(lambda v: v.get("type") == "control_response"
                                   and (v.get("response") or {}).get("request_id") == rid, 15)
            answer = got
            if not got or "not connected" not in json.dumps(got[2]):
                break
            time.sleep(2)
        steps.append({"channel_enable": answer[2]["response"] if answer else None,
                      "at_s": answer[1] if answer else None, "tries": tries})
        # The registration line, where the answer was success, follows it.
        time.sleep(3)
        record["steps"] = steps
        passed = answer is not None

        if not proc.stdin.closed:
            proc.stdin.close()
        try:
            proc.wait(timeout=30)
            record["exit_after_stdin_close"] = proc.returncode
        except subprocess.TimeoutExpired:
            record["exit_after_stdin_close"] = None
        record["shutdown"] = stop_group(proc.pid, lambda s: time.sleep(s))
        passed = passed and record["shutdown"]["group_empty"]
        session.thread.join(5)
        (out / "transcript.txt").write_text("".join(json.dumps(v) + "\n" for _, v in session.lines))

        log = debug.read_text(errors="replace") if debug.exists() else ""
        record["channel_lines"] = [l[25:] for l in log.splitlines() if "Channel" in l and SERVER in l]
        record["policy_lines"] = [l[25:][:300] for l in log.splitlines() if POLICY_LINES.search(l)][:20]
        stderr = (out / "stderr.txt").read_text(errors="replace")
        record["stderr_head"] = stderr.splitlines()[:10]
        record["env_passed"] = (passed_names if spec["where"] == "container"
                                else sorted(k for k in env if k not in login) + sorted(login))
    finally:
        if proc is not None and proc.poll() is None:
            stop_group(proc.pid, lambda s: time.sleep(s), steps=(("group", 9, 5),))
        if started_container:
            removed = podman("rm", "-f", "-t", "5", container)
            gone = podman("container", "exists", container)
            record["container_removed"] = {"exit": removed.returncode, "exists_after": gone.returncode == 0}
            passed = passed and gone.returncode != 0
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
    print(f"{run}: steps={json.dumps(record['steps'])} channel={record['channel_lines']} passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
