---
role: "architect-cto"
class: rationale
topic: "j540-restart-diagnostic-reads-policy-in-force-2026-11-05"
description: "j540 (2026-11-05): ADR-0041 A 2026-11-05 ruled for ux-designer-01's Q1/Q2 (01a1ac83) — background_restart_requires_user_authentication reads the policy IN FORCE (the record's byte), never the configured key_unlock_policy; no Try again for…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 8909f8da7a8e626e
---

## j540 (2026-11-05): ADR-0041 A 2026-11-05 ruled for ux-designer-01's Q1/Q2 (01a1ac83) — background_restart_requires_user_authentication reads the policy IN FORCE (the record's byte), never the configured key_unlock_policy; no Try again for a pending reseal; #571 (2 work, 3 fix) e667ea873 after rounds 1-4 (F5: Unsynced keeps the pending change shown, ADR-0042 second clause); round 5 clean (F1 config docs, F2 misquote, F3 locked-state input, F4 digest+row clause) and ux-01 OBSERVATION 01a1aca1: the policy in force is the record on disk, the reseal's when it wrote one, Unsynced counts as written, owner's word; closes when it merges

ux-designer-01's QUESTION 01a1ac83-a33c (design note
`design/2026-11-05-unlock-policy-pending.md` 40a98b009, for
rust-ui-dev-01's j213): while a reseal is pending (ADR-0042 A 2026-11-05,
my j535) the record opens under the OTHER policy, so a diagnostic read
from the configured `key_unlock_policy` says the opposite of what a
restart does in both directions.

Ruled, ADR-0041 A 2026-11-05 (InterWeave #571, e0b5385d0): the
diagnostic's inputs are the effective availability mode and the
authenticated policy byte of the record the start opened; the configured
policy is never an input and reaches the status model only as the pending
change shown by name beside it. The same rule the 2026-10-10 amendment
gave the diagnostic's other input (one effective value, never the authored
field). Q2: no Try again — every start retries; the reseal runs before the
host takes the profile lock, so an in-session retry is a service restart
(an Unlock prompt under user-presence); revisit when an in-app unlock-policy
control exists, whose apply is the retry.

Where the predicate is computed stays the owning roles': `profile-config`'s
`runtime.rs` reads `self.key_unlock_policy` today with a doc comment
naming the configured combination; told rust-ui-dev-01 (INFO 01a1ac8e),
reply to ux-designer-01 01a1ac8d. Propagated: §14, android-key-custody.md,
human-client-android.md, failure-model.md, digest.

Round 1 of #571 found three real things, fixed in 5f3eb77c2: the config
schema, configuration.md and the human-android example still derived the
diagnostic from the configured field (F1); the history note misquoted
prior wording (F2); and the rule named no input in the locked state (F3:
the header's policy byte, parsed before any cipher call, authenticated
only by the open; a UserNotAuthenticated refusal is the user-presence
key's alone, so the two readings agree). The design note 40a98b009 is on
ux-designer-01's j93 branch, not main: cited as such in §14.

**Why:** the diagnostic describes a fact about the next restart, and only
the record's policy decides it; a status model that reads the target
instead of the fact is wrong exactly while it matters.
**How to apply:** any status derived from a setting that can lag its
application reads the applied value; the configured value appears as a
pending change, never as the status. Related: [[j535-adr-0042-failed-reseal-never-refuses-start-2026-11-05]].

**CLOSED 2026-11-05:** #571 merged 1aadab26f at 18:20:31Z on the owner's word at e667ea873. The origin branch still carries ad5b58965 (two j539 commits, pushed after the merge by mistake; on #575 anyway); the delete was refused by the classifier.

*References: j535-adr-0042-failed-reseal-never-refuses-start-2026-11-05*

*Observed 2026-11-05 (architect-cto)*
