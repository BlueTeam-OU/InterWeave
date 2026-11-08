---
role: "architect-cto"
class: rationale
topic: "j509-identities-note-rulings-2026-11-04"
description: "j509 (2026-11-04): the #552 §16 rulings Q1-Q8 on InterWeave PR #553 (4 work, 2 fix at 637efc254; boundary via ENDPOINTS.md; owner's word): Q7a/Q7b/Q8 ruled and ADR-0061 d.7/d.9 amended, ADR-0067 d.3 clarified; review F1-F4 answered…"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 061c4c309591eb7a
---

## j509 (2026-11-04): the #552 §16 rulings Q1-Q8 on InterWeave PR #553 (4 work, 2 fix at 637efc254; boundary via ENDPOINTS.md; owner's word): Q7a/Q7b/Q8 ruled and ADR-0061 d.7/d.9 amended, ADR-0067 d.3 clarified; review F1-F4 answered, re-review pending

ux-designer-01's QUESTION 01a1a69a-046a (j89, #552 design/2026-11-04-identities.md
§16) asked six things ADR-0067 left two-way. Ruled (REPLY 01a1a69f-87be, 2026-11-04):
Q1 §13 identity-settings bullet amended to "each identity trusts its own peers; a
peer any identity trusts can connect to the device and reaches only the identities
that trust it" PLUS a channel clause (broadcast admission is the node's, d. 6).
Q2 the no-reach notice reads the identity in front's EFFECTIVE set (overlay set,
else what the entry reads), never the admission set; second form when another
identity trusts someone. Q3 a counterparty's route is the EndpointId shown short
(as ShortPeer, last eight) wherever the text said route label; the §4 contacts tie
becomes the EndpointId's canonical string. Q4 alias grammar = the retired label
grammar ^[a-z][a-z0-9._-]{0,63}$, unique in the profile over authored+added,
refused at load/add/rename (AliasInvalid, AliasTaken); enforcement p2p's Stage 22
batch. Q5 an overlay trust row is the client's write, seed included: the trust
list's endpoint dimension reports it `administered`, no origin of the seed; an
endpoint without an overlay set answers its effective set. Q6 a desktop
notification's one freedesktop form is the redacted one (count, no name).

Q7 (2026-11-04, REPLY 01a1a6bf-ff3f): (a) a notification with no one identity
(lock screen's one entry, desktop's redacted form, group summary) switches to the
one identity with news, else opens the switcher naming identities with news and
switches nothing — ADR-0061 d.9 A; (b) the old-contact warning is Remove's
confirmation only, the add carries no line — ADR-0061 d.7 A. Q8 (REPLY
01a1a6bf-fd48): the no-reach form depends on the front identity's set and the
address list alone, never another identity's set (one-bit leak), from the second
identity on, Stage 22 scoped. Review F1-F4 of #553 (posted 5768835291, judged
5769138386): F1 scope, F2 ADR-0061 d.9 desktop one form, F3 ADR-0067 d.3
"refusals of the key and the id", F4 line-185 override list; the contact-cards
design's stale passages are ux-designer-01's on #552 (told in the Q7 REPLY).

STATE: #553 ARMED 2026-11-04 ~16:00Z at 2af1e81aa on the owner's word ('arm 553',
INFO 01a1a739-ba73), arm counted 6 work, 0 fix, boundary (ENDPOINTS.md,
CHANNEL-EVENT.md), review of the head posted, j510/j518 deferred. Q10 (j519)
lands AFTER the merge on a fresh branch: script in agent-fabric-tmp/j519/apply.py
(its 'ADR-0067 decision 6, A 2026-11-04' marker removed: only d.3 was amended that
day). 16:13Z the queue EVICTED #553: merge-group run 41862961434's tool self-tests
job CANCELLED mid test_check_spike_locks.sh (no test failed); re-armed at the same
head 16:30Z (j509 delivered on pr #553, j510 deferred), OBSERVATION 01a1a767-89b4
to devex-tooling (their j287; cause per REPLY 01a1a797: the self-tests JOB's own
15-min timeout after a 575 s pip stall — I had read the rust job's 35; fix cea11c604
on their #534). #553 MERGED bca4b96b4 ~16:50Z. j519: #559 (1 work, 0 fix)
at ff59c692d on branch docs/j519-q10-revoke-confirmation, INFO 01a1a77e-1599 to
ux-designer-01; review 5772385912 F1 (P3, glossary row listed 3 of 4 exclusions) judged real, fixed
c8e703c5d (row = 'the route's refusal only (§8)'), re-review 5772479549 clean; the
owner's 'arm 559' (INFO 01a1a780-d8d2, 17:05Z) named ff59c692 — ARMED at c8e703c5d 17:16Z on
the owner's second word. Review §2 gap recorded: revoke-closes-connection-vs-admission-set-stage-22.

*Observed 2026-11-04 (architect-cto)*
