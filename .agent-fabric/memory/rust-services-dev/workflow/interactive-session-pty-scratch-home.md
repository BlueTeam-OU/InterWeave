---
role: "rust-services-dev"
class: workflow
topic: "interactive-session-pty-scratch-home"
description: "Driving an interactive claude --channels session in a pty from a fresh scratch HOME — the first-run screens, the folder-trust default that exits, and what the interactive debug log and terminal show"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 3936f5b49d7eab5f
---

## Driving an interactive claude --channels session in a pty from a fresh scratch HOME — the first-run screens, the folder-trust default that exits, and what the interactive debug log and terminal show

Measured 2026-11-16 on Claude Code 2.1.322 (PR #691, `packaging/claude-plugin/evidence/2026-11-16-install-steps-left-out.md` section 5):

- A new scratch HOME shows, before the prompt: the text-style picker (Enter keeps Dark mode), the security notes ("Press Enter to continue"), then the folder-trust question whose DEFAULT is "No, exit" — answer it Down, Enter. Each stops appearing once answered (stored in `~/.claude.json`). A run that ends at these screens starts no bridge.
- The login's token reaches the session as `CLAUDE_CODE_OAUTH_TOKEN`, read from `~/.config/agent-fabric/secrets.env` (`export` line, shlex-parse) into the child env only.
- The interactive log line is `Channel notifications registered` (twice), not the print session's `re-registered after reconnect`.
- Without step 7, the interactive terminal itself showed `plugin interweave@interweave is not on the approved channels allowlist (use --dangerously-load-development-channels…` (one run, with `--debug-file`), unlike a print session.
- The fleet's managed hooks and the `easel` server come along with the copied `/etc/claude-code/managed-settings.json`; an interactive session started in auto mode, so `status` needed no approval.
- Inside the bubblewrap private `/etc`, `status` shows `log.file` null / `directory_not_private` and `ConfigDirUnguarded`: a stand-in artifact (uid 65534 owns `/home` in the namespace), as on 2026-11-11.

**Why:** the first exploratory run sent Enter into the trust question and the session exited; the screens cost two runs to learn.
**How to apply:** script `expect:I trust this folder,down,enter` for the first session in a new HOME, then `expect:shift+tab to cycle` for the prompt. Related: [[spike-builds-scratch-home-install]], [[plugin-upgrade-and-configure-facts]].

*References: plugin-upgrade-and-configure-facts, spike-builds-scratch-home-install*

*Anchors: `packaging/claude-plugin/evidence/2026-11-16-install-steps-left-out.md::## 5. Step 8's interactive session`, `spikes/spike-001/run_tty.py::expect:`*

*Observed 2026-11-16 (rust-services-dev)*
