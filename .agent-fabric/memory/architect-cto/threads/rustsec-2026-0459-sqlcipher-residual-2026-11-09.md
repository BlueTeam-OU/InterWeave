---
role: "architect-cto"
class: threads
topic: "rustsec-2026-0459-sqlcipher-residual-2026-11-09"
description: "RUSTSEC-2026-0459 (CVE-2026-119375, SQLite < 3.54 uninitialized page cache from a -wal of another page size): devex-tooling's j306 bump to rusqlite 0.41 / libsqlite3-sys 0.39.1 turns cargo-deny green, but the human store's SQLCipher build…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c60153ae4917bc5d
---

## RUSTSEC-2026-0459 (CVE-2026-119375, SQLite < 3.54 uninitialized page cache from a -wal of another page size): devex-tooling's j306 bump to rusqlite 0.41 / libsqlite3-sys 0.39.1 turns cargo-deny green, but the human store's SQLCipher build (sqlcipher-src 419.0.6, SQLite 3.53.4) stays affected by the advisory's own words; ruled 2026-11-09: no vendoring, no hold, record the residual by name with its re-check trigger (sqlcipher-src >= 420) and the dependency-check gap it shows

Found by rust-ui-dev-02 (OBSERVATION 01a1c020, 2026-11-09), verified by me against the advisory text (rustsec.org RUSTSEC-2026-0459, issued 2026-11-09, CVSS 6.8 local): "the bundled-sqlcipher feature ... in 0.39.0 and 0.39.1 depends on sqlcipher-src =419.0.6, which bundles SQLite 3.53.4 and is affected"; exposure "only a process that opens a database file, with its -wal, that a less trusted party could have written". devex-tooling's branch `develop-qzapp/devex-tooling/fix/j306-rusqlite-0-41-rustsec-2026-0459` at 58f4df7a3 (Cargo.lock libsqlite3-sys 0.39.1, sqlcipher-src 419.0.6+sqlcipher-4.19.0-sqlite-3.53.4; deny.toml admits blessing and WTFPL on the owner's word) is green on cargo-deny because the check matches crate versions.

