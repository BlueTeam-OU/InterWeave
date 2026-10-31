---
role: "architect-cto"
class: rationale
topic: "cloudflare-admin-delegation-2026-10-25"
description: "Owner 2026-10-25: architect-cto holds a full-admin Cloudflare credential and mints scoped API tokens for the other agents; the token model read from Cloudflare's docs, what the owner must give, and the open handover question (another…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - eeffe8b70a4d4285
---

## Owner 2026-10-25: architect-cto holds a full-admin Cloudflare credential and mints scoped API tokens for the other agents; the token model read from Cloudflare's docs, what the owner must give, and the open handover question (another login's store is the fabric's provisioning path)

Owner's direction 2026-10-25 (~06:40Z, in the j316 session): "you have
to have full administrative account at cloudflare, so you can delegate
and create api token for other agents".

Read from Cloudflare's docs the same day (fundamentals/api
create-via-api; account-owned-tokens): two token kinds. A USER token
acts as the member and needs `User > API Tokens > Edit` (the "Create
additional tokens" template only) to mint others, each limited to the
creator's own user resources. An ACCOUNT-OWNED token (`cfat_` prefix) is
a service principal, survives a member leaving, is minted at
`/accounts/{account_id}/tokens` by a token holding `Account > Account API
Tokens > Edit`, by a member who is Super Administrator or holds "API
Token Provisioning"; a minted token's permissions are a SUBSET of the
creator's, and Super Administrators see every account token. Not
supported by account tokens: Page Rules, Registrar, Turnstile, Super Bot
Fight Mode, Zero Trust client, Intel. So "full admin" means: my member
identity Super Administrator (or Token Provisioning) on the account, and
the credential I hold an account-owned token carrying `Account API
Tokens: Edit` plus the union of what the lanes will need (Workers
Scripts, D1, Workers Routes/custom domains, DNS on the zones, Stream,
Images, Account Settings Read), else I cannot delegate what I lack.

Who needs what, from the tree (gzapp origin/main): status_web's Worker
+ D1 + custom domain (ADR-066; web-dev / edge-hosting lane); Stream and
Images for the media plane (ADR-039 §2.1), whose RUNTIME credential is
the stored `cloudflare_credentials` row set through the admin API
(§2.8) — a token I mint, the operator sets; DNS for the zones (devex-
tooling / deployment). InterWeave: nothing on Cloudflare at this
writing.

Handover rule (own-secrets skill, ADR-012): a value never passes
through me. The owner types `fabric-secrets store set CLOUDFLARE_API_TOKEN`
(and `CLOUDFLARE_ACCOUNT_ID`) in a terminal as my login. A token I mint
for ANOTHER login cannot be `store set` by me into their store; the path
is the fabric's provisioning (`provision share`, ADR-012 §5 rules 3-4,
fabric-coordinator's) or a producer pipe on their side — open, to ask
fabric-coordinator once the owner has given the credential.

**Why:** per-lane, per-purpose tokens with expiry keep a leak bounded
to one agent's scope and make rotation one lane's act; account-owned
beats user-owned because it outlives any member and is auditable by the
Super Administrator.
**How to apply:** mint account-owned tokens only, scoped to the lane's
products and zones, with an expiry; name them `<project>-<role>-<purpose>`;
never a token broader than the lane's remit; the mint runs under
`fabric-secret-run CLOUDFLARE_API_TOKEN -- <cli>` and the value goes
provider → store, never printed.

**2026-10-25 ~07:15Z — the credential exists and verifies.** The owner
set `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` in my store (own
names). Read through `fabric-secret-run` with a script that prints shape
and names only: an account-owned token (`cfat_` prefix, 53 chars) named
`deploy-agent`, status active, no expiry, account-scoped, holding Account
API Tokens Write, Workers Scripts Write, Workers Routes Write, D1 Write,
DNS Write, Images Write, Stream Write, Account Settings Read; the account
is a personal-account shape (its name is the owner's mail alias). The
user verify endpoint answers "Invalid API Token" for it, as expected for
an account-owned token; `/accounts/<id>/tokens/verify` answers active.
So delegation is possible for every permission in that list and for no
other (the subset rule). The two scripts are in the session scratchpad
(`cf-verify.sh`, `cf-policies.sh`), reusable for the next check.

**2026-10-25 07:24Z — first consumer: edge-hosting (gzapp-status Worker, their j17, REQUEST 01a171f1-d427).** They ask a token no wider than Account Workers Scripts Edit, Zone Workers Routes Edit and Zone DNS Read on gzapp.ge (and gzapi.ge), ~7-day expiry, no D1/KV/R2/Pages/billing; and name the handover routes they accept: (a) the owner types it as their login, or (b) fabric-coordinator's `fabric-secrets store put edge-hosting NAME` (stdin) + `fabric-ctl edge-hosting secrets-sync` — so a coordinator-side put verb EXISTS (policies/AUTHORITY.md "An identity's secrets": only the login or its parent writes a store). Open: how the minted value reaches the coordinator's stdin without passing through me; `provision share` of an own name from my store (ADR-012 §5 r3) is the candidate — asked in QUESTION 01a171f0. Mint only once the route is settled, so the 7-day clock does not run in a store nobody can read.

**2026-10-25 07:26Z — fabric-coordinator's answer (REPLY 01a171f3).**
No cross-login path exists today: `provision` is parent-only (ADR-038,
the parent's mirror), `provision share` copies one parent value to every
child (SHARED_NAMES) — the wrong shape for a per-login token. The fit is
a new `provision issue-key cloudflare` minter beside openrouter/openai,
run by the parent with a MINT-ONLY credential (Account API Tokens Write
alone) in the coordinator's store — the owner's call, put to them by the
coordinator; the per-login permission list is mine to write, committed,
reviewed by them; it lands with an ADR-012 amendment the coordinator
writes. Today's only route: the owner creates each token (dashboard, or
I specify and they mint) and types it into the login's store with
`fabric-secrets store set`; I hand the login the NAME and token id, never
the value. Consequence: a token I mint through the API lands in MY
process and can reach no one, so until issue-key exists I SPECIFY tokens
(name, permissions, zones, expiry) and the owner mints them in the
dashboard. Per-login provider names need no registry.

**2026-10-25 07:30Z — edge-hosting's final scope (REPLY 01a171f4-b3cb):**
j17 deploys nothing (a pause of the live Worker via the API; the live
hostname is status.gzapp.ge, the committed config says status.gzapi.ge —
a separate finding): account Workers Scripts Write; zone gzapp.ge Workers
Routes Write + DNS Write (to re-attach the custom domain on the way
back); zone gzapi.ge DNS Read; 7 days; no D1. Token name
gzapp-edge-hosting-status-worker. Handed to the owner as a dashboard
spec; j356 blocked on the owner's mint + store set as edge-hosting.

**2026-10-25 ~08:05Z — the owner's direction on the handover.** "As soon
as InterWeave is in, it permits direct secure communication, so this will
solve it": the minted value travels from my process to the receiving
agent's over InterWeave direct messaging (the fleet carrier, InterWeave
ADR-0055 / agent-fabric ADR-064, python-dev-01's build, my j321), never
through GZCoord, a terminal or a person. What makes that sound: a direct
message is Noise-encrypted and peer-authenticated node to node (end to
end between the two nodes, a relay seeing ciphertext only), one node per
login on the host, 48 KiB ceiling ample. What it still needs, so that no
SESSION sees the value: a typed handover on both bridge ends — the mint's
stdout piped into the carrier's send (sender side), and the carrier's
receive piping a secret-kind payload straight into `fabric-secrets store
set NAME` (receiver side), neither surfacing in a conversation; delivery
acknowledged (transport is non-durable, AcceptedV2 = queue admission);
the kind under ADR-0055's automated-client payloads, mine to shape with
fabric-coordinator (the carrier's). ADR-0064 identity-level E2E is
proposed, not needed for this: the node is the login. Until the carrier
is deployed: the owner mints in the dashboard and types the value (c).
fabric-coordinator's `issue-key cloudflare` minter stands as their
proposal to the owner; the owner's direction supersedes it as the
long-run shape.
Owner, right after: "at control panel level" — the handover is the
control plane's feature (the fabric's carrier integration / control
panel, fabric-coordinator's) over InterWeave as transport; NOT an
InterWeave payload kind. Corrected to fabric-coordinator (INFO
01a1720a-ish, the second INFO). InterWeave owes only its transport
facts (ceiling, AcceptedV2 semantics, non-durability).

*Observed 2026-10-25 (architect-cto)*
