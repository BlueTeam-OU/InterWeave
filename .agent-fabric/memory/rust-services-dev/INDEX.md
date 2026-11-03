---
role: "rust-services-dev"
class: index
description: "What rust-services-dev knows and where it lives."
tier: 1
distilled_at: "2026-10-31"
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

- [`../agent-fabric/memory/domains/rust-services-dev/domain/adapter-composed-text-indistinguishable.md`](../agent-fabric/memory/domains/rust-services-dev/domain/adapter-composed-text-indistinguishable.md) — An adapter feeding an LLM host never composes its own text into the content body; facts it derives go in structured metadata, because unescaped content is indistinguishable from peer text
- [`../agent-fabric/memory/domains/rust-services-dev/domain/assert-only-what-transport-promises.md`](../agent-fabric/memory/domains/rust-services-dev/domain/assert-only-what-transport-promises.md) — A load test asserts only what the transport's contract promises (exactly-once, bounds); order and drop counts it does not promise are reported as measurements, and a request asking for more is answered before code
- [`../agent-fabric/memory/domains/rust-services-dev/domain/backpressure-blocks-keepalive.md`](../agent-fabric/memory/domains/rust-services-dev/domain/backpressure-blocks-keepalive.md) — A consumer that applies backpressure by not reading its single stream also stops answering liveness probes on it, so a paused consumer is closed as wedged and accepted messages are lost
- [`../agent-fabric/memory/domains/rust-services-dev/domain/cargo-exports-ssl-cert-env.md`](../agent-fabric/memory/domains/rust-services-dev/domain/cargo-exports-ssl-cert-env.md) — cargo sets SSL_CERT_FILE/SSL_CERT_DIR for every child; rustls-native-certs then reads them instead of the host store
- [`../agent-fabric/memory/domains/rust-services-dev/domain/cargo-machete-silent-pass.md`](../agent-fabric/memory/domains/rust-services-dev/domain/cargo-machete-silent-pass.md) — cargo-machete 0.9.2 default mode skips dev-deps; --with-metadata reports unreadable crates clean with exit 0
- [`../agent-fabric/memory/domains/rust-services-dev/domain/claude-code-print-session-channel-facts.md`](../agent-fabric/memory/domains/rust-services-dev/domain/claude-code-print-session-channel-facts.md) — Claude Code 2.1.286 -p sessions with a channel: mid-turn events queue, closed stdin still answers, manual mode denies MCP tools, plugin configure needs --values-stdin
- [`../agent-fabric/memory/domains/rust-services-dev/domain/credential-companion-resolved-together.md`](../agent-fabric/memory/domains/rust-services-dev/domain/credential-companion-resolved-together.md) — A value that must match the backend credential (account id, org id) has to be resolved with the credential, per attempt; a static header diverges on an account switch
- [`../agent-fabric/memory/domains/rust-services-dev/domain/credential-coupled-value-one-read.md`](../agent-fabric/memory/domains/rust-services-dev/domain/credential-coupled-value-one-read.md) — A value that must stay consistent with a rotating credential (a per-account pseudonym key) is read in the same atomic read as the credential, never from a second source
- [`../agent-fabric/memory/domains/rust-services-dev/domain/eprintln-panics-on-closed-stderr.md`](../agent-fabric/memory/domains/rust-services-dev/domain/eprintln-panics-on-closed-stderr.md) — In a stdio child of a host that may die, never eprintln!; it panics on EPIPE (exit 101)
- [`../agent-fabric/memory/domains/rust-services-dev/domain/git-commit-stamp-watches-per-ref-backend.md`](../agent-fabric/memory/domains/rust-services-dev/domain/git-commit-stamp-watches-per-ref-backend.md) — A cargo build script that stamps the git commit goes stale unless it watches what each ref backend actually writes
- [`../agent-fabric/memory/domains/rust-services-dev/domain/guarded-kind-classifier-fails-toward-guard.md`](../agent-fabric/memory/domains/rust-services-dev/domain/guarded-kind-classifier-fails-toward-guard.md) — When a classifier decides whether a payload takes a protective path, a malformed or unreadable instance of the guarded kind must classify as that kind (three-way answer), never fall through to the forward-raw path
- [`../agent-fabric/memory/domains/rust-services-dev/domain/hyper-cancellation-and-head-deadline.md`](../agent-fabric/memory/domains/rust-services-dev/domain/hyper-cancellation-and-head-deadline.md) — What hyper 1.x itself does on body drop and idle connections, and how gateway tests separate it from the gateway's own guards
- [`../agent-fabric/memory/domains/rust-services-dev/domain/hyper-client-closes-finished-sized-body.md`](../agent-fabric/memory/domains/rust-services-dev/domain/hyper-client-closes-finished-sized-body.md) — A test proving the gateway closed an upstream connection must leave that response's body unfinished — hyper's client closes a fully-received sized body itself
- [`../agent-fabric/memory/domains/rust-services-dev/domain/json-line-bound-encoded-size.md`](../agent-fabric/memory/domains/rust-services-dev/domain/json-line-bound-encoded-size.md) — A byte bound on a JSON line must measure the encoded form: control chars cost 6 bytes escaped, 7 when re-embedded in a JSON string
- [`../agent-fabric/memory/domains/rust-services-dev/domain/leak-test-failure-messages.md`](../agent-fabric/memory/domains/rust-services-dev/domain/leak-test-failure-messages.md) — A secret-leak test reports a hit by name and offset, never the bytes; a drop's log line names its class, never the error's Display
- [`../agent-fabric/memory/domains/rust-services-dev/domain/loss-counter-at-the-drop-site.md`](../agent-fabric/memory/domains/rust-services-dev/domain/loss-counter-at-the-drop-site.md) — Count dropped messages where the drop happens, never from the event the drop emits
- [`../agent-fabric/memory/domains/rust-services-dev/domain/loss-counter-is-a-session-read.md`](../agent-fabric/memory/domains/rust-services-dev/domain/loss-counter-is-a-session-read.md) — Expose loss counts as a pull read scoped to the session with its open instant, not as a pushed field or a cross-session sum
- [`../agent-fabric/memory/domains/rust-services-dev/domain/managed-settings-linux-path.md`](../agent-fabric/memory/domains/rust-services-dev/domain/managed-settings-linux-path.md) — Where Claude Code 2.1.286 reads an organisation's managed settings on Linux, and how to present them without root
- [`../agent-fabric/memory/domains/rust-services-dev/domain/mcp-refused-start.md`](../agent-fabric/memory/domains/rust-services-dev/domain/mcp-refused-start.md) — Claude Code does not restart a stdio MCP server that exited; a start that cannot work serves a refusing server instead of exiting
- [`../agent-fabric/memory/domains/rust-services-dev/domain/measurement-claim-scoped-to-runs.md`](../agent-fabric/memory/domains/rust-services-dev/domain/measurement-claim-scoped-to-runs.md) — Reporting a spike or live measurement: one run is that run's outcome, not a rule; an unreproduced cause is inferred; 'no log line' means none among the patterns searched; unrecorded conditions are named
- [`../agent-fabric/memory/domains/rust-services-dev/domain/mutation-restore-mtime-stale-build.md`](../agent-fabric/memory/domains/rust-services-dev/domain/mutation-restore-mtime-stale-build.md) — restoring a file from a backup after a mutation test leaves cargo's mutated build in place; touch the file before the next run
- [`../agent-fabric/memory/domains/rust-services-dev/domain/nofollow-directory-symlink-enotdir.md`](../agent-fabric/memory/domains/rust-services-dev/domain/nofollow-directory-symlink-enotdir.md) — openat(O_DIRECTORY|O_NOFOLLOW) on a symlink fails ENOTDIR on Linux, not ELOOP; classify with fstatat
- [`../agent-fabric/memory/domains/rust-services-dev/domain/package-from-tracked-files.md`](../agent-fabric/memory/domains/rust-services-dev/domain/package-from-tracked-files.md) — Assemble a release package from the files git tracks, and refuse any untracked or ignored file; cp -R ships strays that .gitignore hides
- [`../agent-fabric/memory/domains/rust-services-dev/domain/pipefail-head-sigpipe.md`](../agent-fabric/memory/domains/rust-services-dev/domain/pipefail-head-sigpipe.md) — `cmd | head -1` under `set -euo pipefail` can end a shell script silently; quit inside the producer instead
- [`../agent-fabric/memory/domains/rust-services-dev/domain/proxy-time-bounds-are-separate-clocks.md`](../agent-fabric/memory/domains/rust-services-dev/domain/proxy-time-bounds-are-separate-clocks.md) — A streaming proxy's time bounds are three separate clocks (commit deadline, upstream idle, hard ceiling); what each one counts, and the race when a timeout fires
- [`../agent-fabric/memory/domains/rust-services-dev/domain/refusal-test-proves-its-cause.md`](../agent-fabric/memory/domains/rust-services-dev/domain/refusal-test-proves-its-cause.md) — A test that a network path is refused must assert why it failed (elapsed time against the timeout, a counter on the intended drop rule), or a missing route or a RST passes it
- [`../agent-fabric/memory/domains/rust-services-dev/domain/response-commitment-point.md`](../agent-fabric/memory/domains/rust-services-dev/domain/response-commitment-point.md) — Where a proxied response commits, and what a proxy may and may not do on each side of that point (retry, failover, its own error, an upstream failure)
- [`../agent-fabric/memory/domains/rust-services-dev/domain/s19-driver-shape.md`](../agent-fabric/memory/domains/rust-services-dev/domain/s19-driver-shape.md) — Driving a nested claude TUI through a pty: match menu text with spaces squeezed out, and redact ~/ paths
- [`../agent-fabric/memory/domains/rust-services-dev/domain/serde-derive-accepts-arrays.md`](../agent-fabric/memory/domains/rust-services-dev/domain/serde-derive-accepts-arrays.md) — A derived serde struct also deserializes from a JSON array; a field probe that must only accept objects needs a map-only visitor
- [`../agent-fabric/memory/domains/rust-services-dev/domain/serde-tagged-unit-variant-ignores-unknown.md`](../agent-fabric/memory/domains/rust-services-dev/domain/serde-tagged-unit-variant-ignores-unknown.md) — serde internally tagged enum unit variant accepts unknown keys despite deny_unknown_fields; use an empty struct variant
- [`../agent-fabric/memory/domains/rust-services-dev/domain/spike-fact-is-its-configuration.md`](../agent-fabric/memory/domains/rust-services-dev/domain/spike-fact-is-its-configuration.md) — a spike's wire observation describes the client configuration it ran; check it in the configuration the gateway serves and commit it to the record before a contract relies on it
- [`../agent-fabric/memory/domains/rust-services-dev/domain/tracing-optional-field-omitted.md`](../agent-fabric/memory/domains/rust-services-dev/domain/tracing-optional-field-omitted.md) — How to make a tracing log field appear only when a value is recorded (absent, not "None"), and validating an HTTP header's shape before logging it
- [`../agent-fabric/memory/domains/rust-services-dev/domain/tracing-set-default-misses-events.md`](../agent-fabric/memory/domains/rust-services-dev/domain/tracing-set-default-misses-events.md) — a thread-scoped tracing subscriber in a unit test misses events when other tests hit the same callsite; log tests need their own binary
- [`../agent-fabric/memory/domains/rust-services-dev/domain/validator-shares-runtime-construction.md`](../agent-fabric/memory/domains/rust-services-dev/domain/validator-shares-runtime-construction.md) — a validator that copies the runtime's acceptance rule as a pattern drifts both ways; validate by calling the constructor the runtime uses

