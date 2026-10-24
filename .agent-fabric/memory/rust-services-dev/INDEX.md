---
role: "rust-services-dev"
class: index
description: "What rust-services-dev knows and where it lives."
tier: 1
distilled_at: "2026-10-24"
---

# rust-services-dev — knowledge index

A session is given the charter and brief (in the launch prompt),
and the project's remit with a pointer to this index (from the
session-start hook) — nothing below. Open a slice when its cue
matches what you are doing; a `workflow` slice says how a kind of
work is done here, so read the matching ones before that work.
Paths are relative to this working copy; `../agent-fabric/` is the
control plane checked out beside it.

## charter

- [`../agent-fabric/identities/roles/rust-services-dev/charter.md`](../agent-fabric/identities/roles/rust-services-dev/charter.md) — The fleet's network-services developer in Rust: proxies, gateways and API servers — streaming, cancellation, retry and failover, TLS and credential boundaries — proven over real sockets.

## brief

- [`../agent-fabric/identities/roles/rust-services-dev/brief.md`](../agent-fabric/identities/roles/rust-services-dev/brief.md) — How rust-services-dev works day to day, in any project: the service boundary written as a contract first, a failure matrix before done, wire behaviour proved over real sockets, and credentials kept inside the process.

## domain

- [`../agent-fabric/memory/domains/rust-services-dev/domain/cargo-exports-ssl-cert-env.md`](../agent-fabric/memory/domains/rust-services-dev/domain/cargo-exports-ssl-cert-env.md) — cargo sets SSL_CERT_FILE/SSL_CERT_DIR for every child; rustls-native-certs then reads them instead of the host store
- [`../agent-fabric/memory/domains/rust-services-dev/domain/cargo-machete-silent-pass.md`](../agent-fabric/memory/domains/rust-services-dev/domain/cargo-machete-silent-pass.md) — cargo-machete 0.9.2 default mode skips dev-deps; --with-metadata reports unreadable crates clean with exit 0
- [`../agent-fabric/memory/domains/rust-services-dev/domain/hyper-cancellation-and-head-deadline.md`](../agent-fabric/memory/domains/rust-services-dev/domain/hyper-cancellation-and-head-deadline.md) — What hyper 1.x itself does on body drop and idle connections, and how gateway tests separate it from the gateway's own guards
- [`../agent-fabric/memory/domains/rust-services-dev/domain/hyper-client-closes-finished-sized-body.md`](../agent-fabric/memory/domains/rust-services-dev/domain/hyper-client-closes-finished-sized-body.md) — A test proving the gateway closed an upstream connection must leave that response's body unfinished — hyper's client closes a fully-received sized body itself
- [`../agent-fabric/memory/domains/rust-services-dev/domain/managed-settings-linux-path.md`](../agent-fabric/memory/domains/rust-services-dev/domain/managed-settings-linux-path.md) — Where Claude Code 2.1.286 reads an organisation's managed settings on Linux, and how to present them without root
- [`../agent-fabric/memory/domains/rust-services-dev/domain/nofollow-directory-symlink-enotdir.md`](../agent-fabric/memory/domains/rust-services-dev/domain/nofollow-directory-symlink-enotdir.md) — openat(O_DIRECTORY|O_NOFOLLOW) on a symlink fails ENOTDIR on Linux, not ELOOP; classify with fstatat
- [`../agent-fabric/memory/domains/rust-services-dev/domain/pipefail-head-sigpipe.md`](../agent-fabric/memory/domains/rust-services-dev/domain/pipefail-head-sigpipe.md) — `cmd | head -1` under set -o pipefail fails intermittently and silently; use sed's q instead
- [`../agent-fabric/memory/domains/rust-services-dev/domain/s19-driver-shape.md`](../agent-fabric/memory/domains/rust-services-dev/domain/s19-driver-shape.md) — Driving a nested claude TUI through a pty: match menu text with spaces squeezed out, and redact ~/ paths
- [`../agent-fabric/memory/domains/rust-services-dev/domain/serde-derive-accepts-arrays.md`](../agent-fabric/memory/domains/rust-services-dev/domain/serde-derive-accepts-arrays.md) — A derived serde struct also deserializes from a JSON array; a field probe that must only accept objects needs a map-only visitor
- [`../agent-fabric/memory/domains/rust-services-dev/domain/serde-tagged-unit-variant-ignores-unknown.md`](../agent-fabric/memory/domains/rust-services-dev/domain/serde-tagged-unit-variant-ignores-unknown.md) — serde internally tagged enum unit variant accepts unknown keys despite deny_unknown_fields; use an empty struct variant
- [`../agent-fabric/memory/domains/rust-services-dev/domain/spike-fact-is-its-configuration.md`](../agent-fabric/memory/domains/rust-services-dev/domain/spike-fact-is-its-configuration.md) — a spike's wire observation describes the client configuration it ran; check it in the configuration the gateway serves and commit it to the record before a contract relies on it
- [`../agent-fabric/memory/domains/rust-services-dev/domain/tracing-set-default-misses-events.md`](../agent-fabric/memory/domains/rust-services-dev/domain/tracing-set-default-misses-events.md) — a thread-scoped tracing subscriber in a unit test misses events when other tests hit the same callsite; log tests need their own binary
- [`../agent-fabric/memory/domains/rust-services-dev/domain/validator-shares-runtime-construction.md`](../agent-fabric/memory/domains/rust-services-dev/domain/validator-shares-runtime-construction.md) — a validator that copies the runtime's acceptance rule as a pattern drifts both ways; validate by calling the constructor the runtime uses

