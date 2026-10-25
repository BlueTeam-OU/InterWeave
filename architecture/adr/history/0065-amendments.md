# ADR-0065 — amendment history

### Amendment 2026-10-25 — The version items landed and the first tag was cut

**Trigger.** Implementation implications said "Until they land, the release
run's present rule (bare triple, hand-set `versionName`, plugin at `0.0.0`)
stands and the first tag waits for decision 5's single source." Every item
landed by 2026-10-24 (#394, #386, #398, #404, #402) and the owner cut
`v0.1.0` on 2026-10-25, so the sentence was false on `main`.

**Ruled.** Nothing of the decision changes. The sentence is past tense, names
the pull requests and the tag, and sends the run's outcome and the release
page's contents to plan §22's record, which this record never duplicated.

**Propagation.** Implementation implications; this note; the log row; the
digest entry's state line.

### Amendment 2026-10-25 — The Linux archive's minimum platform is public API: raising it is breaking, lowering it a patch

**Trigger.** The external review of `v0.1.0` (handed to p2p-network-dev
by the owner on 2026-10-25) found that the Linux archive states no
minimum platform while its `human-desktop` needs glibc 2.43, the glibc of
the `ubuntu-latest` image the release job built on: a host at 2.42
refuses it before `main`. Decision 2 listed what a previous version's
peer, client, profile or script relies on and said nothing of the host
the programs run on, so a floor raise read as "a dependency bump that
changes nothing relied on", PATCH, which is the opposite of what the
person installing it sees.

**Ruled.** The minimum platform the Linux archive states on its release
page joins decision 2's public API: a floor the release run builds the
archive on and checks every program in it against before publishing.
Raising it is MAJOR (MINOR before `1.0.0`), lowering it PATCH, and an
archive needing more than its page states is a defect corrected by the
PATCH that states the floor and builds on it, never by replacing a
published archive. The first floor and the version that first states it
are plan §22's record (architect-cto's REPLY 01a1736d-9ab5 of
2026-10-25 to p2p-network-dev's question 01a17366-34c8).

**Propagation.** Decision 2; Implementation implications (the floor's
three sites by lane); this note; the log row; the digest entry and its
keyword row; the README row; plan §22.