## solution

- [`.agent-fabric/memory/rust-services-dev/solution/bridge-pull-event-double-escaped.md`](.agent-fabric/memory/rust-services-dev/solution/bridge-pull-event-double-escaped.md) — claude-channel pull mode escapes an event twice on the host line; any size bound on an event must measure both modes
- [`.agent-fabric/memory/rust-services-dev/solution/channels-allowlist-gate.md`](.agent-fabric/memory/rust-services-dev/solution/channels-allowlist-gate.md) — What registers a plugin channel in Claude Code 2.1.286: dev flag only; --channels needs Anthropic's ledger or an org allowedChannelPlugins
- [`.agent-fabric/memory/rust-services-dev/solution/claude-plugin-runtime-facts.md`](.agent-fabric/memory/rust-services-dev/solution/claude-plugin-runtime-facts.md) — Claude Code 2.1.286 plugin runtime, measured CLI-only: userConfig defaults/--config reach argv; a directory marketplace runs in place; MCP stderr goes to ~/.cache/claude-cli-nodejs
- [`.agent-fabric/memory/rust-services-dev/solution/plugin-version-from-workspace.md`](.agent-fabric/memory/rust-services-dev/solution/plugin-version-from-workspace.md) — How the Claude plugin and claude-channel carry the ADR-0065 release version: build.sh, release archive, --version stamp