## solution

- [`.agent-fabric/memory/rust-services-dev/solution/bridge-pull-event-double-escaped.md`](.agent-fabric/memory/rust-services-dev/solution/bridge-pull-event-double-escaped.md) — claude-channel pull mode escapes an event twice on the host line; any size bound on an event must measure both modes
- [`.agent-fabric/memory/rust-services-dev/solution/channels-allowlist-gate.md`](.agent-fabric/memory/rust-services-dev/solution/channels-allowlist-gate.md) — What registers a plugin channel in Claude Code 2.1.286: dev flag only; --channels needs Anthropic's ledger or an org allowedChannelPlugins
- [`.agent-fabric/memory/rust-services-dev/solution/claude-plugin-runtime-facts.md`](.agent-fabric/memory/rust-services-dev/solution/claude-plugin-runtime-facts.md) — Claude Code 2.1.286 plugin runtime, measured CLI-only: userConfig defaults/--config reach argv; a directory marketplace runs in place; MCP stderr goes to ~/.cache/claude-cli-nodejs
- [`.agent-fabric/memory/rust-services-dev/solution/plugin-version-from-workspace.md`](.agent-fabric/memory/rust-services-dev/solution/plugin-version-from-workspace.md) — How the Claude plugin and claude-channel carry the ADR-0065 release version: build.sh, release archive, --version stamp

## workflow

- [`.agent-fabric/memory/rust-services-dev/workflow/git-rm-then-commit-stages-only-the-rm.md`](.agent-fabric/memory/rust-services-dev/workflow/git-rm-then-commit-stages-only-the-rm.md) — After `git rm` plus file edits, a bare `git commit` commits only the deletion; a build passing on the tree proves nothing about the commit
- [`.agent-fabric/memory/rust-services-dev/workflow/mutation-runs-save-the-fix-first.md`](.agent-fabric/memory/rust-services-dev/workflow/mutation-runs-save-the-fix-first.md) — Before a hand mutation to show a test fails, commit (or copy to a known path) the real change first; `git checkout <file>` to revert also wipes uncommitted work
- [`.agent-fabric/memory/rust-services-dev/workflow/narrow-before-driving.md`](.agent-fabric/memory/rust-services-dev/workflow/narrow-before-driving.md) — Before building a run driver or asking for an allow rule, check what a CLI-only path can measure
- [`.agent-fabric/memory/rust-services-dev/workflow/spike-tmpdir-sun-len.md`](.agent-fabric/memory/rust-services-dev/workflow/spike-tmpdir-sun-len.md) — A spike run that starts transport daemons fails "path must be shorter than SUN_LEN" with TMPDIR in the session scratchpad
- [`.agent-fabric/memory/rust-services-dev/workflow/supply-on-a-for-branch.md`](.agent-fabric/memory/rust-services-dev/workflow/supply-on-a-for-branch.md) — How a supply to another agent's branch is delivered and closed here — for/ branch off their commit, fabric-jobs deliver, REPLY
- [`.agent-fabric/memory/rust-services-dev/workflow/workflow-carried-2026-10-24.md`](.agent-fabric/memory/rust-services-dev/workflow/workflow-carried-2026-10-24.md) — To keep two open InterWeave PRs off one library crate, withdraw a small self-contained change from your own PR and re-land it after the other merges, instead of asking for an exception

## recall

- [`../agent-fabric/identities/roles/rust-services-dev/recall.md`](../agent-fabric/identities/roles/rust-services-dev/recall.md) — Where rust-services-dev's knowledge lives — charter, remit, distilled slices, this agent's memory — and how to trace a claim to its sources.
