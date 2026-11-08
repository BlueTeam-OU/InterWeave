---
role: "ux-designer"
class: threads
topic: "j89-stage22-identities"
description: "j89 Stage 22 identities design note (#552 merged 84e89348, design/2026-11-04-identities.md) — what it decided, what is still owed"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - e8cbf09e97498e9a
---

## j89 Stage 22 identities design note (#552 merged 84e89348, design/2026-11-04-identities.md) — what it decided, what is still owed

architect-cto-01 REQUEST 01a1a689 (2026-11-04): design for plan §30 that opens no stage.
#552 opened 2026-11-04 (2 work commits at d60e7bd5e): design/2026-11-04-identities.md, plus
pointer lines in contact-cards §5.1, §5.2, §5.7.2, §5.7.5, §6.3 (the note governs there since ADR-0067).
Decisions: Trust is an acting screen (ADR-0067 d.6 per-identity trust; replaces cards §5.7.2's
"trust is the device's"); no config origin on identity trust rows; Add = card only, alias made
from the name; lost add answer resolved by listing the derived id; one lock-screen entry, one
notification per identity in one group; route menu (⋮) holds Move + Remove from this contact.
Sent: REPLY 01a1a691 (taken), REPLY 01a1a69a (PR), QUESTION 01a1a69a-046a (Q1-Q6, §16).
Rulings Q1-Q6: architect REPLY 01a1a69f, landing as PR #553 (not the docs/ branch first named);
written into the note 29b060e34. Round 1 (d60e7bd5): F1-F11 all judged real, answered 0421e51fc,
PR comment 6631343044; Q7 (lock entry with several identities switches none; before-add line)
QUESTION 01a1a6a8. language-culture-en REQUEST 01a1a6a9 (their j242, branch
develop-qzapp/language-culture-en/for/ux-designer-01/j242-identities-words). Rounds 2-3 answered
(787774b31, 4df228e12); round 4 clean on 4df228e1. Q7 (a: switcher when several have news; b: Remove
dialog alone) and Q8 (no-reach form by front set + address list only) ruled on #553, written 06fe47d71.
j242 delivered 5a19fee36 (REPLY 01a1a71c), fast-forward folded; its 6 questions answered in b1ea0f615
(REPLY 01a1a71f-42d5); Q9 (one-identity TrustExplanation/ConfirmRevoke vs device list) QUESTION 01a1a71f-2302
to architect-cto, sent queued (no session). Rounds 5-8 posted (F17-F24 answered); Q9 ruled (REPLY 01a1a727); Q10 (revoke strings vs channel posts) QUESTION 01a1a736 open. Q10 ruled (01a1a738), words 01a1a73b, written cd2a7b09; rounds 9-10 posted; ARMED on owner word at cd2a7b09 with F26 (P3, §16 closing para credits ConfirmRevoke to 01a1a730 not 01a1a73b) deferred. Owed after merge: follow-up for F26, tell architect-cto the merge commit.
Arm: 4 work commits, needs architect-cto AUTHORIZES #552@<head> or owner's word.
Owed: post round 5, Q9 ruling; arm needs
the owner's word or architect-cto's (under 8 work commits).
Left out on purpose: identity-key.md's revision for ADR-0067 d.5 (new identity, not new key).
Related: [[j88-identity-keyboard-answers]], [[j77-start-window-u]].

DELIVERED 2026-11-04: #552 merged 84e89348 (10 work, 7 fix, 10 rounds); REPLY 01a1a770 to architect-cto.
Owed: F26 (P3, deferred at arm): §16 closing paragraph credits ConfirmRevoke to REPLY 01a1a730-ee4c;
it should say replaced under Q10 (REPLY 01a1a73b). Written on #561 as 688ed9104 (2026-11-04).

*References: j77-start-window-u, j88-identity-keyboard-answers*

*Observed 2026-11-04 (ux-designer)*
