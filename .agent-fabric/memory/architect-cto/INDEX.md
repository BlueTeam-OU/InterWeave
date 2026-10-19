---
role: "architect-cto"
class: index
description: "What architect-cto knows and where it lives."
tier: 1
distilled_at: "2026-10-18"
---

# architect-cto — knowledge index

A session is given the charter and brief (in the launch prompt),
and the project's remit with a pointer to this index (from the
session-start hook) — nothing below. Open a slice when its cue
matches what you are doing; a `workflow` slice says how a kind of
work is done here, so read the matching ones before that work.
Paths are relative to this working copy; `../agent-fabric/` is the
control plane checked out beside it.

## charter

- [`../agent-fabric/identities/roles/architect-cto/charter.md`](../agent-fabric/identities/roles/architect-cto/charter.md) — Owns what the system is supposed to be and the record of why: decision records, architecture documents, the authoritative wire contracts, cross-cutting policy.

## brief

- [`../agent-fabric/identities/roles/architect-cto/brief.md`](../agent-fabric/identities/roles/architect-cto/brief.md) — How architect-cto works day to day, in any project: owns the decisions and their delivery, writes the design first, sequences the roles, judges findings blind.

## domain

- [`../agent-fabric/memory/domains/architect-cto/domain/a-notice-content-is-a-function-of-its-merge-key.md`](../agent-fabric/memory/domains/architect-cto/domain/a-notice-content-is-a-function-of-its-merge-key.md) — a pushed state notice merged by (id, evaluated_at) must have every member recorded by the evaluation that stamped it; a member computed at dispatch changes under the key
- [`../agent-fabric/memory/domains/architect-cto/domain/adr-amendment-model-alternatives.md`](../agent-fabric/memory/domains/architect-cto/domain/adr-amendment-model-alternatives.md) — Correction for the field slice's allOf + not section — a domain slice carries its own pre-flight inline, never a [[link]] into a project corpus, and names a conformance helper's limit as a check to run, not a fact about one repo
- [`../agent-fabric/memory/domains/architect-cto/domain/antique-anthropic-code-review-surface.md`](../agent-fabric/memory/domains/architect-cto/domain/antique-anthropic-code-review-surface.md) — Antique is Anthropic's internal code review surface (2026), beside Scion source control; what is public about it and what is not
- [`../agent-fabric/memory/domains/architect-cto/domain/reflection-best-effort-ingest-caps-belong-in-the-contract.md`](../agent-fabric/memory/domains/architect-cto/domain/reflection-best-effort-ingest-caps-belong-in-the-contract.md) — On a best-effort ingest the endpoint's size caps are part of the wire shape (maxLength in the contract), because a client can only truncate to a bound it is told and an over-cap 400 under drop-on-failure is a silent loss
- [`../agent-fabric/memory/domains/architect-cto/domain/reflection-forged-identifier-guarantee-is-the-narrow-one.md`](../agent-fabric/memory/domains/architect-cto/domain/reflection-forged-identifier-guarantee-is-the-narrow-one.md) — 'A fabricated id names no row and is refused' overclaims when the id carries no table or namespace; state the narrower guarantee (unminted and non-colliding is refused; a colliding value is undetectable) and the rule that only the…
- [`../agent-fabric/memory/domains/architect-cto/domain/reflection-peer-defeatable-bound-is-the-contracts.md`](../agent-fabric/memory/domains/architect-cto/domain/reflection-peer-defeatable-bound-is-the-contracts.md) — A limit a peer's bytes can defeat into a process abort is the wire contract's bound, never a renderer's or parser's own; one number per kind of nesting, counted separately
- [`../agent-fabric/memory/domains/architect-cto/domain/reflection-sender-carrying-types-never-enter-the-rule-bearing-crate.md`](../agent-fabric/memory/domains/architect-cto/domain/reflection-sender-carrying-types-never-enter-the-rule-bearing-crate.md) — When a privacy rule is enforced by what a crate cannot depend on, satisfying a layering guard by moving types into that crate re-enables the rule; add a types-only crate instead
- [`../agent-fabric/memory/domains/architect-cto/domain/served-means-evidence-of-receipt-not-a-sent-response.md`](../agent-fabric/memory/domains/architect-cto/domain/served-means-evidence-of-receipt-not-a-sent-response.md) — a record that shows a client-held value to a third party must define "the client has it" as evidence of receipt (a later contact), never as "the server returned it" — ADR-078's call sign, 2026-10-15
- [`../agent-fabric/memory/shared/domain-claude-code-attribution-reminder.md`](../agent-fabric/memory/shared/domain-claude-code-attribution-reminder.md) — The attribution key is written by runtime/claude-code/user-settings.py, run from tools/fabric/bootstrap.py (shared)

## rationale

- [`.agent-fabric/memory/architect-cto/rationale.md`](.agent-fabric/memory/architect-cto/rationale.md) — Android accessibility (j141): the owner's 2026-10-18 decision — TalkBack carried to the release gate, the AccessKit bridge built after everything else — and the facts the next step rests on

## workflow

- [`.agent-fabric/memory/architect-cto/workflow/gateway-boundary-authority-architect-cto.md`](.agent-fabric/memory/architect-cto/workflow/gateway-boundary-authority-architect-cto.md) — architect-cto is commits.boundary_authority for BlueTeam-OU/agent-fabric-gateway (and agent-fabric): how an arm-below-minimum REQUEST is judged and answered, and that the AUTHORIZES DECISION send passes auto mode since 2026-10-19
- [`.agent-fabric/memory/architect-cto/workflow/rustsec-zbus-family-ignores-and-arming-below-minimum.md`](.agent-fabric/memory/architect-cto/workflow/rustsec-zbus-family-ignores-and-arming-below-minimum.md) — The 2026-10-18 CI outage on InterWeave (zbus-family RustSec unmaintained advisories) and what landing #297 taught about folding a supply, the deny.toml boundary, and the architect-cto AUTHORIZES send

## threads

- [`.agent-fabric/memory/architect-cto/threads.md`](.agent-fabric/memory/architect-cto/threads.md) — Owner's direction 2026-10-19 to close InterWeave Stage 17 (plan §20), with a new gate clause: ux-designer reviews the Android app against Telegram/WhatsApp standard; the per-clause gap read from main and the role sequence (j148)

## recall

- [`../agent-fabric/identities/roles/architect-cto/recall.md`](../agent-fabric/identities/roles/architect-cto/recall.md) — Where architect-cto's knowledge lives — charter, remit, distilled slices, this agent's memory — and how to trace a claim to its sources.
