---
role: rust-services-dev
class: remit
project: interweave
description: "What the services role covers in InterWeave: the Claude Code Channel bridge and its plugin package, from Stage 16's carry."
origin:
  - agent: user
    host: develop-qzapp
  - agent: architect-cto-01
    host: develop-qzapp
---

# rust-services-dev — remit in InterWeave

The charter (agent-fabric
`identities/roles/rust-services-dev/charter.md`) is the function; this
is what it covers in this repository. Written by fabric-coordinator on
2026-10-20 from architect-cto-01's proposal, when it gave
rust-services-dev-02 the Stage 16 carry (plan §22) and the holder
asked whose lane it was. The bridge is a protocol adapter, the
charter's own clause: until then it was p2p-network-dev's. A role that
wants its remit changed proposes it.

**Yours here.** `apps/claude-channel/`, the Claude Channel bridge — the
MCP server between Claude Code and the transport daemon over IPC v2 — as
the protocol adapter the charter names: its process lifecycle, streaming
and cancellation over the socket; its packaging
(`packaging/claude-plugin/`, `architecture/plugin/PACKAGING.md` for what
it must be); and every measurement the plan (§19 "Carried by name, from
SPIKE-001", §22) carries for the bridge — marketplace distribution and
`--channels` with a published plugin, `userConfig` substitution,
organisation-policy gating, permission relay, size limits on content and
meta, behaviour under load, the 2026-07-28 revision, non-interactive
delivery and the untried `-p` timings — each recorded in `SPIKES.md`.
Which holder takes which is architect-cto's to assign. They reuse
SPIKE-001's drivers and record their evidence under
`spikes/spike-001/runs/`, as plan §19 step 5 does: a new run directory
there for a bridge measurement is yours to add, the one exception to
`spikes/` being p2p-network-dev's; a change to the drivers or to an
earlier run is asked of their holder as a supply.

**Where it meets the others.** The daemon, the substrate and the IPC v2
code stay p2p-network-dev's (`crates/`, `apps/transport-daemon`,
`apps/transportctl`): what the bridge needs there is asked of a holder
as a supply onto your branch, never edited in place. The IPC v2 contract
(`architecture/contracts/LOCAL-IPC.md`, the bridge's default grant
policy included) and the channel semantics (ADR-0002) are
architect-cto's. The GZCoord protocol the bridge carries — what a
message is, its grammar, semantics and conformance — is
fabric-coordinator's (`agent-fabric/communication/gzcoord/protocol/`),
and the bridge is built to those rules; a payload it cannot carry as
specified is a finding to fabric-coordinator, never a redefinition here.
`architecture/` and its stage records are architect-cto's to write and
the owner's to close (`architecture/plugin/` decisions that touch what a
message is route to fabric-coordinator): you report what a step proved
in its record and never flip a stage's status. `.github/`, `.claude/`
and `tools/gh/` are devex-tooling's. Anything that leaves this host — a
published plugin, a marketplace entry — is a QUESTION to architect-cto
first.

**The rules that are not style.** The workspace's rules hold for you
as for every Rust role here: `[workspace].members` grows one stage at a
time, a libp2p or other feature flag is a dependency decision, and the
host's constraints stand (`cargo -j 2`, two test threads, one
invocation at a time, no target dir on tmpfs). Two open pull requests
never change the same crate, nor both the workspace `Cargo.toml` or
`Cargo.lock`. The merge queue is on; the review is the review class's
blind review of the finished head, posted on the PR. The project's
`CLAUDE.md` §9 and `pr-lifecycle` skill carry the procedure.

**More than one holder.** The role has two holders here
(rust-services-dev-01 and rust-services-dev-02), each with its own
pull request; architect-cto assigns each its share of a carry, and two
open pull requests never change the same crate. Each blind-reviews its
own; the other may be asked for a second reading, never as the review.

**Where the work is.** `workspace.metadata.interweave.status` in
`Cargo.toml` says which stage is open; the stage, its carries and the
lane each is in are its section of
`architecture/roadmap/BOTTOM-UP-IMPLEMENTATION-PLAN.md`. Read it
against the tree before each step.
