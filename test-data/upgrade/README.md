<!-- SPDX-License-Identifier: Apache-2.0 -->
# upgrade

The previous build, as the files it leaves behind (plan §22, Stage 19's
upgrade and rollback rows; architect-cto's ruling on j30's Q1,
2026-10-20). Each `<commit>/` directory is the XDG tree the build of that
commit wrote in ordinary use of the shipped desktop profile
(`human-desktop`), and every later build is tested against it by
`tests/desktop-e2e/tests/human_app/release.rs`:

- **upgrade**: this build reads the tree — same PeerId, the trust and
  endpoint overlays in effect, the store's three retention sets kept and
  migrated — and rewrites none of the key, the profile or the overlays;
- **rollback**: this build meets a NEWER format of each artefact (the
  store's `user_version` one past its own, an overlay with a field it
  does not know, `schema_version: 3`), refuses it, writes nothing and
  keeps the PeerId — what an older build owes a tree a newer one wrote.

The real older binaries are the previous-build axis
(`tools/ci/previous-builds.txt`): `release.rs` runs each entry's daemon
and `transportctl` old -> new -> old over this tree and records what the
old build does on it. No older desktop client is on the axis, so the
client's half of that is a `limit`.

## A tree

| path (under the XDG root named first) | written by |
|---|---|
| `data/…/identity.key` | `transportctl identity restore --new` from the golden phrase |
| `config/…/config.yaml` | the shipped `human-desktop.yaml`, two peers allowed, listening on `127.0.0.1:0` |
| `state/…/trust-overlay.json` | the daemon, answering `transportctl trust allow` and `trust revoke` |
| `state/…/endpoint-overlay.json` | the daemon, answering `transportctl endpoints disable claude` |
| `state/…/human/human.sqlite` | the store crate the window writes with: one pending outbound, one unread, one kept; checkpointed, no WAL |
| `cache/…/peers.json` | the daemon |
| `tree.json` | what the tree holds: the commit, the store schema, the PeerId, the peers, the endpoint and the three rows' ids |

Git keeps no file modes, so a case puts back the ones the build gave:
directories `0700`, `config.yaml` `0644`, every other file `0600`.
`.gitattributes` keeps git from rewriting any byte of a tree.

**TEST-ONLY KEY MATERIAL.** Every tree's `identity.key` is the all-zero
entropy of `fixtures/identity/ed25519-bip39-entropy-v1.json` — the
public BIP-39 vector, PeerId `12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN`
— and has no secrecy whatsoever. The other PeerIds in a tree are random
public identities whose keys were never kept. A real key never enters
this directory (`CLAUDE.md` §6).

## Making one

From a clean checkout of the commit, with the commit's build:

```sh
test-data/upgrade/make-tree.sh
```

It builds the three desktop executables, runs the one ignored case that
writes the tree (`release::write_the_previous_build_tree`), and leaves it
in `test-data/upgrade/<the commit's first eight characters>/`. A tree is
never edited by hand and never regenerated in place: a later build's
tree is a new directory, and `release.rs`'s `PREVIOUS_TREE` names the
one the cases read.

| tree | made by | store schema | note |
|---|---|---|---|
| `d3a193e9/` | `d3a193e9`, on main `074ba19d` | 7 | the first: today's formats, so it proves reading, not migrating from an older format, which `crates/human/store`'s cases prove in isolation |
