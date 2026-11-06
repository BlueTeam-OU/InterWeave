# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S30: three of the four carries S29 left that this host can
measure -- another model's context, an event at the bound in a
conversation already holding others, and a burst past ten. The fourth,
the daemon's broadcast drop count, is s30_drops.py's.

    SPIKE_CLAUDE=<claude binary> python3 s30_print.py <run>

Every run is S29's setup (s29_print.py): the plugin installed as
``interweave@interweave`` from the local marketplace, into a scratch
CLAUDE_CONFIG_DIR, under bubblewrap with a private ``/etc`` holding S22's
allow entry, ``--channels`` naming the plugin, ``--allowedTools`` for
``reply`` and ``status``, and one session of

    claude -p --input-format stream-json --output-format stream-json --verbose
           --include-partial-messages

with the first user turn written 5 s after the start. The runs:

  a-<model>-<bytes>   S29's s runs on another model: ``--model
                      claude-<model>-5-5`` (opus or haiku); the plugin's
                      binary is s30_stub.py, which writes one channel
                      event whose line is <bytes> long, the content one
                      letter repeated, once the first turn's result is out
  b-accumulate        ``--model claude-sonnet-5-5``; s30_stub.py writes
                      events of the bound's length (327,680 bytes) with
                      random base64url content, one at a time: event k
                      once the turn event k-1 started has its result,
                      until a result is an error or EVENTS_CAP is reached
  b-accumulate-nocompact
                      as b-accumulate, with ``DISABLE_COMPACT=1`` in the
                      session's environment: a switch read from the
                      binary, not documented, which b's compaction makes
                      the only way to reach a refusal
  c-burst-<n>         S29's h with n messages (50 or 200): the release
                      bridge, S16's two daemons, the far peer a second
                      bridge on daemon B, which sends its n direct
                      messages back to back once the first turn's result
                      is out -- each ``send`` answered before the next is
                      made, a refused one retried 1 s later -- and a tap on
                      the bridge's stdout (s29_tap.py); the first turn
                      asks for one reply each, naming its number

Each result's usage is recorded (``results``), so the prompt's size in
tokens is read beside its bytes. The session is stopped by closing its
stdin, then as S22 stops one. Long strings are elided before extract.py
distils the run, as S29 does. EVIDENCE ONLY.
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
from s16_run import PROFILE, STRANGER, Home, Peer, free_port, private_ipv4
from s19_channels import long_term_token, seed_config
from s22_org_channels import ALLOW, DROP_IN, MARKETPLACE, PLUGIN, TOOL, cli, private_etc, purge_if_token, scrub, \
    stop_group, token_in
from s24_print import CHANNELS, SERVER, Session
from s25_print import timeline
from s29_print import ALLOWED, BOUND, STUB_FIRST, TRAFFIC, bridge_wrapper, elide, elide_line, init_record, \
    strings_with

HERE = pathlib.Path(__file__).resolve().parent
BURST = ("This is a test of the InterWeave channel. Reply now with the single word READY. Later, <channel> tags "
         "will arrive, each with a number after # in its body. For each tag, call the " + TOOL + "reply tool once "
         "with that tag's reply_token and the content ack followed by a space and that number, for example ack 3, "
         "and do nothing else.")
MODELS = {"opus": "claude-opus-5-5", "haiku": "claude-haiku-5-5", "sonnet": "claude-sonnet-5-5"}
# Four of the compaction cycles a probe found every third event.
EVENTS_CAP = 12
TOO_LONG = "Prompt is too long"


def spec_of(name):
    m = re.fullmatch(r"a-(opus|haiku)-(\d+)", name)
    if m:
        return {"kind": "stub", "model": MODELS[m.group(1)], "sizes": [int(m.group(2))], "answer_s": 600}
    if name in ("b-accumulate", "b-accumulate-nocompact"):
        return {"kind": "stub", "model": MODELS["sonnet"], "sizes": [BOUND] * EVENTS_CAP, "filler": "base64url",
                "each": True, "answer_s": 300,
                "env": {"DISABLE_COMPACT": "1"} if name.endswith("nocompact") else {}}
    m = re.fullmatch(r"c-burst-(50|200)", name)
    if m:
        n = int(m.group(1))
        return {"kind": "bridge", "first": BURST, "tap": True,
                "messages": ["S30 {nonce} #" + str(k) + ": hello from the peer" for k in range(1, n + 1)],
                "acks": [f"ack {k}" for k in range(1, n + 1)], "answer_s": 300 if n == 50 else 900}
    return None


