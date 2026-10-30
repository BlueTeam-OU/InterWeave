---
role: "architect-cto"
class: rationale
topic: "reflection-a-set-two-writers-maintain-is-derived-not-kept"
description: "when an operator and a client both write into one trust list, no invariant the client \"keeps\" over it can hold; keep each writer's set apart and let the transport derive the composed set"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-28"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ca485b37996faabd
---

## when an operator and a client both write into one trust list, no invariant the client "keeps" over it can hold; keep each writer's set apart and let the transport derive the composed set

ADR-0067 decision 6 (IW #506, rounds 2-3, 2026-10-28) went through three drafts because each stated a rule over one list that two writers shared: first "an entry without a subset inherits the node's list" (empty for the authored first identity, which had no subset and no place to get one, since config.yaml is never rewritten and the overlay had no such member); then "the client keeps the node's list equal to the union of the identities' sets" (an operator's endpoint-less allow could not survive beside it, and an inheriting agent endpoint read every peer a person trusted in an identity). What held: three sets kept apart — the operator's list (written by endpoint-less calls alone), each identity's set in the endpoint overlay, the admission set DERIVED by the transport as their union at load and on every write — with each endpoint's effective set at the route ordered (overlay set, else authored subset narrowing the operator's list, else the operator's list and never the derived set), and a sentence saying who seeds the first identity and what applies before the seed.

**Why:** a "narrows, never widens" subset rule is empty for an entry that has no subset and no writable place for one; and an invariant a client maintains over a set another actor also writes forces the client to undo that actor's writes.

**How to apply:** before writing a trust or policy rule across several writers, list every writer and give each its own stored set; make any composed set a derived value with its deriver and bound named; walk the authored first entry (the one with no overlay record) through the rule; state the seeding and the pre-seed window. Related: [[reflection-a-p2-every-round-on-a-new-shape-means-the-adr-has-no-design]], [[identity-endpointid-derived-from-key-2026-10-28]].

*References: identity-endpointid-derived-from-key-2026-10-28, reflection-a-p2-every-round-on-a-new-shape-means-the-adr-has-no-design*

*Observed 2026-10-28 (architect-cto)*
