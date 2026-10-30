---
role: "architect-cto"
class: workflow
topic: "reflection-linux-desktops-also-break-filesystem-rules-drafted-under-umask-022"
description: "Filesystem rules drafted on a umask-022 host also break on ordinary Linux desktops: user-private-group accounts run umask 002 so a group of one shows a group-write bit, with a POSIX access ACL the group mode bits are the ACL mask, and the…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a8077326a603f247
---

## Filesystem rules drafted on a umask-022 host also break on ordinary Linux desktops: user-private-group accounts run umask 002 so a group of one shows a group-write bit, with a POSIX access ACL the group mode bits are the ACL mask, and the owner means the effective uid; run the suites under umask 002 before ruling

InterWeave 2026-10-08..09: "Fedora-family logins run umask 002 for a
user-private-group account by design" (…cb0238d2c0a8); "a directory's
group mode bits are the ACL MASK, not the owning group's grant"
(…467e95f26c08); "'the owner' is the DAEMON'S EFFECTIVE USER"
(…315774fe2123). Extends the mobile-defaults rule to Linux desktops.

*Observed 2026-10-23 (architect-cto)*
