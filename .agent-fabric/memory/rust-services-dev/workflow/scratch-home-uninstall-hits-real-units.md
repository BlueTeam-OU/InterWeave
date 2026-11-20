---
role: "rust-services-dev"
class: workflow
topic: "scratch-home-uninstall-hits-real-units"
description: "A scratch HOME is not a fresh account for uninstall.sh — it finds the uid's own user manager and stops every interweave@ unit, the login's live gzcoord pair included"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 4d50d40f4c01f8e4
---

## A scratch HOME is not a fresh account for uninstall.sh — it finds the uid's own user manager and stops every interweave@ unit, the login's live gzcoord pair included

On InterWeave j65 (2026-11-11) I followed packaging/claude-plugin/INSTALL.md's "Removing it" under a scratch HOME
and a private XDG_RUNTIME_DIR. The archive's uninstall.sh falls back to /run/user/$(id -u) on purpose
(packaging/linux/uninstall.sh header) and stopped and disabled this login's live interweave-daemon@gzcoord and
interweave-client@gzcoord: down 24 s, re-enabled with `systemctl --user enable --now` on both.

**Why:** a scratch HOME isolates files, never the uid. Anything that talks to the user manager or to /run/user/<uid>
reaches the real account's state.

**How to apply:** under a scratch HOME, run no script that calls `systemctl --user` (uninstall.sh, an `enable`) on a
login that runs fleet units. Stop your own transient units by name, and record the step as not run. Related:
[[host-managed-settings-drop-mcp-config]].

*References: host-managed-settings-drop-mcp-config*

*Anchors: `interweave:packaging/linux/uninstall.sh::own="/run/user/$(id -u)"`*

*Observed 2026-11-11 (rust-services-dev)*
