---
role: "architect-cto"
class: workflow
topic: "arming-words-2026-10-27-ii"
description: "arming words given 2026-10-27 evening as InterWeave below-minimum authority, each on a read head with its posted review"
tier: 1
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 86c2750fd7757c6a
  - d1ccc8be4ea3a410
  - d642cc06ff53b848
---

## arming words given 2026-10-27 evening as InterWeave below-minimum authority, each on a read head with its posted review

- #493 (2 work, 3 fix) ux-designer-01: withheld at ca22809c (citation ahead of main), given at 82637f90 after 5d4d3393 folded (REPLY 01a17eb0-c76b); merged.
- #494 (5 work, 4 fix, boundary) rust-ui-dev-01: given at 9c5a9305 (REPLY 01a17ed7-bc2e); merged.
- #495 (6 work, 5 fix, boundary) rust-ui-dev-02: given at 7f51ca08 (REPLY 01a17f3f-9d1e).
- #496 (3 work, 2 fix) rust-ui-dev-01 Connect late-trust line (j174): given at 44123456 (REPLY 01a17f42-ba75-7fa6-963f-89365e790c32) with checks still pending (the arm waits on them itself).

**How to apply:** fabric-pr gate + review-status + arm --any-owner --dry-run first; the AUTHORIZES line goes in the header block (before the body), 40-hex head; a send refused on presence during a relaunch is retried, never forced blind.
- gzapp #1188 (2 work, 2 fix) web-dev-01 TableScroller region (j43): given at 7c1a4fb4 (REPLY 01a17fed-9352-79c5-984a-3bc5fc08c703), 2026-10-28 00:35Z, the owner asked 10-27 and silent; composed from the gzapp working copy so REPOSITORY is gzapi-org/gzapp.
- #497 (2 work, 4 fix) ux-designer-01 start-window s3 scope + Connect Tab stop: given at 767d191a (REPLY 01a180d5-5ef8-7180-a308-067d31328afa), 2026-10-28 04:50Z; told them a 7.1 read-proposal sentence follows from rust-ui-dev-02.
- #499 (3 work, 5 fix) p2p-network-dev-01 v0.2.0 release check + RUSTSEC-2026-0414 lock bump: given UNASKED at 281a8707 (DECISION 01a18200-5b50-70ef-b91d-aa70334bf092), 2026-10-28 10:40Z, to unblock every PR.
- #504 (3 work, 2 fix, boundary) p2p-network-dev-01 port-0 fixture flake: given at 65c16491 (REPLY 01a1826a-5258-7c48-8d06-df1af580482f), 2026-10-28 12:12Z.
- #507 (3 work, 2 fix, test-only free_port class) given at 8848b5c2 (j451, REPLY 01a182a9-bc92-7aa2-a433-5526825ca052, 2026-10-28)
- 2026-10-28 ~14:00Z the owner: "arm 506 505 507" — #505 (1 work, 1 fix) armed at 9c23f083 (queued 2), #506 (2 work, 8 fix) armed at a37ccde7, #507 already merged by p2p-network-dev-01 on my word
- #508 (2 work, 5 fix, human-cli G8 peers state + G7 listen filters) given at 7e020b2d then again at aab0ec8b after F5's test (j452, REPLY 01a182fb-e1af-7960-9fb4-9295b421b6ed; 2026-10-28 ~14:50Z); a word covers one head only — a push between my read and the send costs a re-ask
- #510 (1 work, 15 fix, ux-designer-01, design/2026-10-27-start-window.md B3 placement answers) given at 4f8f04de (j453, 2026-10-30 ~05:15Z)
- 2026-10-30 ~05:20Z the owner: "arm 509" — #509 (1 work, 1 fix) armed at f0fd0559, queued at 1
- #511 (1 work, 2 fix, p2p-network-dev-02, Pixel run record route_hold 4/4 + paths 2/2) given at babdad9d (j455, 2026-10-30 ~06:00Z)

2026-11-03 (j495): InterWeave #532 (4 work, 5 fix, 1 merge; rust-services-dev-02, SPIKE-001 S28) AUTHORIZES at
8557bf955 by REPLY 01a1a131-e899 to REQUEST 01a1a130-1cb3; read the README diff from 7f70b352 and the two clean
reviews on that head first. Its S28 bullets are what j490 amends in CLAUDE-CODE-CHANNEL.md and plan §19 after it lands.

2026-11-03 (j496): InterWeave #535 (7 work, 10 fix, 1 merge; rust-ui-dev-01 j196, store schema v13 = boundary)
AUTHORIZES at 21792a6e2 by REPLY to REQUEST 01a1a140-ac1f; read schema.rs's migration_13 (DDL only, covering
indexes, retention tables untouched), STATE.md v13 (my supply cb1b1a674) and the clean round-5 review on that head.
Their kept_disable desktop-e2e failure under load is their j197, not the boundary.

