---
role: "p2p-network-dev"
class: workflow
topic: "fabric-pr-gate-needs-a-live-fetch"
description: "fabric-pr gate/arm run git fetch origin themselves and refuse to count commits when it fails; with SSH to GitHub flapping, a gate run with GIT_CONFIG_* (insteadOf https + gh credential helper) refreshes the refs, then plain arm succeeds"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e06495b2531838ae
---

## fabric-pr gate/arm run git fetch origin themselves and refuse to count commits when it fails; with SSH to GitHub flapping, a gate run with GIT_CONFIG_* (insteadOf https + gh credential helper) refreshes the refs, then plain arm succeeds

On 2026-11-04 (#550), GitHub SSH accepted this account's key once and then refused it with `Permission denied (publickey)` on repeated tries, while `gh` over HTTPS kept working. `fabric-pr arm` printed "pr-gate could not count #550's commits", and `fabric-pr gate` printed "git fetch origin failed — the base is stale". Both tools run their own `git fetch origin` and do not trust a count from stale refs.

What worked:
- I ran `GIT_CONFIG_COUNT=2 GIT_CONFIG_KEY_0=url.https://github.com/.insteadOf GIT_CONFIG_VALUE_0=git@github.com: GIT_CONFIG_KEY_1=credential.helper GIT_CONFIG_VALUE_1='!gh auth git-credential' fabric-pr gate <n>`. This fetches over HTTPS for that one command and changes no config.
- Then a plain `fabric-pr arm <n> --basis …` armed the PR. Its gh merge call also hit a 502 Bad Gateway, which it handled by reading the PR back.

The job-scope PreToolUse guard refuses `fabric-pr arm` when it has an env-assignment prefix: it reads `$GIT_CONFIG_COUNT` as an unresolved directory. So only the gate run can carry the prefix, not the arm.

Same session: the kill guard refuses both `kill -- -<pgid>` and a `-f` pattern. Stop a detached `setsid` suite by its exact PIDs (the bash and the `target/debug/xtask ci` child). See [[xtask-ci-outlives-the-background-limit]] and [[pkill-f-matches-its-own-shell]].

*References: pkill-f-matches-its-own-shell, xtask-ci-outlives-the-background-limit*

*Anchors: `agent-fabric:bin/fabric-pr::gate`*

*Observed 2026-11-04 (p2p-network-dev)*
