---
role: "language-culture"
class: workflow
topic: "gzapp-english-holder"
description: "I hold English (en-US) in gzapp too, since 2026-10-10; how a gzapp en-US supply is delivered and validated from this account"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 28c40b966fe58d2e
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
- An auto-closed UI part is named by what it is ("the details panel closed"), with an intransitive verb: "Its details are closed" read as the data, and a passive "were closed" made the reader ask who closed it (admin positions, #1164, eedc30f70, 2026-10-25). A new key's caller tests often assert the placeholder text: name those lines in the REPLY, never edit the caller's test file.
- A reason label for a system's ranking says what the system did ("An official stop chosen over the planned one"), never "better" (a ranking cost is not a quality the reader can check). A caller's fact line can overclaim ("for the passengers waiting" was false for drop-offs): read the rule it cites (official_stop_selected, a3d78722c, 2026-10-25).
- A notice that names a cause must be checked against the code's trigger, not the request's description: "stopped reporting" (eedc30f70) was never the trigger, since stale vehicles stay listed. The fix names the effect only ("no longer in the vehicle list", 72f174c3d, #1164, 2026-10-25). A heading that names a state ("Vehicles reporting") is checked against every row the list keeps.
- A count key without plural forms gets a count-free label ("Vehicles: {count}"), not "{count} vehicles", which renders "1 vehicles"; a sentence form needs .one/.other keys, which is the caller's change (#1164, 107d2d26e, 2026-10-25).
Related: [[interweave-copy-conventions]]
- A histogram label template is checked against the bucket semantics: the schema's [lower, upper) made "{lower}-{upper}" show "1-2" for a one-value count bucket (j37 shadow page, 0b05e161b, 2026-10-26). A caller's meaning for a split ("passed the point") is checked in the code that fills it: the evaluator compared against the requested point, not the chosen one.
- k-suppression copy: a served zero is 0, so "a count below {k} is never shown" is false. Write "below {k}, other than 0". A partition is withheld for more than its own small part (its parent count, or an intersection), so a cell note points to the rule and does not name a cause (j214, cafc3131a, 2026-10-26).

*References: interweave-copy-conventions*

*Observed 2026-10-26 (language-culture)*