**Ruled 2026-11-09:** no vendored SQLCipher (ADR-0051's shape is for a defect only a fork can fix; a release is the remedy here), no hold on the bump. Impact within the record: ADR-0060 "What it does not protect against" puts a same-uid process (Android app uid, desktop same user) outside the store's threat model (ADR-0037, plan §28, SPIKE-005 not run), and a party writing a `-wal` beside `human.sqlite` in the app-private directory is that party. Sent: REPLY to rust-ui-dev-02 and INFO to devex-tooling (the j306 PR body names the residual and the re-check trigger).

**Delivered (j565, 2026-11-09): BlueTeam-OU/InterWeave#601 (1 work, docs only, below minimum: the owner's word), commit 99947e4df, plan §21 Record bullet + §20 'live gaps' sentence + CLAUDE.md §8 third gap; devex-tooling's branch at 58f4df7a3 had no PR yet, so the record cites the branch and sha. Round 1 on 99947e4df: F1 P2→P3 (CLAUDE.md present tense ahead of the bump), F2 P3 (trigger keyed on the crate major), F3 P3 (carry named no section/role); all judged real, fixed in 9f3e7822a (§22 carry, devex-tooling closes, architect-cto holds); round 2 found F4 (plan §20 sentence still dated the gap), fixed 17a51ab50; round 3 clean on 17a51ab50. #601 (1w,2f) ARMED 17a51ab50 on the owner's word 'arm 601' 2026-11-09 12:3xZ, BLOCKED: tree checks + tool self-tests red on main since ae7c0dc9a (10:04Z) on RUSTSEC-2026-0459 itself (libsqlite3-sys 0.38.2); unblocks when devex-tooling's j306 (58f4df7a3, no PR yet at 12:40Z) lands — then fold main into #601, re-review the fold, re-arm (a push disarms). INFO sent to devex-tooling.** What it owed: the residual carried by name in the plan (Stage 18 §21's carried list or the dependency section it fits), the re-check trigger (a libsqlite3-sys release depending on sqlcipher-src >= 420, SQLite >= 3.54), and CLAUDE.md §8's dependency-check paragraph gaining a third gap beside RustSec/GHSA and the vendored-crate one: a RustSec patched range that covers a feature build still affected. Cites devex-tooling's PR once it has a number (add the number on #601 if it opens before #601 merges; otherwise leave the branch citation, which stays true). Related: [[rustsec-zbus-family-ignores-and-arming-below-minimum]].

**2026-11-09 12:4xZ:** devex-tooling's j306 is #602 (2w, boundary, unarmed; head 373ea5547, with rust-ui-dev-02's column_names supply). p2p-network-dev-02's question (OBSERVATION 01a1c039, carried in #602's body): do the store's opens of files it did not write (plaintext rewrite's read, a restore) fall under the ruling? Ruled yes (DECISION sent; comment on #602): every open in crates/human/store is on `human.sqlite`/`<suffix>` in the private human dir (at_rest.rs 154-159, 557, 647); no restore/import path exists in v1; a copied-in state root carries key and store already. **Owed at #601's fold of main after #602 lands:** cite #602 by number, add the open-paths sentence to the §21 item citing 01a1c039, re-review the fold, re-arm.

**2026-11-09 13:5xZ:** #602 merged 295cf217a (13:43Z). Folded main into #601 (f59fec4be) + record update 93a503c7f (#602 cited, gap live, open-paths ruling in §21) — the push disarmed #601 as designed. Round 4 on 93a503c7f: F1-F4 fixed, F5 P3 (open-paths sentence said "every open in the crate" while `HumanStore::open(path)` is public, test-only callers) — judged real, fixed 89d92fc14 (scoped to what a client reaches; a client calling the plaintext opener voids the reasoning). Round 5 re-review dispatched. #601 now 2w 2f 1 merge; needs the owner's word again for the new head.

**Round 5 on 89d92fc14 clean (posted 13:52Z).** #601 (2w,3f,1 merge) awaits the owner's word for head 89d92fc14. Pre-existing P3 the reviewer found (store.rs:202-206 doc comment says the rewrite reads through `HumanStore::open`; it reads through `open_within`) sent as OBSERVATION to rust-ui-dev-01.

**ARMED 89d92fc14 on the owner's word 'arm 601' (2026-11-09 ~16:1xZ), queued at 1.** When it merges: j565 done, `fabric-jobs switch` → j566 (ADR-0058 supply commit+push+REPLY to devex-tooling).

**CLOSED: #601 (2w,3f) merged 1fb9ee07e 2026-11-09 on the owner's word at 89d92fc14.** The residual is on main in plan §21 and CLAUDE.md §8; the re-check trigger (sqlcipher-src lock metadata sqlite-3.54.0+) is devex-tooling's to read closed, architect-cto moves the record; §22 takes the carry if Stage 18 closes first.

**2026-11-12, trigger fired and closed in the tree:** RUSTSEC-2026-0502 (sqlcipher-src, patched >= 420.0.0) turned check_dependencies red; devex-tooling's #637 (fd4679a7f, rusqlite 0.42.0 / libsqlite3-sys 0.40.0 / sqlcipher-src 420.0.0+sqlcipher-4.20.0-sqlite-3.54.1) answers it. My record move is j640 = BlueTeam-OU/InterWeave#638 (plan §21 dated closing note, §20 sentence; branch develop-qzapp/architect-cto-01/docs/j640-plan-s21-rustsec-0459-residual-closed): round 1 one P3 (tag date; v0.3.0 was tagged 2026-11-11 on c0bbe40a9, a commit of 2026-11-10), fixed 0fa8cdfcf, plus work commit c3d3b16ce (§20 names the vendored-crate gap). Below minimum, my own PR: arms on the owner's word after the re-review of f4e55db3..c3d3b16c is posted clean. The note states v0.3.0 artifacts carry sqlcipher-src 419.0.6 within the item's stated limit and asks no re-cut. This thread closes when #638 is on main.

**CLOSED 2026-11-12.** The record move, #638 (2 work, 1 fix), merged fd50cc9fd at 12:46Z on the owner's word at c3d3b16c; j640 and j647 done. Nothing is owed on this subject.

*References: rustsec-zbus-family-ignores-and-arming-below-minimum*

*Observed 2026-11-12 (architect-cto)*
