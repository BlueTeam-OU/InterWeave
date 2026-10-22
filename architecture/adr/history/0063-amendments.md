# ADR-0063 — amendment notes

### Amendment 2026-10-21 — Remove of an added identity deletes its key with it, and a restore on fresh state says every identity gets a new key

rust-ui-dev proposed the acceptance criteria for the card and the key
(plan §29 steps 1–2) the same evening the record landed and asked two
things it did not say: what Remove does to the key, and whether the
recovery flow tells the person about the key change. Ruled: Remove of
an added identity (ADR-0061 decision 7) deletes its key file and its
Keystore alias with the endpoint, the conversations, the contacts and
the card — the key is the identity's — while a counterparty keeps the
contact it accepted, so a later identity on the freed label is a key
change to them; and the phrase-recovery flow on fresh state says,
before it takes the phrase, that every identity gets a new key and
counterparties will see a key change for each (decision 1 now says
so, and IDENTITY-RECOVERY.md item 12 binds `transportctl` to it;
"fresh state" is a state root with no `identity-keys/` directory). The seventeen criteria, two of them
device rows, are in `human-client-ui.md` §13 with their record lines.

### Amendment 2026-10-22 — The record's custody is a floor: a filesystem-only key file is refused at open where a wrapping key exists

Plan §29 step 2 found the desktop with no wrapping key (ADR-0060
decision 3's filesystem-only floor until SPIKE-007) and asked what the
IDK1 file is there and what Android does with such a file. Ruled: the
header carries a custody field under the associated data; the four open
refusals (magic, version, EndpointId against the file name, public key
against the secret) hold on every platform, the AEAD on top where a
wrapping key exists; a platform that holds a wrapping key refuses a
filesystem-only record at open as a downgrade, named, and the person
creates a new key (a key change, decision 5); a desktop refuses a sealed
record as unreadable here; nothing re-seals between custodies silently.
The reason is the copy: a key file moved to a weaker platform must not
quietly become as weak as that platform. Propagation: decision 2; this
note; the log row; the digest entry.
