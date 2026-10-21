# Human-client application state and persistence

Status: design only. Message-content retention is normative through [`RETENTION.md`](./RETENTION.md) and ADR-0044.

## Selected store

Use a versioned SQLite database behind a Rust `human-store` crate on desktop and Android. The application storage schema is independent of transport configuration/state and may be deleted/rebuilt without changing PeerId or trust policy.

The store is **not** a conventional permanent conversation-history database. Message content is durable only in the retention states defined by `RETENTION.md`.

The store is encrypted at rest as a whole — pages, write-ahead log and journal — under a per-profile store key the platform's custody seals (A 2026-10-21, ADR-0060): on Android an `AndroidKeyStore`-wrapped key under the `background-compatible` policy, on the v1 desktop one owner-only sealed key file `<state_dir>/human-store.key`, under the state root and outside this directory so a copy of `<state_dir>/human` leaves it behind, at ADR-0038's `filesystem-only` level, moving into ADR-0038's envelope when it lands. The mechanism is SPIKE-011's to select within ADR-0060 decision 4; an invalidated key is a store-health condition with discard-and-recreate offered directly; a missing key or a damaged file goes to recovery mode (§Migrations), never an identity condition.

## Logical tables

Conceptually:

```text
contacts(contact_id, display_name, avatar_ref?, notes?, created_at, updated_at)
contact_routes(contact_id, peer_id, endpoint_id, device_label?, verification_note?, last_seen?)
conversation_index(conversation_id, peer_id, endpoint_id?, channel_id?, title?, last_activity?, last_shared_card?)   -- last_shared_card: A 2026-10-21, ADR-0059 decision 2, content-free, in the §Logical tables review
own_cards(card_id, label, name, surname?, photo?, created_at, updated_at)   -- A 2026-10-21, ADR-0059 decision 2: the person's own cards, written by them, outside ADR-0044's three states
pending_outbound(local_row_id, conversation_id, app_message_id, transport_message_id,
                 destination_peer?, destination_endpoint?, channel_id?, media_type?, payload,
                 created_at, last_attempt_at?, retry_state)
unread_inbound(local_row_id, conversation_id, app_message_id, source_peer, source_endpoint?,
               channel_id?, media_type?, payload, received_at)
kept_inbound(local_row_id, conversation_id, app_message_id, source_peer, source_endpoint?,
             channel_id?, media_type?, payload, received_at, read_at, kept_at)
settings(key, value)
read_pairs(origin, app_message_id, at)
```

