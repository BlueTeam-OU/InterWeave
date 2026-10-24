---
role: "language-culture"
class: workflow
topic: "gzapp-english-holder"
description: "I hold English (en-US) in gzapp too, since 2026-10-10; how a gzapp en-US supply is delivered and validated from this account"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8cb09bf97f7df098
  - e3fdad871df77bab
---

## I hold English (en-US) in gzapp too, since 2026-10-10; how a gzapp en-US supply is delivered and validated from this account

- The owner placed language-culture-en as gzapp's English holder on
  2026-10-10 (fabric-coordinator REPLY 01a12521; the remit's "The holders"
  changes with fabric-coordinator's gzapp drain PR). Before that, gzapp's
  remit said English had no holder and callers drafted en-US.
- Clone: ~/projects/gzapp, bootstrapped with agent-fabric
  runtime/claude-code/bootstrap.sh.
- Delivery: one commit in one turn onto the caller's branch (gzapp
  CLAUDE.md, supplier rule), touching product/i18n/en-US.json only,
  Kind: work + Supplier-Review trailer. ka-GE and ru-RU holders push their
  own commits onto the same branch.
- No pnpm on this account: run tools/validate_i18n/validate.js from a
  scratchpad mirror (copy the package, link product/ contracts/ apps/
  packages/, ajv installed in the scratchpad); tools/validate_i18n/node_modules
  is NOT gitignored, so never link it into the tree.
- First supply: #1075 common.health.chip-spoken "{cluster}: {summary}",
  2abcd59a7.
- gzapp dictionaries support plural forms (.one/.few/.many/.other): use them rather than a count-free template when a count can be 1 (web-dev-01, 2026-10-10).
- When a label or step is renamed, grep for the CONCEPT (its other names: "preview", "validate", "i18n"), not only the old label: 0deacbb08 missed new-description (#1087, 2026-10-10).
- An error that only a direct API caller can trigger (a bad wire value the screen never sends) keeps the WIRE terms, not the screen's: state-invalid says draft/active/retired (backend-dev-02, 01a127a5, 2026-10-10). Check who can trigger an error before aligning its words.
- Inbox watch: only ever `gzcoord-inbox --until-delivery` as a background Bash command; never start-and-kill one in a shell line (twice on 2026-10-10; a reader killed mid-poll could swallow a delivery). Check --history after any such slip.
- Before changing an en-US value, find every test that reads the real dictionary TRANSITIVELY (e.g. status_web imports it via src/i18n/dictionaries.ts), not only files importing en-US.json directly: #1099 review F1, 2026-10-11.
- Proactive i18n work in gzapp: offer it as a contributor branch to an open PR that already touches product/i18n, not as a standalone PR (the CI-minutes programme targets standalone i18n-only PRs; the owner said "fold" for #1107, 2026-10-13).
Related: [[interweave-copy-conventions]]

*References: interweave-copy-conventions*

*Observed 2026-10-13 (language-culture)*
