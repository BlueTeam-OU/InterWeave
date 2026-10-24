---
role: "p2p-network-dev"
class: workflow
topic: "gpg-keyboxd-stale-lock"
description: "gpg/keyboxd hangs \"waiting for lock (held by N)\" after a hard kill; commit signing times out; how to clear it"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "p2p-network-dev-02"
    host: "develop-qzapp"
    project: interweave
    working_copy: InterWeave
derived_from:
  - 61757e99c2f3280c
---

## gpg/keyboxd hangs "waiting for lock (held by N)" after a hard kill; commit signing times out; how to clear it

On 2026-10-18/19, commit signing on this account timed out: "gpg: signing failed: Connection timed out", with INV_SGNR from ots-git-gpg-wrapper.sh. One cause was keyboxd's database lock, ~/.gnupg/public-keys.d/pubring.db.lock, which was hard-linked to `.#lk…<host>.<pid>` and held by a pid from the 2026-10-17 crash. That pid had since been reused by another login's bash, so `ps -p` showed a process even though the holder was dead.

The fix, asked by fabric-coordinator in REQUEST 01a152fb:
1. `gpgconf --kill keyboxd`;
2. remove only the lock files that name the dead pid;
3. kill keyboxd again.

After that, `gpg --list-keys` answered. Signing still needed the owner to re-import the secret key (`new-agent.sh --signing-key-only`), so a dead lock is not the only cause. Before retrying a commit, test signing with `echo t | ots-git-gpg-wrapper.sh -bsau <key>`. A `ps -p` hit is not proof the holder is alive: check that the pid is one of this login's own processes.

*Observed 2026-10-19 (p2p-network-dev)*
