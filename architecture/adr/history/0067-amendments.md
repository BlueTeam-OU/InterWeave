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

Three things the record left to the builder, found by p2p-network-dev-01 while cutting Stage 22's first batch and ruled the same day. (1) Decision 3 said the entry "carries the key's reference" and that an entry whose key file is missing loads key-missing, which needs the id without the key; the reference is now the file's name, `<EndpointId>.key`, never a path and never a bare id, derived and checked against the header when the file is there and taken by grammar when it is not, with `EndpointIdNotDerived` for the retired `id` member and for any other `key` value. (2) Decision 3 and STATE.md said the directory `identity-keys/` was "renamed" to `endpoint-keys/`. That was wrong: the `IDK1` header carries the endpoint's id, a label before this record, and on Android every header byte is the seal's associated data, so a file cannot be moved into the new directory — it is rewritten, and on Android re-sealed. The loader cannot do it either: the daemon reads a key file's public part alone, and it does not start on the old entry. Hence `transportctl endpoint adopt`, offline and at filesystem custody, in decision 4, and the embedded host's re-seal on Android. Adopt covers an identity alone: an endpoint that was no identity (the bridge's, an automated client's) never had a key, ADR-0063 having keyed identities only, so it is given one by `endpoint init` and its id is new; adopt refuses an alias with no key file by naming `endpoint init`, and the refused start names whichever of the two applies to each entry (rust-services-dev-02's finding against this note's first text, the same day). (3) Decision 7 refused a profile by id "with the reason named" and did not say who writes the new entry. Ruled: no InterWeave program rewrites an authored `config.yaml`, and no package upgrade step does; the refusal names the adopt command and the entry to write. This is the clean break the owner's word of 2026-10-28 allows, there being no installed base and the fleet's profiles being re-rendered at the update. The desktop upgrade test is rewritten for it (plan §30). Propagation: decisions 3, 4 and 7; this note; the log row; the digest entry; `ENDPOINTS.md` §Configured endpoint set and §Default direct endpoint; `config.schema.yaml`; STATE.md (where the store lives, schema v14's closing sentence); plan §30 Activate.

### Amendment 2026-11-19 — The trust methods' endpoint dimension is part of IPC 3.0, not a minor

Decision 6, the Consequences and the Implementation implications each said the trust methods gain the endpoint dimension "as an IPC minor", written on 2026-10-28 when the IPC was major 2 and the dimension would have been a minor of it. Decision 7 then moved the IPC to major 3 in the same release, and writing major 3's rules into `LOCAL-IPC.md` showed the two do not fit: the dimension is a new member of two closed params shapes, which ADR-0017 makes a major once a production build speaks the shape (A 2026-10-07), and major 3 has no production build until the Stage 22 release is tagged. Under ADR-0017's pre-release rule a schema takes an additive property into its `x.0` itself, so the dimension is part of IPC 3.0 and of the release's first build, with no minor to negotiate and no gate in a mirror. The cost is a constraint on the release: it is not tagged between the batch that opens major 3 and the batch that builds the dimension. Propagation: decision 6, the Consequences, the Implementation implications; this note; the log row; the digest entry; `LOCAL-IPC.md` §Version negotiation and phases; `ENDPOINTS.md` §Configured endpoint set; plan §30 Activate.

### Amendment 2026-11-21 — The Android profile document is the app's own: two forms, replaced at three moves, its first key minted at onboarding

