---
role: "language-culture"
class: solution
topic: "per-identity-trust-strings"
description: "InterWeave copy after ADR-0067 (trust per identity): every trust string has an As form naming the identity in front; wording rules for it"
tier: 2
knowledge_scope: full
distilled_at: "2026-11-08"
origin:
  - agent: "language-culture-en"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 5a86dff37ba53b81
---

## InterWeave copy after ADR-0067 (trust per identity): every trust string has an As form naming the identity in front; wording rules for it

Since ADR-0067 d. 6 trust is per identity. On #552 (design/2026-11-04-identities.md §13, j242, ab1ca5de9, 2026-11-04), every labels.rs trust string that said "you", "this profile" or a bare passive got an `...As` form for two or more identities. There were 27 of them: ConfirmAllow, ConfirmRevoke, TrustStep*, Connect*, PeersNoTrust, ContactDeleteConfirm, MoveRouteConfirm, the route and Peers rows.
- Never "Trust {peer} as {identity}": it can be read as "trust the peer to be that identity". Buttons are "Trust as {identity}" (accessible name "Trust as {identity}: {peer}", the visible label first). Dialogs and titles are "Do you want {identity} to trust {peer}?", never an inverted "Should".
- An As form drops any claim that depends on another identity's trust: "connects only after you trust", "connections will close now". Saying it would disclose another identity's peers (§4 of that note).
- The one-identity texts stay unchanged. With one identity held, the identity is never named.
- A route is "conversation" to the person: "Conversation removed from {contact}", like ConfirmRouteMoved.
Related: [[solution-carried-2026-10-10]], [[interweave-copy-controls-snackbars]]

*References: interweave-copy-controls-snackbars, solution-carried-2026-10-10*

*Anchors: `design/2026-11-04-identities.md::ConfirmAllowAs`, `crates/human/ui-model/src/labels.rs::ConfirmAllow`*

*Observed 2026-11-04 (language-culture)*
