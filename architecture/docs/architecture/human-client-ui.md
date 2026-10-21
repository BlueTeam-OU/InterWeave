# Human client — shared UI and interaction design

Status: architecture/design only. This document defines first-party interaction semantics, not a transport protocol.

## 1. Information architecture

The desktop and Android clients expose the same conceptual areas with platform-appropriate navigation:

```text
Chats
Channels
Contacts / Routes
Network
Settings
  +-- Profile / PeerId
  +-- Trust
  +-- Endpoints
  +-- Connectivity infrastructure
  +-- Recovery / device security
  `-- Diagnostics
```

Desktop may use multi-pane navigation; Android uses mobile navigation and one-primary-task screens. The shared Rust UI model exposes the same state/actions without requiring identical layouts.

## 2. Onboarding

First-run onboarding is explicit about transport identity:

1. create a new profile identity, or enter the dedicated recovery flow;
2. show the resulting local PeerId as a machine/device identity, never a human account name;
3. configure or import trust/bootstrap/connectivity settings separately from identity recovery;
4. create/enable the local `human` EndpointId;
5. explain direct-vs-relayed reachability without exposing infrastructure peers as contacts;
6. optionally join channels and import contact routes.

A new identity is not silently created when an established profile fails to unlock. Recovery and configuration restore remain separate operations.

## 3. Contacts and routes

A contact is application metadata and may contain multiple device routes:

```text
Alice
  Desktop  -> PeerId A / human
  Phone    -> PeerId B / human
