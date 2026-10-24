---
role: "architect-cto"
class: threads
topic: "adr-0059-0060-contacts-encrypted-store-2026-10-21"
description: "ADR-0059 (contact card as a chat kind, accepted before it is a contact) and ADR-0060 (human store encrypted at rest under a platform-sealed key): the owner's direction, the bounds I chose, where the work waits (plan §29, SPIKE-011, PR #332)"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5e26a2cbf0e5c974
---

## ADR-0059 (contact card as a chat kind, accepted before it is a contact) and ADR-0060 (human store encrypted at rest under a platform-sealed key): the owner's direction, the bounds I chose, where the work waits (plan §29, SPIKE-011, PR #332)

The owner, 2026-10-21, in session: an internal contact system,
"absolutely separated from the Android contact system", each person
loads name/surname/photo and sends it to a counterparty "with some tag",
bound to the id it came from; same on desktop, maybe automated clients;
then "all contacts and conversations data should go in secure storage
if not already". Before this the plan had only the local half (schema
v5 `contacts`/`contact_routes` since Stage 14, nothing writing them;
contacts UX carried out of Stage 15 as carry 11).

**Recorded on IW PR #332 (branch adr/contact-cards-encrypted-store,
3 work commits, off main 65c0ab2a):**
- ADR-0059: `contact_card` HumanChatV2 kind (name 1-64 scalars NFC,
  surname optional, photo optional ≤24,576 bytes PNG/JPEG ≤256×256),
  explicit "Share my card" only, never broadcast, received card = a
  proposal under retention rules, written to contacts only on Accept,
  photo decoded only after Accept, no contacts permission/provider
  (tree check owed), third-party cards rejected for v1. Bounds are
  mine, not the owner's words — flagged to the owner.
- ADR-0060: whole store (pages+WAL+journal) ciphertext under a 32-byte
  per-profile key never derived from the seed; Android: IWK1 custody
  seam under `background-compatible` (store must open unattended for
  unread durability); desktop v1: owner-only sealed key file at
  ADR-0038 `filesystem-only`, joins SPIKE-007's envelope later.
  Mechanism = SPIKE-011 (page-cipher rusqlite amalgamation first,
  column AEAD only as recorded fallback). Status Accepted on the
  owner's direction; property decided, mechanism spiked, as ADR-0038.
- Plan §29 Stage 21 (recorded, NOT opened; opens on the owner's word),
  SPIKE-011 in SPIKES.md + §3 table; Stage 15 carry 11 pointed to §29.

**Why these shapes:** the parser rejects any kind but `text` today, so
a new kind costs nothing shipped; a card written to contacts unasked
lets any trusted peer rename itself; column-only encryption leaves the
social graph (conversation_index, routes) in clear; `user-presence`
for the store key would lose unread content on unattended restarts.

**Owner's later words the same day, all on #332 (head 71a5035b, 7 work):** "open stage 21 after 17 closes" (plan §29 opens from Stage 17's closing record, which j148 names); "interweave data must never go on backup on cloud or work with app to move between phones" and "the backup for the future will be done between the app via interweave itself eventually, not via external provider" → ADR-0060 decision 8: no platform backup/transfer/external provider ever (allowBackup=false + total extraction rules already declared; manifest check owed in §29 step 2, the retention suite's item 13 cannot prove it); the future backup is device-to-device over InterWeave's own protocol, its own ADR later.

**Open:** blind review of e692c9e2, then re-review e692c9e2..71a5035b; owner's word to arm #332 (under
minimum); owner to confirm/adjust the photo bound and the v1 desktop
level; rust-ui-dev does the work when the stage opens; #324's ADR-0058
rows will sit adjacent in README/digest — expect a textual conflict
when the second of #324/#332 lands. Related:
[[adr-0058-third-party-notices-2026-10-21]].

