---
role: "architect-cto"
class: threads
topic: "macos-windows-packaging-postponed-2026-10-28"
description: "the owner, 2026-10-28: macOS and Windows packaging are postponed indefinitely, an option after the whole plan is complete; the v1 release matrix is Linux and Android; recorded on InterWeave #505 (plan §21/§22, READMEs)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 722279ee0e8c5ba1
---

## the owner, 2026-10-28: macOS and Windows packaging are postponed indefinitely, an option after the whole plan is complete; the v1 release matrix is Linux and Android; recorded on InterWeave #505 (plan §21/§22, READMEs)

Owner's words: "macOS and Windows packaging are indefinitely postpone as option to do after all the plan is completed." Recorded on #505 (1 work, 8d458b10): plan §22's sentence that held them as open rows with a lane and a host owed, §21's release-matrix consequence (was Linux, macOS and Android; now Linux and Android) and its C-04 Windows carry row, README.md's packaging line, packaging/README.md. The macOS runtime-directory rule, the macOS runner, the Windows binding and its BSL-1.0 decision stay carries by name with no date; no stage's close waits on them. Same day the owner asked whether PeerId and EndpointId derive from a key: PeerId from the node's Ed25519 key, the EndpointId a configured label "not cryptographically derived" (ENDPOINTS.md), the identity key (ADR-0063) carried by the card.

**How to apply:** no REQUEST for macOS/Windows work goes to any lane; a question about Windows (named pipe, ACL, BSL-1.0) is answered "postponed, an option after the plan". Related: [[stage-19-activation-2026-10-20]].

*References: stage-19-activation-2026-10-20*

*Observed 2026-10-28 (architect-cto)*
