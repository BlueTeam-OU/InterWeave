# ADR-0067 — amendment notes

### Amendment 2026-11-04 — The load refusals decision 3 lists are the key's and the id's

Decision 3 read "the only refusals at load are an entry that names an
id instead of a key, an id in a file's header that does not derive
from the key beside it, and an id that collides with another entry's",
contrasting them with the key-missing state in the same clause. Read
as every refusal of the entry it was false on the day it was written
(the loader also refuses the endpoint bound and a duplicate entry) and
contradicted the alias grammar and uniqueness `ENDPOINTS.md` gave the
local alias on 2026-11-04 (`AliasInvalid`, `AliasTaken`, the retired
label's checks carried over), which PR #553's review named (F3). The
clause now says whose refusals it lists and that the alias keeps a
label's. Propagation: this note, the log row, the digest entry.

### Amendment 2026-11-19 — A connected peer is closed when it leaves the admission set, not at every revoke

Decision 6 said when a peer leaves the admission set ("when no set
holds it, the operator's included") and nothing of the connection the
peer holds at that moment. `TRANSPORT.md` said "Revoking a connected
peer also closes its data-plane connection", and ADR-0037 "A revoking
set closes the peer's connections": both written for the node's one
list, where a revoke and leaving the set were one event. Under
decision 6 they are two: an identity's revoke, or an endpoint-less
one, of a peer another set still holds removes nothing from the
admission set. A blind review of #559 (2026-11-04) named the
disagreement, outside that PR's range; it is settled here on the day
Stage 22 opened, before `PeerTrustPolicy` is built from decision 6.
The rule: the connection closes when the peer leaves the admission
set, by whichever write removed it from its last set, with the same
`PeerDisconnected { reason_class: policy }`; a revoke that leaves the
peer in another set closes nothing and is the route's refusal for
that endpoint. The parenthesis "a peer trusted by any endpoint may
connect" became "a peer in the admission set may connect", since the
operator's list admits a peer no endpoint may hold. The same day's
review of this amendment (#730: the first round's risk on directory
access, and the judgement of its F1) found that the revoke's two
other effects had no rule: `LOCAL-IPC.md` and the
`trust-set-params` schema said a revoke "drops its cached directory",
and `failure-model.md` that it removes directory access. Both follow
the close: the cache is the node's and so is the admission of a
directory query, so they go when the peer leaves the admission set.
The clause first gave as its reason that "no endpoint holds a part of
either", which the directory protocol contradicts: within an admitted
query each endpoint's row is filtered by that endpoint's own set
(`transport/libp2p/ENDPOINTS.md` §Endpoint directory protocol; #730's
second round, F4), and the clause now says so. Propagation: this note, the log
row, the digest entry, `TRANSPORT.md` §`TrustPolicyChanged`,
`LOCAL-IPC.md`'s `admin.trust.set` paragraph, the `trust-set-params` schema's description, `LOCAL-CLIENT.md`
§7 item 11, `ENDPOINTS.md`'s trust paragraph, `human-client-ui.md`'s
revoke sentence, ADR-0037's Decision sentence with its own note,
`failure-model.md`'s row and `testing.md`'s item 26. The record's Status and its Implementation
sentence said Stage 22 "opens on the owner's word"; it opened that
day, and both say so.

### Amendment 2026-11-19 — The break reaches the human store: a stored label is rebound by its key or discarded

Decision 7 said what the clean break does to profiles, fixtures, schemas
and protocol versions, and nothing of the human store, whose every
stored remote endpoint was a label: `contact_routes.endpoint_id`,
`conversation_index.endpoint_id`, a pending row's destination, the
inbound tables' and `read_pairs`' source endpoint. rust-ui-dev-02's
review of `schema.rs` at v13 (their j125, 2026-11-19) asked what the
Stage 22 migration makes of them, offering a drop of the routes and
conversations, a legacy marker column, a rebind by the recorded
identity key, or a discard of the store. Ruled: the rebind where a key
was recorded, the discard of what has none, no marker column (it would
keep the label grammar in the store's readers) and no discard of the
whole store (decision 6 seeds the first identity's trusted set so that
a person keeps the counterparties they had). The discard deletes
unread and kept content of key-less conversations, which ADR-0044
holds durable, and with it every pending row and unread card proposal
(the review of this amendment's first text, #730 round 4, F1: a
pending text carries its sender's label as `from_endpoint` and a card
is signed over it, so the first text's "payload byte-identical" would
have sent what a receiver drops) and the second copy of a message where
two copies meet because two routes derive one id; it is recorded as this record's exception, under the
owner's word of 2026-10-28 that no installed base exists and the ids
are recreated, and RETENTION.md §7 names it as the only one. The
first identity's key is the identity key it already held, ruled the
same day from decision 3. STATE.md's schema v14 and §Migrations carry
the rule; `human-client-ui.md` §13 its criterion.

### Amendment 2026-11-19 — A key sealed under a label is adopted, not renamed; the entry's `key` is the key file's name; no program rewrites `config.yaml`

Three things the record left to the builder, found by p2p-network-dev-01 while cutting Stage 22's first batch and ruled the same day. (1) Decision 3 said the entry "carries the key's reference" and that an entry whose key file is missing loads key-missing, which needs the id without the key; the reference is now the file's name, `<EndpointId>.key`, never a path and never a bare id, derived and checked against the header when the file is there and taken by grammar when it is not, with `EndpointIdNotDerived` for the retired `id` member and for any other `key` value. (2) Decision 3 and STATE.md said the directory `identity-keys/` was "renamed" to `endpoint-keys/`. That was wrong: the `IDK1` header carries the endpoint's id, a label before this record, and on Android every header byte is the seal's associated data, so a file cannot be moved into the new directory — it is rewritten, and on Android re-sealed. The loader cannot do it either: the daemon reads a key file's public part alone, and it does not start on the old entry. Hence `transportctl endpoint adopt`, offline and at filesystem custody, in decision 4, and the embedded host's re-seal on Android. (3) Decision 7 refused a profile by id "with the reason named" and did not say who writes the new entry. Ruled: no InterWeave program rewrites an authored `config.yaml`, and no package upgrade step does; the refusal names the adopt command and the entry to write. This is the clean break the owner's word of 2026-10-28 allows, there being no installed base and the fleet's profiles being re-rendered at the update. The desktop upgrade test is rewritten for it (plan §30). Propagation: decisions 3, 4 and 7; this note; the log row; the digest entry; `ENDPOINTS.md` §Configured endpoint set and §Default direct endpoint; `config.schema.yaml`; STATE.md (where the store lives, schema v14's closing sentence); plan §30 Activate.
