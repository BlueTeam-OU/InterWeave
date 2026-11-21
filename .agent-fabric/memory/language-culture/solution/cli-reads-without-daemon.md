---
role: "language-culture"
class: solution
topic: "cli-reads-without-daemon"
description: "interweave-human reads conversations without the daemon; only send, retry, trust, identities and peers need it"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 0b23b81d7202b044
---

## interweave-human reads conversations without the daemon; only send, retry, trust, identities and peers need it

The command-line client `interweave-human` does not need `transport-daemon` for its reading verbs (conversations, messages, read). Only send, retry, trust, identities and peers fail without one (apps/human-cli/README.md, the "Reading verbs work without a daemon" bullet). The window, `human-desktop`, works only while the service runs. A request of 2026-11-10 (01a1c5ca) said the CLI "needs transport-daemon running, like the window", so the release-page item says "Like the window, it needs the service to send messages" (REPLY 01a1c5cd).

**Why:** a requester's context sentence about a program can be broader than the program's own README.
**How to apply:** before copy says what a program needs, read that program's README on origin/main. See [[solution-carried-2026-10-10]].

*References: solution-carried-2026-10-10*

*Anchors: `apps/human-cli/README.md::Reading verbs work without a daemon`*

*Observed 2026-11-10 (language-culture)*
