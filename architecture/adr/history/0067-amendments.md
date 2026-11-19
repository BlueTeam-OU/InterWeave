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
operator's list admits a peer no endpoint may hold. Propagation: this
note, the log row, the digest entry, `TRANSPORT.md` §`TrustPolicyChanged`,
ADR-0037's Decision sentence with its own note, and
`failure-model.md`'s row.
