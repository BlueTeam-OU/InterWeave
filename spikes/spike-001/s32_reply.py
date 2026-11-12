# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S32 (b): when a peer's message alone gets an answer through
``reply`` (the README's "Not established", facts 68 and 71).

    SPIKE_CLAUDE=<claude binary> python3 s32_reply.py <told>-<message>-<arrival>-<N>
    python3 s32_reply.py table     # the committed runs, per cell, and tokens spent

Every run is S31's d setup (s31_print.py): the plugin installed as
``interweave@interweave`` from the local marketplace, into a scratch
CLAUDE_CONFIG_DIR, under bubblewrap with a private ``/etc`` holding S22's
allow entry, ``--channels`` naming the plugin, ``--allowedTools`` for
``reply`` and ``status``, ``--model claude-sonnet-5-5`` (the tier S31
ran on), and one session of

    claude -p --input-format stream-json --output-format stream-json --verbose
           --include-partial-messages

with the first user turn written 5 s after the start. The plugin's binary
is s32_reply_stub.py, which answers ``initialize`` with the bridge's own
instructions and ``tools/list`` with the bridge's seven push-mode tools,
both read from ``apps/claude-channel/src/mcp.rs`` when the run starts
(``bridge_surface``), and writes one direct message with the bridge's
meta when the driver creates its trigger. No run has daemons. A cell:

  <told>     what the session was told at start:
               ``none``    nothing about events (S28's and S29's e)
               ``prompt``  the first turn says peer messages arrive and are
                           to be answered with ``reply``
               ``promptw`` ``prompt``, its work saying "write" where
                           ``prompt``'s says "reply" (``WORK_WRITE``);
                           ``text``'s work says "write" in both
               ``plugin``  the same sentence appended to the server's
                           instructions, the first turn as ``none``'s;
                           ``plugin.json`` carries no instructions of its
                           own, so the server's are the plugin's
  <message>  ``greet``, a greeting (S29's); ``question``, a question with a
             one-word answer; ``task``, a request to call ``status`` and
             report what it says
  <arrival>  fact 84's three situations: ``tool``, written at the first
             turn's first ``status`` call, the turn calling it three times,
             each answered after 3 s; ``text``, written at the first
             turn's first text delta, the turn writing the numbers one to
             a hundred; ``idle``, written once the first turn's ``result``
             is out, stdin open

N is the run's number in its cell. A reply is a ``reply`` call whose
``reply_token`` is the event's, read from the stub's own record; its time
is from the stub's write of the event to its receipt of the call. The
session is stopped by closing its stdin once no ``result`` has come for
25 s after the event, at most 180 s after it. Long strings are elided
before extract.py distils the run, as S29 does. EVIDENCE ONLY.
"""

import hashlib
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

from s28_era import tree_stamp
from spike_common import CLAUDE, REPO, child_env, claude_version, raw_dir
from s19_channels import long_term_token, seed_config
from s22_org_channels import ALLOW, DROP_IN, MARKETPLACE, PLUGIN, TOOL, cli, private_etc, purge_if_token, scrub, \
    stop_group, token_in
from s24_print import CHANNELS, SERVER, Session
from s25_print import timeline
from s29_print import ALLOWED, elide, elide_line, init_record
from s30_print import MODELS, result_record
from s31_print import dashed, is_text_delta, is_tool_use

HERE = pathlib.Path(__file__).resolve().parent
MCP_RS = REPO / "apps" / "claude-channel" / "src" / "mcp.rs"
PUSH_TOOLS = ("broadcast", "send", "reply", "join", "leave", "identity", "status")
TOLD = ("none", "prompt", "promptw", "plugin")
MESSAGES = {
    "greet": "S32 {nonce}: hello from the peer on daemon B",
    "question": "S32 {nonce}: which city is the capital of Australia?",
    "task": "S32 {nonce}: please call your status tool and tell me what it says about the transport.",
}
ARRIVALS = ("tool", "text", "idle")
# One sentence, the same in the prompt and in the instructions, so the
# two cells differ only in where it is.
ANSWER = ("Messages from peers on the InterWeave transport arrive as <channel> tags; answer each one by calling "
          "reply with its reply_token.")
WORK = {
    "tool": "Call the " + TOOL + "status tool three times, one call after another, then reply with the single "
            "word READY.",
    "text": "Write the numbers from one to one hundred in words, separated by commas, then the single word READY.",
    "idle": "Reply now with the single word READY.",
}
# The same work with "write" for "reply": in prompt's cells the work's
# "reply with READY" beside ANSWER's reply tool sent READY to the peer.
WORK_WRITE = {
    "tool": WORK["tool"].replace("then reply with", "then write"),
    "text": WORK["text"],
    "idle": "Write the single word READY now.",
}
TOOL_DELAY_MS = 3000
SETTLE_S, AFTER_EVENT_S = 25, 180


def rust_str(literal):
    """A Rust string literal's body as the compiler reads it, for the
    escapes mcp.rs uses: a backslash-newline continuation (the newline
    and the next line's leading whitespace dropped), ``\\n`` and ``\\"``."""
    text = re.sub(r"\\\n\s*", "", literal)
    text = text.replace("\\n", "\n").replace('\\"', '"')
    if "\\" in text:
        raise SystemExit("mcp.rs: an escape bridge_surface does not read")
    return text


def bridge_surface():
    """The bridge's instructions and its push-mode tools/list, read from
    its source, so the stub shows the model what the bridge does and a
    change to either is measured, not copied stale."""
    source = MCP_RS.read_text()
    m = re.search(r'pub const INSTRUCTIONS: &str = "((?:[^"\\]|\\.)*)";', source, re.S)
    if not m:
        raise SystemExit("mcp.rs: INSTRUCTIONS not found")
    instructions = rust_str(m.group(1))
    tools = []
    arm = re.compile(r'ToolName::(\w+) => \(\s*"((?:[^"\\]|\\.)*)",\s*json!\((\{.*?\})\),\s*&\[(.*?)\],\s*\)', re.S)
    for name, description, props, required in arm.findall(source):
        name = name.lower()
        if name not in PUSH_TOOLS:
            continue
        props = re.sub(r'string\("((?:[^"\\]|\\.)*)"\)', r'{"type": "string", "description": "\1"}', props)
        tools.append({"name": name, "description": rust_str(description),
                      "inputSchema": {"type": "object", "properties": json.loads(props),
                                      "required": json.loads("[" + required + "]"),
                                      "additionalProperties": False}})
    if tuple(t["name"] for t in tools) != PUSH_TOOLS:
        raise SystemExit(f"mcp.rs: push tools read as {[t['name'] for t in tools]}")
    return {"instructions": instructions, "tools": tools}


def spec_of(name):
    m = re.fullmatch(rf"({'|'.join(TOLD)})-({'|'.join(MESSAGES)})-({'|'.join(ARRIVALS)})-([1-9]\d?)", name)
    if not m:
        return None
    told, message, arrival = m.group(1), m.group(2), m.group(3)
    work = (WORK_WRITE if told == "promptw" else WORK)[arrival]
    first = "This is a test. " + (ANSWER + " " if told in ("prompt", "promptw") else "") + work
    return {"told": told, "message": message, "arrival": arrival, "first": first,
            "extra_instructions": ANSWER if told == "plugin" else None}


def stub_record(path, started):
    if not path.exists():
        return []
    out = []
    for e in map(json.loads, path.read_text().splitlines()):
        body = e["body"]
        if e["dir"] == "in" and isinstance(body, dict) and body.get("method") == "ping":
            continue
        # The surface is the run's server_sha256, from mcp.rs at its tree;
        # each run repeating it whole would only bury the calls.
        result = body.get("result") if isinstance(body, dict) else None
        if e["dir"] == "out" and isinstance(result, dict) and ("tools" in result or "instructions" in result):
            body = {"id": body.get("id"), "result": "tools/list" if "tools" in result else "initialize"}
        out.append({"at_s": round(e["wall"] - started, 3), "dir": e["dir"], "body": body})
    return out


def calls_of(stub):
    """Each tools/call the stub received: its name, arguments and time,
    and the refusal the stub answered it with, if it refused it."""
    refused = {e["body"]["id"]: e["body"]["error"]["message"] for e in stub
               if e["dir"] == "out" and isinstance(e["body"], dict) and "error" in e["body"]}
    return [{"at_s": e["at_s"], "name": (e["body"].get("params") or {}).get("name"),
             "arguments": (e["body"].get("params") or {}).get("arguments"),
             "refused": refused.get(e["body"].get("id"))}
            for e in stub if e["dir"] == "in" and isinstance(e["body"], dict)
            and e["body"].get("method") == "tools/call"]


def event_of(stub):
    """The event the stub wrote: when, and its reply_token."""
    for e in stub:
        if e["dir"] == "out" and isinstance(e["body"], dict) and \
                e["body"].get("method") == "notifications/claude/channel":
            return e["at_s"], e["body"]["params"]["meta"]["reply_token"]
    return None, None


def texts_after(lines, at):
    """What the model wrote, in text, from the event's second on."""
    out = []
    for t, v in lines:
        if t < at or v.get("type") != "assistant":
            continue
        for c in (v.get("message") or {}).get("content") or []:
            if c.get("type") == "text" and c.get("text", "").strip():
                out.append({"at_s": t, "text": c["text"][:800]})
    return out


def entries_of(transcript, nonce):
    """How the session's transcript took the event in: each entry carrying
    ``S32 <nonce>``, with its type, the operation or attachment kind and
    its timestamp (s31_print.py's entries_of)."""
    out = []
    for line in transcript.read_text().splitlines():
        value = json.loads(line)
        if f"S32 {nonce}" not in json.dumps(value):
            continue
        detail = value.get("operation") or (value.get("attachment") or {}).get("type") or value.get("subtype")
        out.append({"type": value.get("type"), "detail": detail, "timestamp": value.get("timestamp")})
    return out


def run_session(spec, scratch, out, config, workdir, policy, record):
    trigger, stub_log = scratch / "trigger", scratch / "stub.jsonl"
    debug = out / "debug-full.txt"
    io = ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose",
          "--include-partial-messages"]
    flags = ["--model", MODELS["sonnet"], *ALLOWED]
    cmd = [CLAUDE, *io, *flags, "--debug-file", str(debug), *CHANNELS]
    record["command"] = ["claude", *io, *flags, "--debug-file", "$RAW/debug.txt", *CHANNELS]
    env = child_env({"CLAUDE_CONFIG_DIR": str(config), **long_term_token()})
    started = time.time()
    record["started_at"] = time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime(started)) + f".{int(started % 1 * 1000):03d}Z"
    proc = subprocess.Popen(private_etc(policy) + cmd, cwd=workdir, env=env, stdin=subprocess.PIPE,
                            stdout=subprocess.PIPE, stderr=open(out / "stderr.txt", "w"), text=True,
                            start_new_session=True)
    session = Session(proc, started)
    steps, ok, fired_at = [], False, None

    def at():
        return round(time.time() - started, 1)

    def fire():
        nonlocal fired_at
        pathlib.Path(f"{trigger}.1").write_text("go\n")
        fired_at = at()
        steps.append({"trigger": 1, "at_s": fired_at})

    def settle():
        """Until no turn is running and no result has come for SETTLE_S
        since the event, at most AFTER_EVENT_S after it; a result before
        the event does not count. A turn is running from its init to its
        result: closing stdin under one interrupts it, which the first
        version of this rule did to one turn slow to answer."""
        end, last_at = time.time() + AFTER_EVENT_S, None
        while time.time() < end and proc.poll() is None:
            kinds = [(t, v.get("type"), v.get("subtype")) for t, v in session.lines]
            after = [t for t, k, _ in kinds if k == "result" and t >= fired_at]
            if after:
                last_at = after[-1]
            starts = [i for i, (_, k, s) in enumerate(kinds) if k == "system" and s == "init"]
            ends = [i for i, (_, k, _) in enumerate(kinds) if k == "result"]
            running = bool(starts) and (not ends or starts[-1] > ends[-1])
            if last_at is not None and not running and at() - last_at > SETTLE_S:
                break
            time.sleep(0.5)

    try:
        time.sleep(5)
        session.write({"type": "user", "message": {"role": "user", "content": spec["first"]}})
        steps.append({"first_turn_written_at_s": at()})
        if spec["arrival"] in ("tool", "text"):
            test = (lambda v: is_tool_use(v, "status")) if spec["arrival"] == "tool" else is_text_delta
            seen = session.wait_for(test, 120)
            steps.append({"running_turn_signal_at_s": seen[1] if seen else None, "signal": spec["arrival"]})
            if seen:
                fire()
            first = session.wait_for(lambda v: v.get("type") == "result", 180)
            steps.append({"first_turn_result_at_s": first[1] if first else None})
            # The event must have come while the turn ran, or the cell is
            # not the one named.
            ok = bool(seen and first and first[1] >= fired_at)
        else:
            first = session.wait_for(lambda v: v.get("type") == "result", 120)
            steps.append({"first_turn_result_at_s": first[1] if first else None})
            if first:
                fire()
            ok = first is not None
        if fired_at is not None:
            settle()
    finally:
        if not proc.stdin.closed:
            proc.stdin.close()
        try:
            proc.wait(timeout=30)
            record["exit_status"] = proc.returncode
        except subprocess.TimeoutExpired:
            record["exit_status"] = None
        record["exited_by_s"] = round(time.time() - started, 1)
        record["shutdown"] = stop_group(proc.pid, lambda s: time.sleep(s))
        session.thread.join(5)
    record["steps"] = steps
    lines = [(t, elide(v)) for t, v in session.lines]
    (out / "transcript.txt").write_text("".join(json.dumps(v) + "\n" for _, v in lines))
    record["timeline"] = timeline(lines)[:400]
    record["inits"] = init_record(session.lines)
    record["results"] = [result_record(t, v) for t, v in session.lines if v.get("type") == "result"]
    full = debug.read_text(errors="replace") if debug.exists() else ""
    (out / "debug.txt").write_text("".join(elide_line(l) + "\n" for l in full.splitlines()))
    debug.unlink(missing_ok=True)
    mine = [l for l in full.splitlines() if SERVER in l]
    record["notification_lines_at"] = [l[:24] for l in mine if "notifications/claude/channel" in l][:10]
    stub = stub_record(stub_log, started)
    record["stub"] = stub
    calls = calls_of(stub)
    event_at, token = event_of(stub)
    # A reply is one the bridge would have taken: a refused call reached
    # no peer.
    replies = [c for c in calls if c["name"] == "reply" and not c["refused"]
               and (c["arguments"] or {}).get("reply_token") == token]
    record["outcome"] = {
        "event_written_at_s": event_at,
        "replied": bool(replies),
        "reply_after_s": round(replies[0]["at_s"] - event_at, 3) if replies else None,
        "reply_content": (replies[0]["arguments"] or {}).get("content") if replies else None,
        "reply_calls": len(replies),
        "calls_after_event": [{"name": c["name"], "at_s": c["at_s"],
                               **({"refused": c["refused"]} if c["refused"] else {})} for c in calls
                              if event_at is not None and c["at_s"] >= event_at],
        "text_after_event": texts_after(session.lines, fired_at) if fired_at is not None else [],
        "event_turns": sum(1 for t, v in session.lines if v.get("type") == "system"
                           and v.get("subtype") == "init" and fired_at is not None and t >= fired_at),
    }
    transcripts = sorted((config / "projects").glob("*/*.jsonl"), key=lambda p: p.stat().st_mtime)
    if transcripts:
        latest = transcripts[-1]
        record["event_entries"] = entries_of(latest, record["nonce"])
        entries = [json.loads(l) for l in latest.read_text().splitlines()]
        (out / "session.jsonl").write_text("\n".join(json.dumps(elide(v)) for v in entries) + "\n")
    return ok, record


def committed_runs():
    """Each committed run's record, read back from its evidence.md."""
    for path in sorted((HERE / "runs").glob("s32-reply-*/evidence.md")):
        m = re.search(r"## Run\n\n```json\n(.*?)\n```", path.read_text(), re.S)
        if m:
            yield json.loads(m.group(1))


def table():
    """Per cell: runs, replies, the median seconds from the event to the
    reply, and the tokens the cell's sessions spent (every result's
    input, cache creation, cache read and output tokens)."""
    cells, spent = {}, {"input": 0, "cache_creation": 0, "cache_read": 0, "output": 0}
    for r in committed_runs():
        if not r.get("passed"):
            continue
        c = cells.setdefault(r["cell"], {"runs": 0, "replies": 0, "after": [], "tokens": 0})
        c["runs"] += 1
        c["replies"] += bool(r["outcome"]["replied"])
        if r["outcome"]["reply_after_s"] is not None:
            c["after"].append(r["outcome"]["reply_after_s"])
        for res in r["results"]:
            u = res.get("usage") or {}
            for k in spent:
                n = u.get(f"{k}_input_tokens" if k in ("cache_creation", "cache_read") else f"{k}_tokens") or 0
                spent[k] += n
                c["tokens"] += n
    print("| Cell (told-message-arrival) | Runs | Replies | Median s to reply | Tokens |")
    print("|---|---|---|---|---|")
    order = {k: i for i, k in enumerate((*TOLD, *MESSAGES, *ARRIVALS))}
    for cell in sorted(cells, key=lambda s: [order[p] for p in s.split("-")]):
        c = cells[cell]
        after = sorted(c["after"])
        median = (after[len(after) // 2] if len(after) % 2 else (after[len(after) // 2 - 1] + after[len(after) // 2]) / 2) \
            if after else None
        print(f"| `{cell}` | {c['runs']} | {c['replies']} | {'-' if median is None else f'{median:.1f}'} | "
              f"{c['tokens']:,} |")
    print(f"\ntokens: {json.dumps(spent)} total {sum(spent.values()):,}")


def main() -> int:
    if sys.argv[1:] == ["table"]:
        table()
        return 0
    if len(sys.argv) != 2 or spec_of(sys.argv[1]) is None:
        raise SystemExit(f"usage: SPIKE_CLAUDE=<binary> {sys.argv[0]} "
                         f"<{'|'.join(TOLD)}>-<{'|'.join(MESSAGES)}>-<{'|'.join(ARRIVALS)}>-<N>")
    if not os.environ.get("SPIKE_CLAUDE"):
        raise SystemExit("set SPIKE_CLAUDE: S32 names the build it measures")
    name = sys.argv[1]
    spec = spec_of(name)
    version = claude_version()
    run = "s32-reply-" + name
    stamp = tree_stamp()
    out = raw_dir(run)
    scratch = pathlib.Path(tempfile.mkdtemp(prefix="s32-")).resolve()
    if scratch == REPO or REPO in scratch.parents:
        scratch.rmdir()
        raise SystemExit("TMPDIR must be outside the repository")

    nonce = secrets.token_hex(4)
    surface = bridge_surface()
    if spec["extra_instructions"]:
        surface["instructions"] += "\n" + spec["extra_instructions"]
    message = MESSAGES[spec["message"]].format(nonce=nonce)
    record = {"run": run, **stamp, "claude_version": version, "nonce": nonce, "cell": name.rsplit("-", 1)[0],
              "told": spec["told"], "message_kind": spec["message"], "arrival": spec["arrival"],
              "first_turn": spec["first"], "message": message,
              "instructions_added": spec["extra_instructions"],
              "instructions_chars": len(surface["instructions"]),
              "server_sha256": hashlib.sha256(json.dumps(surface, sort_keys=True).encode()).hexdigest(),
              "stub_tool_delay_ms": TOOL_DELAY_MS if spec["arrival"] == "tool" else 0}
    passed = False
    try:
        config, workdir, built, policy = (scratch / d for d in ("config", "work", "marketplace", "policy"))
        for d in (config, workdir, policy):
            d.mkdir()
        path = policy / DROP_IN
        path.parent.mkdir(parents=True)
        path.write_text(json.dumps(ALLOW, indent=2) + "\n")
        record["managed_settings"] = {"path": f"/etc/claude-code/{DROP_IN}", "content": ALLOW}
        server = scratch / "server.json"
        server.write_text(json.dumps(surface))
        wrapper = scratch / "claude-channel-wrapper"
        env = {"SPIKE_SERVER": server, "SPIKE_MESSAGE": message, "SPIKE_LOG": scratch / "stub.jsonl",
               "SPIKE_TRIGGER": scratch / "trigger", "SPIKE_TOOL_DELAY_MS": record["stub_tool_delay_ms"]}
        assigns = " ".join(f"{k}='{v}'" for k, v in env.items())
        wrapper.write_text(f"#!/bin/bash\nexec env {assigns} python3 -I '{HERE / 's32_reply_stub.py'}'\n")
        wrapper.chmod(0o755)
        subprocess.run(["bash", str(REPO / "packaging" / "claude-plugin" / "build.sh"), str(built), str(wrapper)],
                       check=True, capture_output=True)
        record["setup"] = [cli(config, "plugin", "marketplace", "add", str(built)),
                           cli(config, "plugin", "install", f"{PLUGIN}@{MARKETPLACE}")]
        seed_config(config, workdir)
        record["sandbox"] = "bwrap: the host's /etc entries read-only, /etc/claude-code a scratch directory read-only"
        ok, _ = run_session(spec, scratch, out, config, workdir, policy, record)
        record["env_passed"] = sorted(child_env({"CLAUDE_CONFIG_DIR": "", "CLAUDE_CODE_OAUTH_TOKEN": ""}))
        # A turn the close interrupted, or one that failed, measured nothing.
        passed = ok and record["shutdown"]["group_empty"] and \
            all(r["subtype"] == "success" for r in record["results"])
    finally:
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
    print(f"{run}: replied={o['replied']} after={o['reply_after_s']} event_turns={o['event_turns']} "
          f"calls={[c['name'] for c in o['calls_after_event']]} passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
