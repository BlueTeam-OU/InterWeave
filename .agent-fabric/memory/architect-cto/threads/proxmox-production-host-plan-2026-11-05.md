---
role: "architect-cto"
class: threads
topic: "proxmox-production-host-plan-2026-11-05"
description: "The production Proxmox node s13337 (cloud provider, VPN-only): owner's decisions 2026-11-05, devex-tooling's read of the node, the plan file awaiting the owner's fine review, and the provider questions"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c2563b9b39fe217f
---

## The production Proxmox node s13337 (cloud provider, VPN-only): owner's decisions 2026-11-05, devex-tooling's read of the node, the plan file awaiting the owner's fine review, and the provider questions

Owner 2026-11-05: a NEW role (infra/platform) owns the hypervisor layer;
tenants gzapp production + stage, a fleet-agents VM (edge-hosting first),
an InterWeave public node; the existing VPN (Qubes VPN VM = develop-qzapp's
net VM) is the overlay; the owner holds only the provider user
`cld9-012544@pve` (password), not root; public addressing and storage/backup
target are the PROVIDER's to answer; **all four VMs run AlmaLinux 10 latest
stable** (Alma 9 excluded: glibc 2.34 < InterWeave's 2.35 floor).
devex-tooling's read (REPLY 01a1aabc-4947): PVE 9.2.25, one empty node, plain
LVM 899 GB (133 GB unexplained), vmbr0 a private-network address/24 default route, vmbr1
"OPNsense WAN" no addr, vmbr2 a private-network address/24 "OPNsense LAN" + route
a private-network address/24 via a private-network address; token `!fabric` priv-sep off, VM admin on /vms,
no Sys.*; 73–82 ms from develop-qzapp.
Plan file: ~/.claude/plans/i-have-just-received-fluttering-elephant.md —
saved for the owner's fine review; NOT yet sent to fabric-coordinator.
Fabric-coordinator's own record: `j422-owner-answers-q1-q7` (H2 = Proxmox,
agent moves with its key, ADR-034 order wins: j433/j434 before any second host).

**Why:** every later host decision (registry rows, door, credential home, tenant VMs) traces to these words.
**How to apply:** after the owner's review, send the REQUEST to fabric-coordinator (role, ADR, registry `almalinux` enum + dnf platform file, ADR-048/038/064 amendments) and hand the owner the provider question list. See [[owner-reminder-5b-limits-at-the-second-host]], [[owner-priority-cli-and-mcp-on-prod-host-2026-10-22]].

*References: owner-priority-cli-and-mcp-on-prod-host-2026-10-22, owner-reminder-5b-limits-at-the-second-host*

*Observed 2026-11-05 (architect-cto)*
