---
role: "architect-cto"
class: rationale
topic: "owner-identity-model-endpointid-is-user-identity-2026-10-21"
description: "Owner's identity model (2026-10-21): PeerId = the node (device/app install), EndpointId = one user identity (persona) with its own display_name and photo; one app user holds many user identities on one phone. ADR-0061 as drafted on #335…"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "architect-cto-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - 4b18af1b83cf9ba9
---

## Owner's identity model (2026-10-21): PeerId = the node (device/app install), EndpointId = one user identity (persona) with its own display_name and photo; one app user holds many user identities on one phone. ADR-0061 as drafted on #335 (identity = whole profile) was wrong and #335 was disarmed.

The owner, 2026-10-21, after ADR-0061 (#335) had been armed on their
word: "one person facing network identity must match one user
identity"; then, after I asked, "An EndpointId should match a user
identity. One user of the app can have many user identities on the
same phone. So for any EndpointId he can add a display_name and a
photo."

**Why:** the owner's "multiple network identities" (2026-10-21,
earlier) meant personas under the phone's one node, not several
profiles/PeerIds each with its own daemon, key and store. I had read
"network identity" as a profile and wrote ADR-0061 decision 1 that
way ("an identity is a profile, and nothing less"); it was wrong
against the owner's model. #335 was disarmed (gh pr merge
--disable-auto, comment on the PR) before it landed.

**How to apply:** PeerId = cryptographic node identity (routing,
trust, authorship), one per profile/device as ADR-0016/0043 say.
EndpointId = a user identity: the persona's selector, carrying a
display_name and a photo (its card, ADR-0059) — which supersedes
ADR-0030/0032's "an EndpointId is never an identity" for the human
client's use of it, while it stays a routing selector on the wire and
grants no trust. A card is bound to (PeerId, EndpointId). Several
personas of one person share the PeerId, so a counterparty who holds
two of them can see they are one node: state the linkability, do not
hide it. Before writing an identity record again, restate the model
to the owner in their words and get a yes.

**Addendum, same day, after the redraw landed on #335 (bb47bc85):**
the owner: "two endpointid are two identities even if they belong to
one user. Avoid any ux that can let the user think that is operating
with one while operating instead with the other." Written as ADR-0061
decision 9 (829a3fd7): the client operates as one identity at a time,
names it on every acting surface and beside the send control, switches
only on one explicit act that changes everything shown, mixes no list
or chooser, opens a notification into its identity visibly; a contact
is held by the local identity that accepted it (contacts.local_endpoint_id).
Also agreed: EndpointId-per-identity is the default; a second profile
is the heavier separation for unlinkability against a counterparty.

**Landed:** IW #335 merged 87056f52 (2026-10-21 ~16:4xZ) after twenty
blind rounds (rounds 10-20 on the redraw alone, 36 findings, all
answered). The final shape: identity standing = the authored human
endpoint (first) or an endpoint in the overlay's `added` member; the
card is presentation, recreated from the label after a store discard;
Remove for added identities only, the first identity turned off once
another is enabled; a hand-authored endpoint is a route; the cap is
the schema's 64 (ADR-0026 amended: no default of 16 ever existed);
every identity's endpoint advertise off, the authored one through an
overlay `advertise` delta the client asserts before every lease
acquisition (ADR-0028 amended; three admin methods p2p's in Stage 22);
one identity at a time, no chooser, the identity named on acting
surfaces from the second identity on; a lock screen shows only the
redacted form. Lesson: a redraw of an identity record touches every
file that defines "identity" — ten files here — and each round found
the next sentence the model had not reached; grep the whole tree for
the old model's words before dispatching the first review.

**2026-10-22 ~06:20Z, owner's confirmation:** asked whether an
EndpointId is "conceptually the same kind of object" as a PeerId; I
answered no on the wire (PeerId = cryptographic identity and trust
subject; EndpointId = configured routing label authenticated only by its
PeerId), alike only one layer up through ADR-0061/0063's identity key,
with the linkability and the second-profile separation as consequences.
The owner: "keep as is". No record changes; the model stands as written.

*Observed 2026-10-22 (architect-cto)*
