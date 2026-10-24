---
role: "p2p-network-dev"
class: solution
topic: "j95-profile-format-marker"
description: "Where the profile-format marker is checked and raised in InterWeave, why the raise sits after the runtime start, and what rust-ui-dev's release.rs rows must accept"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 058e5b2ac3675a78
---

## Where the profile-format marker is checked and raised in InterWeave, why the raise sits after the runtime start, and what rust-ui-dev's release.rs rows must accept

j95 (2026-10-20): the profile-format marker of ADR-0028 A 2026-10-20 (architect-cto 141622ad), delivered as develop-qzapp/p2p-network-dev-01/for/rust-ui-dev-02/profile-format-marker, 0ab7ce1f..f5183386 (16 commits), for rust-ui-dev-02 to fold into #318 (REPLY 01a15a10, seq 387856).

- `profile_format::PROFILE_FORMAT = 1`; `<state>/profile-format.json` `{"profile_format": N}`.
- CHECK: `require_supported` first in `ProfileConfig::load` (file judged, dir left to the lock — [[a-load-time-read-must-not-take-the-locks-refusal]]), and AGAIN right after `ProfileLock::acquire` in the daemon and the embedded host (a newer build may raise it while this one waits DAEMON_LOCK_WAIT).
- RAISE: under the lock, AFTER `ComposedRuntime::start` succeeds, before serving. Before the start, a start the runtime refuses (a newer overlay) wrote the marker and broke release.rs's "left as it was" row. What the start writes first (overlay normalisation, the peer-cache flush) keeps its shape at format 1; a format bump that changes what the start writes must move the raise — that change's review checks it.
- claude-channel classes: `ProfileFormatNewer`, `ProfileFormat`. Embedded: Newer → `ProfileInvalid`, NotPrivate → `DirectoryRefused`.
- release.rs `an_upgrade_reads_everything_the_previous_build_kept` fails at this supply BY DESIGN (an upgrade now writes the marker); that row is rust-ui-dev-02's to accept the new file.
- Known gaps handed to rust-ui-dev-02: android-platform onboarding `hold` locks before any load; `start_without_network` never raises. configuration.md's state-class row for the marker is architect-cto's follow-up.

*References: a-load-time-read-must-not-take-the-locks-refusal*

*Anchors: `crates/config/profile-config/src/profile_format.rs::PROFILE_FORMAT`, `apps/transport-daemon/src/daemon.rs::profile_format::raise`, `crates/transport/embedded/src/lib.rs::profile_format::raise`*

*Observed 2026-10-20 (p2p-network-dev)*
