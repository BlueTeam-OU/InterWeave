# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 Andrea Benetton
"""SPIKE-001's S32 stub: a legacy-era channel server answering as the
bridge does (fact 66), that shows the model what the bridge shows it --
the bridge's instructions, its seven push-mode tools and a direct
message's meta -- and records what the model calls. EVIDENCE ONLY.

- ``SPIKE_SERVER``: a JSON file the driver writes from
  ``apps/claude-channel/src/mcp.rs`` (s32_reply.py's ``bridge_surface``):
  ``instructions`` and ``tools``, answered as they are. A case that adds
  a sentence to the instructions adds it there.
- One event, a direct message whose content is ``SPIKE_MESSAGE``, written
  when ``<SPIKE_TRIGGER>.1`` exists, with the meta the bridge gives a
  direct message (S29's e runs show it whole) and a fresh
  ``reply_token``.
- A call whose arguments the bridge's tool input refuses is refused as
  the bridge refuses it (``refusal``): Claude Code does not hold a call
  to the advertised schema, and a model has called ``reply`` with
  ``text`` or ``message`` in place of ``content``.
- ``status`` answers after ``SPIKE_TOOL_DELAY_MS``, so an event can be
  made to arrive while a tool runs; ``reply`` and ``send`` answer as the
  bridge did in S29 when the remote transport accepted.
- Every record carries ``wall``, the wall clock, so the driver places
  the stub's writes and the model's calls against the session's times.
"""

import json
import os
import secrets
import sys
import threading
import time

START = time.monotonic()
LOG = open(os.environ.get("SPIKE_LOG", "/dev/null"), "a", buffering=1)
TRIGGER = os.environ.get("SPIKE_TRIGGER", "")
MESSAGE = os.environ.get("SPIKE_MESSAGE", "")
TOOL_DELAY = int(os.environ.get("SPIKE_TOOL_DELAY_MS", "0")) / 1000
with open(os.environ["SPIKE_SERVER"]) as f:
    SERVER = json.load(f)
LOCK = threading.Lock()
CAPABILITIES = {"experimental": {"claude/channel": {}}, "tools": {}}
# A PeerId of the bridge's shape; no daemon holds it.
PEER = "12D3KooWB4t1hTSbycvTVir2JRGyeAeFBPcHxCakj8XNXFuXSNpn"
ACCEPTED = "remote transport accepted at endpoint claude"
STATUS = {"endpoint_lease_state": "held", "endpoint": "claude", "joined_channels": [],
          "transport": {"daemon": "connected", "peers_connected": 1},
          "broadcast_loss": {"queue_overflow": 0, "ingress_refused": 0}}


def record(direction, body):
    entry = {"t": int((time.monotonic() - START) * 1000), "wall": time.time(), "dir": direction, "body": body}
    with LOCK:
        LOG.write(json.dumps(entry) + "\n")


def send(msg):
    line = json.dumps(msg, ensure_ascii=False)
    record("out", msg)
    with LOCK:
        sys.stdout.write(line + "\n")
        sys.stdout.flush()


def result(id_, body):
    send({"jsonrpc": "2.0", "id": id_, "result": body})


def text_result(id_, text):
    result(id_, {"content": [{"type": "text", "text": text}], "isError": False})


def event_on_trigger():
    def run():
        path = f"{TRIGGER}.1"
        while not os.path.exists(path):
            time.sleep(0.05)
        meta = {"delivery_mode": "direct", "source_peer": PEER, "source_endpoint": "claude",
                "destination_endpoint": "claude", "message_id": secrets.token_hex(16),
                "received_at": time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime()) + ".000Z",
                "reply_token": secrets.token_urlsafe(16), "payload_encoding": "utf8"}
        send({"jsonrpc": "2.0", "method": "notifications/claude/channel",
              "params": {"content": MESSAGE, "meta": meta}})
        record("event", "event written")
    threading.Thread(target=run, daemon=True).start()


def refusal(name, arguments):
    """What the bridge answers a call's arguments that its tool input
    refuses, or None: ``parse_call`` (``channel-core``'s ``tools.rs``)
    deserialises into a ``deny_unknown_fields`` struct, so serde names an
    unknown field first, in the order given, then a missing one, and
    ``serve.rs`` answers either as invalid params. An unknown tool is
    ``unknown tool``. The grammar checks past the shape (a PeerId, a
    channel, the payload ceiling) are not reproduced."""
    tool = next((t for t in SERVER["tools"] if t["name"] == name), None)
    if tool is None:
        return "unknown tool"
    schema = tool["inputSchema"]
    fields = [f"`{p}`" for p in schema["properties"]]
    # serde's own phrasing of what a struct expects.
    expected = ("there are no fields" if not fields else f"expected {fields[0]}" if len(fields) == 1 else
                f"expected {fields[0]} or {fields[1]}" if len(fields) == 2 else
                "expected one of " + ", ".join(fields))
    for key in arguments or {}:
        if key not in schema["properties"]:
            return f"{name}: unknown field `{key}`, {expected}"
    for key in schema["required"]:
        if key not in (arguments or {}):
            return f"{name}: missing field `{key}`"
    return None


def answer_tool(id_, name, arguments):
    refused = refusal(name, arguments)
    if refused:
        send({"jsonrpc": "2.0", "id": id_, "error": {"code": -32602, "message": refused}})
        return

    # Off the reading thread, so a delayed answer holds up nothing else.
    def run():
        if name == "status":
            time.sleep(TOOL_DELAY)
            text_result(id_, json.dumps(STATUS))
        elif name in ("reply", "send"):
            text_result(id_, ACCEPTED)
        elif name == "identity":
            text_result(id_, json.dumps({"peer_id": PEER, "endpoint": "claude"}))
        else:
            text_result(id_, "ok")
    threading.Thread(target=run, daemon=True).start()


def main():
    record("event", f"start tools={len(SERVER['tools'])} tool_delay_s={TOOL_DELAY}")
    for line in sys.stdin:
        if not line.strip():
            continue
        try:
            msg = json.loads(line)
        except ValueError as e:
            record("event", f"unparsable line: {e}")
            continue
        record("in", msg)
        method, id_ = msg.get("method", ""), msg.get("id")
        if method == "server/discover" and id_ is not None:
            send({"jsonrpc": "2.0", "id": id_, "error": {"code": -32601, "message": "method not found"}})
        elif method == "initialize" and id_ is not None:
            result(id_, {"protocolVersion": msg.get("params", {}).get("protocolVersion", ""),
                         "capabilities": CAPABILITIES,
                         "serverInfo": {"name": "interweave", "version": "0.0.0"},
                         "instructions": SERVER["instructions"]})
        elif method == "notifications/initialized":
            event_on_trigger()
        elif method == "tools/list" and id_ is not None:
            result(id_, {"tools": SERVER["tools"]})
        elif method == "tools/call" and id_ is not None:
            params = msg.get("params") or {}
            answer_tool(id_, params.get("name"), params.get("arguments"))
        elif id_ is not None and method:
            if method == "ping":
                result(id_, {})
            else:
                send({"jsonrpc": "2.0", "id": id_, "error": {"code": -32601, "message": "method not found"}})
    record("event", "stdin closed")


if __name__ == "__main__":
    main()
