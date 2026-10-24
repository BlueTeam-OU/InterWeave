---
role: "architect-cto"
class: threads
topic: "owner-priority-cli-and-mcp-on-prod-host-2026-10-22"
description: "Owner 2026-10-22: a full command-line client and the Claude MCP bridge must run on the stage/production host by next week (edge-hosting agent moves there); the five REQUESTs and the open risks"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - cab9d5415bcf3cb2
---

## Owner 2026-10-22: a full command-line client and the Claude MCP bridge must run on the stage/production host by next week (edge-hosting agent moves there); the five REQUESTs and the open risks

Owner, 2026-10-22 ~07:00Z: "delivering a working full fledge cli and
claude mcp is now maximum priority as from the next week the
edge-hosting agent will be moved to the host that will have the stage
and production environment."

Read of the tree: no command-line human client exists on main (apps/:
transportctl, human-desktop, human-android, transport-daemon,
claude-channel); ADR-0059 decision 9 and plan §29 step 4 define it;
the bridge exists (Stage 16) with the plugin package build (#355) and
the marketplace install measured for the development flag only
(`--channels` without the flag needs Anthropic's or the org's allowlist,
fact 23, read from the binary); packaging/linux has the two user units
and no automated client (the fabric's).

Dispatched (job j259): rust-ui-dev-02 the CLI, batch 1 by 2026-10-29
(REQUEST 01a1626b-e820); rust-services-dev-02 the plugin install
procedure on a fresh account from the release archive, run once
(01a1626b-eff4); p2p-network-dev-01 the daemon's host deployment
procedure and unit, run once (01a1626b-f7e1); devex-tooling the tagged
release run and what blocks a tag (01a1626b-feab, by 10-24);
fabric-coordinator INFO on what the fabric owes (the client program,
the agent's profile/trust, placement) (01a1626c-077c). Stage 17's
close continues beside, lower.

**Risks named to the owner:** `--channels` on a published plugin
without the development flag; the release tag is the owner's; the
automated client is not InterWeave's.

**2026-10-22 09:23Z:** CLI batch 1 LANDED, #368 (13 work) merged
abc79056: apps/human-cli, binary interweave-human, the batch-1 verbs
with --json, exit codes 0-6 (6 = still on its way when --wait ends).
Owed: the binary in the archive/install.sh (p2p-network-dev-01 j105 +
devex-tooling, after j99). Batch 2 (cards, proposals, contacts) next.
Also landed today: #366 (5 work, 5 fix) the host deployment procedure
+ run-directory race fix; #363 (1 work, 1 fix) the release dispatch
dry run; #362 (1 work, 4 fix) ADR-0062 scan path. Open: rust-services-
dev-02's bridge INSTALL.md (session run waits on the allow entry),
p2p-network-dev-01's j99 notice derivation (the archive), rust-ui-dev-
01's j136 encrypted store.

**CORRECTION 2026-10-22 ~09:50Z, the owner:** "366 approved even if i
tell only to accelerate interweave software development not thinking at
how to deploy when we are not still know the details of infrastructure
we will rent." I over-read "on the host" into deployment procedures
(DEPLOYING.md, INSTALL.md runs, a second login, the release dry run).
Redirected: rust-services-dev-02 to the bridge's software gaps (restart
on a bad start, log file, userConfig, version/upgrade, replay across a
daemon restart); p2p-network-dev-01 lands j99 as owed release tooling
then returns to daemon work; the second-login REQUEST to
fabric-coordinator withdrawn; devex-tooling asked nothing more. The CLI
(rust-ui-dev-02) and step 3 (rust-ui-dev-01) were software and stand.
**Lesson:** when the owner names an outcome ("working X"), build X;
do not infer the operations around it.

**2026-10-22 11:02Z:** the bridge's five gaps LANDED, #371 (17 work,
3 fix) merged 7708bd35: a refused start served as an MCP refusal with
its class, profile/endpoint as plugin userConfig, status.lease_gap (+
the pull receive's lease_gap), S20/S21 measurements, INSTALL.md. Next:
j42 (the 320 KiB host-line bound, base64url by size). CLI batch 2
landed (#369). Step 5 transport half landed (#367). Carried risks on
#371: the refusal text's `claude plugin configure` is unmeasured; an
INSTALL.md bullet names one cause twice.

**2026-10-22 ~19:55Z, "proceed to what is eventually missed" (the
owner, after "arm 382"):** read of main 8ef07343: CLI batches 1–2 on
main (#368, #369), batch 3 (peers by address) written on rust-ui-dev-
02's j41 branch waiting on #378; bridge complete for the development-
flag path (#371, #375, #376); no tagged release; the notices on #381.
The one gap in real use: `--channels` needs the development flag.
Decided: the fleet takes the ORGANISATION path (`channelsEnabled: true`
+ `allowedChannelPlugins` in the host's managed settings, the
deployment's to write); no Anthropic allowlist publication asked for
now, revisited when a second organisation needs it. REQUEST 01a16532-
ab3a to rust-services-dev-02: measure it as SPIKE-001 S22 (controls:
without the setting, and channelsEnabled false), record in the spike
README, SPIKES.md, INSTALL.md, PACKAGING.md; my §19/§22 carry sentences
supplied onto their branch (job j291). Also REQUEST 01a16532-ba46 to
fabric-coordinator (login `user`): propose the ADR-033 successor —
GZCoord over InterWeave on the one host, dual run beside the relay,
`interweave-human send/listen --json` as the carrier process, one
profile per agent account, trust roster and address mapping the
fabric's; preconditions a tagged release and ADR-032's freeze. j260
(second-account run) closed as withdrawn.

*Observed 2026-10-22 (architect-cto)*
