---
role: "ux-designer"
class: threads
topic: "j97-message-not-copied"
description: "j97 long-message wording — PR #581 (MessageNotCopied final retry line, \"Open the whole message\"); arming under 8 commits needs the owner's word"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 741bff6d5e7dd8bb
---

## j97 long-message wording — PR #581 (MessageNotCopied final retry line, "Open the whole message"); arming under 8 commits needs the owner's word

j97 (2026-11-06): PR BlueTeam-OU/InterWeave#581 updates design/2026-11-04-long-message.md.
- MessageNotCopied en = language-culture-en's final "Could not copy the message source. Choose Copy message source to try again." (OBSERVATION 01a1aeac). Architect-cto's placeholder "...Try again." was superseded because the snackbar has no Try again button. A retry line names the control that retries.
- The ReadWholeMessage control is "Open the whole message" (OBSERVATION 01a1aea8); "Read" is a message-status word.
- Key renamed ReadWholeMessage -> OpenWholeMessage by rust-ui-dev-02 (4177c66a3); the note follows in 9c895be73. ReaderSent keeps its key.
- labels.rs follows via rust-ui-dev-02 (REQUEST 01a1aea7 from language-culture).
Lesson: a placeholder handed over by the builder is not the final wording. Wait for language-culture's line before writing the en column ([[wording-pass-keeps-conditions]]).

*References: wording-pass-keeps-conditions*

*Observed 2026-11-06 (ux-designer)*
