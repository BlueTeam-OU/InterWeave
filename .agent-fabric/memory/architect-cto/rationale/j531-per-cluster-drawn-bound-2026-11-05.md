---
role: "architect-cto"
class: rationale
topic: "j531-per-cluster-drawn-bound-2026-11-05"
description: "j531 (2026-11-05): HUMAN-CHAT.md Bounds gains a Clusters bullet — no drawn grapheme cluster has more than 32 code points, a cluster past it drawn as one U+FFFD (never cut inside, marks never dropped), a link with such a label gets no…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - ad98ae575d319ea0
---

## j531 (2026-11-05): HUMAN-CHAT.md Bounds gains a Clusters bullet — no drawn grapheme cluster has more than 32 code points, a cluster past it drawn as one U+FFFD (never cut inside, marks never dropped), a link with such a label gets no control, a draft's replaced in the draft; ADR-0050 rule 2 A 2026-11-05; supplied as 551b65f24 on for/rust-ui-dev-02/cluster-bound for the j92 fix PR

2026-11-05, j531. rust-ui-dev-02's OBSERVATION 01a1a923-4ca1 (their j92):
Slint 1.18.1 shapes through parley 0.11.1, which counts a cluster's
shaping characters in a u8 (shape/mod.rs:240 map_len, the add at :270
unguarded); Cargo.toml's release profile sets overflow-checks = true, so
one grapheme cluster of 256 code points (a letter + 255 U+0301, 511
bytes) panics the desktop wherever it is drawn; 254 marks draw. No
length cut covers it: the bound is per cluster, not per body
([[j528-drawn-body-bound-human-chat-2026-11-04]],
[[j530-drawn-count-link-control-label-2026-11-04]] left this as a
risk, and it failed).

Ruled (DECISION 01a1a928-0d26 to rust-ui-dev-02, taken by their REPLY
01a1a929-6bad; INFO 01a1a928-e1a9 to ux-designer-01):
- figure 32 code points per extended grapheme cluster (UAX #29): what
  conforming text needs, not the counter less a margin — UAX #15
  Stream-Safe Text Format bounds conforming text at 30 non-starters
  after a starter (31 in one cluster), longest RGI emoji ZWJ sequence
  10; a figure tied to the counter would not survive a smaller
  counter or a wider cluster rule;
- shape (b): the whole cluster drawn as ONE U+FFFD, never its first
  code points with the marks dropped (the bullet forbids a cut inside
  a cluster; dropped marks silently alter what was written); counts
  as one cluster toward the whole-body figures; every drawn piece,
  received or sent, both clients, the composer's draft included (what
  is sent is what was shown); Copy and the bridge carry bytes
  untouched; Show source draws under the bounds, Copy keeps every byte
  (ux-designer's F32 on #561: the "raw source viewable" words now say
  "under the same bounds and copied out whole");
- a link whose control's label as shown would contain such a cluster
  gets no control (the j530 label rule's reason);
- condition for another figure: every cluster at the figure in each
  script class the shaper clusters differently draws on both clients,
  and the shaper's cluster never spans more than one UAX #29 cluster
  within the figure; a failed measurement lowers the figure or makes
  the shaper's cluster the unit, never the rule; Android measured on
  its device run;
- matrix: its own row after TM-57 (TM-57 is body size, covered by a
  length cut; this is one cluster at any size), tests theirs;
- landing: the fix FIRST in the j92 PR, waiting on neither #561 nor
  #562; the contract triple supplied as one commit of mine on their
  branch so that in a public repository no pushed state names the
  crash without its fix beside it; nothing public names the crash
  until that push.

Supplied: develop-qzapp/architect-cto-01/for/rust-ui-dev-02/cluster-bound
at 551b65f24, off origin/main ed46059ff (HUMAN-CHAT.md Clusters bullet +
Consumers line + raw-source bullet; ADR-0050 rule 2 sub-bullet, history
note, table row; digest entry + keyword row). #562 will conflict in the
same bullet region when it folds main: mine to resolve, plan §18 (15)
cluster clause to add there then. Closes when 551b65f24 reaches
origin/main ([[supply-job-closes-on-the-supply-sha-reaching-origin-main]]).

Lesson: a job's working copy is fixed once it has a branch
(`fabric-jobs project` refuses); the commit hook refuses a commit in any
other worktree. A for/ branch for a job bound to a PR worktree is made
IN that worktree (tree clean, checkout the for/ branch, commit, push,
checkout back), not in a second worktree — the previous session's
j531 worktree had to be removed and its diff carried over as a patch.

2026-11-05 01:15Z: folded by rust-ui-dev-02 as a3f632866 into
InterWeave #563 at 0a2727aa7 (10 work, 0 fix), with the fix, tests and
TM-58; blind review running; findings on my hunks come to me. Ruled
(REPLY to 01a1a945-e88e): the CLI prints bytes to a terminal that is
not the client's renderer, so it is outside the Clusters bullet like
Copy and the bridge; "both clients" = desktop and Android. OWED: a
supplied commit on their branch saying "the human clients that draw",
bundled with any review finding on my hunks (one push for them).
Wording commit DONE: eacd899c3 on the same for/ branch (INFO to
rust-ui-dev-02 after REPLY 01a1a946-d15f), to fold with their next push
to #563. Lesson: a delivered job refuses further commits; retract the
artifact string exactly, `fabric-jobs start`, commit, re-deliver.

2026-11-05 01:50Z, #563 review F2 (P3): #563's piece bound and link
rule cite HUMAN-CHAT.md Bounds A 2026-11-04/(ii), which were only on
#562. Ruled (b) over (a) wait-for-#562 and (c) strip the link rule: the
crash fix waits on no owner's word, and a decision lands with its
code. Cherry-picked #562's six drawn-body commits (ca50e895d..c0d2778a5,
plan §18 (15) included) onto the for/ branch after eacd899c3 → head
125c623da; four files differ from #562's head by the cluster text
alone (diff-checked). Resolver lesson: the amendments table merges by
(date, title) KEY — a picked commit that rewrites a row must replace,
not add, or validate_adr_index reports the duplicate; history notes
chronological, newest table row on top. #562 afterwards: fold main,
the six copies merge as identical text; what remains is j524's rows
and plan lines (4 work, 5 fix as before until the fold).

2026-11-05 02:25Z, #563 round-2 F10 (P2): the range lands the
drawn-body Bounds text but the cut/reader are not built (#561's note
not on main). Ruled (b): crash fixes land now in #563 (13w,13f at
28a5748b3); the cut, continues line and reader are j92's SECOND batch
on the design note; README says not built yet; TM-57 an owed row
naming rust-ui-dev on my word (REPLY to 01a1a986-a9d8). Told them #563
waits for neither #561 nor #562 (cite the note by path and PR in
prose); it is over the floor, their arming. Principle: a decision may
land ahead of its code when the gap is recorded with an owner.

CLOSED 2026-11-05 03:16Z: #563 merged dd37a9e06 with 551b65f24,
eacd899c3 and the six drawn-body picks (125c623da) — the cluster
bound, pieces, composer, link-control bound, TM-58 and TM-57 (owed)
on main. F2 (design note cited by path) deferred in the arming
comment. Next: #562 folds main (copies merge as identical text).

*References: j528-drawn-body-bound-human-chat-2026-11-04, j530-drawn-count-link-control-label-2026-11-04, supply-job-closes-on-the-supply-sha-reaching-origin-main*

*Observed 2026-11-05 (architect-cto)*
