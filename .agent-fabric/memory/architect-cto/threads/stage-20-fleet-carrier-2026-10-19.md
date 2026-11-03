---
role: "architect-cto"
class: threads
topic: "stage-20-fleet-carrier-2026-10-19"
description: "Owner's direction 2026-10-19 that the fleet uses InterWeave as its transport while the coordination protocol stays OUT of the InterWeave repo (general-purpose transport, human-to-human too): ADR-0055 + plan §28 Stage 20 re-cut generic…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-31"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a2b22735a223f5ea
  - dd073e8ee57ea673
  - e3b0a0e3391e8dbc
---

## Owner's direction 2026-10-19 that the fleet uses InterWeave as its transport while the coordination protocol stays OUT of the InterWeave repo (general-purpose transport, human-to-human too): ADR-0055 + plan §28 Stage 20 re-cut generic (automated clients as first-party consumers); what the survey and p2p-network-dev-01 measured

The owner (2026-10-19): "go on with interweave so we should start to use
instead of current transport". Written as ADR-0055 (Proposed) and plan
§28 Stage 20 on develop-qzapp/architect-cto-01/docs/fleet-carrier
(5bd1a4bd, 4db45e6b; numbered §28 so §23-§27 citations do not move).
Shape: one daemon + one carrier process per login (systemd user units
from packaging/linux), carrier = first-party local client on endpoint
`gzcoord`, client kind `gzcoord-carrier`, channel `gzapp:gzcoord`; TO ->
direct.send, TO-ROLE/BROADCAST -> publish; a committed roster in
agent-fabric maps <host>/<login> -> PeerId + loopback port and renders
every config.yaml; durability is the carrier's (journal pending, retry
same message_id 1 s..5 min unbounded, never given up; inbound journaled
before offered); delivery stays pull; loopback one host first, no
connectivity behaviour; relay kept as fallback + control plane until the
fabric's phase 2. InterWeave's steps: transportctl identity init|show,
the two unit templates, architecture/clients/agent/AGENT-CARRIER.md,
tests/fleet-carrier (two daemons on loopback; a loopback static-bootstrap
dial has no test on main). Survey facts (2026-10-19): transportctl has no
profile-create/identity-show verb; config.yaml hand-written; XDG_RUNTIME_DIR
has no fallback; per-client queue 256, no spill; AcceptedV2 = queue
admission; no_route when the endpoint is not leased; client roles of
autonat/relay/dcutr are composed Some for every daemon profile (servers
off); mDNS is built but refuses loopback (CLAUDE.md clause corrected).
Fabric's half asked of fabric-coordinator (REQUEST sent 2026-10-19, BY
2026-10-21): ADR-033 successor, roster + bootstrap rendering, the carrier
(python over IPC v2 or claude-channel pull mode), inbox/send on the
journal, ADR-029 phase 2. Open for the owner: arming the record PR
(acceptance), whether a second PR may open while #301 waits, and whether
a fleet-wide ordered history is a requirement.

[… 6 earlier entries of this thread moved to stage-20-fleet-carrier-2026-10-19-log.md on 2026-10-28, not drained …]
2026-10-19 ~19:35Z: **#312 MERGED 3d9331a5** on the owner's word "arm 312"
(given mid-turn; armed with fabric-pr arm, basis posted). Stage 20 is
CLOSED on main: plan §28 Met block, ADR-0055 Accepted (body, README
row, digest heading), AUTOMATED-CLIENT.md status; the status value
stays stage-17. j156 and j151 delivered. Local branch deleted; main at
3d9331a5.
**2026-10-25 ~08:20Z — "can we deploy InterWeave on agent-fabric?" read from both trees (agent-fabric origin/main c43eb63b; InterWeave 0 tags).** ADR-064 (Accepted 2026-10-23) build order §7 vs main: (1) tagged InterWeave release — MISSING, no `v*` tag on origin, rule 4 forbids a checkout build; (2) roster schema + per-account provisioning — on python-dev-02's agent-fabric #373 (OPEN: client unit program, installer, new-agent step 7b, roster.schema.json); `runtime/interweave/roster.json` not built (README only); (3) carrier + adapters — #365 MERGED 2026-10-24; (4) rule 13's two-account validation on the host + the port-refusal measurement — not recorded; (5) dual run, (6) parity report in docs/live-checks, (7) owner's word amending ADR-033 — none. Also operational: the Python 3.14.9 pin (#369) needs `fabric-ctl all upgrade fabric` + `python_pin.py install` as root per host; my account's clone has no `gzcoord-carrier` on PATH yet. InterWeave side: DEPLOYING.md ran once under a scratch HOME (evidence 2026-10-22), never a second uid; each account needs lingering and its own port.
- 2026-10-25 ~08:30Z owner: "if well tag 0.1.0 then move one with all things still missed maximum priority" (the owner tags; tag not yet on origin at send). Dispatched: fabric-coordinator REQUEST (items 1-6: arm #373, the release pin, host upgrade + Python pin + lingering/ports, keys + roster, rule 13, dual run/parity/ADR-033 word); devex-tooling REQUEST (watch v0.1.0's release run, report artifacts, DEPLOYING rows, plan §22 sentences); p2p-network-dev-01 REQUEST (second-uid DEPLOYING run from the archive + the port-refusal and SO_REUSEPORT measurements, run file); python-dev-02 INFO. j321 is this thread.
- 08:35Z p2p-network-dev-01 took the second-uid run (branch docs/j125-second-uid-deployment-run); needs from the OWNER a new login `interweave-deploy` (useradd -m -U, enable-linger, no sudo, SSH login for p2p-01's key) and the v0.1.0 archive. Asked of the owner in my session. gzapp #1169 (14 work, 0 fix) is 13b's PR, head 29ac5257 with my four supplies; its review next.
**2026-10-28 08:23Z (fabric-coordinator REPLY 01a1819b-7c32):** fleet pin moved to v0.2.0 (agent-fabric #415, commit 32ade966, run 40568434157, archive sha verified); ADR-064 s6/s8 name the pin, so InterWeave text cites release.json `current`, not a version. Dual run since 10-26: 641 pairs, relay 641 / InterWeave 634, 0 dup, 0 FROM mismatch, 0 sent-and-lost, 7 "no InterWeave delivery recorded"; declared soak window opens once all 30 run one version (only python-dev-02/-03 on 0.2.0). minisig not yet checked by the fabric (their j449). Nothing asked of me.
**2026-10-28 11:50Z (fabric-coordinator REPLY 01a18257-7b2c):** all 30 accounts on InterWeave 0.2.0 (channel ok, units both); rule 13 green on 0.2.0 (python-dev-02 j97, record on agent-fabric #417 branch c1ece31f); not exercised: mixed 0.1.0/0.2.0, unit restart after refused port, duplicate window, load. Parity window counts from now; left: dual run, parity report, owner's word on ADR-033. Nothing asked of me.
**12:17Z (fabric-coordinator INFO 01a18271-12b7):** ADR-064 parity window 2026-10-28T12:00Z..29T12:00Z; rollback drill ~29T00:00Z; nothing asked of InterWeave.

*Observed 2026-10-28 (architect-cto)*
