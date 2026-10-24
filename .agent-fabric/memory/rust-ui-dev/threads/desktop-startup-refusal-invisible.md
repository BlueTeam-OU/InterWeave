---
role: "rust-ui-dev"
class: threads
topic: "desktop-startup-refusal-invisible"
description: "human-desktop's start-up refusals: once stderr-only, now a RefusalWindow (#327, merged e97f7313); what it covers and what is still not built"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-ui-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ca4d64e2456fa057
---

## human-desktop's start-up refusals: once stderr-only, now a RefusalWindow (#327, merged e97f7313); what it covers and what is still not built

Found in #318's review (2026-10-20): every start-up refusal of `apps/human-desktop`
(`run.rs` `refuse(code, ...)`: EX_CONFIG 78 for a profile load refusal such as a newer
profile-format marker, EX_DATAERR 65 for a store needing recovery, EX_TEMPFAIL for a held
instance lock) is a stderr line and an exit code, with no window. A person who starts the
client from a desktop launcher sees nothing at all. `startup.rs`'s `Opened::Blocked` doc says
"the app shows why", which holds only for a terminal or the journal.

The release page's Linux text ("It shows which file stopped it") leans on this. Not fixed in
#318 (pre-existing, outside the batch). The follow-up is mine (client UX): a minimal refusal
window or a desktop notification for start-up refusals, its words language-culture-en's.
It may need architect-cto, because the no-window behaviour came from their Q4 ruling on the
store's Blocked states. Bring it as a problem with this evidence and the alternatives.
Related: [[profile-format-marker]].

**Decided and built (2026-10-20):** architect-cto chose (a), as plan §18 clause (4)'s screen that
was never built (DECISION 01a15a5f; the record supply is bc574d2f). Built as j31, PR #327: the
`RefusalWindow` in ui-slint and `apps/human-desktop/src/refusal.rs`, with AT-SPI e2e in
`tests/desktop-e2e/tests/human_app/refusal.rs`. Raising the open window (exit 75) is not built.

**Landed 2026-10-20T22:37Z, #327 merged as e97f7313.** Final English is language-culture-en's
(Supplier-Review REPLY 01a15aec, 01a15af5). Every refusal with a known file or folder shows it
(a lock error falls back to `human_dir()`); a profile-folder error with a path is a privacy
one and shows RefusalNotPrivate. Try again runs under the held lock, and a press after
success runs nothing (`press()`). Still open: a SIGTERM while the screen is up exits by
default action, not the refusal's class (review R2, left unspecified); Windows/macOS and
Orca by a person are unverified.

*References: profile-format-marker*

*Observed 2026-10-20 (rust-ui-dev)*