**Review rounds on #332 (2026-10-21):** 6 blind rounds; 20 findings, all
judged real/partly; the ones worth remembering: the shape schema must
change with the prose (F1); migration order must scrub while the
plaintext holds its name and mark the complete copy durably (F13, F17);
"key missing" vs "damaged" need a precedence and the recovery screen a
confirmed discard (F19); Android store key needs its own alias namespace
apart from the IWK1 seed custody (F3); the desktop key is
<state_dir>/human-store.key outside human/ (F2).

**Owner's later words the same day, folded into #332:** several own
cards, choose per conversation (ADR-0059 d2); "a cli way to do
everything must exist" → ADR-0059 d9, a command-line human client on the
shared crates, the stage's one new app; transportctl gains nothing.

**Owed next (own PR after #332): ADR-0061, a person holds several
network identities** — "a user should have multiple network identities":
several profiles (each its own PeerId, store, key, trust set; ADR-0016
already allows many per host, ADR-0043 says distinct PeerIds), the
client attaching to several and the person choosing which identity a
conversation runs under; Android = several embedded runtimes or one
active profile at a time (ADR-0041 to read); recovery phrase per
identity. Read ADR-0016, 0041, 0042 custody aliases, STATE.md per-profile
store first.

**Bridge decision 2026-10-21 (DECISION 01a15d3c-20aa to rust-services-dev-01, j57):**
CHANNEL-EVENT.md §Content gains ONE exception: a HumanChatV2 contact_card
is parsed with chat-protocol's parser (never a second one), meta carries
it (app_kind, card_name, card_surname, card_photo_bytes,
card_photo_type), content stays empty; malformed and broadcast cards
are dropped with a bridge-local count, never forwarded raw; code after
plan §29 step 1 lands. Both sentences are my supply onto their branch
(j216). Lesson: the schema's `card` object must live inside the
contact_card `then` branch, not top-level properties — the
negative-conformance suite (tests/transport-contract) mutates every
top-level property of a text envelope and holds schema and parser to the
same verdict; a top-level card gave 15 disagreements and a red rust job
on the armed PR (fixed 118dab19).

**Landed:** #332 merged b03e6f30 (ADR-0059, ADR-0060, plan §29, SPIKE-011,
schema 2.1.0). **Next PR #335** (branch adr/several-network-identities):
ADR-0061 (identity = a whole profile; desktop client of several daemons;
Android one runtime per active identity, bound 4 → SPIKE-012; conversation
keyed {local PeerId, remote route}; contacts per identity never merged;
unlinkable vs counterparty, not vs network observer), plan §30 Stage 22
recorded not opened; ADR-0059 A 2026-10-21 (photo pixels only, proposal
has no Keep, contacts name/photo provenance columns — ux-designer-01's
#334 A1-A3); §29 step 1's parser three-way answer (j57). Under minimum:
owner's word to arm.

**#335 review lessons (5 rounds, 16 findings, head bd31b69a reviewed
clean, awaiting the owner's word):** (1) never promise unlinkability of
two identities against a counterparty — Identify sends listen/observed
addresses on every connection, the source address is seen, DHT records
publish addresses; and the remedy I named (relay-only, DCUtR/relay/
autonat off) is refused by config.schema.yaml's literals (ADR-0034), so
it is a revisit, not a setting; (2) an amendment claimed in a log row
must be in the body AND in every normative text (RETENTION.md's Keep
rule needed the card carve-out + case 17); (3) a chooser condition must
count identities HELD, not active, or a dormant one defaults silently;
(4) a cross-identity leak rule needs a numbered decision + §10 text +
gate case; (5) do not write "the schema enforces it" unless a migration
and a test exist; (6) an Accept rule must join a PeerId's existing
contact or Move's "never two names" is hollow; (7) ADR-0041 amended as a
triple: one runtime per active identity, stay-reachable per identity,
restart of that identity's runtime alone.

*References: adr-0058-third-party-notices-2026-10-21*

*Observed 2026-10-21 (architect-cto)*