## workflow

- [`.agent-fabric/memory/rust-services-dev/workflow/cross-repo-job-needs-fresh-session.md`](.agent-fabric/memory/rust-services-dev/workflow/cross-repo-job-needs-fresh-session.md) — A job in another repository must run in a session started in that clone — fabric-review brief refuses a repository outside the session's clone
- [`.agent-fabric/memory/rust-services-dev/workflow/git-rm-then-commit-stages-only-the-rm.md`](.agent-fabric/memory/rust-services-dev/workflow/git-rm-then-commit-stages-only-the-rm.md) — After `git rm` plus file edits, a bare `git commit` commits only the deletion; a build passing on the tree proves nothing about the commit
- [`.agent-fabric/memory/rust-services-dev/workflow/mutation-runs-save-the-fix-first.md`](.agent-fabric/memory/rust-services-dev/workflow/mutation-runs-save-the-fix-first.md) — Before a hand mutation to show a test fails, commit (or copy to a known path) the real change first; `git checkout <file>` to revert also wipes uncommitted work
- [`.agent-fabric/memory/rust-services-dev/workflow/narrow-before-driving.md`](.agent-fabric/memory/rust-services-dev/workflow/narrow-before-driving.md) — Before building a run driver or asking for an allow rule, check what a CLI-only path can measure
- [`.agent-fabric/memory/rust-services-dev/workflow/post-review-from-transcript.md`](.agent-fabric/memory/rust-services-dev/workflow/post-review-from-transcript.md) — How to get a review subagent's report into fabric-pr post-review without retyping it
- [`.agent-fabric/memory/rust-services-dev/workflow/review-range-with-main-merge-is-partial.md`](.agent-fabric/memory/rust-services-dev/workflow/review-range-with-main-merge-is-partial.md) — A re-review whose range contains a merge of origin/main counts as partial coverage; review merge-base..head instead
- [`.agent-fabric/memory/rust-services-dev/workflow/spike-driver-commit-before-cited-runs.md`](.agent-fabric/memory/rust-services-dev/workflow/spike-driver-commit-before-cited-runs.md) — SPIKE-001 runs: commit the driver first, then re-run every case at that commit, so the README cites one revision
- [`.agent-fabric/memory/rust-services-dev/workflow/spike-facts-cite-committed-runs.md`](.agent-fabric/memory/rust-services-dev/workflow/spike-facts-cite-committed-runs.md) — Writing a SPIKE-001 measurement — every fact must rest on a committed run's recorded field or be marked; design runs to isolate each claim
- [`.agent-fabric/memory/rust-services-dev/workflow/spike-runs-ipc-socket-path-length.md`](.agent-fabric/memory/rust-services-dev/workflow/spike-runs-ipc-socket-path-length.md) — SPIKE-001 runs with daemons fail "the daemon never served" when TMPDIR is the long session scratchpad — SUN_LEN
- [`.agent-fabric/memory/rust-services-dev/workflow/spike-tmpdir-sun-len.md`](.agent-fabric/memory/rust-services-dev/workflow/spike-tmpdir-sun-len.md) — A spike run that starts transport daemons fails "path must be shorter than SUN_LEN" with TMPDIR in the session scratchpad
- [`.agent-fabric/memory/rust-services-dev/workflow/supply-on-a-for-branch.md`](.agent-fabric/memory/rust-services-dev/workflow/supply-on-a-for-branch.md) — How a supply to another agent's branch is delivered and closed here — for/ branch off their commit, fabric-jobs deliver, REPLY
- [`.agent-fabric/memory/rust-services-dev/workflow/workflow-carried-2026-10-24.md`](.agent-fabric/memory/rust-services-dev/workflow/workflow-carried-2026-10-24.md) — To keep two open InterWeave PRs off one library crate, withdraw a small self-contained change from your own PR and re-land it after the other merges, instead of asking for an exception

## threads

- [`.agent-fabric/memory/rust-services-dev/threads.md`](.agent-fabric/memory/rust-services-dev/threads.md) — Open: INSTALL.md's headless paragraph names only the re-registered-after-reconnect line as the sign of a missing channel; channel_enable logs 'registered'

## recall

- [`../agent-fabric/identities/roles/rust-services-dev/recall.md`](../agent-fabric/identities/roles/rust-services-dev/recall.md) — Where rust-services-dev's knowledge lives — charter, remit, distilled slices, this agent's memory — and how to trace a claim to its sources.
