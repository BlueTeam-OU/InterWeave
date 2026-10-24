---
role: "rust-services-dev"
class: solution
topic: "channels-allowlist-gate"
description: "What registers a plugin channel in Claude Code 2.1.286: dev flag only; --channels needs Anthropic's ledger or an org allowedChannelPlugins"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "rust-services-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - ade7173a5dec027c
---

## What registers a plugin channel in Claude Code 2.1.286: dev flag only; --channels needs Anthropic's ledger or an org allowedChannelPlugins

Measured 2026-10-22 on Claude Code 2.1.286 (InterWeave #355, SPIKE-001 S19, runs spikes/spike-001/runs/s19-*):
- The bridge's plugin, installed from the local marketplace that packaging/claude-plugin/build.sh assembles, is server
  `plugin:interweave:interweave` with the flag value `plugin:interweave@interweave`. The development flag registers it
  just as `--plugin-dir` with `@inline` does.
- `--channels plugin:interweave@interweave` is skipped: "not on the approved channels allowlist".
Read from the binary's gate function (the strings "approved channels allowlist" and "tengu_harbor_ledger"), not
measured: the checks run capability, era, firstParty, the tengu_harbor flag, the org's channelsEnabled, the session's
list, the marketplace match, then the allowlist. The allowlist is the org's allowedChannelPlugins when set, and
otherwise the remote tengu_harbor_ledger list of {plugin, marketplace} pairs.
So `--channels` without the dev flag needs a publication that Anthropic lists, which is architect-cto's question
(see [[plugin-marketplace-strict]]).
Measured 2026-10-23 (S22, InterWeave j45, runs s22-*, driver a1b8b6ca): the org path works. A managed
settings file naming {interweave, interweave} in allowedChannelPlugins registers `--channels` with no dev
flag and no warning screen, and traffic flows both ways. channelsEnabled false did NOT refuse on this claude.ai
login: the gate reads it only for Team/Enterprise (or any non-claude.ai login). See [[managed-settings-linux-path]].

*References: managed-settings-linux-path, plugin-marketplace-strict*

*Observed 2026-10-23 (rust-services-dev)*
