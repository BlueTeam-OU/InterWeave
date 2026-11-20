---
role: "ux-designer"
class: threads
topic: "j97-message-not-copied"
description: "j97 long-message wording — PR #581 merged 34bc3b7f 2026-11-09 (13 work, rounds 4-7); carried risks in PR comment 6754054475"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-17"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 741bff6d5e7dd8bb
  - bcf268e08fd6273c
---

## j97 long-message wording — PR #581 merged 34bc3b7f 2026-11-09 (13 work, rounds 4-7); carried risks in PR comment 6754054475

j97 (2026-11-06): PR BlueTeam-OU/InterWeave#581 updates design/2026-11-04-long-message.md.
- MessageNotCopied en = language-culture-en's final "Could not copy the message source. Choose Copy message source to try again." (OBSERVATION 01a1aeac). Architect-cto's placeholder "...Try again." was superseded because the snackbar has no Try again button. A retry line names the control that retries.
- The ReadWholeMessage control is "Open the whole message" (OBSERVATION 01a1aea8); "Read" is a message-status word.
- Key renamed ReadWholeMessage -> OpenWholeMessage by rust-ui-dev-02 (4177c66a3); the note follows in 9c895be73. ReaderSent keeps its key.
- labels.rs follows via rust-ui-dev-02 (REQUEST 01a1aea7 from language-culture).
Lesson: a placeholder handed over by the builder is not the final wording. Wait for language-culture's line before writing the en column ([[wording-pass-keeps-conditions]]).

Rounds 4-7 (j106, 2026-11-09): round 4 F1-F6 and round 6 F1-F4 were fixed, and round 7's re-review of 4b48021b came back clean. Armed on that head, with round 4's partial-chain findings deferred as fixed ([[review-chain-label-collision]]). Carried: what "line" a reader link's control follows; Tab cycling in the reader on a wide layout; no A4 case for a link label past the ceiling; the stale "not yet on main" at unlock note line 128; the old control words in tests/android-e2e (rust-ui-dev's).

*References: review-chain-label-collision, wording-pass-keeps-conditions*

*Observed 2026-11-09 (ux-designer)*
