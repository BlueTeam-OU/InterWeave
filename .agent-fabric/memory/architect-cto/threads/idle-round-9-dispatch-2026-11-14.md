---
role: "architect-cto"
class: threads
topic: "idle-round-9-dispatch-2026-11-14"
description: "owner asked for InterWeave work for the idle p2p-network-dev logins; two REQUESTs from the hold rule's Not-proved list; #667 armed"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 284d2c9d8ffb1696
---

## owner asked for InterWeave work for the idle p2p-network-dev logins; two REQUESTs from the hold rule's Not-proved list; #667 armed

2026-11-14, the owner in session: the network developers are idle, give them
InterWeave work. Both had every branch on main and no open PR. Two REQUESTs,
reply expected, from the "Not proved, by name" list of CONNECTIVITY.md s14
(A 2026-11-13):
- p2p-network-dev-01, 01a1da16-2f97: the same-connection choice over two real
  Swarms (crossing real dials) and an outage past several dial-backoff steps.
- p2p-network-dev-02, 01a1da16-31ac: the phone's cost of the 15 s hold ping,
  held vs control, as a device record; first a reply naming what it needs
  (phone, APK from rust-ui-dev-01, the owner's hand) — I ask the owner once.
Still carried after both: a real NAT/carrier drop, 180 s on two real hosts.
Whether the phone keeps 15 s is my ruling after the figures; s14 wording is mine.

Same session, j706: BlueTeam-OU/InterWeave#667 (1w,7f) ARMED at 5f60b5db on the
owner's word ("arm 667"), j705 deferred on the arm (its wait is #670; new branch
after the merge). Asked the owner whether the word also accepts ADR-0068 rule 7
(marked as stated for the first time). On merge: close j686.

