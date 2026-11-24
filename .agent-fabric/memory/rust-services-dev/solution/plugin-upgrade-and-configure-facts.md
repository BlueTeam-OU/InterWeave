---
role: "rust-services-dev"
class: solution
topic: "plugin-upgrade-and-configure-facts"
description: "Measuring the claude plugin upgrade or configure: v0.2.0's archive carries the plugin; configure merges, does not validate, \"\" is refused"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-services-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - aa26f989ca255440
---

## Measuring the claude plugin upgrade or configure: v0.2.0's archive carries the plugin; configure merges, does not validate, "" is refused

Measured 2026-11-16 on Claude Code 2.1.322 (PR #684, `packaging/claude-plugin/evidence/2026-11-16-install-steps-left-out.md`):

- The v0.2.0 release Linux archive carries `claude-plugin/` too, its plugin.json at 0.2.0 (v0.1.0's not checked), so a release-to-release
  upgrade can be run without a local build. A local higher version needs `[workspace.package] version` bumped in a scratch
  worktree (build.sh refuses `--version` ≠ workspace); the bridge release build took ~24 min at `-j 2`.
- With a directory marketplace, `claude plugin list` and the next session see the new version as soon as the folder is
  replaced; `marketplace update` + `plugin update` only move installed_plugins.json and the cache.
- `plugin configure --values-stdin` merges keys, does not validate values; `{"log_level": ""}` shows `not set` but the
  bridge gets `""` and refuses (class usage). Unknown key → exit 1.
- A second session's bridge takes the endpoint at its next retry once the first ends (~6 s, one run).

**How to apply:** for the next plugin measurement, start from these instead of assuming only one release exists.
Driving a session: `--channels plugin:interweave@interweave` must be in the argv, or no channel registers (an easy miss).

*Anchors: `packaging/claude-plugin/interweave/.mcp.json::--log-level`, `apps/claude-channel/src/main.rs::the_log_level_is_optional_and_one_of_four`*

*Observed 2026-11-16 (rust-services-dev)*
