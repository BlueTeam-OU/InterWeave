---
role: "architect-cto"
class: workflow
topic: "owner-rule-never-wait-on-owner-to-arm-2026-11-09"
description: "Owner's rule 2026-11-09 (fabric-coordinator broadcast 01a1c1dc): a session never stops to ask the owner for an arming word; it REQUESTs the repository's arming authority by message (architect-cto in InterWeave and gzapp) and takes its…"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - b2ce4472cef57669
---

## Owner's rule 2026-11-09 (fabric-coordinator broadcast 01a1c1dc): a session never stops to ask the owner for an arming word; it REQUESTs the repository's arming authority by message (architect-cto in InterWeave and gzapp) and takes its next job; my own PRs below the minimum still need the owner's word, asked by message, not at the prompt

The owner, 2026-11-09, after three sessions sat at their prompt asking "do I have your word to arm?": never wait on the owner to arm. When arm needs a word (below the work-commit minimum, a boundary path, a deferral), send a REQUEST TO the login `fabric-pr arm --dry-run` names as the arming authority (fabric-coordinator in agent-fabric, architect-cto in a project), with the PR, head and counts; take the next job meanwhile; the AUTHORIZES header in the REPLY is what arm reads. Only money, accounts, a new host or accepting an ADR still go to the owner.

**Why:** an idle session waiting on a human is the cost the fabric exists to remove; the authority role exists so the word has an owner who is always online.

**How to apply (as architect-cto):** expect a steady stream of AUTHORIZES REQUESTs for InterWeave and gzapp ([[architect-cto-arms-gzapp-interweave]]); each is read head + posted review first ([[arming-words-2026-10-27-ii]]). For my OWN below-minimum PRs the remit keeps the owner's word (`authority_own_prs` false): ask it in a message to the owner's channel or in the session reply, never by stopping on the question, and work the next job while waiting.

*References: architect-cto-arms-gzapp-interweave, arming-words-2026-10-27-ii*

*Observed 2026-11-09 (architect-cto)*
