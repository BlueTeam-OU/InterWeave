---
role: "p2p-network-dev"
class: threads
topic: "pr228-judge-before-create-part-2"
description: "#228 MERGED 0e17a32e 2026-10-08 (j35-j40): judge-before-create, FIFO-safe opens, ADR-0028 #231 private-group predicate, umask-independent suites, 120 s keepalive bound; what is carried (2 of 2)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 6ec26bca81a8c629
---

## #228 MERGED 0e17a32e 2026-10-08 (j35-j40): judge-before-create, FIFO-safe opens, ADR-0028 #231 private-group predicate, umask-independent suites, 120 s keepalive bound; what is carried (2 of 2)

Part 2 of [[pr228-judge-before-create]].

- **MERGED 0e17a32e, 2026-10-08 19:14 UTC**, armed after #231 merged (owner's direct word "Arm #231, then #228"; relays of "merge 228" from rust-ui-dev and 02 did not set an order). Told rust-ui-dev (creates-nothing assertion can land), 02, architect-cto.
- Carried: pin test CLIENT_SILENCE_TIMEOUT_MS vs ipc-protocol's constant (job, blocked on #230); identity key FIFO window (stated limit); NSS hang with no timeout (risk); NFSv4 ACLs (ADR stated limit).
- Lessons: an Agent prompt is literal (a shell substitution went verbatim; see [[agent-prompt-does-no-shell-expansion]]); a python heredoc carrying prose must be quoted (<<'PY') or the shell runs what it quotes; a watcher must exit on failure too; CodeQL uid/user-name alerts recur on every push touching refusal tests — dismiss per devex policy.
- j41 → PR #236 (1 work, unarmed, review clean at 5e55e061; branch develop-qzapp/p2p-network-dev-01/test/client-silence-pin). Under the floor: next profile-config work joins it. Optional carried: compare Durations instead of as_millis (truncation).
- #236 batch (2026-10-09, owner said "on 236"): j42 190c1c4d (Duration pin), j43 09021beb (another_uid non-root precondition on the ancestor/link-rule tests only), F1 doc fix 7741538e, j44 6cb59136 (NSS reads under NSS_READ_DEADLINE 5 s on a helper thread; refuses "did not answer within 5s"; NameService: Clone+Send+'static). Wording question to architect (seq 29330): kept "owner's" vs their "daemon user's".
- #236 rounds 4-6: F1 (spawn/disconnect/timeout named apart) + F3 (names_clone gone) + control 2 s → c9107076; N1 doc + panic bound <4 s → c6e39653; clean at c6e39653. #236 = 4 work, 3 fix: arms after architect's ADR "The name-service read is bounded" PR merges (not opened yet at 2026-10-09 02:30) — and needs the owner's word or more work (under 8).
- j45 f8b8aa33 (NSS_READ_OUTSTANDING: one read per process; refuses "an earlier name-service read has not returned"; Drop clears on return/panic; per-test guard). architect-cto: conforming to ADR-0028 text in #239 (3 work 2 fix, docs). #236 = 5 work, 3 fix; arms after #239 merges AND (8 work or owner's word).
- #236 later (2026-10-09): d44111e4 (guard via NameService::outstanding(), required; F1 parallel tests 257/300 → 0), 1693dd82 j46 (ReadGate Mutex+Condvar, waiters wait up to own deadline; architect ruling (b) seq 29607), 1abd4b48 (pin HostNames → process gate). Round 8 (of d44111e4) saved, to post combined with the review of 1693dd82..1abd4b48. A reviewer saw the wait test fail once at 1693dd82 (mutant-shared target suspected); I got 0/50 + 0/20. #239 = architect's ADR text, awaiting owner's word.
- #236 head 0e0b9484 review-clean (round 10 posted): a96330d0 (back-to-back asserts gate free per answer; reorder mutation 5/5), 0e0b9484 (one-budget test; recv_timeout(deadline) mutation fails; "flag" → "gate"). 6 work, 7 fix. Arms after #239 (4 work 5 fix, green, reviewed, waiting owner's word) merges, and needs owner's word (under 8). Reviewer leftovers cleaned each round (~650 MB each).
- **#236 MERGED 2405b7ce (2026-10-09)** after #239 (architect's ADR text) on the owner's "merge both". Carried: R2 (a read entering with no budget left still spawns its thread; an early `left.is_zero()` refusal would skip it); the optional per-walk (euid, gid) answer reuse architect permitted; R1 (a hung service blocks each trust write's task 5 s). Target cleaned 1.8 GiB.
- 2026-10-09 architect-cto's plan (seq 32615): (1) j47 → PR #241 (zero-budget read starts no thread; review-clean at 81dd6f7f, 1 work 3 fix, unarmed — needs owner's word or more work); (2) j48 §20 step 1: embedded runtime host on Android with the in-process adapter + trust boundary plumbed; NOT gated by SPIKE-008/009; tell architect the crate/seam before the first commit, and agree the Service seam with rust-ui-dev by REQUEST; (3) j49 §20 step 5 runtime side, the network-change binding API. architect closes the spikes. j48/j49 topic stage-17-runtime → fresh session.

*References: agent-prompt-does-no-shell-expansion, pr228-judge-before-create*

*Observed 2026-11-04 (p2p-network-dev)*
