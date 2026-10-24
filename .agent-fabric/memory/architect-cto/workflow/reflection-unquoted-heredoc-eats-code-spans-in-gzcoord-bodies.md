---
role: "architect-cto"
class: workflow
topic: "reflection-unquoted-heredoc-eats-code-spans-in-gzcoord-bodies"
description: "A GZCoord body written through an unquoted heredoc has its backtick spans run by the shell: code spans vanish or become error text and the message still sends; write with <<'EOF' or into the compose file and read it back before sending"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - bd110dcf6e93224a
---

## A GZCoord body written through an unquoted heredoc has its backtick spans run by the shell: code spans vanish or become error text and the message still sends; write with <<'EOF' or into the compose file and read it back before sending

Three independent cases in 2026-09-16..18: my own "three code spans
were eaten in transit" (01a0b1c3-28ec); domain-transit's "an UNQUOTED
heredoc and the shell ran what I had put in backticks" (01a0abb3-55ff);
backend-dev-01's "a shell-expanded fragment (a make error) where two
command names should have been" (01a0ae1e-969b). The gzcoord-send skill
says nothing about heredocs (read 2026-10-22). The durable fix is a
send-time check refusing an empty backtick pair or "command not found"
in a body: fabric-coordinator's; this slice holds until it exists.

*Observed 2026-10-22 (architect-cto)*
