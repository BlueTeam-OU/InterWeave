---
role: "architect-cto"
class: workflow
topic: "architect-cto-arms-gzapp-interweave"
description: "LIVE since 2026-10-19 12:35Z — architect-cto is below_min_authority and boundary_authority in InterWeave and gzapp (authority_own_prs false): how to give the word (AUTHORIZES line or arm with basis \"architect-cto's word\"), only after…"
tier: 1
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - c0567a001690eae5
---

## LIVE since 2026-10-19 12:35Z — architect-cto is below_min_authority and boundary_authority in InterWeave and gzapp (authority_own_prs false): how to give the word (AUTHORIZES line or arm with basis "architect-cto's word"), only after reading the head and its posted review; own PRs stay the owner's; an ADR's ACCEPTANCE is still the owner's

fabric-coordinator's INFO 01a152a7-3930 (2026-10-19): the owner ruled
that architect-cto arms and merges on gzapp and InterWeave, and that
agents ask architect-cto, not the owner. Today the arm tool lets this
role authorize a security-boundary change under boundary_min
(AUTHORIZES, see [[gateway-boundary-authority-architect-cto]]); a PR
under the work-commit minimum still arms only on the owner's word.

MERGED 2026-10-19 (python-dev-04's INFO 01a153ca): agent-fabric #305
(merge f6f3ee79) carries commits.below_min_authority,
commits.authority_own_prs and the arming session's role binding as the
word under the minimum; the per-repository settings (gzapp, InterWeave)
are devex-tooling's to set after #305's distribution; fabric-coordinator
amends ADR-019, the team prompt and the remits and says when it is
live. A relay of the owner's "python-dev-04 can add job to the current
pr" (2026-10-19) to python-dev-04 was answered: nothing of that job is
left; their #316 is another job and stays under the usual rule. NEVER this role's own PR: that stays the owner's.

**How to apply:** until the "live" message arrives, answer a
below-minimum arm request on gzapp or InterWeave with "the owner's word
is still the route; the mechanism is in flight" and the PR's counts;
after it, arm or authorize as above and say so in the basis. Until fabric-coordinator reports it live, a below-minimum arm still goes by the owner's word.

LIVE 2026-10-19 12:35Z (fabric-coordinator INFO 01a15428-b7ec):
BlueTeam-OU/agent-fabric-settings#9 (f88678e2) and gzapi-org's #8
(2a5ad80f) name architect-cto as below_min_authority AND
boundary_authority in InterWeave and gzapp, authority_own_prs false.
Procedure: a PR under 8 work commits, or a boundary change under the
boundary minimum, arms on my word — `AUTHORIZES: <owner/repo>#<n>@<head>`
in a DECISION or REPLY TO the arming session, or `fabric-pr arm <n>
--any-owner --basis "architect-cto's word ..."` from my session — ONLY
after I have read the head and its posted blind review; the word covers
that head only. My own PRs: still the owner's word. My rule on top: a PR
whose landing ACCEPTS a Proposed ADR (ratification by merge) still gets
the owner's word, since ADR acceptance is the owner's, not an arming
question. fabric-coordinator's two remit PRs (InterWeave #308, gzapp
#1134, under the minimum) will ask me for AUTHORIZES once reviewed.

First use 2026-10-19 12:55Z: fabric-coordinator's remit PRs gzapp#1134
(2 work, 1 fix) @bf05b7cf and InterWeave#308 (3 work, 0 fix) @8b4e42c2,
one DECISION each with the AUTHORIZES header field, after reading both
diffs (one identical paragraph) and both posted reviews (clean, no
threads). Mechanics: compose from the PR's own working copy so PROJECT/
REPOSITORY are right; insert the AUTHORIZES line in the header (before
SUBJECT); the gzapp counts come from the gzapp clone's fabric-pr gate.

Second use 2026-10-19 13:50Z: InterWeave #307 (7 work, 12 fix, 6
merge; validate_adr_index AMENDED phase + the decision-2 lockfile guard
+ my folded ADR-0048/0057 amendments), head cd8b81e2, ten blind reviews
over five rounds, clean, no threads -> AUTHORIZES given; my own commits
on another's PR are not "own PR" for authority_own_prs. Ruled too: test
fixtures (testFixtures* configurations) are test tooling under ADR-0057
decision 2, exempt like lint and test.

*References: gateway-boundary-authority-architect-cto*

*Observed 2026-10-19 (architect-cto)*
