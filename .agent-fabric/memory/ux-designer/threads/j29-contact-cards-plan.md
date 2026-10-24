---
role: "ux-designer"
class: threads
topic: "j29-contact-cards-plan"
description: "Contact-card UX spec (j29, REQUEST 01a15d2a from architect-cto-01) - written as design/2026-10-21-contact-cards.md, branch docs/contact-cards, PR after #333 merges"
tier: 2
knowledge_scope: full
distilled_at: "2026-10-24"
origin:
  - agent: "ux-designer-01"
    host: "develop-qzapp"
    project: interweave
    working_copy: interweave
derived_from:
  - a60d710ce5d07462
---

## Contact-card UX spec (j29, REQUEST 01a15d2a from architect-cto-01) - written as design/2026-10-21-contact-cards.md, branch docs/contact-cards, PR after #333 merges

j29 is the UX spec for ADR-0059 (contact cards) and ADR-0060 decision 6 (the recovery screen's confirmed Discard and recreate), on desktop and Android. Every string is named for language-culture-en, and human-client-ui.md §13 criteria are proposed. It is delivered as my own PR, with a REPLY to architect-cto-01 naming it.

Promised in REPLY 01a15d2d: the PR opens after #333 merges (one open PR rule; different urgency), rebased on #332 if that has merged.

Sources, all on #332's branch develop-qzapp/architect-cto-01/adr/contact-cards-encrypted-store until it merges:
- architecture/adr/0059-* and 0060-*.
- human-client-ui.md: §3 (proposal rules), §4 (title), §5 and §12 (vocabulary), §13.
- HUMAN-CHAT.md §Contact card:
  - name and surname are 1-64 scalar values after NFC;
  - the photo is at most 24,576 B decoded, PNG or JPEG, at most 256x256, and decoded only after Accept;
  - a card received over broadcast is ignored;
  - under a low max_payload_bytes the client offers to send without the photo.
- STATE.md: own_cards(card_id, label, name, surname?, photo?) and conversation_index.last_shared_card.
- Plan §29: steps 2-4 and the exit gate.

Tree facts:
- One shared Slint UI serves both clients (crates/human/ui-slint/ui/app.slint).
- Desktop store recovery is RefusalWindow (Close, Try again; UiText::RefusalStoreRecovery, #327).
- Android maps StoreNeedsRecovery to refused code -5 (apps/human-android/src/jni.rs). I found no Android store-recovery screen; ask rust-ui-dev-01 what Android shows.
- Slint 1.18.1 on Android has no accessibility tree, so TalkBack checks are blocked preconditions.

Design plan (drafted, not written yet):
- Entries: "Your cards" and "Contacts" in Chats' AppOptions menu.
- Card editor:
  - the label is optional and falls back to the name;
  - the photo comes from the Android Photo Picker or the desktop file dialog, and the preview shows exactly what is sent;
  - delete has Undo;
  - a note says a shared card stays with whoever got it, and edits reach them only on the next share.
- Share a card:
  - from the conversation menu, as a sheet (Android) or dialog (desktop), with the last shared card first and a preview of what the peer sees;
  - Share is the confirmation;
  - the sent card is an outbound row under RETENTION's pending rules (an untrusted peer gives a pending row, the F16 rule);
  - a hint shows only in an empty conversation: no dialog, and no stored dismissal.
- Proposal row:
  - a neutral placeholder plus "includes a photo";
  - "This peer says they are <name>", with the whole PeerId in monospace and the same-name note;
  - Accept and Decline, each with Undo;
  - a line saying an unanswered card goes when the app closes.
- Notification: never shows the claimed name.
- Display:
  - the title is the name, with an initials avatar;
  - when two contacts share a name, the short PeerId follows the name;
  - Details says where the name came from.
- Contact page: edit the name, remove the photo, routes listed with each one's trust, delete with a confirmation.
- Discard and recreate:
  - a third action on the recovery screen;
  - its confirm dialog lists exactly what goes and what stays, with Cancel focused and Back = Cancel;
  - a variant when human.sqlite.encrypted exists;
  - on Android, an invalidated key offers it directly;
  - after a discard, Chats opens with a notice.

Open questions for architect-cto:
1. When the person has edited a contact's name locally, what does Accepting a later card change?
2. Is route grouping in the GUI in v1, or CLI only?

Status 2026-10-21: written and committed (84c5a1f9) as design/2026-10-21-contact-cards.md on develop-qzapp/ux-designer-01/docs/contact-cards, off origin/main. #333 armed on architect-cto's AUTHORIZES (REPLY 01a15d3c). Next: when #333 merges, push, open the PR, dispatch the blind review, REPLY to architect-cto naming the PR (with questions A1-A5), REQUEST language-culture-en's read, and ask the owner O1 (whether to withhold the Accept swipe in the same-name case).
Old open question 1 is answered by ADR-0059 ("a local edit is never overwritten"); the spec asks only where that provenance is stored (A1). Old question 2 is now A4.

2026-10-21 later: PR #334 is open. Round 1's blind review (on 84c5a1f9) gave F1-F10. They were judged (9 real, F4 partly) and all fixed, ending at head 561dd871, with the answers posted as a PR comment. language-culture-en's for/ branch was folded three times. Its last head, 29d23fe9, has two recovery bodies: the desktop's keeps the newer-version sentence, because a newer store still reaches R through a failed migration. I asked them for nine new keys (REPLY 01a15d5e). Next: fold their keys, then re-review 84c5a1f9..head, then ask for the word to arm.

2026-10-21 ~08:05Z: #334 is armed at 478fd3c3 after 7 review rounds (F1-F19, all fixed); round 7 was clean. I told architect-cto (REPLY). Still open: O1 (owner, the Accept swipe in the same-name case), and the §13 criteria in §9.1 for architect-cto to rule on. Carried readability nit: §5.1's Card deleted does not point to §6.2's snackbar scope. When #334 merges, deliver j29.
Architect-cto (INFO 01a15d7f): the record text for A1, A3 and A6 is on #335 (head bd31b69a); that PR waits on the owner's word. They rule on the §9.1 criteria once #334 lands. ADR-0061 on #335 makes a conversation belong to one identity, and the card flows gain that in plan §30, not §29; the spec stays single-identity until then.
§9.1 was ruled 2026-10-21 (REPLY 01a15d98). All twelve criteria stand, and they are on #335 at 0a301f56 as human-client-ui.md §13:
- the name-display criterion is narrowed to "before Accept";
- three criteria are added: no Keep on a proposal, a PeerId never under two contacts, a card over broadcast ignored.
The spec already matches: a notification uses the conversation title, which is the short PeerId until Accept.

*Observed 2026-10-21 (ux-designer)*
