# SPIKE-011 — the encrypted human store

The entry, its bounds and its verdict are `architecture/roadmap/SPIKES.md`'s
(SPIKE-011) and ADR-0060's (decision 4); this directory is the harness and
its runs. Regime: **release gate** — the harness's pin moves only in the
change that re-runs the rows and re-records the verdict.

## What it measures, and how

`harness/` is one program, the shipping store (`interweave-human-store`,
taken by revision) opened by its own code, built once per engine:

| build | engine | crypto |
|---|---|---|
| default | SQLite, plaintext — the baseline, the store as it ships | none |
| `--features cipher` | SQLCipher, from `rusqlite`'s `bundled-sqlcipher` | the host's `libcrypto`, linked dynamically |
| `--features cipher-vendored` | the same, `bundled-sqlcipher-vendored-openssl` | OpenSSL built from source, linked statically |

**The key reaches the store without a change to it.** SQLCipher reads a key
given before a connection's first statement, and the store opens its own
connection and runs its pragmas at once. An SQLite auto-extension runs
inside every open, before it returns: the harness registers one that issues
`PRAGMA key = "x'<64 hex>'"` (a raw 32-byte key, so SQLCipher derives
nothing: ADR-0060 decision 2's key is random, not a passphrase). The store's
code is measured as it ships; the production seam is plan §29 step 3's.

**The operations** (none had a benchmark or a budget before this spike):

- `open-cold` — `HumanStore::open` in a fresh process, the files' pages
  evicted from the page cache first (`posix_fadvise(DONTNEED)` after a
  sync), so it reads from the medium.
- `commit-unread` — one `commit_unread_inbound` of a 512-byte payload,
  `synchronous=FULL` as the store sets it.
- `read-walk` — what the client reads to show its conversations
  (`crates/human/app-core/src/listing.rs`, `list_all`): every pending,
  unread and kept row, in its pages of 64 records or 1 MiB. **The store has
  no per-conversation read**; the spike entry's "read-conversation" is this
  walk.

The store is seeded with N **kept** messages and an unread backlog of 20:
a read message not kept is released, so what the person kept is the
store's bulk, and unread and unsent rows are marginal. N = 500 is the
realistic store, 2000 a large one (the owner, 2026-10-21); one shorter
round at 20 000 is a labelled stress row, not where a criterion is judged.

**The deletion property** (RETENTION.md §8): a released and a kept marker,
the release, then a clean close or an abort and the next open, as
`crates/human/store/tests/released_content.rs` does. In a cipher build a
raw byte search finds neither marker — ciphertext — so it proves nothing on
its own. The search that counts runs over every page **as the engine
decrypts it**: `sqlite_dbpage`, compiled into a separate INSPECTOR build
(`LIBSQLITE3_FLAGS=SQLITE_ENABLE_DBPAGE_VTAB`) that only reads a copy. The
files under test are written by the candidate build itself. Its control:
the same search on a store whose marker was not released finds it.

## Running

```sh
export PATH="$HOME/.cargo/bin:$PATH"     # the pinned toolchain, not the system cargo
cd harness
cargo build --release --locked                            # baseline
cargo build --release --locked --features cipher-vendored
LIBSQLITE3_FLAGS=SQLITE_ENABLE_DBPAGE_VTAB CARGO_TARGET_DIR=target-inspect \
  cargo build --release --locked --features cipher-vendored   # the inspector
export SPIKE011_KEY=$(head -c32 /dev/urandom | od -An -tx1 | tr -d ' \n')
target/release/spike-011 versions
target/release/spike-011 bench <empty 0700 dir> <kept> <reps>
target/release/spike-011 deletion <empty 0700 dir> clean|unclean|none
```

Android (`aarch64-linux-android`, API 30 as the app):
`cargo ndk -t arm64-v8a -P 30 build --release --locked --features cipher-vendored`.

## Status

**Recorded: the desktop rows, `REPRODUCTION-2026-10-21.log`**, in one pass
of `run.sh` (Intel Core Ultra 7 155H, ext4, the pinned 1.98.1). Medians of
the three rounds' medians, the cipher builds as a ratio of the plaintext:

| kept | operation | plaintext | SQLCipher, host libcrypto | SQLCipher, vendored OpenSSL |
|---|---|---|---|---|
| 500 | open-cold | 5.1 ms | x1.09 | x1.13 |
| 500 | commit-unread | 2.6 ms | x0.98 | x0.95 |
| 500 | read-walk | 1.8 ms | x1.07 | x1.09 |
| 2000 | open-cold | 5.0 ms | x1.14 | x1.14 |
| 2000 | commit-unread | 2.5 ms | x1.09 | x1.00 |
| 2000 | read-walk | 12.1 ms | x0.93 | x1.02 |
| 20 000 (stress) | read-walk | 1.9 s | x9.5 | x7.7 |

- Within ADR-0060 decision 4's criterion (twice the baseline) at both sizes
  a criterion is judged at, on the desktop. The device rows are owed: the
  Stage 17 reference device is away with the owner.
- **The read walk is quadratic in the store, whatever the engine**: each
  page is `SCAN` plus `USE TEMP B-TREE FOR ORDER BY` (no index on
  `(received_at, row_id)`), so the plaintext walk goes from 1.8 ms at 500 to
  1.9 s at 20 000, and SQLCipher, paying a decryption per page per scan,
  multiplies it. Not this spike's mechanism to fix; reported.
- **The deletion property holds on the encrypted file**, clean close and
  next open after an abort: the decrypted pages hold the kept marker and
  not the released one; the control (no release) finds both. The raw files
  hold neither marker in a cipher build, nor SQLite's plaintext header.
- **Engines.** SQLCipher 4.14.0 is built on SQLite **3.51.3**; the
  plaintext build ships 3.53.2. The host-libcrypto build links the host's
  `libcrypto.so.3` at run time (OpenSSL 3.5.9 here); the vendored build
  links OpenSSL 3.6.3 statically (+5.1 MB to the harness binary) and is the
  only one that builds for `aarch64-linux-android`: the NDK carries no
  OpenSSL (`openssl/crypto.h` not found). The Android binary needs only
  `libc`, `libm`, `libdl` and `liblog`.
- **Added crates** (vendored build only): `openssl-src` 300.6.1+3.6.3
  (MIT/Apache-2.0) and `openssl-sys` 0.9.117 (MIT), both on the allow-list;
  neither is in the root graph today. SQLCipher's own C code is inside
  `libsqlite3-sys` (licence field MIT; the bundled SQLCipher is Zetetic's
  BSD-3-Clause, allowed but not seen by `cargo-deny`). Registry crates, so
  `check_vendored_advisories.sh`, which judges local trees, sees none of
  them; their RustSec coverage is the root graph's `cargo deny` once the
  store depends on them.
- **Failures.** A wrong key, a plaintext (rollback) build on the encrypted
  file, and a cipher build on a plaintext file all end in the store's open
  as `NotADatabase` ("file is not a database"), with SQLCipher logging an
  HMAC failure on page 1 to stderr (page numbers, no key material). A
  rollback build therefore cannot tell the encrypted store from a corrupt
  one by the error.

Owed: the device rows; the key-file format, a missing key file, a version
bump; the plaintext-to-encrypted rewrite under a crash at each step; the
spike's Result in `architecture/roadmap/SPIKES.md`.
