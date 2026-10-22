# human-cli

`interweave-human`, the command-line human client: ADR-0059 decision 9's "a cli way to do everything", plan section 29 step 4's command half, split out by architect-cto-01's request of 2026-10-22.

It is the desktop client's composition with no display: the same store, facade (`crates/human/app-core`) and model (`crates/human/ui-model`), over the daemon's data socket. A view never reaches past the facade, and this one is no exception.

## Status

Being built on `develop-qzapp/rust-ui-dev-02/feat/j41-human-cli`. Two pieces exist so far:

- the grammar (`src/cli.rs`) with its tests;
- the start-up, `src/startup.rs`: the profile's endpoints by the desktop client's rule, the instance lock, and the store open with its refusal classes.

Every verb still answers "not built yet". `--help` prints the grammar and the exit codes the verbs will keep.

## What a script will be able to rely on

These were ruled with architect-cto-01 on 2026-10-22, and the batch-1 verbs are built to them.

- **One client at a time.** This client and the desktop client use one store and one endpoint lease (ADR-0041). Whichever starts second is refused, by name, with exit 4.
- **What a one-shot verb sees.** It sees what the store holds plus what arrives while it runs. Messages arrive only while some client holds the lease.
- **Keep happens at read time.** Keep is `read <conversation> --keep <id>`, in the same process that read the message. RETENTION.md lets a read, unkept message live only for the session that read it, so a Keep of a message read in an earlier run cannot exist.
- **`listen`.** It holds the session open and prints one line per event.
- **Message ids.** An id is `p<row>`, `u<row>` or `k<row>`: the message's row in the store's pending, unread or kept table. It names the same message to the next run while the row stays in that table. A read or a keep moves a message to another table, and so gives it another id. No id survives a discard and recreate of the store.
