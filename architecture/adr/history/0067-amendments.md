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
