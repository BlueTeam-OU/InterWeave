---
role: "architect-cto"
class: workflow
topic: "reflection-read-an-every-binding-rule-against-each-mobile-platform-s-defaults"
description: "Before a filesystem, credential or backup rule is written to bind every binding, read it against Android's and iOS's platform defaults; rules drafted from the Linux desktop break on mobile, where Android's app directories are group-owned…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 3ea775805a8ee97b
---

## Before a filesystem, credential or backup rule is written to bind every binding, read it against Android's and iOS's platform defaults; rules drafted from the Linux desktop break on mobile, where Android's app directories are group-owned 0771 and iOS's default Keychain class travels in backups

InterWeave 2026-10-08: "require_private_dir as written therefore
refuses every app-private directory on Android" (01a11b1f-2246); "a
private directory UNDER files/ is still refused for the group-write bit
even with the boundary at the data directory" (01a11b27-30ca); gzapp
2026-09-29: "That class is NOT ThisDeviceOnly: such items travel in
encrypted device backups" (01a0e9b1-009c). ADR-0028 carries two Android
corrections (2026-10-08, 2026-10-15) and gzapp ADR-025 A 2026-09-28
replaced the Keychain class; neither states the practice. Sibling of
[[reflection-measure-the-host-before-writing-what-an-owed-test-needs]].

*References: reflection-measure-the-host-before-writing-what-an-owed-test-needs*

*Observed 2026-10-23 (architect-cto)*
