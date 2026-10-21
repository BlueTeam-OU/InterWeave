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
WORK=<empty dir> ../run.sh          # the timing, deletion and failure rows
WORK=<empty dir> ../rewrite.sh      # the rewrite and key-file rows
```

A forced crash marks the process non-dumpable before it aborts: a core
would hold the key, and the host's collector keeps it out of this
account's reach.

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
  a criterion is judged at, on the desktop. The device rows were recorded
  later the same day, when the reference device was back (below).
- **The read walk was quadratic in the store, whatever the engine**, at
  the pin (schema v7): each page was `SCAN` plus `USE TEMP B-TREE FOR
  ORDER BY` (no index on `(received_at, row_id)`), so the plaintext walk
  went from 1.8 ms at 500 to 1.9 s at 20 000, and SQLCipher, paying a
  decryption per page per scan, multiplied it. Reported; architect-cto
  ruled the index onto this spike's PR (REPLY 01a15f1a): schema v8,
  below.
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

**Recorded: the device rows, `REPRODUCTION-device-2026-10-21.log`**
(`device.sh`, binaries of bb1f7113 -- its BUILD says one dirty path, the
rustfmt of `migrate.rs` that 6f6f58c4 records -- on the Stage 17
reference device: Pixel 9a, Android 17, API 37, Tensor G4, f2fs, on
battery). The harness ran from shell's `/data/local/tmp`, not the app's
sandbox, on the same userdata partition; each store was opened as the
Android client opens it, `open_profile` under a boundary the run's
directory stands for. Medians of the rounds' medians, SQLCipher
(vendored OpenSSL) as a ratio of the plaintext on the same side:

| kept | operation | v7 plaintext | v7 SQLCipher | v8 plaintext | v8 SQLCipher |
|---|---|---|---|---|---|
| 500 | open-cold | 53.3 ms | 41.7 ms (x0.78) | 53.8 ms | 45.1 ms (x0.84) |
| 500 | commit-unread | 1.1 ms | 1.7 ms (x1.49) | 1.2 ms | 1.8 ms (x1.57) |
| 500 | read-walk | 3.4 ms | 3.6 ms (x1.05) | 3.4 ms | 3.2 ms (x0.97) |
| 2000 | open-cold | 51.4 ms | 42.7 ms (x0.83) | 51.2 ms | 44.6 ms (x0.87) |
| 2000 | commit-unread | 1.2 ms | 1.5 ms (x1.28) | 1.2 ms | 1.9 ms (x1.56) |
| 2000 | read-walk | 9.2 ms | 10.2 ms (x1.10) | 4.9 ms | 7.7 ms (x1.58) |
| 20 000 (stress) | read-walk | 1.66 s | 10.84 s (x6.5) | 99 ms | 127 ms (x1.28) |

- **Within ADR-0060 decision 4 on the device**: every operation within
  twice the baseline at 500 and 2000 kept, on both schema versions (the
  largest, x1.58), and a cold open of 42-54 ms against the one-second
  bound.
- **The cold open is not shown to be cold on the device.** The harness
  evicts with `posix_fadvise(DONTNEED)` after a sync, which Linux
  honours; that f2fs on this kernel dropped the pages was not measured.
  The figures are ten times the desktop's, which reads like a read from
  the medium, and the bound has a factor of twenty to spare.
- **The deletion property holds on the device's encrypted file**: after
  a clean close, and after the next open following an abort, the
  decrypted pages hold the kept marker and not the released one; the
  control finds both; a raw search of the files finds neither.
- **The failures are the desktop's**: a wrong key, the plaintext build on
  the encrypted file, and the cipher build on a plaintext file, each
  `NotADatabase`.
- **Not run on the device**: the rewrite and key-file rows (Android's
  key is sealed by the Keystore, ADR-0060 decision 3, which the harness
  does not prototype), and the system-libcrypto build (Android has no
  libcrypto an app may load).

**Recorded: the index rows, `REPRODUCTION-index-2026-10-21.log`**
(`index.sh 1a4fa590 c4a5af95`: the harness built at the pin, schema v7,
and at schema v8, which indexes `(received_at, row_id)` on both inbound
tables; the two sides interleaved by round on a shared host, load about
37 at the start). Medians of the rounds' medians:

| kept | operation | v7 plaintext | v7 SQLCipher | v8 plaintext | v8 SQLCipher |
|---|---|---|---|---|---|
| 500 | read-walk | 1.9 ms | 2.2 ms (x1.16) | 1.1 ms | 1.2 ms (x1.15) |
| 2000 | read-walk | 13.6 ms | 13.2 ms (x0.97) | 5.3 ms | 5.3 ms (x1.01) |
| 20 000 (stress) | read-walk | 2.50 s | 23.8 s (x9.5) | 223 ms | 146 ms (x0.65) |
| 500 | open-cold | 5.3 ms | 6.0 ms | 5.1 ms | 6.0 ms |
| 2000 | open-cold | 6.9 ms | 8.1 ms | 6.8 ms | 8.3 ms |
| 500 | commit-unread | 2.6 ms | 2.8 ms | 2.2 ms | 2.5 ms |
| 2000 | commit-unread | 2.6 ms | 2.8 ms | 3.0 ms | 3.0 ms |

- The walk is now linear: v8 reads its index (`SEARCH ... USING INDEX`,
  no temporary B-tree, pinned by the store's
  `the_inbound_walk_reads_its_index_not_a_sort`), and SQLCipher's
  multiplier on the stress row is gone with the repeated scans.
- The second index costs a commit nothing measurable here at 500 and
  2000 kept; the stress row's single round on a loaded host is not a
  measurement of it.

**Recorded: the rewrite and key-file rows, `REPRODUCTION-rewrite-2026-10-21.log`**
(`rewrite.sh`, 500 kept; the prototype is `harness/src/migrate.rs`, ADR-0060
decisions 3 and 6 with A 2026-10-21):

- **The sequence.** (a) the profile-format marker raised to 2 (ADR-0028's
  `profile-format.json`, through the shipping `raise_within`), the key
  file written and synced, the plaintext exported under the key to
  `human.sqlite.encrypting`, synced, renamed `.encrypted`, the directory
  synced; (b) the plaintext's rows deleted under `secure_delete`, vacuumed,
  its log truncated, then `-wal`, `-shm` and the file unlinked; (c)
  `.encrypted` renamed `human.sqlite`. A start tells the states apart by
  the names: an `.encrypting` is an export that did not finish (removed,
  the plaintext still the store); an `.encrypted` is a complete copy (the
  store; any plaintext beside it is (b)'s to finish).
- **A crash after each of the seven steps**, then the next start: every
  one ends with all 520 rows, `human.sqlite` alone in `human/`, and no
  file there holding content in clear; the start after it is a plain
  open. At 500 kept the clean rewrite took 238 ms, the full rewrites
  after a crash before the export 224-227 ms, the restarted export 204 ms,
  and the resumed ones 41-50 ms. Control: the seeded plaintext holds the content in clear.
- **The rollback** refuses the profile from the first step on: the
  shipping loader's own `check_within` says "profile format 2, newer than
  this build's 1". It is never the encrypted file's `NotADatabase` a
  rollback meets.
- **The key file** (`IWSK`, a version byte, 32 bytes): missing beside an
  encrypted store, of version 2, and truncated, each refused by name
  (`KeyMissing`, `KeyVersion(2)`, `KeyMalformed`), the store's file
  byte-identical after.
- **Two things the production rewrite must do that the engine does not**:
  `sqlcipher_export` does not copy the header's `user_version`, the
  store's schema version (carried by hand, or the copy reads as version 0
  and migrates from nothing); and `ATTACH` creates a missing database with
  the process umask (0644 measured, refused by the store as
  `PermissionsTooOpen`), so the copy is created empty and 0600 first, as
  the store creates its own file.
- **Limit.** The unlinked plaintext's blocks are not searched: below the
  database, RETENTION.md does not overstate (filesystem and flash), and
  the scrub before the unlink is what the record asks.

Owed: Android's Keystore sealing of the key (decision 3), plan section 29
step 3's. The spike's Result is in `architecture/roadmap/SPIKES.md`.
