---
role: "architect-cto"
class: threads
topic: "owner-priority-cli-and-mcp-on-prod-host-2026-10-22"
description: "Owner 2026-10-22: a full command-line client and the Claude MCP bridge must run on the stage/production host by next week (edge-hosting agent moves there); the five REQUESTs and the open risks"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 310587fbb6af499c
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

**2026-10-24 ~11:20Z, the owner: "do the fabric works to achieve
deployability. I see python guy idleing."** ADR-064 (fabric, accepted
10-23, #356 merged) had no code: no runtime/interweave/, no PR.
InterWeave ships no automated client by decision (packaging/linux
README: the deployment's program runs from ~/.local/libexec/interweave/
client under interweave-client@.service; tests/automated-client is the
conformance reference). REQUEST 01a16d-series to python-dev-01 (lowest-
numbered running, no branch on the path): carrier program over the IPC
v2 data socket (rules 5-10), signed roster (2-3), send/inbox adapters +
arrival log + parity report (11), two-account validation (13); develop
against a checkout build, deploy from the pinned tag. INFO to fabric-
coordinator (login user) to re-home if their queue says. Remaining
preconditions: the tag (InterWeave #402 + owner), the version pin
(fabric-coordinator). Control plane stays on the relay (rule 12) until
parity; then a second endpoint on the same profile by amendment.

**2026-10-25 11:50Z, j321 closed.** The carrier is BUILT and on
agent-fabric main: python-dev-02 took the re-homed REQUEST (fabric-
coordinator REPLY 01a16dae-7264; python-dev-01 dropped its j161),
#365 merged c43eb63b 2026-10-24 19:00Z (tools/fabric/gzcoord/carrier/,
13 suites, live rehearsal against two daemons of checkout d2bbb46d,
which is inside v0.1.0) and #373 merged d1727fed 2026-10-25 09:49Z
(client unit program, installer, roster schema) on the owner's word
"arm 373" relayed in INFO 01a1724a. No contract-gap OBSERVATION came
back; the carrier's names (client kind `automated-client`, endpoint
`gzcoord`, media types under `application/vnd.agentfabric.*`) fit
LOCAL-IPC's free-text client_kind and 128-char media_type. v0.1.0
tagged by devex-tooling on the owner's word (71961578, signed tag
bd641127); release run 40179792207 FAILED at APK signing (empty
release environment); the environment was populated 10:15-10:18Z
(four secret names + FIRST=true, read by name) but no re-run as of
11:45Z (INFO 01a172e2 to devex-tooling). Still open, not mine: the
re-run and the release page (devex-tooling/owner), the pin
runtime/interweave/release.json and rule 13's two-account check
(fabric-coordinator j401), the second-uid deployment run (p2p-network-
dev-01 j125 under the new interweave-deploy login).

**2026-10-25 12:05Z, j322 (CLI batch 4 verification).** v0.1.0 is
PUBLISHED: the owner filled the release environment and re-ran run
40179792207's failed jobs; attempt 2 passed all five at 11:48Z; the
page (not a pre-release, on 71961578) carries the signed APK, the
Linux archive, THIRD-PARTY-NOTICES-android.md, a .sha256 each, the
Made-with-Slint badge and the Android notices inline — read by me from
the page, the run and the hash assets. The owner's step-6 environment
steps were done at 11:54Z (INTERWEAVE_RELEASE_CERT_SHA256 present, no
variables; read by name from the API). Recorded in plan §22 on IW #442
(db424cce, 45d65481). CLI batch 4 (#429, merged 2026-10-24 23:33Z:
identity show/new-key, --accept-key-change, contacts route delete,
peers add -/--file) verified against ADR-0059 d9, ADR-0062 d6,
ADR-0063 d6 and §13: no code gap, nothing owed to rust-ui-dev-02.
Prose gaps fixed on #442: §13 said `contact show` (verb is `contacts
show`); ADR-0059 d9 pointed at architecture/clients/human/ for the
client's record, which is apps/human-cli/README.md (A 2026-10-25 (v),
also recording that `contacts route delete` succeeds only once Stage
22's Move puts a second route under a contact; plan §30 step 2 names
Move and the route-delete control). The whole CLI (batches 1–4) and
the bridge are on main and in v0.1.0's archive (the archive's bin/
interweave-human claim is the README's; not opened by me).

**2026-10-26 04:44Z p2p-network-dev-01 REPLY 01a17685-d456 (to my
01a1722c, j125):** v0.1.0 release page up, Linux archive checks against
its .sha256; account `interweave-deploy` exists (own private group,
lingering yes). BLOCKED on one root step: the account has no
authorized_keys, so their SSH login is refused (publickey). The one-line
install command with their public key is in p2p-network-dev-01's
session for the owner (creates ~/.ssh 0700 + authorized_keys 0600 for
that account only). Work branch
develop-qzapp/p2p-network-dev-01/feat/j125-v0-1-0-on-this-host; the
docs/ branch they first named carries nothing and goes. They start the
run when the login answers.

**2026-10-28 09:07Z (p2p-network-dev-01 REPLY 01a181c3):** j134 item 3 met: v0.2.0 tag/run/archive verified, control_plane + two_hosts pass from the archive; record on #499 (packaging/linux/evidence/2026-10-28-v0.2.0-release-check.md); not shown: APK, second machine, fresh install, the fabric carrier process. j409 now blocked on #499.

**rust-ui-dev-02 REPLY 01a181d8-d0ed (09:30Z):** G8+G7 undertaken as their j76, after their B2b PR (feat/j68-start-window-b2b, 14 work) lands; shape of peers --json proposed before build if it changes today's output.

**p2p-network-dev-02 REPLY 01a181d9-74ab (09:31Z):** peer-list 1.2 last_seen_ms taken as their j88; branch named when pushed; they will say last-established vs last-frame before I write the contract text.

**j445 (10:50Z):** peer-list 1.2.0 contract text 9d6750c8 on for/p2p-network-dev-02/peer-list-1-2-last-seen from e43afd19: last_seen_ms = retained connection lifetime (their measure accepted, not last-frame/last-establishment), 2.12, null vs absent, 456-row page from 2.12, minor history 12/11/10 (the sentence had missed 2.11). REPLY 01a18207-5259-7eb5-b634-16e97256adf3. Their PR follows their suite.

**10:41Z (p2p-network-dev-01 REPLY 01a18219-70fb):** j134 delivered: control_plane.rs #478 (79c84b29), two_hosts.rs #475 (cf7089b5)+#478, v0.2.0 release check #499 (04dcace5); RUSTSEC fix 281a8707 on main; carried: tag key placeholder user ID (owner), APK not run, no second machine, no fresh install.

*Observed 2026-10-28 (architect-cto)*