`read_pairs` (A 2026-10-03, Stage 15 Q5) is the bounded duplicate-suppression record RETENTION.md §5 allows: a content-free pair naming where a message came from (`origin`: its source peer and endpoint for a direct message, its source peer and channel for a broadcast — the store's own inbound identity, so two peers reusing one id on a channel are two pairs, never one) and the application id the sender gave it, with the time the pair was written — nothing else, pinned by the store's shape guard. It is written when a row is marked read and when Keep is removed, so a copy of a message that was read and not kept, re-sent after a restart, does not reappear as unread; a copy carrying the same id with different text is suppressed too, because the id is the sender's claim that it is the same message and a content-free table must take it at its word. At most 4096 rows; the oldest is evicted inside the transaction that inserts the newest. It joins the owner's privacy review beside `conversation_index` and `contact_routes.last_seen`.

A current-session RAM model may also hold transport-terminal outbound messages and read-but-unkept inbound messages for display. Those rows are not serialized as general message history.

No transport private key, trust allowlist, endpoint lease, Kademlia bucket, relay reservation, AutoNAT evidence, transport direct-message dedup record (ADR-0019 — `read_pairs` above is the application's, keyed on the application id), or endpoint-directory cache is stored here.

## Retention transitions

- user sends -> durable `pending_outbound` is committed **before** the first transport call;
- direct `AcceptedV2` -> delete durable pending content;
- successful broadcast publication -> delete durable pending content, without claiming recipient delivery;
- inbound event consumed -> durable `unread_inbound` is committed before normal notification/UI presentation;
- inbound becomes locally read -> delete durable unread content unless the receiver explicitly chooses `Keep`;
- receiver chooses `Keep` after read -> durable `kept_inbound`;
- receiver removes `Keep` -> delete durable kept content.

Remote data can never set the Keep state.

## Message-status honesty

Application state may record:

- local send requested / pending;
- transport accepted remotely for direct;
- broadcast locally published;
- transport rejected/timeout;
- local inbound received;
- local UI read state;
- local receiver Keep state.

It must not label transport acceptance as “read by human” or “processed by app” without a future application-level receipt protocol. Local `read`/`Keep` are not sent remotely in v1.

## Where the store lives

The database and the single-instance lock beside it sit in the human client's own directory under the profile's state root — `ProfilePaths::human_dir()`, `<state_dir>/human` (A 2026-10-03, Stage 15 Q3) — never among the daemon's files (its sockets, profile lock and identity key), and the profile's `identity.key_file` may not point inside it — judged on disk, links followed (`configuration.md`; the lexical check landed with Stage 15's batch 2, the on-disk one A 2026-10-05). Deleting the directory loses only what RETENTION.md allows to be lost. The one client file outside it is the sealed store key, `<state_dir>/human-store.key` (A 2026-10-21, ADR-0060 decision 3): kept out of `human/` so that an export or a copy of the store's directory leaves the key behind, and kept out of the daemon's identity directory because the daemon's secrets are not the client's; deleting `human/` leaves it, harmless and useless alone, and recreating the store replaces it.

## Store health

Unread durability is a human-client property, not a transport guarantee. If SQLite is unavailable/full/corrupt such that new unread content cannot be committed, the human client exposes degraded storage health, releases/disables its direct EndpointId, and suspends its local human broadcast joins/delivery until storage is healthy rather than present itself as a normally durable receiver. Profile `channels.desired` may remain network-prewarmed without local delivery.

There remains a bounded handoff window after transport endpoint-queue acceptance and before human-store commit. `AcceptedV2` is not redefined.

## Migrations

Every schema migration is transactional and versioned. Migration failure puts the human app database into recovery/read-only/export mode; it never triggers transport identity regeneration. Stage 15's desktop client ships the recovery half (A 2026-10-03, Q4): a blocking screen with no session and no lease, showing the database path, offering Quit and Retry, and never renaming, moving or deleting the file; the read-only open and the export are carried by name (plan §18). One step is file-level, by exception (A 2026-10-21, ADR-0060 decision 6): the rewrite of a plaintext `human.sqlite` into an encrypted one — seal and sync the store key, export to `human.sqlite.encrypting`, rename it to `human.sqlite.encrypted` as the durable mark of a complete copy, scrub the plaintext in place while it holds its name, unlink its `-wal`, `-shm` and then the file, rename the encrypted file into place, each with a directory sync — with its crash states told apart by name on the next open: plaintext beside `.encrypting` restarts the rewrite from the intact plaintext, plaintext beside `.encrypted` keeps the copy and resumes the scrub, `.encrypted` alone finishes the rename, a `human.sqlite` that opens under the key is the store; the platform reporting the key invalidated offers discard-and-recreate directly; anything else — a present key that does not open the file, no key file, an unlisted combination of names — goes to recovery mode, in that order. From this date the recovery screen offers a third, confirmed action, **Discard and recreate**, beside Quit and Retry — the one relaxation of its never-delete rule, on the person's word only — because on the desktop a lost key file and a damaged never-migrated store leave the same files and the screen must not trap the person. The recovery screen's never-rename rule is about a store that failed to migrate, and does not apply to this rewrite.

A migration from any prototype/general-history schema must not preserve full history by default. It must explicitly classify rows into pending-outbound, unread-inbound, receiver-kept-inbound, or discard message content according to ADR-0044.

## Backup policy

Android system backup/device transfer excludes the entire human-store database, its sealed store key and the rest of the application's data, permanently (ADR-0060 decision 8, the owner's word of 2026-10-21): InterWeave data never leaves the device through a platform backup, a platform transfer or any external provider; the backup that will exist travels over InterWeave itself between the person's own devices, by a later record; until then a new device starts from the recovery phrase and an empty store.

A future explicit encrypted application backup may include **message content only from `unread_inbound` and `kept_inbound`**. `pending_outbound` is local crash/restart state but is excluded from portable backup to prevent restored/second devices from becoming implicit replay/delayed-send sources.

Contacts/configuration backup remains a separate policy and does not change message-retention eligibility.