class TimedPeer(Peer):
    """s16_run.py's far peer, with the wall clock each line arrived at
    (``at``, beside ``lines``): it reads lines while the driver's sends
    wait for their answers too, so a reply's arrival is not the moment
    the driver next looks."""

    def __init__(self, *args):
        super().__init__(*args)
        self.at = []

    def _read(self, until):
        value = super()._read(until)
        if value is not None:
            self.at.append(time.time())
        return value

    def first_received(self, text):
        return next((at for v, at in zip(self.lines, self.at)
                     if v.get("method") == "notifications/claude/channel" and v["params"]["content"] == text), None)


def stub_wrapper(path, env):
    """The plugin's bin/claude-channel: s30_stub.py, its arguments
    ignored, configured by the environment written here."""
    assigns = " ".join(f"{k}='{v}'" for k, v in env.items())
    path.write_text(f"#!/bin/bash\nexec env {assigns} python3 -I '{HERE / 's30_stub.py'}'\n")
    path.chmod(0o755)


def result_record(at, v):
    usage = v.get("usage") or {}
    return {"at_s": at, "subtype": v.get("subtype"), "is_error": v.get("is_error"),
            "result": (v.get("result") or "")[:300],
            "usage": {k: usage.get(k) for k in ("input_tokens", "cache_creation_input_tokens",
                                                 "cache_read_input_tokens", "output_tokens") if k in usage},
            "model_usage": {m: {k: u.get(k) for k in ("inputTokens", "cacheReadInputTokens",
                                                     "cacheCreationInputTokens", "contextWindow") if k in u}
                            for m, u in (v.get("modelUsage") or {}).items()}}


def burst_entries(transcript, nonce):
    """Per message number, how the session's own transcript took it in:
    every entry whose text carries ``S30 <nonce> #k:``, with its type,
    the operation or attachment kind around it, and its timestamp."""
    pattern = re.compile(rf"S30 {nonce} #(\d+):")
    per = {}
    for line in transcript.read_text().splitlines():
        value = json.loads(line)
        text = json.dumps(value)
        found = sorted({int(k) for k in pattern.findall(text)})
        if not found:
            continue
        kind = value.get("type")
        detail = value.get("operation") or (value.get("attachment") or {}).get("type")
        if kind == "user":
            content = (value.get("message") or {}).get("content")
            detail = detail or ("text" if isinstance(content, str) else
                                ",".join(sorted({c.get("type", "?") for c in content or []})))
        for k in found:
            per.setdefault(k, []).append({"type": kind, "detail": detail, "timestamp": value.get("timestamp"),
                                          "uuid_parent": bool(value.get("parentUuid")), "with": len(found)})
    return per