Found by p2p-network-dev-01 while building the entry by key (Stage 22's batch): the Android first-run document carried `- id: human`, written before Welcome, which the loader of this record refuses (`EndpointIdNotDerived`), and no key existed at that moment to name instead. Three ways were offered: the caller passes the id, the key minted before provisioning; no authored entry, the first identity made through the overlay's `added`; provisioning moved after onboarding's seal. Ruled the same day. The overlay route was refused: ADR-0061 decision 1 makes the first identity the profile's own configured endpoint and its decision 7 leans on that (never removed, only turned off), so a first identity among `added` would change that record's substance. A key before Welcome was refused: decision 4 mints a key where the endpoint is created, and ADR-0063 decision 1 has a restore bring no endpoint key back, so nothing to name exists before Create or Restore. Provisioning after the seal, the first ruling, did not survive rust-ui-dev-01's reading of the app: `config.yaml` was read before the seal, for the onboarding state, for the unlock policy every seal reads and for the log level. Hence two forms of one document and three moves between them, in `ENDPOINTS.md` §Configured endpoint set. Decision 7 said "no InterWeave program rewrites an authored `config.yaml`", and `ENDPOINTS.md` already said of Android that "the profile there is composed, not authored"; the amendment says what that leaves the app: the document that opens with the first-run marker is its own, it replaces it at those moves alone, an earlier release's marker document among them, and a document without the marker is refused and left. `runtime.android.endpoint` takes the id alone and loses the schema default `human`, a label; it is needed for a start, not for a load, the start's refusal being `ConfigError::AndroidNoIdentity`. Stated as the rule and not verified in the app as this was written: that no choice of the person's is kept in `config.yaml` on Android, which is what makes a whole-file replace lose nothing. Read from the tree the same day by p2p-network-dev-01 and by this record's author, and measured on no phone: in the Android crates the file's readers take the onboarding state, the unlock policy and the log level and none writes it, its writers being the provisioning and, in the debug build alone, the instrumentation's stand-in profile.

### Amendment 2026-11-21 — The direct vectors' directory is `fixtures/direct-v3/`

Decision 7 listed the fixture directories whose vectors are regenerated under the derived EndpointId, and named the direct one `fixtures/direct-v2/`, its name while the protocol was `/interweave/direct/2.0.0`. The Stage 22 build moved the directory to `fixtures/direct-v3/` with the protocol's major (`DIRECT.md` §Selected primitive; `fixtures/direct-v3/README.md` says the same), so the body named a path that no longer existed, and the two schemas whose `x-contract.fixtures` listed the fingerprint vector under it failed `validate_contracts.py` until they named the new one. The body now names the directory as it is and says what it was. Nothing decided changes: which vectors are regenerated, the frames' `V2` names and the fingerprint vector's bytes stand as they were.

### Amendment 2026-11-21 — The IPC vectors' directory is `fixtures/ipc-v3/`

Decision 7 named the IPC fixture directory `fixtures/ipc-v2/`, its name while the IPC's major was 2. The Stage 22 build moved it to `fixtures/ipc-v3/` with the major, and its two files with it (`ipc-v3-payload-fit.json`, `ipc-v3-frame-golden.json`; `fixtures/ipc-v3/README.md` says the same), so the body and the plan's fixture table named a path that no longer existed; p2p-network-dev-01 reported both while building the batch. The body now names the directory as it is and says what it was. Nothing decided changes: which vectors are regenerated and the 128 KiB ceiling they measure stand as they were. The `tests/ipc-v2` suite keeps its name, as `tests/direct-v2` does.

### Amendment 2026-11-22 — An endpoint overlay keyed by a label is carried to the id by adopt, init and M3; the loader refuses a configured alias and maps nothing

Found by rust-ui-dev-01 in the committed upgrade tree and read on the Stage 22 batch the same day: the previous release's `endpoint-overlay.json` was `{"enabled": {"claude": false}}`, keyed by the label. Decisions 3 and 7 said what becomes of a label in `config.yaml`, in a key file's name and in the human store, and nothing of the one in the overlay. As the gap was found, the loader read the label through the grammar's transitional path and normalisation dropped it as an unconfigured endpoint's, so the endpoint the operator had turned off came back enabled across the upgrade, silently; with that path deleted the file would have been refused as unparseable, with no command to repair a state file nobody authored. Ruled the same day. The loader stays strict and maps nothing, since a mapping at load is the compatibility window decision 7 refuses. The carry belongs to the step that makes the id, which is the one that holds both names: `endpoint adopt`, `endpoint init --alias <label>`, and on Android the move that writes the document's second form; the overlay is the transport's own state, so carrying it rewrites no authored file. A refusal for every label was refused: a label matching no configured alias is a retired endpoint's, can change nothing that runs, and would have been a refusal with no command that fits, so it leaves as an unconfigured id does, and the refusal is kept for a label that is a configured alias, where the named command ends it. A third command to drop a label, and telling the operator to remove the file, were both refused: the file is authorisation. p2p-network-dev-01 measured that no release from v0.1.0 to v0.4.0 had a caller that wrote either member on a phone, so the Android carry finds nothing in practice and exists so that the strict rule cannot leave a phone refused with no command. `ENDPOINTS.md` §Configured endpoint set carries the rule.

### Amendment 2026-11-22 — The endpoint directory is `/interweave/endpoints/2.0.0`, its read bound 1710 bytes

Decision 7 said the versions say the break and named the direct protocol's new id and the IPC's new major; it did not name the endpoint directory's protocol, which read `/interweave/endpoints/1.0.0` in every text. p2p-network-dev-01 asked, building the batch, whether the directory keeps its id with entries of the new grammar or takes a major. It takes a major. The decision is a rule ("by version and by name, never frame by frame") and its list of ids was an instance of it, not its extent; the same sentence already gave the endpoints schema family a major; and ADR-0017's test answers the same way, since a `1.0.0` reader refuses nothing about a 52-byte entry it then treats as a label while a reader of this release refuses every label. Under one id a node of the release before, answering honestly, would have been read as a `ProtocolViolation`; with the major the two negotiate no common protocol and the querier reports `ProtocolUnsupported`.

The same ruling fixes the ceiling. `1.0.0` bounded a response at 1 + 8 + 4 + 1 + 32 × 65 = 2094 bytes, 32 entries at the 64-byte label ceiling. Every entry is now exactly 52 bytes, so the largest legal response is 1 + 8 + 4 + 1 + 32 × 53 = 1710 bytes, and the codecs' read bound is that number: decision 2's "under every 64-byte ceiling" meant no ceiling had to grow, not that a read bound stays where no legal frame reaches it, and a bound no legal frame reaches is one no test can fail on. The frame vectors under `fixtures/endpoints/` are regenerated under it, as decision 7 already said of that directory. The frames keep their `V1` names and tags.

Propagation: the body's decision 7; `ENDPOINTS.md` §Endpoint directory protocol; ADR-0031, ADR-0036 and ADR-0047, each with its own note; the `endpoints/directory-response` schema's `wire-protocol`; DESIGN.md, data-flows.md, rust-blueprint.md and the README's two protocol lists; the digest. The plan's Stage 8 section and the dated reviews keep `1.0.0` as records of their time.