```

UI labels such as `Alice`, `phone`, `human`, avatars, and notes are **not authenticated by transport**. Trust indicators are tied to the exact PeerId. Adding a second route to an existing contact does not inherit trust from the first route.

**Where a name comes from (A 2026-10-21, ADR-0059).** The person types it, or the counterparty sends a contact card (HUMAN-CHAT.md §Contact card) and the person accepts it. A received card is shown in its conversation as a proposal — "this peer says they are <name>", the PeerId the trust indicators are tied to beside it, and a note when a contact of that name already exists — with Accept and Decline; nothing reaches `contacts` or `contact_routes` until Accept, which creates or updates the contact with the card's name and photo — `display_name` is the name, then a space and the surname when there is one; no separate column — and the route `{source_peer, source_endpoint}`, and a later card from the same route is a new proposal. A card on a route whose PeerId already has a route under a contact joins that contact rather than creating one, and the proposal says so before Accept — naming the contact it will update, and that the person's own edits stay — so one peer is never shown under two names; moving a route moves every route of its PeerId together, and the dialog says so (ADR-0059 decisions 3 and 4, A 2026-10-21). A proposal is a conversation item under §6's rules: unread until shown, and once shown and not accepted it goes when the process exits like any item the person did not keep, and it has no Keep — Accept is the only way a card outlives the session, and an unaccepted proposal cannot be kept as content (A 2026-10-21); the contact, once accepted, is not a conversation item and stays when the conversation empties and vanishes. A locally edited value is never overwritten by a card. The photo is decoded only after Accept and shown at a fixed size; before it, a neutral placeholder. Sending a card is an explicit action on one conversation ("Share a card"), which the client may offer when a conversation is first opened and never performs on its own; the person keeps several cards of their own (A 2026-10-21, ADR-0059 decision 2), each with name, optional surname and optional photo under a local label, edited in the client's settings and stored in the same store, and the share action asks which one, offering the card the conversation last shared first. The conversation's details show which card the counterparty last received; no card and no message discloses that other cards exist. The client declares no contacts permission and reads no platform address book (ADR-0059 decision 6). Contacts are per identity (A 2026-10-21, ADR-0061 decision 4): each identity's list is its own, the client shows no cross-identity list and suggests no match across identities; the one cross-identity act is the person copying a contact's routes to another identity, explicitly, which creates nothing in that identity's trust set.

When endpoint-directory lookup is used, returned EndpointIds are displayed as remote-advertised routing labels, not verified application roles. Unknown/offline/policy-denied remote routes remain intentionally indistinguishable at the transport error layer.

## 4. Conversations

A direct conversation is keyed locally by the identity it speaks as and the selected remote route, `{local PeerId, remote PeerId, EndpointId?}`, a channel conversation by the local identity and the ChannelId (A 2026-10-21, ADR-0061: a conversation never changes identity; whenever the client holds more than one identity, active or dormant, starting a conversation asks which one speaks, a dormant identity offered as "activate and speak as" under the active bound, every conversation shows which one it speaks as in its details and beside the composer, a conversation of a dormant identity shows it unreachable and refuses to send until activated, and a client holding one identity asks nothing). The identity settings state, in plain words, that two identities on one device share its addresses and can be linked by a counterparty who talks to both, by the network, by a shared relay and on the LAN, and that no setting today prevents it (ADR-0061 decision 5). Human-readable titles can be changed without altering transport addresses. A direct conversation's title carries the route label only when it is needed to tell conversations apart: while this client holds one conversation with a peer, the title is the short PeerId (or the contact's name, once contacts exist; two contacts with one name are told apart in every title by the short PeerId, never by a route label, which is peer-asserted and forgeable, A 2026-10-21) and the route is shown on the conversation's details as a peer-asserted label; the moment a second conversation with the same peer on another route exists, every title for that peer carries its route label, worded as a route and never as a role. ADR-0032's rule is about what the label IS when shown — peer-controlled routing, never identity — not about where it must appear (ruled 2026-10-19, #304 F6).

The first-party text payload uses `HumanChatV2`. Text is markdown rendered inside the closed subset of ADR-0050 — raw HTML shown literally, allowlisted link schemes, no automatic image fetch, raw source always viewable. Remote content cannot trigger trust/config changes, commands, link opening, downloads, recovery operations, or admin actions.

## 5. Message-status language

The UI must not claim stronger delivery semantics than transport provides.

Allowed states include:

```text
Sending
Accepted by remote transport
Failed / timed out
Received locally
```

Do not label `AcceptedV2` as `Read`, `Seen`, `Processed by person`, or even stronger application delivery unless a future application-level receipt protocol supplies that evidence.

For broadcast, current-session UI may show that publication was accepted by the local transport; it cannot claim every subscriber received the message. Durable pending content is removed on successful publication per ADR-0044.

## 6. Message retention interaction

The human UI implements [`clients/human/RETENTION.md`](../../clients/human/RETENTION.md):

- outbound pending messages survive restart until direct `AcceptedV2`, successful broadcast publication, explicit cancel/delete, or a future separately designed terminal policy;
- inbound unread messages survive restart;
- when an inbound message enters local read state, its durable unread copy is removed unless the receiver explicitly chooses **Keep**;
- **Keep** is available only after local read state and is always a receiver-local action;
- read-but-unkept inbound and transport-terminal outbound messages may remain visible in RAM for the current session, but they disappear across app/process restart;
- removing Keep deletes the durable kept copy;
- no remote payload, EndpointId, sender label, notification action, or application extension may force Keep.
- a read-but-unkept inbound message may be dropped from the session early by the receiver's explicit choice (**Dismiss**; its snackbar and announcement say it was dismissed, in language-culture's words): it writes nothing to the store; an undo offered for it returns it to the session, and once that undo closes it cannot be kept and never comes back; no content of it survives restart (its `read_pairs` entry, written at read, remains). The action is named Dismiss, never Hide, because Hide would promise a return the store cannot make; **Hide** is a conversation's action, which a new message in it brings back to the list (its hidden messages do not return) (the owner's decision, A 2026-10-16);
- an explicit cancel runs at once and deletes the durable pending record; an undo offered for it restores that record, not a new one — the same `app_message_id`, the same transport message id, the destination selector, the media type, the stored byte-identical payload and the attempt count, committed durable before any attempt (RETENTION.md §2, case 16) — so the undo is RETENTION.md §2's retry: a direct receiver that admitted an attempt in flight answers it from its dedup entry while that entry exists (ADR-0019: its TTL or its eviction, and never across the receiver's restart), after which it may present the message again, as for any retry; an undo is offered only while the session still holds the record, and the sender may not know whether the cancelled attempt reached the receiver (the facade reports `may_have_reached` on the cancelled row), which is why the restored send must be the same message and not a new one (A 2026-10-16).

The UI should distinguish `Unread` from `Kept`. Unread is temporary durability for user availability; Kept is an explicit post-read retention choice. Neither state is transmitted to the sender in v1.

## 7. Connectivity display

Ordinary users see normalized status only — five states, no peer
counts, no addresses:

```text
online, reachable directly
online, reachable through a relay
online, partial (some peers may not reach this client)
offline (the transport reports itself unavailable)
unknown (no status read yet, or the status connection is down — never shown as offline)
```

The states are normative; the words a person reads for each are
language-culture's: the states are the `Connectivity` enum in
`crates/human/client-api`, the words its table in
`crates/human/ui-model/src/labels.rs`, never quoted here (A 2026-10-09).

An established peer route may optionally show `direct` or `relayed`. A DCUtR `PeerPathChanged` updates the route indicator without creating a fake reconnect/new-message event.

Infrastructure PeerIds, AutoNAT probe details, relay multiaddrs, Kademlia buckets, and raw failure traces belong to advanced diagnostics/admin surfaces.

## 8. Trust/settings interaction

Trust mutation is always an explicit local settings action. A message may contain a PeerId as text, but clicking/copying it cannot auto-allowlist it.

Desktop settings use the admin IPC binding. Android settings use `LocalAdminPort`. Network/event callbacks never receive either authority.

High-impact changes require a confirmation view that shows the exact PeerId and scope. Removing trust should warn that active application connectivity to that peer will be evicted.

## 9. Recovery UX

### Desktop

Standard v1 keeps mnemonic backup/restore in the stopped-runtime `transportctl identity ...` workflow. The human UI may open documentation or guide the operator to the tool but does not obtain mnemonic words through daemon IPC.

### Android

There is no standalone CLI requirement. A dedicated local recovery flow stops/locks the TransportRuntime, invokes the identity component directly, and may display/import the 24 words on-device. It is not routed through `LocalDataSession` or `LocalAdminPort`. During the complete recovery flow the window that shows or takes phrase material has `FLAG_SECURE` set before its first word can draw and keeps it until a frame without the flow has drawn, and on API 33+ `setRecentsScreenshotEnabled(false)` for the same span, so Recents shows a blank card and no thumbnail, screenshot or recording carries a word; the flow runs in the app's own Activity (amended 2026-10-19: a dedicated non-exported Activity excluded from Recents was the first wording, and stays allowed; the protection is the window's flags from before the first word, not a second task), and no intent, extra or deep link from outside the app starts a phrase flow. The phrase must not enter screenshots/screen recording/task snapshots, clipboard, autofill, analytics, logs, saved-instance state or normal free-text IME input. Import uses the in-app BIP-39 word-list picker defined by the Android custody design; any temporary IME-assisted filtering requests no suggestions/autocorrect and `IME_FLAG_NO_PERSONALIZED_LEARNING`, but that request is not treated as a guarantee that an arbitrary IME behaves correctly. Exact platform behavior is validated in SPIKE-008/009 and release testing.

Both platforms state prominently what the phrase restores and what it does not: the identity only — contacts, trust, endpoint and connectivity settings and messages are not restored by it. Where the platform has an export of those settings (the desktop's authored `config.yaml` and trust files), the statement adds that they are backed up separately; where it has none (Android in standard v1: no export exists, system backup and device transfer exclude the profile, and profile/config migration is Stage 19's, plan §22), the statement names the fact and asks the person for nothing they cannot do (A 2026-10-19, ux-designer's Q5).

Both platforms, and `transportctl identity restore` on the desktop, warn before the phrase is taken that restoring an identity which may still be running on another device puts one PeerId on two nodes, which ADR-0043 forbids, and that this device cannot check it (ADR-0043, Operational implications: "Recovery UX warns when importing an identity expected to still be active elsewhere"). The warning is a statement in the flow, never a gate: nothing refuses, no flag is added, and on the command line it is a line printed before the prompt (A 2026-10-21: Stage 18's TM-48 row found no surface carried the sentence; `tests/SECURITY-MATRIX.md`).

## 10. Notifications

A notification names the identity it arrived on and carries nothing of another identity's contacts or content; the Android service's one persistent notification names the active identities and nothing else (A 2026-10-21, ADR-0061 decision 9).

Notification content is generated only from messages already accepted into the human application/session.

- notification previews are user-configurable;
- sensitive previews may be hidden on lock screens;
- a notification tap opens local conversation context only;
- notification action buttons, if added later, must not perform trust/admin/recovery operations;
- a notification never proves the sender is a human merely because the route endpoint is named `human`.

Desktop notification integration is optional per OS packaging. Android stay-reachable mode uses its foreground-service notification independently from per-message notifications.

## 11. Accessibility and localization

First-party UI must provide semantic labels/actions, keyboard navigation on desktop, screen-reader-friendly controls, scalable text, sufficient touch targets on Android, and localization-safe layouts. PeerIds and cryptographic identifiers must remain copyable in exact canonical form even when surrounding UI is localized.

## 12. Error presentation

Map stable transport/local errors to user-actionable messages without exposing hidden route-policy distinctions. Examples:

- `UnauthorizedPeer` -> peer is not trusted for this profile;
- `RemoteEndpointUnavailable`/wire `no_route` -> selected route is currently unavailable;
- `PeerUnreachable` -> no usable network path;
- `Overloaded` -> remote/local transport is temporarily busy;
- `EndpointInUse` -> this local endpoint is already owned by another client/session.

Raw internal codes remain available in diagnostics.

## 13. Shared UI acceptance tests

- no UI label promotes EndpointId/display name to authenticated identity;
- AcceptedV2 never renders as read/seen;
- DCUtR path change does not create a duplicate logical connection/conversation event;
- explicit trust mutation shows exact target PeerId;
- remote text cannot invoke admin/recovery handlers;
- desktop and Android render the same HumanChatV2 fixture consistently;
- pending outbound and unread inbound survive restart; read-unkept/transport-terminal messages do not;
- receiver Keep can be set only after read and cannot be forced by remote content;
- Dismiss writes nothing to the store, and no content of a dismissed row is in the store after restart; no message action is labelled Hide;
- an undone cancel restores the pending record's `app_message_id`, transport message id, destination selector, media type, stored payload bytes and attempt count unchanged (a new `row_id` is expected), never a re-serialised or re-identified copy (A 2026-10-16);
- accessibility tree contains meaningful labels/actions for message, route, trust, and connectivity controls.

Contacts, cards and the store's discard (A 2026-10-21, ADR-0059, ADR-0060; ruled on ux-designer-01's proposal, `design/2026-10-21-contact-cards.md` §9.1). These bind the two graphical clients; the command-line client (ADR-0059 decision 9) meets the same rules in its own form — a share names the card and the conversation as arguments, and a discard performs only with its confirmation flag:

- before Accept, a received card's name is never shown without the full PeerId beside it, and never in the conversation list, a notification or an announcement — a notification for a received card names the short PeerId, not the card;
- a card writes no contact or route row before Accept; Decline and an unanswered card write none; a proposal has no Keep;
- a received photo is not decoded before Accept;
- Accept's label and context line state what it will create or change before it runs; a same-name contact on another PeerId is named before acceptance; a card on a route of a PeerId already under a contact names that contact as the one it will update;
- a PeerId never ends under two contacts: Accept joins the PeerId's existing contact, and Move moves every route of the PeerId together, the dialog saying so;
- no card is sent except on Share in the chooser of one direct conversation; channels offer no Share; a card received over broadcast is ignored and shows nothing;
- nothing a counterparty receives reveals another card's existence or label;
- a locally edited name or photo is unchanged by any later Accept, and the contact page says where each came from;
- two contacts with one name are told apart in every title by the short PeerId — never by a route label, which is peer-asserted and forgeable (§4), and never by colour alone;
- every received name is rendered as isolated plain text;
- the sent photo carries no image metadata;
- Discard and recreate deletes nothing without a confirmation that lists what goes — only the unfinished plaintext when the complete copy is present and opens under the key, the copy and the key with it when it does not (ADR-0060 decision 6) — with its initial focus on Cancel, and identity and trust are unchanged after it;
- every action of this section has a non-gesture control and an accessibility-tree label.

## 14. Android availability-policy diagnostic

When Android is configured with both `availability_mode=stay-reachable` and `key_unlock_policy=user-presence`, the configuration is valid but the UI/status model must expose `background_restart_requires_user_authentication=true`. User-facing copy must explain that the endpoint can stay reachable while the currently unlocked service remains alive, but it cannot automatically recover reachability after process/service restart until the user authenticates.

## 15. HumanChat reply rendering

An inbound HumanChatV2 `reply_to` may reference an application message that is not present in the current retention/session store. The message is still valid and must render normally; the client may show a neutral placeholder (its wording language-culture's, in `labels.rs`) and preserve the referenced ID for bounded diagnostics/retention metadata. It must not auto-fetch, create transport traffic, reject the message, or infer tampering solely because the referenced message is absent locally.
