---
role: "architect-cto"
class: rationale
topic: "stage-host-s13337-route-a-2026-11-09"
description: "Owner's word 2026-11-09: the InterWeave CLI + Claude MCP bridge stage host is s13337 and the deploy route is (a), edge-hosting runs DEPLOYING.md in its own login there; why not the fabric-coordinator account; the three messages that carry…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ed4ed4dfe3263c38
---

## Owner's word 2026-11-09: the InterWeave CLI + Claude MCP bridge stage host is s13337 and the deploy route is (a), edge-hosting runs DEPLOYING.md in its own login there; why not the fabric-coordinator account; the three messages that carry it (j259)

The owner (2026-11-09, after asking why not deploy in the fabric-coordinator account): "the stage host is s13337, go with route (a)". Route (a) = edge-hosting runs `packaging/linux/DEPLOYING.md` in its own login on s13337; no deploy login, nothing in the coordinator's account.

**Why not the coordinator's account** (given to the owner from the tree): one profile per account (ADR-0055 d5, DEPLOYING.md), so a daemon there is the coordinator's profile, reachable by no other login; that login is the host operator with sudo and the operator signing key every control daemon trusts, the worst place for an inbound network daemon; and no other role executes in it (the refused cross-uid shell). When the control plane itself rides InterWeave, the coordinator gets its OWN profile in its own account, beside the stage deployment.

**Carried by:** REQUEST 01a1c187-1875 to fabric-coordinator (registry entry, edge-hosting placed and provisioned on s13337, ADR-064/048 amendments naming the host); REQUEST 01a1c187-1a4d to edge-hosting (deploy on the placement word; acceptance: version, PeerId, unit status, port, one end-to-end message; evidence file to p2p-01); REPLY 01a1c187-1d46 to p2p-network-dev-01 (route (c) closed). j259 re-blocked on those two. Host facts: [[proxmox-host-s13337-2026-11-09]]; origin: [[owner-priority-cli-and-mcp-on-prod-host-2026-10-22]].

**Replies (2026-11-09 evening):** fabric-coordinator undertakes the placement as their j715 after j704/j706 and will ask the owner for the s13337 operator login, its key and ssh reachability (REPLY 01a1c188). edge-hosting took the deployment as their j27 (REPLY 01a1c189); my answers (REPLY 01a1c18a): profile name `edge-hosting`; the end-to-end far end is a scratch second profile in their own account on another loopback port (DEPLOYING.md's recorded shape), a cross-host peer waits for the 5b overlay decision; listen 127.0.0.1 until the overlay, then wildcard never the overlay literal; minisign pin `packaging/release-keys.toml` + `tools/ci/minisign.pin`, read from main's tip (ADR-0066). Latest release is v0.2.0; DEPLOYING.md is not in the archive, followed from main.

**Correction (fabric-coordinator REPLY 01a1c18c-7c87, the owner's word in their session, 2026-11-09):** s13337 is the Proxmox NODE and runs gzapp production; no agent runs on it. The stage host is **H2, a VM on that node, AlmaLinux 10**, created by fabric-coordinator through a pool-scoped PVE API token the owner mints; the registry entry names the VM's address, edge-hosting's login goes on the VM. Told edge-hosting (INFO to them, 2026-11-09): wait for the coordinator's word, read "s13337" as "H2"; EL10 porting findings expected (Fedora-only proofs so far; glibc floor met; linger/polkit, SELinux on home binaries to check).

*References: owner-priority-cli-and-mcp-on-prod-host-2026-10-22, proxmox-host-s13337-2026-11-09*

*Observed 2026-11-09 (architect-cto)*
