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
header carries a custody field under the associated data; the five open
refusals (magic, version, a custody this build does not know, EndpointId against the file name, public key
against the secret) hold on every platform, the AEAD on top where a
wrapping key exists; a platform that holds a wrapping key refuses a
filesystem-only record at open as a downgrade, named, and the person
creates a new key (a key change, decision 5); a desktop refuses a sealed
record as unreadable here; the one re-seal is upward and at adoption —
the desktop re-sealing its own key files when it joins the SPIKE-007
envelope, as the store key does (ADR-0060 decision 3), never at open,
never downward, never on arrival from another device — the hand-copied
state root's files are the desktop's own by then and are re-sealed with
the rest.
The reason is the copy: a key file moved to a weaker platform must not
quietly become as weak as that platform. Propagation: decision 2; this
note; the log row; the digest entry.

### Amendment 2026-10-24 — A missing key file outside recovery is a loss, and a refused file goes with the confirmed new key

Building the Android identity key (rust-ui-dev-02's j53b) and the
person's side of §13 bullets 31 and 33 (ux-designer-01's QUESTION
01a16d88), two gaps: decision 1 gave a new key silently on fresh
state through the recovery flow, where the person is warned, and said
nothing of a key file missing on a root that is otherwise there,
where the build minted a key silently for an identity whose contacts
would see a key change unannounced; decision 2 said the person
"creates a new key" after a refusal and not what becomes of the
refused file. Ruled: a missing file is a silent first key only for an
identity that never shared its card — the share record alone decides,
since the identity's own public key is held nowhere once its file is
gone (rust-ui-dev-02's question; a first draft also named the
accepted-card routes, which record other identities' keys) — else a
key-missing state with a confirmed new key; a refused or exposed file is removed only on that
confirmation and the new key created after the removal (a first
draft had the new key written first, which one Keystore alias per
identity forbids: #411's review), a gap resolving to the key-missing
state, a loose directory of the client's own tightened first and a
link or another owner's directory refused with the fix named (#411's
round 2), with the update offered first for a newer build's record and
a retry first for a cipher refusal, and nothing kept aside. The
alternative of keeping a refused file beside the live one was refused
as a second secret at rest with no reader. Propagation: decision 1;
this note; the log row; the digest entry; `human-client-ui.md` §13's
bullets 31 and 33 read with it; `design/2026-10-24-identity-key.md`
(#411), whose §5.3 states the gap's outcome for an identity that
never shared.

### Amendment 2026-10-27 — The silent first key also counts an own-card row pending outbound, or one that may have left

Building contact-cards 5.3's leaving and share rules (rust-ui-dev-02's
j54, QUESTION 01a17abc), two rules met at one column: 5.3 sets a
conversation's `card_shared_at` when the own-card row reaches Accepted
by remote transport, while the build wrote it when the card was
queued, so the 2026-10-24 amendment's "share record alone" agreed with
the build and not with the design. Written as 5.3 says, a key file
lost while the first card is still Sending — it survives a restart as
pending outbound — left no record, the next start made a key silently,
and the old-key card could still arrive, making the contact's next
card a key change the person was never told of. Ruled: `card_shared_at`
stays at Accepted and nothing earlier, since the stored value is
person-facing and a Sending card can still be cancelled; the silent
key's predicate gains the pending own-card rows of the identity, the
predicate the share-again list already reads, so one store predicate
serves both sites and the two cannot part again; a failed store read
goes on counting as shared. The alternative of writing `card_shared_at`
at queue time was refused as naming a disclosure that may never happen.
The same day, from #467's review (rust-ui-dev-02's QUESTION 01a17afd):
a card cancelled after an attempt that may have reached the remote
(`Cancelled { may_have_reached: true }`, the outcome-unknown class of
the agreed amendment A1, erring toward "may") left neither a share
record nor a pending row, so the window stayed open for it. Ruled: a
conversation records that a card of the identity may have left this
device from the first such attempt, Accepted or not, cancelled or not,
cleared by nothing short of a store discard; the silent key and the
share-again list read that record beside the pending rows, and
`card_shared_at` alone stays what Details shows. The alternative of
accepting the window was refused: the rule exists for exactly the
counterparty who may hold the old key. The record is a store change,
built in rust-ui-dev-02's later batch; until it lands, #467 names the
window as a limit and the plan's §29 carries it.
Propagation: decision 1; this note; the log row; the digest entry;
`design/2026-10-24-identity-key.md` §7 and §8 (ux-designer-01, by
locator); the code on rust-ui-dev-02's branch, one PR.

### Amendment 2026-10-28 — The key is the endpoint's, its EndpointId derived from it; the key change is withdrawn

ADR-0067 makes the identity key this record introduced the key of the
endpoint itself, for every endpoint kind, and derives the EndpointId
from its public key as the PeerId is from the node's. Decision 1 names
that; decision 2's directory is `endpoint-keys/`; decision 4's key
change — "a route whose key differs from the recorded one is a key
change: the proposal says so ... the person may accept the new key,
which replaces the old one in that route" — cannot arise, since a
different key is a different EndpointId and another identity of the
node, and a card whose key does not derive its endpoint is malformed;
the proposal, `--accept-key-change` and the replace rule are withdrawn,
and decision 6's command-line form becomes the creation of a new
identity. The key-missing state (A 2026-10-24) and the silent first key
(A 2026-10-27) stand, resolved by a new identity rather than a new key
under the old name. Propagation: the Status line; decisions 1, 2, 4, 5,
6, the Security and Operational implications (each with a tombstone
in place); this note; the log row; the digest entry; STATE.md's
`contact_routes` and key-file sentences; the README row.
