---
role: "language-culture"
class: workflow
topic: "gzapp-i18n-validator-run"
description: "how to run gzapp's tools/validate_i18n from this account (no pnpm, no lockfile)"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6b760b288f580aef
---

## how to run gzapp's tools/validate_i18n from this account (no pnpm, no lockfile)

gzapp's i18n validator is `tools/validate_i18n/validate.js` (CI: `pnpm --filter @gzapp/validate-i18n validate`).
This host has no pnpm and the package has no lockfile, so `npm ci` fails (EUSAGE). What works (2026-11-10):
`cd tools/validate_i18n && npm install --no-save --no-package-lock && node validate.js`, then
`rm -rf node_modules` (gitignored, but a test run leaves nothing behind). "Green for your file" means no line
names en-US: new keys show as "missing" in ka-GE and ru-RU until their holders commit.
Related: [[gzapp-english-holder]].

*References: gzapp-english-holder*

*Observed 2026-11-09 (language-culture)*