def main() -> int:
    if len(sys.argv) != 2 or spec_of(sys.argv[1]) is None:
        raise SystemExit(f"usage: SPIKE_CLAUDE=<binary> {sys.argv[0]} "
                         "{a-opus-<bytes>|a-haiku-<bytes>|b-accumulate|b-accumulate-nocompact|c-burst-50|c-burst-200}")
    if not os.environ.get("SPIKE_CLAUDE"):
        raise SystemExit("set SPIKE_CLAUDE: S30 names the build it measures")
    name = sys.argv[1]
    spec = spec_of(name)
    version = claude_version()
    run = "s30-" + name
    bridge_kind = spec["kind"] == "bridge"
    head = subprocess.run(["git", "rev-parse", "--short=9", "HEAD"], capture_output=True, text=True,
                          cwd=REPO).stdout.strip()

    subprocess.run(["cargo", "build", "-q", "-j", "2", "--release", "-p", "interweave-claude-channel"], check=True,
                   cwd=REPO)
    if bridge_kind:
        subprocess.run(["cargo", "build", "-q", "-j", "2", "-p", "interweave-transport-daemon", "-p",
                        "interweave-transportctl"], check=True, cwd=REPO)
    bridge = REPO / "target" / "release" / "claude-channel"
    daemon, ctl = REPO / "target" / "debug" / "transport-daemon", REPO / "target" / "debug" / "transportctl"

    out = raw_dir(run)
    scratch = pathlib.Path(tempfile.mkdtemp(prefix="spike001s30-")).resolve()
    if scratch == REPO or REPO in scratch.parents:
        scratch.rmdir()
        raise SystemExit("TMPDIR must be outside the repository")

    nonce = secrets.token_hex(4)
    live = []
    record = {"run": run, "tree": head, "claude_version": version, "nonce": nonce, "kind": spec["kind"],
              "model_flag": spec.get("model"), "env_set": spec.get("env", {}), "first_turn": spec.get("first", STUB_FIRST)}
    passed = False
    try:
        config, workdir, built, policy = (scratch / d for d in ("config", "work", "marketplace", "policy"))
        for d in (config, workdir, policy):
            d.mkdir()
        path = policy / DROP_IN
        path.parent.mkdir(parents=True)
        path.write_text(json.dumps(ALLOW, indent=2) + "\n")
        record["managed_settings"] = {"path": f"/etc/claude-code/{DROP_IN}", "content": ALLOW}
        bridge_log, tap_log = scratch / "bridge-stderr.log", scratch / "tap.jsonl"
        stub_log, trigger = scratch / "stub.jsonl", scratch / "trigger"
        wrapper = scratch / "claude-channel-wrapper"
        if bridge_kind:
            bridge_wrapper(wrapper, bridge_log, bridge, tap_log if spec.get("tap") else None)
        else:
            record["stub_sizes"], record["stub_filler"] = spec["sizes"], spec.get("filler", "a")
            stub_wrapper(wrapper, {"SPIKE_LOG": stub_log, "SPIKE_TRIGGER": trigger, "SPIKE_NONCE": nonce,
                                   "SPIKE_SIZES": ",".join(map(str, spec["sizes"])),
                                   "SPIKE_FILLER": spec.get("filler", "a"),
                                   **({"SPIKE_EACH": "1"} if spec.get("each") else {})})
        subprocess.run(["bash", str(REPO / "packaging" / "claude-plugin" / "build.sh"), str(built), str(wrapper)],
                       check=True, capture_output=True)

        peer = peers = None
        a_env = {}
        if bridge_kind:
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
            peer = TimedPeer(bridge, b, out / "peer.jsonl")
            live.append(peer)
            a_env = a.env()

        install = [*TRAFFIC] if bridge_kind else []
        record["setup"] = [cli(config, "plugin", "marketplace", "add", str(built)),
                           cli(config, "plugin", "install", f"{PLUGIN}@{MARKETPLACE}", *install)]
        seed_config(config, workdir)

        debug = out / "debug-full.txt"
        io = ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose",
              "--include-partial-messages"]
        flags = [*(["--model", spec["model"]] if spec.get("model") else []), *ALLOWED]
        cmd = [CLAUDE, *io, *flags, "--debug-file", str(debug), *CHANNELS]
        record["command"] = ["claude", *io, *flags, "--debug-file", "$RAW/debug.txt", *CHANNELS]
        record["sandbox"] = "bwrap: the host's /etc entries read-only, /etc/claude-code a scratch directory read-only"
        env = child_env({"CLAUDE_CONFIG_DIR": str(config), **a_env, **spec.get("env", {}), **long_term_token()})
        started = time.time()
        record["started_at"] = time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime(started)) + f".{int(started % 1 * 1000):03d}Z"
        proc = subprocess.Popen(private_etc(policy) + cmd, cwd=workdir, env=env, stdin=subprocess.PIPE,
                                stdout=subprocess.PIPE, stderr=open(out / "stderr.txt", "w"), text=True,
                                start_new_session=True)
        live.append(proc.pid)
        session = Session(proc, started)
        steps = []
        time.sleep(5)
        session.write({"type": "user", "message": {"role": "user", "content": record["first_turn"]}})
        steps.append({"first_turn_written_at_s": round(time.time() - started, 1)})

        attempts = []

        def peer_send(text):
            deadline = time.time() + 120
            while True:
                at = round(time.time() - started, 3)
                answer, error = peer.tool("send", {"peer": peers["a"], "endpoint": "claude", "content": text})
                attempts.append({"at_s": at, "chars": len(text), "answer": answer[:120], "error": error})
                if not error or time.time() > deadline:
                    return not error
                time.sleep(1)

        def results_since(index):
            return [(i, at, v) for i, (at, v) in enumerate(session.lines[index:], index)
                    if v.get("type") == "result"]

        first = session.wait_for(lambda v: v.get("type") == "result", 120)
        steps.append({"first_turn_result_at_s": first[1] if first else None})
        sent = None
        if first and bridge_kind:
            sent = [peer_send(t.format(nonce=nonce)) for t in spec["messages"]]
            steps.append({"peer_sends": len(sent), "peer_sends_accepted": sum(sent),
                          "peer_send_attempts": len(attempts),
                          "peer_send_refused_count": sum(1 for a in attempts if a["error"]),
                          "peer_send_refused": [a for a in attempts if a["error"]][:20],
                          "peer_first_send_at_s": attempts[0]["at_s"] if attempts else None,
                          "peer_last_send_at_s": attempts[-1]["at_s"] if attempts else None})
        elif first and spec.get("each"):
            # One event per turn: the next once this one's turn has its result.
            sent, since, per_event = [], first[0] + 1, []
            for k in range(1, len(spec["sizes"]) + 1):
                pathlib.Path(f"{trigger}.{k}").write_text("go\n")
                at = round(time.time() - started, 1)
                got = session.wait_for(lambda v: v.get("type") == "result", spec["answer_s"], since)
                sent.append(True)
                per_event.append({"event": k, "trigger_at_s": at, "result_at_s": got[1] if got else None,
                                  **(result_record(got[1], got[2]) if got else {"result": None})})
                if not got or got[2].get("is_error") or TOO_LONG in (got[2].get("result") or ""):
                    break
                since = got[0] + 1
            steps.append({"per_event": per_event})
        elif first:
            trigger.write_text("go\n")
            sent = [True]
            steps.append({"trigger_at_s": round(time.time() - started, 1)})

        acks = {k: None for k in spec.get("acks", [])}
        exited, last_result, deadline = None, None, time.time() + spec["answer_s"]
        cap = deadline + spec["answer_s"]
        while sent is not None and not spec.get("each") and time.time() < min(deadline, cap):
            if peer:
                peer.drain(0.2)
                for k in acks:
                    if acks[k] is None:
                        at = peer.first_received(k)
                        acks[k] = None if at is None else round(at - started, 1)
                if acks and all(acks.values()):
                    break
            else:
                time.sleep(1)
            if exited is None and proc.poll() is not None:
                exited = round(time.time() - started, 1)
                deadline = min(deadline, time.time() + 10)
            results = [at for _, at, _ in results_since(first[0] + 1)]
            if results and results[-1] != last_result:
                last_result = results[-1]
                deadline = time.time() + 20
        if acks:
            got = [v for v in acks.values() if v is not None]
            steps.append({"acks_received": len(got), "acks_expected": len(acks),
                          "first_ack_at_s": min(got) if got else None, "last_ack_at_s": max(got) if got else None,
                          "acks_missing": [k for k, v in acks.items() if v is None][:50]})
        steps.append({"session_exited_by_s": exited, "last_result_after_send_at_s": last_result})
        record["steps"] = steps
        if peer:
            received = [v["params"]["content"] for v in peer.lines
                        if v.get("method") == "notifications/claude/channel"]
            record["peer_received_count"] = len(received)
            record["peer_received_distinct"] = len(set(received))
            record["peer_received"] = received[:20]

        if not proc.stdin.closed:
            proc.stdin.close()
        try:
            proc.wait(timeout=30)
            record["exit_status"] = proc.returncode
        except subprocess.TimeoutExpired:
            record["exit_status"] = None
        record["exited_by_s"] = round(time.time() - started, 1)
        record["shutdown"] = stop_group(proc.pid, lambda s: time.sleep(s))
        passed = first is not None and sent is not None and record["shutdown"]["group_empty"]
        live.remove(proc.pid)
        session.thread.join(5)
        lines = [(at, elide(v)) for at, v in session.lines]
        (out / "transcript.txt").write_text("".join(json.dumps(v) + "\n" for _, v in lines))
        record["timeline"] = timeline(lines)[:400]
        record["inits"] = init_record(session.lines)
        record["results"] = [result_record(at, v) for at, v in session.lines if v.get("type") == "result"]
        record["compact_boundaries_stdout_at_s"] = [at for at, v in session.lines
                                                    if v.get("subtype") == "compact_boundary"]

        full = debug.read_text(errors="replace") if debug.exists() else ""
        (out / "debug.txt").write_text("".join(elide_line(l) + "\n" for l in full.splitlines()))
        debug.unlink(missing_ok=True)
        mine = [l for l in full.splitlines() if SERVER in l]
        record["server_error_lines"] = [elide_line(l[25:]) for l in mine
                                        if re.search(r"rror|xceed|too (?:long|large)|closed|disconnect", l)][:40]
        notified = [l[:24] for l in mine if "notifications/claude/channel" in l]
        record["notification_lines"] = len(notified)
        record["notification_lines_at"] = notified[:40]
        # The host's compaction decisions, with their times; the token
        # count in each line is redacted by the build itself.
        record["autocompact_lines"] = [l[:24] + " " + l[33:].strip()[:160] for l in full.splitlines()
                                       if "autocompact:" in l or "[reactive-compact] finished" in l][:80]
        record["too_long_lines"] = [elide_line(l)[:300] for l in full.splitlines() if TOO_LONG in l][:10]
        if bridge_log.exists():
            record["bridge_stderr_lines"] = len(bridge_log.read_text(errors="replace").splitlines())
        if tap_log.exists():
            tap = [{"at_s": round(e.pop("wall") - started, 3), **e}
                   for e in map(json.loads, tap_log.read_text().splitlines())]
            record["tap_lines"] = len(tap)
            record["tap"] = tap[:20] + (tap[-5:] if len(tap) > 20 else [])
        if stub_log.exists():
            record["stub"] = [{"at_s": round(e["t"] / 1000, 3), "dir": e["dir"], "bytes": e.get("bytes"),
                               "body": e["body"] if e["dir"] == "event" else
                               (e["body"] or {}).get("method") or ("id " + str((e["body"] or {}).get("id"))),
                               **({"content_chars": e["body"]["content_chars"]}
                                  if isinstance(e["body"], dict) and "content_chars" in e["body"] else {})}
                              for e in map(json.loads, stub_log.read_text().splitlines())]
        record["env_passed"] = sorted(child_env({"CLAUDE_CONFIG_DIR": "", **{k: "" for k in a_env}, **spec.get("env", {}),
                                                 "CLAUDE_CODE_OAUTH_TOKEN": ""}))
        transcripts = sorted((config / "projects").glob("*/*.jsonl"))
        if transcripts:
            if bridge_kind:
                per = burst_entries(transcripts[-1], nonce)
                record["burst_entries"] = {str(k): per[k] for k in sorted(per)}
                # How each message entered: a user message of its own starts
                # a turn; a queued_command attachment joins the running one.
                first_of = {k: next((e for e in v if e["type"] in ("user", "attachment")), None)
                            for k, v in per.items()}
                record["burst_summary"] = {
                    "seen_in_transcript": len(per),
                    "started_a_turn": sorted(k for k, e in first_of.items() if e and e["type"] == "user"),
                    "joined_a_running_turn": sum(1 for e in first_of.values()
                                                 if e and e["detail"] == "queued_command"),
                    "neither": sorted(k for k, e in first_of.items() if not e or (
                        e["type"] != "user" and e["detail"] != "queued_command")),
                    "missing": [k for k in range(1, len(spec["messages"]) + 1) if k not in per]}
            entries = [json.loads(l) for l in transcripts[-1].read_text().splitlines()]
            record["compact_boundaries"] = [
                {"timestamp": v.get("timestamp"),
                 **{k: (v.get("compactMetadata") or {}).get(k)
                    for k in ("trigger", "preTokens", "postTokens", "cumulativeDroppedTokens", "durationMs")}}
                for v in entries if v.get("subtype") == "compact_boundary"]
            kept = [json.dumps(elide(v)) for v in entries]
            (out / "session.jsonl").write_text("\n".join(kept) + "\n")
        if peer:
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
    (out / "run.json").write_text(json.dumps(elide(record), indent=2) + "\n")
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
    print(f"{run}: steps={json.dumps(elide(record['steps']))[:1500]} passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
