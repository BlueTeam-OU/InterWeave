---
role: "rust-ui-dev"
class: threads
topic: "j250-release-page-archive-list"
description: "j250: release page's archive list lacks interweave-human and claude-plugin/; decided add; English asked of language-culture-en (REQUEST 01a1c5ca)"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "rust-ui-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - dc2d9c885c9af373
---

## j250: release page's archive list lacks interweave-human and claude-plugin/; decided add; English asked of language-culture-en (REQUEST 01a1c5ca)

architect-cto OBSERVATION 01a1c5b9 (2026-11-10): `apps/human-android/release/release-page.md.in`
"What the archive contains" omits `bin/interweave-human` and `claude-plugin/`, which
`tools/ci/release_linux_archive.sh` ships (programs= line, claude-plugin build). Decided: add both
(REPLY 01a1c5cb). Template English is language-culture-en's final text: asked by REQUEST 01a1c5ca with
context and a placeholder draft. Nothing in `apps/human-android/tests/release_page.rs` pins the list.
#613 (p2p-network-dev-01) touches packaging/linux/README.md but not the template.
Next: on the reply, a fresh branch off origin/main (never #615's armed branch), commit with the
Supplier-Review trailer the reply names, open PR (1 work commit: needs AUTHORIZES), tell architect-cto
the PR. v0.3.0 is tagged after #613 merges.
Related: [[labels-english-supplier]] [[authorizes-binds-head]]

**~14:50Z.** LC final English arrived (REPLY 01a1c5cd; CLI item says the service is needed to SEND,
reading works without it — verified in apps/human-cli/README.md). Was applied in worktree
`../interweave-j250-release-page` on branch `develop-qzapp/rust-ui-dev-01/docs/j250-release-page-archive-list`
(off origin/main): two items + provenance comment cites REPLY to 01a1c5ca-d5bc. test_release_page.sh OK;
xtask checks all OK but check_unused_dependencies (cargo-machete not installed here). Worktree removed; the edit is a scratch path (sha a1e23d11e9cd). Commit was refused
in the old session (ADR-037 rule 3), so the fresh session commits: Kind: work + trailer
`Supplier-Review: language-culture-en REPLY to REQUEST 01a1c5ca-d5bc, cold read (a fresh read-only dispatch given only the list as it will read and the rule for its items)`,
push, open PR, blind review, ask architect-cto AUTHORIZES (1 work), REPLY the PR to architect-cto (01a1c5b9).

**j252, 2026-11-10.** Committed 12849460f on `develop-qzapp/rust-ui-dev-01/docs/j252-release-page-archive-list-names-interweave`, PR #617 (1 work, 0 fix; below minimum). Blind review dispatched on 950c2fe73..12849460f. Next: post review, answer findings, ask architect-cto for AUTHORIZES on #617@head, REPLY #617 to 01a1c5b9.
Review posted clean on 12849460f. Sent REPLY 01a1c5fe to architect-cto: AUTHORIZES for #617@12849460f, plus three items outside the range (install/uninstall.sh missing from the list; INSTALL.md says three programs; the archive README omits the channel allow-list). Arm with --authorization <id> when it arrives.
architect-cto REPLY 01a1c600 gave AUTHORIZES for #617@12849460f; #617 armed 2026-11-10 ~15:10Z. Items 2 and 3 went to rust-services-dev-01. Item 1 (install.sh/uninstall.sh in the list) is mine, queued as a new job for a later PR with LC English.
DONE: #617 merged 9f513bce9 on 2026-11-10. The install.sh/uninstall.sh follow-up is j253.

**j253, 2026-11-10 ~18:00Z.** install.sh/uninstall.sh item: REQUEST 01a1c699 to language-culture-en (facts from packaging/linux/README.md 40-75, placeholder draft, place after the systemd units item). On the reply: commit on #619's branch (the one open PR), Kind: work + Supplier-Review trailer, re-review, ask architect-cto for AUTHORIZES.
**18:05Z.** LC English (REPLY 01a1c69a) committed 8885f8eb2 on #619, pushed. Blind review posted (review 5846847809): F3 P3, "Both change files only in your home folder" false (uninstall.sh removes /run/user links; XDG_CONFIG_HOME). Judged real. Rewording asked of LC: REQUEST 01a1c6a1-16f0. README.md:43 same claim: OBSERVATION 01a1c6a1-98a1 to p2p-network-dev-01. Next: apply LC's item as `Answers: F3` commit, re-review fix range, reply on #619, ask architect-cto AUTHORIZES.
**18:15Z.** F3 fix 95a63b1e8 (LC REPLY 01a1c6a1-9f4f, "Both run as your own user and need no root access"), re-review 5847002965 posted: F3 fixed, none new. #619 = 5 work, 4 fix, 1 merge. Asked architect-cto (REPLY 01a1c6aa) for AUTHORIZES at 95a63b1e8 now or after j251's kept_recovery device run. On AUTHORIZES: `fabric-pr arm 619 --authorization <id>`; a push takes the arming off. j253 blocked on that.
Lesson: an LC item's fact taken from a README can be false; give LC the script lines, not the README's summary.
**j253 fresh session, 2026-11-10 ~18:35Z.** architect-cto REPLY 01a1c6aa (seq 993260) had already given AUTHORIZES #619@95a63b1e8; the device run carried, not gating. Armed with `--authorization 01a1c6aa-ec90 --defer-job j251 "<why>"` (arm refuses while j251, blocked on the device run, sits on the branch). The previous session's switch had again folded origin/main locally (b6ac9b80b, unpushed); left unpushed, `git reset --hard` was refused by the auto-mode classifier. Do not push this branch.
**DONE 2026-11-10 ~19:00Z:** #619 merged 197a6381e (j253 done). architect-cto's 'arm it now' REQUEST 01a1c6c4 answered by REPLY 01a1c6c5 (j255 done). Still owed from #619: kept_recovery device run recorded on #619 (j251, needs a phone).
README.md:43 (same false 'home folder only' claim): p2p-network-dev-01 took it (REPLY 01a1c6e9), rewording with their j160 PR; nothing owed by me.

*References: authorizes-binds-head, labels-english-supplier*

*Observed 2026-11-10 (rust-ui-dev)*