Answers 2026-11-14 ~12:46Z: p2p-network-dev-01 took it as j168 (starts off main
after #667 lands; branch name to follow). p2p-network-dev-02 took it as j110
(REPLY 01a1da17-e971): needs the Pixel 9a ~8 h exclusive at the end of the phones
list, the wireless-debugging port at setup and each read, the phone unplugged and
untouched for a 3 h held + 3 h control window, and the owner's word for a profile
write (one identity, one allowed contact). I ruled debug build accepted for the
first row, Wi-Fi only / one peer / no NAT as stated limits (my REPLY, same hour).
ASKED OF THE OWNER in session, not yet answered; relay the answer to
p2p-network-dev-02 before any device step.

2026-11-14 ~12:55Z: the owner answered with the Pixel's wireless-debugging address
only. Both leases free, so the Pixel slot went to p2p-network-dev-02 now (DECISION
in reply to 01a1da17-e971): setup allowed; the profile write and any uninstall of
another signer still wait for the owner's explicit word (asked again in session).
~12:58Z: the owner gave the A40's address too; lease free, no other open ask, so it
went to p2p-network-dev-02 as j110's second row (DECISION, same terms; the Pixel row
keeps its slot if another role asks for the A40 first).
~13:00Z: p2p-network-dev-02 (REPLY 01a1da1f-c5aa) holds both leases, stopped at the
signer read: both phones carry 0.3.0 + .test under the shared installer debug key
(18f39ab0...), its own is 00a92280..., so it needs the owner's word to uninstall
WITH DATA (its option (a), my recommendation). Asked ux-designer-01 and
rust-ui-dev-01 (QUESTIONs, reply expected) whether a job still needs that install;
then one ask to the owner: uninstall per free phone + the profile write. Phones stay
plugged until a window starts (told the owner).
13:02Z: BlueTeam-OU/InterWeave#667 (1w,7f) MERGED 600c6f735 on the owner's word at
5f60b5db; j686 and j706 close on it. j705 (S36 propagation) takes a new branch.
ux-designer-01: both phones FREE TO WIPE (01a1da21-07d3); rust-ui-dev-01's answer
still owed; the phones follow-up is its own queued job.
14:47Z (j707): rust-ui-dev-01 had not answered 01a1da20-5b37-7ff1-8ed5-349f9b586a23
(session running since 04:14Z); chased as QUESTION 01a1da86-68cc-77d3-a85f-36e4d3adfafb
(FREE TO WIPE or KEEP per phone). j707 blocked on it. On the answer: one ask to the
owner (uninstall WITH DATA per free phone + the profile write), relay to
p2p-network-dev-02 in reply to 01a1da1f-c5aa-7400-906a-92bd6c3fda73.
18:42Z: p2p-network-dev-02 asked hold or release (01a1db5d-821b); I ruled RELEASE
(REPLY 01a1db5e-222e); both phones released (INFO 01a1db5e-8e53, -9463), nothing
changed on either. On the owner's word (uninstall WITH DATA + profile write) I
send a DECISION and it takes the leases again; Pixel row keeps priority.
rust-ui-dev-01 still silent on 01a1da86-68cc.
2026-11-15 00:06Z: rust-ui-dev-01 answered (REPLY 01a1dc86-5c44): Pixel 9a and A40
both FREE TO WIPE. Both roles now say free. ASKED THE OWNER in session (one ask):
uninstall org.interweave.human + .test WITH DATA on both phones, and the profile
write (one identity, one contact). On the word: DECISION to p2p-network-dev-02 in
reply to 01a1da1f-c5aa; it takes the leases again (released 2026-11-14 18:42Z).
2026-11-15 ~04:35Z (j707): THE OWNER'S WORD GIVEN in session: both phones, uninstall
org.interweave.human + .test WITH DATA and the profile write (one identity, one
contact). He also ACCEPTED ADR-0068 rule 7 as written (mark removal queued in j720).
DECISION 01a1dd7d-4c5a to p2p-network-dev-02: A40 row first; the Pixel goes first to
ux-designer-01 (j129, add-a-peer item 12, 60-90 min, its QUESTION 01a1dce6-0b34; my
REPLY 01a1dd7d-bca6), which tells p2p-network-dev-02 by INFO at release. Its later
foreground-only run (after rust-ui-dev-01 j280) is a new slot on a fresh install.
p2p-network-dev-02's timeline (REPLY 01a1dd7e-9c73), per phone, A = address received:
A..A+25 min setup (charged to 90 %+), then "unplug <phone> now" through me; control
U..U+3 h, held U+3 h 10..U+6 h 10, plug back at U+6 h 15 on "done with <phone>".
OWED BY THE OWNER: the current wireless-debugging address of each phone (asked in
session 04:40Z); A40's goes to p2p-network-dev-02, the Pixel's to ux-designer-01 now
and to p2p-network-dev-02 again after the release. Addresses go in messages only,
never in this store.
Same session: #675 arming word given to rust-ui-dev-02 at 2fd50669 (REPLY
01a1dd7e-c38b, j719 queued: deliver reply:01a1dd7e-c38b after a switch activates it).
2026-11-15 04:45Z: the owner gave the Pixel's address; sent to ux-designer-01 (INFO
01a1dd86-b2b7). 04:47Z it answered (REPLY 01a1dd88-2603): ping and the TCP port answer, adb
connect fails six times; it holds no lease, slot not started. ASKED THE OWNER in session: the
port on the Wireless debugging page itself (not the pairing dialog) and whether Paired devices
still lists ux-designer-01's login (else a re-pair at the phone). The A40's address is still owed.
05:58Z: owner's answer on the Pixel connect port and pairing STILL OWED when the session moved to
j720; on it, send the address to ux-designer-01 in reply to 01a1dd88-2603. Same session: chased
fabric-coordinator for agent-fabric #620 F1's record text on the owner's ask (QUESTION 01a1dd9b);
it answered it is writing it onto #620's branch (REPLY 01a1dda1), nothing owed by me.
2026-11-15 ~21:20Z (after the fleet hold lifted 21:15Z): THE OWNER GAVE BOTH ADDRESSES in session;
relayed to p2p-network-dev-02 (INFO 01a1e113-fa61, reply expected, in reply to its 01a1e113-0744:
j110 resumed, rebuilding at eb72c4f4b). The Pixel's was typed with five groups; I read it by a TCP
probe from the host (the reading that answered). STILL OWED BY THE OWNER for the Pixel: Stay
reachable on, "Don't keep activities" on then off, charge 90 %+ (asked again in session). Next:
p2p-network-dev-02's reply (adb connects? unplug times) -> tell the owner when to unplug each phone.
21:25Z: p2p-network-dev-02 connects to both (REPLY 01a1e115-3bdc). A40 row: its "A40 ready for the
switch" ~22:00Z (owner turns Stay reachable on, Home), "unplug A40 now" ~22:05Z, "done with A40"
~04:20Z; each word comes to me, I pass it to the owner in session. PIXEL: someone reinstalled
org.interweave.human at 07:36:34Z (new uids); ux-designer-01, rust-ui-dev-01, rust-ui-dev-02 all
answered NOT MINE; DECISION 01a1e119 to p2p-network-dev-02: the Pixel is its own and free, fresh
setup after the A40 leaves room, then "Pixel ready for the owner" -> the owner's three actions.
Asked the owner in session whether the 07:36Z install was by his hand.
21:28Z: WITHDRAWN: the Pixel's "07:36Z" install was p2p-network-dev-02's own setup at 06:36:34Z (dumpsys
prints local time, UTC+1; INFO 01a1e11b-41e8). Three sessions and the owner told. The Pixel keeps its
identity and contact; install -r at eb72c4f4b after the A40 leaves room, then "Pixel ready for the owner".

### Phones, state 2026-11-16 02:10Z (j707)
- Both phones set up by p2p-network-dev-02 and waiting on the owner's hand (INFO 01a1e120-5bc2 A40, 01a1e121-0c30 Pixel, 01a1e183-701b: at 23:20Z no Stay reachable on the A40; morning is fine).
- Owner's steps, told to him three times in session, not yet confirmed: A40: Stay reachable on, Home, on charger. Pixel: "Don't keep activities" on then off, Stay reachable on, Home, charge to 90 %+, report its level.
- Times count from each switch S: unplug S+5 min on p2p-network-dev-02's word (I relay "unplug A40 now" / "unplug Pixel now"), plug back after S+6 h 20.
- On the owner's confirmation: REPLY to p2p-network-dev-02 in reply to 01a1e183-701b with the time and the Pixel's level.
- Hygiene round 2026-11-16 (REQUEST 01a1e211-dcdf, broadcast): 17 clean worktrees wholly on main removed, 16.9 GB freed; kept: j639 dir (branch j640 docs), j660, j686 (1 commit each on no remote, the owner's decision), j707, j720 (3.8 GB target, #680 open).
- gzapp: j730 = fold language-culture-ru's 197af2e8b + 1dc7d99df into a PR of mine (REPLY 01a1e219-e70e); j727 ka-GE term still with the owner.

*Observed 2026-11-16 (architect-cto)*
