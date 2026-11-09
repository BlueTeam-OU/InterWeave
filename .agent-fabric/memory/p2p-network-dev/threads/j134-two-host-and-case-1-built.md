---
role: "p2p-network-dev"
class: threads
topic: "j134-two-host-and-case-1-built"
description: "j134 (2 of 3) — two-host overlay and case 1 as built: wildcard listener measured, netns in_overlay, #475 and #478 merged, review carries, CLI wire-byte lesson"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "p2p-network-dev-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 1e70ca6635fdfbb0
---

## j134 (2 of 3) — two-host overlay and case 1 as built: wildcard listener measured, netns in_overlay, #475 and #478 merged, review carries, CLI wire-byte lesson

Part 2 of j134 (REQUEST 01a17c02, architect-cto); part 1 is j134-control-plane-over-interweave.

### Built 2026-10-27 (second session)

- `tests/support/src/e2e/netns.rs` now has two topologies: `in_second_network` (NATs + relay) and `in_overlay` (A 100.64.0.1 w0 -- w1 100.64.0.2 B, nothing else), sharing PRELUDE and NEW_ROOT. `overlay_down`/`overlay_up` run from the pivoted root, so they use `nsenter`, never `ip -n` (ip -n remounts /sys, refused there). `ip link set w0 netns A` works after the pivot.
- `tests/automated-client/tests/two_hosts.rs`: the daemons listen on `0.0.0.0`. MEASURED 2026-10-27: with the listener on the overlay literal (`/ip4/100.64.0.x/tcp/p`) the same case fails at "both closed on the link's loss within 10s" -- the detector never sees the address leave (a specific-IP listener reports no interface change). Reported to architect-cto: 5b's "or on a wildcard" is not an even choice. The wildcard is what closes the connection within 10 s of `ip link del` (mutation: `network_change::closes` -> false fails it at "both closed on the link's loss"). A static entry connects in about 0.19 s (mutation 2's failure time).
- ADR-0052 rule 9 admitted the 100.64/10 literal from config with no transport change. `admin.peers.add_address` checks rule 2's floor instead, a different door.
- Flake seen once in 6 runs: `second_network.rs` "A of B: begun relayed", left None (the route_established event was never read). Carried in #475's body, not fixed.
- Case 1 (carrier on gzcoord + `interweave-human listen --input --consume --endpoint fabric-control` on one profile) waits for rust-ui-dev-02's verbs PR on main. After both land: the release run (Stage 19) is mine.
- Ruled 2026-10-27 (architect seq 607718): 5b = wildcard ALONE; their 3be2078a folded ff on #475 (now 4 work incl. two supplies). Arm only when case 1 is in and the blind review is posted.
- 2026-10-27 OWNER: "merge 475". It is armed without case 1. The blind review of 38db9e38 found no P1/P2. CARRY to the case-1 PR: F1 (two_hosts.rs doc "stays unmeasured"); F2 (a third case: literal listener, connection still open after CLOSE); F3 (DEPLOYING.md 168-171/300-302 name the wildcard + cite 5b); F4 (evidence file: quote config.yaml, move unquoted facts under "does not show"); the in_overlay nft message; ADR-0062 run-time door on a 100.64 literal. The ADR halves of F1/F2 went to architect-cto (OBSERVATION 01a17c62). CI ran both two_hosts tests on GitHub and they PASSED; no devex step was needed.
- OWED after #475 merges: run two_hosts on a local merge of main + #476 (dev-02, f7544cdd; it withholds 100.64/10 listeners from Identify and touches network_change.rs off_local). Send the result to p2p-network-dev-02 (REPLY 01a17c70). Never push that merge.
- #475 MERGED cf7089b5 on 2026-10-27, armed on the owner word at 7 work. The proof on main+#476 (head 78f0282b) is running on a local branch tmp/j134-proof-476: delete it after; the case-1 follow-up branches off origin/main.
- Follow-up PR #478 (branch feat/j134-case-1-and-review-carries) opened 2026-10-27, unarmed: F1-F4 + nft message done + architect note (ii) folded (5d551a69). Case 1 still waits on rust-ui-dev-02 verbs. Proof main+#476 run: 2/2 pass twice, sent to dev-02 (REPLY 01a17c80). RULED (01a17c8d): the add floor stays; a 5b peer is a config.yaml entry by design (ADR-064 renders config from the roster). Their a8fe9ea1 is folded on #478; build nothing.
- Case 1 BUILT 2026-10-27: control_plane.rs on #478 (c2e92d16), after #477 merged (73579a46). Facts: interweave-human --endpoint applies to listen/send only; flags may go anywhere; `messages <peer>/<endpoint> --json` prints no line when no row is left; clippy doc_markdown flags a bare "GZCoord" in a doc comment. Blind review of 73579a46..c2e92d16 dispatched. #478 has 7 work: arming below 8 needs the owner word. Item 3 (the release run) follows #478.
- #478 at 886f06e9: three blind rounds; round 3 clean. Carried risks: carrier_b not re-checked for id(2) after the typed sends; ids(carrier_a) read before them. Needs the owner word to arm (7 work < 8). Lessons: CLI wire bytes are a HumanChatV2 envelope (media application/vnd.interweave-human-chat+json;v=2), so a raw-text compare is vacuous; a "never printed" check needs a printable frame plus a positive control. fabric-review next takes the previous rr<N>.json request, and `fabric-review save <pr> <round>` must run first.
- #478 MERGED 79c84b29 2026-10-27 (owner word at 7 work). Items 1+2 on main. Item 3 release: ADR-0066 says the owner signs the tag by hand. My part: a version-bump PR, the cases taking the released binaries (INTERWEAVE_PACKAGE_BIN like human_cli.rs), then running them against the verified archive with an evidence record. Asked the owner for the version and go-ahead.

*Observed 2026-11-03 (p2p-network-dev)*
