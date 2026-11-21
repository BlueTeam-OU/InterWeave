---
role: "rust-services-dev"
class: solution
topic: "human-client-needs-chat-envelope"
description: "A bridge reply to a human client (interweave-human, human-desktop) is shown only as a HumanChatV2 envelope with its content_type; plain text is accepted and shown as nothing"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 8396c87c7aae55ea
---

## A bridge reply to a human client (interweave-human, human-desktop) is shown only as a HumanChatV2 envelope with its content_type; plain text is accepted and shown as nothing

Measured 2026-11-11 on the v0.3.0 archive with Claude Code 2.1.322 (InterWeave j65, PR #627,
packaging/claude-plugin/evidence/2026-11-11-v0.3.0-install-run.md):
- A `reply` in plain text to an `interweave-human` peer returns `remote transport accepted at endpoint human`, and the
  client shows nothing. `interweave-human status` counts it as neither unread nor undisplayable; the facade's
  diagnostics count it as `malformed_unknown_media_type`, which the CLI does not print.
- The reply was shown (u2) once the model sent `{"v":2,"kind":"text","app_message_id":<32 hex>,"text":…,"reply_to":…}`
  with `content_type` `application/vnd.interweave-human-chat+json;v=2`. It sent that only after the first user turn
  told it to.
- The peer's endpoint must be held by a running client (`interweave-human … listen`). After a one-shot `send` exits,
  `reply` fails with `RemoteEndpointUnavailable`.
- `peers add` refuses a loopback address as `special_use`. Two profiles on one host need the host's LAN address.

**Why:** a peer test that reads "no reply" may be a format mismatch, not a bridge failure.

**How to apply:** for a bridge↔human run, keep a listener on the peer, and give the envelope in the first user turn.
Related: [[scratch-home-uninstall-hits-real-units]].

*References: scratch-home-uninstall-hits-real-units*

*Anchors: `interweave:packaging/claude-plugin/INSTALL.md::A peer that is a human client`, `interweave:tests/desktop-e2e/tests/claude_channel.rs::the_human_clients_facade_exchanges_with_the_bridge_plain_and_compressed`*

*Observed 2026-11-11 (rust-services-dev)*
