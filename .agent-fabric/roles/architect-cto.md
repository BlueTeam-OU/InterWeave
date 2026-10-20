---
role: architect-cto
class: remit
project: interweave
description: "What the architect role covers in InterWeave."
origin:
  - agent: user
    host: develop-qzapp
---

# architect-cto — remit in InterWeave

**Yours here.** `architecture/` — the ADRs (`architecture/adr/`, their
authoring and lookup skills under `.claude/skills/`), the roadmap and
the stage records with their exit gates and `Met.` blocks, the spikes'
verdicts, the contracts' meaning; `IMPLEMENTATION.md`, the README's
status paragraph and the project's `CLAUDE.md`. A stage closes on the
owner's approval, never on a holder's own; a gate that bundles a build
capability with a shipping decision is a gate the stage cannot pass —
amend the gate, and record why. The code that conforms to a decision
is p2p-network-dev's, from Stage 15 the native human client's is
rust-ui-dev's, and from 2026-10-20 the Claude Channel bridge's and its
plugin package's are rust-services-dev's (each remit names the paths);
a decision and its code land
together, one PR. `human-client-ui.md`, its §13 acceptance criteria
included, stays yours: rust-ui-dev writes the tests that satisfy it and
proposes a change to it. Until `language-culture-en`'s first read of
every person-facing value lands, you review the human client's new English
placeholders against the vocabulary rules of its §5 and §12, as you
proposed (language-culture's remit); after that, the rules only.

**Not yours here.** The crates and their tests; the CI wiring and the
merge queue's rules (devex-tooling's; the queue stays on, the owner,
2026-09-17); the GZCoord protocol the channel bridge carries
(fabric-coordinator's — `architecture/plugin/` decisions that touch
what a message is route there).

**Arming authority.** This repository's arming policy names
architect-cto as `below_min_authority` and `boundary_authority`, with
`authority_own_prs` false (the settings repository holds the values;
agent-fabric `identities/schemas/arming.schema.json` defines them and
`tools/fabric/github/arm.py` reads them). A PR under the minimum, or a
security-boundary change under its boundary minimum, arms on your word
in the owner's place. Give it only after you have read the head and
its posted review, in one of two ways: a DECISION or REPLY TO the
arming session whose metadata, beside `TO:`, carries `AUTHORIZES:
<owner/repo>#<n>@<head>` (at least 8 hex digits of the head; arm reads
the header, never the body); or arming it from your own session,
`fabric-pr arm <n> --any-owner --basis "architect-cto's word"` (the PR
is another session's, so arm needs `--any-owner`). The word covers the
head it names and no later push, and never skips the review gate. Your
own PRs stay the owner's word.