- 2026-11-03 #541 (1 work, 0 fix) at 2c8de9f0, REPLY 01a1a276-01cd: spike-001 README Amended markers. Ruled: a bullet quoting a sentence the document has since EXTENDED (quoted words kept, clause added) counts as replaced; precedent the 2026-10-24 marker on the plan §19 bullet.

- 2026-11-03 #541 second REPLY 01a1a277-68f6 after a stray digit in the first header ("2c8de9f0d"): arm compares the header as a PREFIX of the full sha, so copy the 8 hex digits from `gh pr view --json headRefOid`, never retype. Owner said "arm 541" too; my session could not arm (publish held to a fresh session after a closed job), so the word went by message.

- 2026-11-04 #542 (2 work, 4 fix) at 34caadbf, DECISION 01a1a3d6-f98a to rust-ui-dev-02 on the owner's "merge 542"; head copied from gh, not retyped.

**2026-11-04 02:48Z InterWeave #546 (2w, 2f; rust-ui-dev-02 j82, focus rings over neighbours + the ui-slint README correction):** word given at 00b59469 by REPLY 01a1a475-8268-7722-9c8b-9e91e3dd553c with AUTHORIZES (their REQUEST 01a1a474-2d6f, my j504); three chained reviews, F1-F4 fixed, 0 threads, checks pending; the arm dry-run is refused in a session whose job has ended, so the gate state was read from fabric-pr review-status and gate instead.

- 2026-11-04: InterWeave #547 (2 work, 2 fix) word given at dfd3df88 (REPLY 01a1a513-f6a1); j83 Tab-onto-text-field ring, ui-slint only; checks pending when given, arm waits on green.

- 2026-11-04: InterWeave #549 (1 work, 1 fix) word given at d77fa609 (REPLY 01a1a571-eee0); j84 press-on-text-field ends keyboard mode on desktop; F1 (P3, winit wiring untested) deferral accepted, desktop-e2e case carried to rust-ui-dev-02.

- IW #555 (3w, 1f, spikes only) word given at 5d3e7fe6 on 2026-11-04 (REPLY to rust-services-dev-01 01a1a6e3-3b43; head reviewed + re-reviewed clean, 0 threads, checks pending at the time).

- 2026-11-04 ~15:12Z IW #557 (1 work, 1 fix; rust-services-dev-01 j73, INSTALL.md's headless bullet on S29 facts 70/71/75) word given at c2fe24c9 by REPLY 01a1a71d-7a46 (j515): one-file diff read against the facts, two blind reviews, round 2 clean, 0 threads, checks pending.

2026-11-04 j520: #558 (rust-services-dev-01, INSTALL.md facts 45/46/74 one-run wording; 1 work, 1 fix) word given at f83b860c, REPLY 01a1a74c-31e1 with AUTHORIZES; round-2 re-review clean, F1 (P3) fixed; fact 74 read on origin/main.

- 2026-11-05 #572 (7 work, 4 fix, 1 merge; docs only) at 7525d75aa — ux-designer-01, REPLY 01a1acaf-4bfb with AUTHORIZES header; review of the head posted 17:04Z, 0 threads.

- 2026-11-05: #579 (4 work, 3 fix; design/ only, not a boundary) at a94096d7e8d6ee93af9ac71df3e4c9d4c5fefee8, ux-designer-01, REPLY 01a1adc0-6845 (seq 865097) with AUTHORIZES in the metadata; read §11 at the head, rounds 3-5 (F1-F9 judged fixed, last clean), the two carried-risk comments. Lesson: fabric-jobs switch refuses while a REPLY-EXPECTED request is unanswered, so an arming ask that arrives mid-job is answered in the same session, not the next.

- #580 (3w,3f, spikes/ only) rust-services-dev-01 j75, SPIKE-001 S30 facts 76-79, at 6b08f8c2 on 2026-11-06 (REPLY 01a1ae65-8611 with AUTHORIZES); re-review clean at the head, F1/F2 judged fixed, nothing under architecture/.

- 2026-11-07: #588 (1 work, 0 fix; apps/human-android/android/osv-scanner.toml only) at 761868e2 for p2p-network-dev-02, REPLY 01a1b7fa-7b34 with `AUTHORIZES: BlueTeam-OU/InterWeave#588@761868e2` (REQUEST 01a1b7f9): bcprov CRITICAL GHSA-9pwp re-dated to 2026-12-07 before its 2026-11-10 expiry would fail the tree checks on main; Guava GHSA-xxph entered; GHSA-c3fc (fixed in 1.80.2) removed; review clean but a P3 on a control-plane slice; judgement, scan and provider reading on the PR.

- **#591 (1w,1f, CLAUDE.md §9 + ci.yml comments: APK job required)** word given 2026-11-08 at b34221389 (REPLY 01a1b9f1-7d19 to devex-tooling, IN-REPLY-TO 01a1b9f0-6f7e, j554; repeated as 01a1b9fb-ceb1 against a second REQUEST 01a1b9fb-2280, j555); devex arms after its j299 ruleset/policy change lands the fifth context.

*Observed 2026-11-08 (architect-cto)*
