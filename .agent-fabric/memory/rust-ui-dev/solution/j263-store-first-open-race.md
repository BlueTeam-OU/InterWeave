---
role: "rust-ui-dev"
class: solution
topic: "j263-store-first-open-race"
description: "j263 store keying race (PRAGMA key or rekey, extended 1) — cause, measurement, fix state; uncommitted in worktree interweave-j263-store-keying-race, patch a scratch path"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c95dedd00b7316ea
---

## j263 store keying race (PRAGMA key or rekey, extended 1) — cause, measurement, fix state; uncommitted in worktree interweave-j263-store-keying-race, patch a scratch path

2026-11-12, j263 (devex-tooling OBSERVATION 01a1cf44). `HumanStore::open_profile` intermittently failed with SQLCipher's "An error occurred with PRAGMA key or rekey" (SqliteFailure code Unknown, extended 1) when two threads opened stores at once (tests/trust_boundary.rs).

**Cause:** SQLCipher's amalgamation. `sqlite3_initialize` sets `isInit = 1` and releases `pInitMutex`. Only after that does it run `SQLITE_EXTRA_INIT` (`sqlcipher_extra_init`). A second thread whose first SQLite call lands in that gap returns OK from init and keys its connection. `sqlcipherCodecAttach` then sees `sqlcipher_init == 0` and returns `SQLITE_ERROR`. This happens once per process, at the first SQLite use. Present in sqlcipher-src 419.0.6 and 420.0.0.

**Measured:**
- Raw rusqlite, 8 threads, fresh processes: 2/300 and 7/1500 failed. With one in-memory open first: 0/1500.
- Through the store's real `open_profile` with the engine start removed: 5/1500, about 11 minutes under load.

**Fix:** `crates/human/store/src/engine.rs`, a `Once` that makes one in-memory open before any other, with `open`, `open_with_flags` and `open_in_memory`. All 9 opens in `store/src` go through it. The unit test `no_connection_is_opened_past_the_engine` refuses any other open in `src/`. The multi-process test `tests/first_open_race.rs` is ignored by default. On 2026-11-12 the patch was applied on branch develop-qzapp/rust-ui-dev-01/feat/j263-store-trust-boundary-rs-open-profile (off 66fbe010b, sqlcipher-src 420.0.0), and the doc figures were corrected to the measured ones.

**Reach:** rusqlite is a production dependency of the store only. app-core, transport-client and human-cli have it as a dev-dependency, and human-android has none. So the engine `Once` orders every SQLite use in the desktop, CLI and Android processes. Reachability, read on 2026-11-12 and not run: no shipped client makes two first SQLite opens at once. human-desktop's startup.rs, human-cli's startup.rs and android-platform's service.rs each open one store per process. The Android host is a per-process `ServiceHost::global` OnceLock behind the profile lock. So today the race is reachable from tests (trust_boundary.rs, which opens from two threads). The fix guards any future concurrent first open.

**State 2026-11-12:**
- Committed as 52a73d376 (Kind: work); the fold of main is b6db42667.
- PR #641 merged 9229d5609 on 2026-11-12 (1 work, 3 fix), armed on the owner's word after three review rounds (round 3 clean at 920c0a944). Job j263 closed done.
- The fixed arm on sqlcipher-src 420: 0 of 1500 processes failed (777 s).
- Blind review dispatched on fd50cc9fd..b6db42667.
- Replied to devex-tooling: REPLY 01a1cfd8.
- Removed the old worktree, its branch and a scratch path.
- Review rounds (2026-11-12): round 1 on b6db42667 gave 3 P3s, F1-F3. Round 2 on c35825ea7: F2 partly fixed. Round 3 on 920c0a944: clean, all three fixed. All rounds are posted and saved.
- Arm dry-run passes everything except the count rule, so it needs the owner's word ("arm 641") or architect-cto's AUTHORIZES for #641@920c0a94.
- Risk left open, outside the store's lane: running the ignored first_open_race test in CI would be devex-tooling's work; the guard is a source scan.

*Anchors: `crates/human/store/src/store.rs::open_within`, `crates/human/store/tests/trust_boundary.rs::open_profile`*

*Observed 2026-11-12 (rust-ui-dev)*
