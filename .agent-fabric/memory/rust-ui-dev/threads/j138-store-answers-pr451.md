---
role: "rust-ui-dev"
class: threads
topic: "j138-store-answers-pr451"
description: "InterWeave j138 / PR #451 — Android store screen keyed on the hub's counted start answers; state, evidence, what is left (device rows)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 381d1a36789a5354
---

## InterWeave j138 / PR #451 — Android store screen keyed on the hub's counted start answers; state, evidence, what is left (device rows)

j138 (#377 review r3 risks 1-2 + pre-existing non-store refusal) is PR #451, branch
develop-qzapp/rust-ui-dev-01/feat/j138-377-follow-up-android-store-screen, worktree
~/projects/interweave-j138-377-follow-up-android-store-screen, base main 3cda6984.

- d33b0fb8 android-platform: `ToView::Store(StoreAnswer, StoreGeneration)` replaces
  `StoreRefused`; every ServiceHost start says one answer under the start lock
  (`answer()` wrapper around `start_locked`/`start_offline_locked`), NetworkDenied says none;
  `discard_store` -> `StoreDiscarded { discarded, answered }`; `refused_before_start()`.
- 8a691345 human-android: store_follow decision table on (answer, generation, done_at);
  jni.rs `refused_early` for provision/identity/Keystore refusals.
- Evidence: host tests + 8 mutations each failing a test; Android target clippy -D warnings clean;
  `cargo xtask ci` all green except test_release_sign_archive (no minisign on this host, env).
- Left: device rows (Keystore store key, R/K, discard) on the Pixel, blocked on the owner's
  Wireless debugging address (same block as j158). 2 work commits < 8: arming needs the
  owner's word or more commits (j161 is a candidate commit for this PR).
- Review fixes: 5ad677df/feb2eaa0/60f23b37 (F1-F3, round 2 judged all fixed), then cfe15eb0:
  the F1 arm swap made two adjacent same-body arms and CI's clippy (match_same_arms) went red.
  Run host clippy -D warnings after EVERY fix commit before pushing, not only the tests.
- Matrix (j163): a3c92915 moved TM-52..56; r4 F1/F2 fixed in 18a5757c/61406de6/5f534fca. Open work in a
  matrix row goes in the TESTS cell as `owed:` (one marker per `; ` item), the limit says what is
  unproved. 61406de6 broke check_security_matrix.py and I pushed it: I read `| tail -1`, which
  showed an unrelated line. Read the check's exit code, never its last line.
- Session PATH lacked ~/.cargo/bin again: export PATH=$HOME/.cargo/bin:$PATH before cargo.

Related: [[j136-encrypted-store]] [[android-target-clippy]] [[pixel-checks-owed]]

2026-10-27, device rows on the A40 at main bdc7ac89:
- Keystore store key: a cold start opened the store (start answered 1).
- A lock change (None, then a PIN) does NOT invalidate anything. The store key, the idk1 identity keys and the iwk1 seed key (profile policy background-compatible, config.yaml key_unlock_policy) are all background-compatible. Only a user-presence seed key is lock-bound.
- R (recovery) reached through test hook 9db1ee1e on the local branch test/j155-drop-store-key-hook (E2eCase case drop-store-key retires interweave.store.human-android). The start answered -5 StoreNeedsRecovery.
- The owner saw the R screen ("cannot read its stored data", no way past), the discard dialog's list, then the yellow deleted notice in Chats.
- Files after: human-store.key re-created 09:07 and human.sqlite new. identity.iwk1, identity-keys/human.key, trust-overlay and config.yaml kept. A restart at 09:10 answered 1.
- K (key lost) stays host-tested: a background-compatible key cannot be invalidated by the platform on purpose.

*References: android-target-clippy, j136-encrypted-store, pixel-checks-owed*

*Observed 2026-10-25 (rust-ui-dev)*
