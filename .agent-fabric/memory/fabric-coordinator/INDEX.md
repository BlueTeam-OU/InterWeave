---
role: "fabric-coordinator"
class: index
description: "What fabric-coordinator knows and where it lives."
tier: 1
distilled_at: "2026-10-12"
---

# fabric-coordinator — knowledge index

A session is given the charter and brief (in the launch prompt),
and the project's remit with a pointer to this index (from the
session-start hook) — nothing below. Open a slice when its cue
matches what you are doing; a `workflow` slice says how a kind of
work is done here, so read the matching ones before that work.
Paths are relative to this working copy; `../agent-fabric/` is the
control plane checked out beside it.

## charter

- [`../agent-fabric/identities/roles/fabric-coordinator/charter.md`](../agent-fabric/identities/roles/fabric-coordinator/charter.md) — Owns the control plane: agent-fabric's role definitions, catalogue, routing policy, authority rules and the GZCoord protocol; the only role that changes what other roles are.

## brief

- [`../agent-fabric/identities/roles/fabric-coordinator/brief.md`](../agent-fabric/identities/roles/fabric-coordinator/brief.md) — How fabric-coordinator works day to day: the control plane's only writer — roles, routing, policy, the protocol, the launch — changed in small verified commits, distributed to every account, never project truth.

## domain

- [`../agent-fabric/memory/domains/fabric-coordinator/domain/apache-ossie.md`](../agent-fabric/memory/domains/fabric-coordinator/domain/apache-ossie.md) — Apache Ossie (ex Open Semantic Interchange, OSI) — vendor-neutral JSON/YAML spec for exchanging semantic models (datasets, relationships, metrics, ai_context) across BI, analytics and AI tools; what it is, its shape, its maturity
- [`../agent-fabric/memory/domains/fabric-coordinator/domain/apostrophe-breaks-quoted-python.md`](../agent-fabric/memory/domains/fabric-coordinator/domain/apostrophe-breaks-quoted-python.md) — An apostrophe in a comment inside a shell script's single-quoted python block silently ends the quote; bash -n passes and the failure surfaces as a runtime syntax error far from the edit
- [`../agent-fabric/memory/domains/fabric-coordinator/domain/autonat-client-crate-facts.md`](../agent-fabric/memory/domains/fabric-coordinator/domain/autonat-client-crate-facts.md) — What libp2p-autonat 0.15.0's v2 CLIENT actually does — measured, and contradicting several accepted documents
- [`../agent-fabric/memory/domains/fabric-coordinator/domain/deepseek-harness.md`](../agent-fabric/memory/domains/fabric-coordinator/domain/deepseek-harness.md) — DeepSeek Harness (dsh) — DeepSeek's open-source, everything-is-a-plugin agent harness on Cordis; what it is, its shape, what maps onto the fabric (runtime adapter, Agent Teams vs GZCoord, Agent Notes vs ADRs, credentials, approval)
- [`../agent-fabric/memory/domains/fabric-coordinator/domain/locale-suffixes.md`](../agent-fabric/memory/domains/fabric-coordinator/domain/locale-suffixes.md) — The fabric's locale suffixes: ge is Georgian (language-culture-ge, script Georgian), ru Russian; German would be de — never read ge as German
- [`../agent-fabric/memory/domains/fabric-coordinator/domain/minigraf.md`](../agent-fabric/memory/domains/fabric-coordinator/domain/minigraf.md) — Minigraf — embedded single-file bi-temporal graph database with Datalog (Rust; Python/Node/WASM/mobile bindings), pitched as graph memory for AI agents; what it is, its limits, what it could mean for the fabric's memory corpus
- [`../agent-fabric/memory/domains/fabric-coordinator/domain/slice-correction-attribution-writer-name.md`](../agent-fabric/memory/domains/fabric-coordinator/domain/slice-correction-attribution-writer-name.md) — Correction for memory/shared/domain-claude-code-attribution-reminder.md: the writer bootstrap.sh runs is runtime/claude-code/user-settings.py (attribution-off.py until 2026-09-20), and it also sets showThinkingSummaries and verbose
- [`../agent-fabric/memory/domains/fabric-coordinator/domain/software-factory-loops-graphs-harnesses.md`](../agent-fabric/memory/domains/fabric-coordinator/domain/software-factory-loops-graphs-harnesses.md) — Ivo Kund's "software factory" (2026-09-11) — planning and execution loops, a dependency graph, a harness, an outer loop; what it claims and where the fabric already matches
- [`../agent-fabric/memory/domains/fabric-coordinator/domain/yamux-silent-downgrade.md`](../agent-fabric/memory/domains/fabric-coordinator/domain/yamux-silent-downgrade.md) — Tuning libp2p yamux via any setter silently swaps the muxer to vulnerable yamux 0.12.1, and cargo-deny cannot see it; InterWeave now guards it with check_yamux_muxer.sh
- [`../agent-fabric/memory/shared/domain-claude-code-attribution-reminder.md`](../agent-fabric/memory/shared/domain-claude-code-attribution-reminder.md) — The attribution key is written by runtime/claude-code/user-settings.py, run from tools/fabric/bootstrap.py (shared)

## recall

- [`../agent-fabric/identities/roles/fabric-coordinator/recall.md`](../agent-fabric/identities/roles/fabric-coordinator/recall.md) — How to ask the control plane about itself: status, routing, lint, migration records.
