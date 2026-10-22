# human-cli

`interweave-human`, the command-line human client: ADR-0059 decision 9's "a cli way to do everything", plan section 29 step 4's command half, split out by architect-cto-01's request of 2026-10-22.

It is the desktop client's composition with no display: the same store, facade (`crates/human/app-core`) and model (`crates/human/ui-model`), over the daemon's data socket. A verb is a script of what a person does in the window (view a conversation, type and press Send, press Cancel on a row, confirm a trust change), and the model resolves it into the commands the facade carries out (`src/session.rs`, `src/verbs.rs`). A view never reaches past the facade, and this one is no exception. `identities` is the exception the model leaves: no model holds an endpoint's administration, so it calls the daemon's admin methods, as `transportctl` does, narrowed to the profile's human endpoints.

`interweave-human --help` prints every verb and every exit code.

## Batches

- **Batch 1 (on main, #368):** `status`, `conversations`, `messages`, `read [--keep]`, `send`, `cancel`, `retry`, `unkeep`, `listen`, `trust list|allow|revoke`, `identities list|enable|disable|default`, `store health`.
- **Batch 2:** `card show|set|share`, `proposals`, `proposals read [--accept|--decline]`, `contacts`, `contacts show|rename|photo|delete`.
- **Store discard and recreate** (ADR-0060) wait on the store's action landing (plan section 29 step 3).
- **Batch 3** waits on the daemon's `admin.peers` address methods (p2p-network-dev's step 5): peers add, remove and list by address, and this profile's own address.
- Creating, renaming and removing an identity (ADR-0061's display name and photo per endpoint), and moving a route between contacts, are Stage 22's.

## What a script can rely on

These were ruled with architect-cto-01 on 2026-10-22.

### Exit codes

| code | meaning |
|---|---|
| 0 | done |
| 1 | refused: by this tool, by the store or by the daemon |
| 2 | a usage error (the usage is printed on stderr) |
| 3 | no daemon answered for the profile |
| 4 | another client holds this profile's message store |
| 5 | the message store cannot be used: it needs recovery, is not private to this user, cannot be opened now, or (`store health`) cannot hold new messages now |
| 6 | a message is still on its way when `--wait` ends: it stays pending in the store, and its id is printed |

Why a verb failed is said on stderr, in English for a person. A script reads the exit code, never the wording.

### Output

- `--json` is accepted by every verb that reads (`status`, `conversations`, `messages`, `read`, `listen`, `trust list`, `identities list`, `store health`, `card show`, `proposals`, `proposals read`, `contacts`, `contacts show`) and by `send`, `cancel`, `retry`, `unkeep` and `card share`. It prints one JSON object per line. Every control character, line separator and bidirectional control in a string is escaped, so a line is always one line.
- `send`, `retry`, `cancel` and `unkeep` print `<id> <status>`, or `{"id": ..., "status": ...}` with `--json`. `send` and `retry` print it also when they fail with exit 1 or 6, so a script holds the id to retry or cancel.
- Every status and state is a fixed token, never a translated label:
  - a message's `status` is one of `sending`, `not-confirmed`, `needs-attention`, `accepted`, `published`, `cancelled`, `unread`, `read`, `kept`;
  - `session` is one of `ready`, `opening`, `reconnecting`, `no-daemon`, `refused`, `storage-degraded`, `closed`;
  - `connectivity` is one of `online-direct`, `online-relay`, `online-partial`, `offline`, `unknown`. `unknown` never means `offline`;
  - `listen`'s `event` is one of `message`, `proposal`, `outbound`, `session`, `connectivity`, `path`, `disconnected`.
- `accepted` means the remote endpoint's bounded queue admitted the message (`AcceptedV2`). It does not mean a person read it.
- Times are Unix milliseconds in JSON (`local_at_ms`, `sent_at_ms`, `last_activity_ms`). `sent_at_ms` is what the sender asserts: shown, never an order.
- Text output is for a person and may change. Text from a peer is printed with every control character but newline and tab replaced, and every line separator and bidirectional control, so a message cannot drive the terminal or reorder what it shows.

### A conversation and a message id

- A conversation is `<peer>` (its default endpoint), `<peer>/<endpoint>`, or `#<channel>`, as `conversations` prints it.
- A message id is `p<row>`, `u<row>` or `k<row>`: the message's row in the store's pending, unread or kept table. It names the same message to the next run while the row stays in that table. Reading or keeping a message moves it to another table and so gives it another id, which the verb prints. No id survives a discard and recreate of the store.

### What a run sees

- **One client at a time.** This client and the desktop client use one store and one endpoint lease (ADR-0041). Whichever starts second is refused, by name, with exit 4.
- **What a one-shot verb sees.** It sees what the store holds plus what arrives while it runs. Messages arrive only while some client holds the lease. `listen` holds it.
- **Reading verbs work without a daemon.** `conversations`, `messages` and `read` show what the store holds, and say on stderr why the session did not open. `status` exits 0 and reports the session as `no-daemon`.
- **Verbs that need the daemon** (`send`, `retry`, `trust`, `identities`) fail with exit 3 when none answers.
- **Keep happens at read time.** Keep is `read <conversation> --keep <id>`, in the same run that read the message. A message read and not kept lives only for the run that read it (RETENTION.md), so a Keep of a message read in an earlier run cannot exist. Every `--keep` id must name a message received in that conversation, checked before anything is marked read. Should anything fail after the read, what was read is still printed before the exit code says so.
- **Pending messages are sent by whichever run opens the session.** A `send` that exits 6 leaves its message pending. Any later run that opens the session (`status` included) attempts it again, until it is accepted or cancelled.
- **`--wait <ms>`** (default 5000) bounds how long a verb waits for the session and for an answer.
- **`listen`** prints until SIGINT or SIGTERM, then exits 0, its lease released. What it prints as a `message` stays unread in the store.
- **`send <conversation> -`** reads the message from stdin, without the one trailing newline a shell adds.
- **`identities default --none`** clears the profile's default endpoint only when it is a human one; another client's default is `transportctl`'s to clear. It reads the default, then clears it: two admin calls, so a `transportctl` change between them is not seen. `identities default <endpoint>` makes one of this client's endpoints the default, whichever endpoint held it before.
- **The identity's key.** Every run opens the key of the profile's first human endpoint, or makes it on the first run, by the desktop client's rule. A key file this build refuses is named on stderr and left as it is; the run goes on with no key, and `card show` and `card share` are refused.
- **A card proposal is decided in the run that shows it.** `proposals` lists every proposal held and reads none. `proposals read <conversation>` shows that conversation's proposals, which reads them (RETENTION.md: a card is unread content until shown), and accepts or declines the ones named with `--accept` and `--decline` in the same run. A proposal shown and not decided is gone when the run ends; the sender can share the card again. Every id is checked before anything is read, and what was shown is printed whatever fails after.
- **A proposal's id** is its card's unread row, `u<row>`, as `proposals` prints it.
- **What a proposal says is the sender's claim**, except the `PeerId`, the route and the key's fingerprint. `key.state` is `new`, `same` or `changed`; a `changed` key is shown with the fingerprint the route recorded, and accepting it replaces the key on that route alone.
- **`card set <name> [<surname>]`** sets the whole own card. `--photo <file>` reads at most 32 MiB, which the client re-encodes to the card's bound; `--no-photo` removes it; with neither, the photo stays as it is.
- **A contact's name or photo set with `contacts rename` or `contacts photo`** is the person's: no later card replaces it.
- **`contacts delete`** deletes the contact, its routes and their recorded keys, out of the store's files before it returns. The next card from one of its routes is a new proposal, its key `new`. The trust allowlist is separate and is not changed.
- **`listen`** prints a card that arrives as a `proposal` event, and leaves it unread.
- **`trust allow` and `trust revoke`** of a peer already in that state exit 0, printing `(already)`.

## Tests

- `src/verbs_tests.rs` and `src/cards_tests.rs`: each verb through the model and the facade, against the in-memory node pair app-core's own tests use, over a file store where the next run is the claim.
- `tests/process.rs`: the binary's `--help`, a usage error and a refused profile.
- `tests/desktop-e2e/tests/human_cli.rs`: every verb of batches 1 and 2 run as the binary against two real daemons.

## Not yet

- The release lists (`tools/ci/release_linux_archive.sh`, `packaging/linux`) do not carry the binary yet. Adding it is devex-tooling's.
