# Several identities in one client: the identity in front, Add, Remove, trust, notifications and a contact's routes — desktop and Android

Status: proposal, 2026-11-04. **This note opened no stage.** Stage 22
(plan §30, several user identities in one client) was roadmap only when
this note was written, and only the owner's word could open it; the
design was the one part of the stage that could go first. The owner opened the
stage on 2026-11-19 (plan §30's opening record), so the stage's steps
now build what follows.

Asked by architect-cto (REQUEST 01a1a689, job j89), read against
`origin/main` b563dd00c, for four things: how every acting screen names
the identity it acts as (§5); Add an identity, Remove one, advertise off
and the lock-screen notification under several identities (§6–§9); the
contact page's route-delete control beside Move (§10); and what the
keyboard reaches on each new screen (§11). It is written against plan
§30's steps and `human-client-ui.md` §3, §4, §10 and §13 as they stand.
Where a step's wording leaves a choice open, this note decides it and
names the alternative. The plan, `human-client-ui.md` §13 and the ADRs are architect-cto's;
what the design needs changed in them is asked in §16, not changed
here.

**What this note replaces.** `design/2026-10-21-contact-cards.md`
designed the identity flows (its §4, §5.1, §5.2, §5.4.1, §5.7, §6.3,
§6.4) before ADR-0067 (A 2026-10-28) made an EndpointId the name of the
endpoint's own key and made trust per identity. Where this note and
those sections differ, this note governs, and each of them (§4, §5.1,
§5.2, §5.4.1, §5.7.1, §5.7.2, §5.7.3, §5.7.5, §5.7.6, §6.3, §6.4)
carries a line pointing here. Wherever those sections say "route
label" of one of the person's own identities, read §6.1's rule: the
card's name, else the alias, as the name, and `ShortEndpoint` where a
second line shows the route. And wherever they say trust is the
device's — cards §5.1's shared-device statement, §5.7.2's Trust row,
`IdentitiesShareDevice`'s last sentence and `TrustAllIdentities` —
read §5.1 and §6.1: since ADR-0067 d. 6 trust is the identity's (and
#553 has `human-client-ui.md` name these four as overridden until
contact-cards is revised: a dependency until #553 is on `main`). What this note does not mention there
stands as written: the editor's photo step and unsaved-changes dialog
(cards §5.2), the switch and its switcher (cards §5.7.1) but for that
rule, what changes the identity in front (cards §5.7.3) but for §5.3
below, the new-conversation rule (cards §5.7.4) and Make default and
Turn off (cards §5.2). A rule is stated in one place;
other sections point to it.

Owner of the design: ux-designer. The words are language-culture's:
§13 names each string, where it shows and what it must say, and its
text is language-culture-en's (§16); a key §13 marks as proposed waits
for their words. Colours are roles of
`design/2026-10-16-android-look.md` §3. Both clients render the same
Slint UI (`crates/human/ui-slint`) from the same `ui-model`; "narrow"
and "wide" are the layouts `app.slint` already has (one pane, two
panes). Built by rust-ui-dev, in steps 1–3 of plan §30.

## 1. What is already decided

The records this note answers to, so a reviewer can check it against
them:

- **ADR-0061**: an identity is an endpoint of the node with its card
  (d. 1); it is created through the administrative boundary (d. 2); a
  conversation speaks as exactly one identity for life (d. 3); a contact
  is one remote identity held by the local identity that accepted it
  (d. 4); what two identities of one node share is said (d. 5); Remove,
  Turn off and every command-line form (d. 7); the client operates as
  one identity at a time and names it on every acting screen, and the
  lock screen's redacted form names none (d. 9).
- **ADR-0067**: the EndpointId is derived from the endpoint's key and
  cannot be chosen (d. 2); what a person types to pick an endpoint is a
  **local alias** that never reaches the wire (d. 3); the add takes a
  key and returns the derived id (d. 4); a different key is another
  identity, and the key-change proposal is withdrawn (d. 5); **trust is
  per identity**: each identity's trusted set is its own, an added
  identity's starts empty, the first identity's is seeded once from what
  the device trusted (d. 6); broadcast admission stays the node's
  (d. 6).
- **Plan §30**: steps 1–3 and the exit gate; the route-delete control
  "beside Move" is ux-designer's to place (step 2).
- **`human-client-ui.md`** §3 (contacts, one identity at a time), §4
  (conversations, the shared-node statement), §10 (notifications) and
  §13 (the Stage 22 criteria, bullets 7 and 13 to 19 and 36 by that
  section's own count).

**Dependencies, not facts.** None of these is on `main`, and every
acceptance case in §14 that needs one says so:

- **D1** `admin.endpoints.add`, `admin.endpoints.remove` and
  `admin.endpoints.set_advertise` (p2p-network-dev, plan §30 Activate;
  `LOCAL-IPC.md` lists only `list`, `revoke`, `set_enabled` and
  `set_default` today), and the embedded host's configuration path for
  the same three on Android. §6.2 relies on one behaviour of the add
  that D1 must name: an add whose key derives an EndpointId the profile
  already holds answers that the id exists, distinguishable from a
  refusal, and creates nothing new. The key's cleanup (§6.2) relies on
  one more answer of the add: an add whose key file is absent when the
  daemon applies it is refused (`key_missing`) and creates nothing, so
  an add the client stopped waiting for cannot create an endpoint after
  the client deleted its key on a list that answered; and a refusal
  means no add of that key will commit. Neither the add nor the remove
  promises more about an answer that was lost: a write the client
  stopped waiting for may still commit after a later list has answered
  (`LOCAL-IPC.md`, A 2026-10-23; ADR-0061 decision 7, A 2026-11-24).
  §7's Remove is written for that: an id read back present after a lost
  answer is a Remove not known to have finished, and what the person is
  told stays true if the identity goes a moment later. Of the remove,
  §7 relies on three answers: a remove of an id the device does not
  hold answers `EndpointUnknown`, told apart from a refusal; a remove
  that leaves the configuration ahead of the running device (ADR-0028)
  answers `Internal`, as a write that failed does; and the remove takes
  the identity's trusted set (its `inbound` and `outbound` members,
  ADR-0028) out of the endpoint overlay with the endpoint.
- **D2** the endpoint dimension of `admin.trust.set` and
  `admin.trust.list` (p2p-network-dev's IPC minor, contract text
  architect-cto's; ADR-0067 d. 6).
- **D3** the alias's grammar and uniqueness rule. `ENDPOINTS.md` on
  `main` names the alias and gives it no grammar. architect-cto ruled it
  (§16 Q4): the retired label grammar `^[a-z][a-z0-9._-]{0,63}$`,
  unique within the profile over authored and added entries, refused by
  name (`AliasInvalid`, `AliasTaken`); the text is on their PR
  BlueTeam-OU/InterWeave#553, and enforcement is p2p-network-dev's
  Stage 22 batch.

**architect-cto's rulings of 2026-11-04** (REPLY 01a1a69f, answering
§16) are cited below as "ruled"; they land as amendments dated A
2026-11-04 in ADR-0061 (decisions 7 and 9), ADR-0067 (decision 3),
`human-client-ui.md` and `ENDPOINTS.md` on PR #553.
Until it is on `main` they are a dependency of the
same kind as D1–D3, and the cases that rest on them say so.

## 2. What the owner asked, and how it is read

The owner, as ADR-0061 and ADR-0067 quote them:

> One user of the app can have many user identities on the same phone.
> So for any EndpointId he can add a display_name and a photo

> two endpointid are two identities even if they belong to one user.
> Avoid any ux that can let the user think that is operating with one
> while operating instead with the other

> trust is per identity (endpointid)

Read here as in contact-cards §1, with one addition from the third
quote: trusting a peer is something the person does **as** an identity,
so the Trust screens are acting screens like Send and Accept, and they
name the identity in front (§5.1). Until ADR-0067, contact-cards §5.7.2
said the opposite (trust was the device's); that row is replaced.

## 3. The person's goals

- "Be the right person to the right people, and never mix them up"
  (contact-cards §2). At every moment they can act, they know which
  identity acts.
- "Start a second identity for work." They think of a name and a face,
  never of a key or an id. They need to know that the new identity
  starts with nobody: it trusts no one and no one has its card.
- "Stop being this identity." They need to see everything that goes
  before it goes, that it cannot be undone, and what the people who knew
  that identity keep.
- "I trust Anna as my work self, not as my private self." Trusting is
  now per identity; a person who learnt the old rule must not trust in
  the wrong one.
- "Nobody who picks up my locked phone learns who I am in this app."
- "This contact has a route I no longer want under it." They want it
  out of the contact without losing the contact or the conversation.

## 4. Rules every screen here keeps

Contact-cards §3 stands, except its "one identity in front" item, which
§5 restates for ADR-0067. In addition:

- **With one identity held, nothing of this note's identity naming is
  shown** (ADR-0061 d. 3): no switch, no "as" line, no identity subtitle,
  no trust-scope line. The first identity is shown only where asked for
  (its card, Settings).
- **The identity is named in words** on every acting screen, never by a
  face or a colour alone (WCAG 1.4.1).
- **No screen in front of one identity shows another identity's
  conversations, contacts, card or trusted peers**, and no count, list,
  suggestion or state is computed from them, with two exceptions: the
  switch's badge, which counts and names nothing (contact-cards
  §5.7.1), and a channel's posts, which the device admits from every
  peer it trusts whichever identity reads the channel (ADR-0067 d. 6,
  broadcast stays node-wide; §5.1). Neither names another identity or
  its peers.
- **An EndpointId is never typed and never chosen** (ADR-0067 d. 2). The
  person meets it only as the short form §6.1 shows.

## 5. The identity in front on every acting screen

With two or more identities held, each screen that can send, share,
accept, trust, create or delete names the identity in front. This table
replaces contact-cards §5.7.2 and is the one list; a screen added later
gets a row here.

| # | screen | acts | how it names the identity, both platforms |
|---|---|---|---|
| N1 | Chats | open, start, delete a conversation | the switch in the app bar (cards §5.7.1) |
| N2 | a conversation | send, share, accept or decline a card, delete messages | `ComposerAs` above the send control with the 24 dp avatar; the send control's name `SendAs`; with no composer, the line where the composer would be |
| N3 | the share sheet | share | `ShareCardAs` (cards §5.3) |
| N4 | a card proposal | accept, decline | `ComposerAs` of its conversation (N2) is on screen; Accept's context (cards §5.4.1) ends with `ProposalAs` ("If you accept, the card is saved in the contacts of {identity}."), a sentence of its own, in every case but S, where Accept is not shown (Close stands in its place, cards §5.4.1) |
| N5 | new conversation | start | `NewConversationAs` at its top (cards §5.7.4) |
| N6 | Contacts, a contact | delete, move, remove a route, rename, remove a photo | the identity as the app bar's subtitle |
| N7 | **Trust** | trust, untrust | the identity as the app bar's subtitle, and `TrustScope` under the title (§5.1) |
| N8 | **Peers** | trust (a row's Trust), add, remove an address | the identity as the subtitle; a row's trust state is the identity's (§5.1) |
| N9 | **add-a-peer's trust step** (`design/2026-10-22-add-a-peer.md` §5.3) | trust | the Trust button's label is `TrustStepAs` ("Trust as {identity}") |
| N10 | an identity's card | save, make default, turn off, remove | that identity as the subtitle; `IdentityNotInFront` when it is not in front (cards §5.7.2) |
| N11 | Your identities | add | no identity in front acts: the list names each identity on its row, and the check marks the one in front |
| N12 | Details | add as a contact, open contact | `DetailsIdentity` (cards §6.4) |
| N13 | discard and recreate | delete stored data | not one identity: the confirmation says it is every identity's data (cards §7.2) |
| N14 | recovery, show my phrase | — | not an identity: the device's (ADR-0061 d. 6) |

Peers' Add and Remove of an address act on the device's address list,
which every identity shares (ADR-0062: one overlay for the node). The
subtitle still shows, since the same screen trusts as the identity
(N8), and `PeersShared` says under the Add button that addresses are
the device's, for every identity. **Alternative named:** move Trust off
the Peers screen so that Peers names no identity. Rejected: add-a-peer
§3 makes trust "a second act on the same screen", which a person needs
right after adding an address, and taking it away would add a trip to
Trust for every new peer.

### 5.1 Trust is the identity's

ADR-0067 d. 6: what the person trusts, they trust as the identity in
front, and only that identity admits the peer at the route. So:

- **Trust** (the page): its list is the trusted set of the identity
  in front (D2), its subtitle names the identity, and `TrustScope` under the
  title says, with two or more identities held: "The peers listed here
  can exchange messages with {identity}. Each of your other identities
  trusts its own peers." With
  one identity, `TrustScope` is not shown (its second sentence would be
  false), and from the Stage 22 release `TrustExplanation` loses "Only"
  and its claim about this profile and says what the list is: the
  peers this identity trusts, who can write to it (ruled, §16 Q9: the
  first identity's set and the device's own list differ from the
  seed's first write on, and a peer only the device's list holds can
  post in a channel). Revoking, with one identity or several, states
  only what the act does at this identity's route: the peer's messages
  to this identity are refused until it is trusted again
  (`ConfirmRevoke`, `ConfirmRevokeAs`; ruled, §16 Q10). Nothing about
  channels, connections, the device's own list or another identity's
  set: a peer the device's own list or another identity's set still
  holds keeps its connection and can still post in a channel.
- **No "From this profile's configuration" line on an identity's
  rows.** Today each row says where it came from (`TrustFromConfiguration`
  or `TrustAddedHere`). After the change the first identity's set is
  seeded once from what the device trusted, and from then on the
  configuration's list no longer decides it (ADR-0067 d. 6), so
  "from the configuration" would tell the person that a change in the
  file would reach this identity, which it does not. Every row of an
  identity's set is the identity's own and can be untrusted here.
  **Alternative named:** keep the line on rows the seed copied. Rejected
  for that reason. Ruled (§16 Q5): the list reports every row of an
  identity's set, the seed's included, as the client's own write, with
  no configuration origin. So, from the Stage 22 release, Trust's rows
  carry no origin line: `TrustFromConfiguration` is retired, and
  `TrustAddedHere` stays only on Peers' address rows (add-a-peer §5.2),
  where it marks where an address came from.
- **Peers rows** (add-a-peer §5.2): the trust state is that of the identity
  in front. `PeerStateTrusted` stays with one identity ("this device
  tries to connect to this address": true, since the device dials every
  peer in its admission set, the union of every identity's set and
  the device's own list; ADR-0067 d. 6), and with two or more reads
  `PeerStateTrustedAs`, the same claim after "Trusted by {identity}". `PeerStateNeedsTrust` today adds "this device does not
  connect to this address", which is false when another identity trusts
  that peer and would tell the person so. With two or more identities
  held, it reads `PeerStateNeedsTrustAs` ("Not trusted by {identity}")
  with no claim about connecting. **Alternative named:** say "another
  of your identities trusts this peer". Rejected: it shows another
  identity's trust, which §4 forbids.
- **The contact page's route rows** show `ContactRouteTrusted` or
  `ContactRouteNotTrusted` from the set of the identity in front (cards §6.3).
- **The trust step after adding an address** (N9) trusts as the
  identity in front, and its button's label says so (`TrustStepAs`).
- **Channels**: a channel's publishers are admitted by the device, for
  every identity (ADR-0067 d. 6, broadcast stays node-wide). With two
  or more identities held, a channel conversation's Details carry
  `ChannelTrustNode` ("A peer can post in this channel if any of your
  identities trusts it, or if it is in this device's own list of trusted
  peers."), which names no identity and no peer. The
  device's own list is the operator's (ADR-0067 d. 6), written by
  `transportctl trust allow` and never by an identity's trust act.
- **The no-reach notice** (add-a-peer §5.6), shown when the device knows
  no address and trusts no peer: in front of an identity that trusts no
  peer, that identity can write to no one even when another identity
  trusts many. This note reads "trusts no peer" as **the set of the
  identity in front is empty** (ruled, §16 Q2: the effective set of the
  identity in front, never the device's admission set). The second form,
  `NoReachIdentity` ("{identity} does not trust any peer yet. To
  exchange messages as {identity}, trust a peer."), its action opening
  Trust, shows from the second identity on when that set is empty and
  the address list is not; with the address list empty too, the notice
  is add-a-peer §5.6's. Ruled (§16 Q8): the form is decided by the set
  of the identity in front and the address list alone, never by what
  another identity trusts. This per-identity reading is the Stage 22
  release's; before it, the one identity's set is the node's (ADR-0062
  d. 8). The notice promises nothing
  about channels, whose publishers the device admits (above). With one
  identity held, `NoReachIdentity` is never shown, and the notice shows
  as add-a-peer §5.6 says, unchanged.

### 5.2 An identity that cannot act now

Contact-cards §5.7.6 stands, with these states added. Each is shown on
the identity's row in Your identities and in the switcher, and in place
of `ComposerAs` in its conversations, the send control disabled with
the same text as its reason:

- **Starting** (`IdentityStarting`): the client is asserting the
  identity's advertise setting before it takes the lease (§8), or is
  taking the lease. No send, no Share, no Accept, no trust act until it
  ends; drafts can be typed. There is no time-out of the design's own:
  while the call blocks, the identity stays Starting (exit gate:
  "with it made to block, no lease is granted until it returns").
- **Could not start** (`IdentityNotHidden`): the advertise assertion
  failed, so the client took no lease. "{identity} is not connected.
  This device could not make sure that {identity} stays unlisted." With **Try again**
  (`TryAgain`) on the identity's row and, in a conversation, at the end
  of that line, which stands in place of `ComposerAs`; the composer's
  field stays, drafts can be typed, and Send stays disabled. Try
  again runs the assertion and then the lease again; through that
  retry's Starting, Try again stays in its place, disabled, with a
  progress indicator in it, and the line reads `IdentityStarting`. With one identity
  held the same line names no identity (`IdentityNotHiddenOne`: "You
  are not connected. This device could not make sure that you stay
  unlisted.").
- **Off** and **no lease** as cards §5.7.6 says.

Entering either state while the send control has focus moves focus to
the composer's field, which stays (§11 F21); focus on the field stays
there.

### 5.3 A switch and the identity's pages

Cards §5.7.3 lists what a switch closes on a wide layout. Since Trust
and Peers are now the identity's (N7, N8), they close with the
identity's other pages: a switch on a wide layout closes Trust and
Peers like a conversation or a contact page, and the pane shows the new
identity's empty-pane placeholder. Your identities is the one page that
belongs to no identity and stays, its check mark moving. On a narrow
layout the switch is reached only from Chats (cards §5.7.3), so neither
page is open at a switch.

## 6. Your identities and Add an identity

### 6.1 The list

Cards §5.1 stands, with these changes:

- **The row's second line** is the identity's short EndpointId,
  `ShortEndpoint` built as `ShortPeer` is ("…" and its last eight
  characters), labelled `IdentityIdLabel` in the accessible name ("Shown
  to the people you write to as …x6gY6Eri"). The route label of the old design no longer
  exists; the alias is local and is not shown on this line (§6.2).
  Wherever an identity is named, the switch and the switcher's rows
  included, its name is the card's name, else its alias (ADR-0061
  d. 1, "named by its label"); the switcher's rows show
  `ShortEndpoint` as their second line, and Details' `DetailsIdentity`
  (cards §6.4) shows it beside the avatar.
  Two identities of one device then differ on this line by their keys,
  never by a word the person chose. Ruled (§16 Q3): from the Stage 22
  release a counterparty's client shows a route as this same short
  EndpointId, in `DirectTitle` and on Details, so this line is what
  the people the identity writes to see.
- **The shared-device statement** `IdentitiesShareDevice` keeps its
  first half (one PeerId and one device's addresses; a second profile on
  the desktop, which the phone cannot run). Its second half changes for
  ADR-0067: "Each identity trusts its own peers. A peer that any of your
  identities trusts can connect to this device. That peer reaches only
  the identities that trust it. In a channel, a peer can post if any of
  your identities trusts it or if it is in this device's own list of
  trusted peers, whichever of your identities reads the channel."
  Ruled (§16 Q1): `human-client-ui.md` §13's criterion is amended to this
  sentence, its channel clause included (ADR-0067 d. 6, broadcast
  stays node-wide); its channel clause names the device's own list
  too, as `ChannelTrustNode` does (§5.1).
- **Add an identity** opens the add form (§6.2), not an empty card page
  with a route-label field.

### 6.2 Add an identity

The person writes a card; the client makes everything else.

**The form** is the identity's card page (cards §5.2) for a card that
does not exist yet: title `AddIdentityTitle` ("New identity"), Photo,
Name (required), Family name, then `IdentityNewNote`: "A new identity
has no contacts and trusts no peer. No one has its card until you share
it." The form says nothing of removed identities: ruled
(§16 Q7 b), what counterparties keep of a removed identity is said in
Remove's dialog alone (§7), since a new identity never takes a removed
one's route. Then **Create** (`CreateIdentity`), filled. No id field, no
label field, no advertise control (§8).

**The alias.** The client makes the alias from the name, under D3's
grammar: the name lowercased; its ASCII letters and digits kept, with
no transliteration; every other run of characters made one `-`, and a
`-` at either end dropped. A result that is empty is `identity`; one
that begins with a digit is prefixed `identity-`. It is cut so that a
suffix still fits the grammar's 64 characters, and when the profile
already holds that alias, `-2`, `-3` … is added, the lowest free one.
So "Anna Rossi" gives `anna-rossi`, a second "Anna Rossi"
`anna-rossi-2`, "Ελένη" `identity` (or `identity-2` when that is
taken), and "2024 team" `identity-2024-team`. It is shown on the identity's page under the card's fields,
read-only, as `IdentityAlias` ("Name to use on the command line: {alias}") with Copy,
so a person who uses the command line can find it. Editing the card's
name later does not change it, since a script may use it; the command
line renames it (ADR-0061 d. 7), and a rename it refuses is its own
(`AliasInvalid`, `AliasTaken`). **Alternative named:** an alias field
in the form. Rejected: most people will never use the command line, an
extra required field is a needless choice (recognition over recall),
and a field whose rules come from a grammar would put errors in front
of a person who only wanted a name. The rule rests on D3 as ruled.

`human-client-ui.md` §13 bullet 18's "a new label or one freed by
Remove … named by its label" reads here as the alias: the client makes
a new one, or one Remove freed (Remove deletes the alias with the
endpoint, and the rule takes the lowest free one), and an identity with
no card is named by it.

**Create**, on a valid form (cards §5.2's input rules):

1. Create shows a progress indicator in itself and the form stops
   taking input; Back and Escape do nothing while it runs, for at most
   the add's wait (below), since what it starts cannot be stopped
   halfway.
2. The client generates the identity's key, seals it, and calls the add
   with its public key (D1; ADR-0067 d. 4); the endpoint is created
   with advertise off and an empty trusted set (ADR-0067 d. 6). The
   form keeps that key for as long as it is open: **every later Create
   on the same form calls the add with the same key**, never a new one,
   so a retry can only make the same EndpointId, which is one identity
   whether the first add landed or not.
3. It writes the identity's card.
4. **Done**: Your identities opens, the new row focused, and
   `ConfirmIdentityAdded` ("{name} was created. It does not trust any peer
   yet.") with
   no Undo; Remove is the way back, and it confirms (§7). The new
   identity is **not** put in front: switching is the person's act
   (cards §5.7.3). With one identity held before, this is the moment
   the switch and the "as" lines appear (cards §5.7.3, "When the
   identities change").

Outcomes other than Done, each with nothing half-made shown as made:

- **At the bound** (64 endpoints, authored and added together; the add
  refuses by name): Create is not offered and `IdentityLimit` shows in
  its place, focused; the form was never opened if the list knew it
  (cards §5.1, "At the bound").
- **Refused or failed** before the endpoint exists (an answer saying
  so): nothing is created, the form stays filled, `IdentityAddFailed`
  above Create, focus on it. The key stays with the form (step 2).
- **The answer was lost**: the connection to the daemon dropped, or no
  answer came within **the add's wait, 30 seconds** (a hypothesis: well
  past a key's sealing and one configuration write on the slowest
  supported phone; falsified if rust-ui-dev measures an add that lands
  later than that). The client knows the EndpointId it asked for, since
  it derives from the key it made, so it reads the endpoint list
  (`admin.endpoints.list`) and looks for that id. Found: it goes on with
  step 3. Not found, or the read fails: the form stays filled with
  `IdentityAddUnknown` ("InterWeave could not check whether {name} was
  created. Choose Create to try again. If {name} was already
  created, Create does not create a second one.") above Create, focus on
  it, and Back enabled. Create there reads the list first and, the id
  absent, calls the add again with the same key (step 2). **Alternative
  named:** try the add again at once, with a new key. Rejected: a
  second key would make a second identity if the first add had landed.
- **The form is left while the outcome is unknown** (Back after
  `IdentityAddUnknown`, or the app ends): the key file is kept, since an
  add still running may yet use it. At the client's next start, and
  whenever Your identities opens, the client lists the endpoints: a
  kept key whose id is listed is an identity, named by its alias until
  it has a card (the card case below); one whose id is not listed by a
  list that answered is deleted. A key is never deleted on a list that
  failed.
- **The add answers that the id exists** (the first add landed between
  a retry's list and its add, D1): as Found, the client goes on with
  step 3; nothing is shown as refused.
- **The form is left after a refusal** (Back after `IdentityAddFailed`):
  the refusal is an answer that nothing was created, so the key file is
  deleted with the form.
- **The endpoint exists and the card could not be written** (store
  degraded or failed): the identity exists and is named by its alias
  until it has a card (ADR-0061 d. 1, "named by its label"). Your
  identities opens with the row focused and `IdentityCardNotSaved`
  under it ("The name and photo were not saved. Open this identity
  to try again."); its page opens with the form's values kept for this
  session.

## 7. Remove an identity, and turning the first one off

Cards §5.2's Remove stands — who may be removed, when it is not offered,
where focus lands — with its dialog changed for ADR-0067:

- **What goes** (`IdentityRemoveLoses`), in this order: its conversations
  and their messages (unread, kept and waiting to be sent); its
  contacts; its trusted peers; its card; its key (`IdentityRemoveKey`:
  "The key of {identity}. No one can use this identity again."); and that people
  who write to it are no longer reached.
- **What may already have gone**: `IdentityRemoveMayHaveReached` with
  the count, as cards §5.2 says.
- **What others keep** (`IdentityRemoveOthersKeep`, replacing
  `IdentityRemoveLabel`): "People who saved {identity} as a contact
  still have its name and photo. Their messages to it reach no one. To
  them, an identity that you add later is a different one." The old line,
  that a later identity on the freed label would be reached under the
  old contact, stopped being true with ADR-0067 d. 2: a new identity has
  a new key and so a new EndpointId.
- **Cancel** first focus, **Remove** filled in `error`, no Undo.
- **Its trusted peers** leave the device's admission set when no other
  set holds them; nothing on the remaining identities' screens changes.
- **Its key path** is the first thing Remove takes, as ADR-0063
  decision 2 says (identity-key §5.3 step 5): a directory that holds
  anything is left and Remove goes on; where nothing is at the path
  (the key file is already missing), there is nothing to take, and
  Remove goes on; every other failure to remove the entry stops
  Remove before anything else is removed, and the
  snackbar is `IdentityChangeFailed`'s Remove text; its words "Nothing
  was changed" are then true. Remove never reports done while the key file
  is still there.
- **Remove's order, and a Remove that does not finish** (ADR-0061
  decision 7 and ADR-0063 decision 2, as ruled on #751): Remove takes
  its key path first, then the platform key (the Keystore alias, on
  Android only), then the identity's rows in the store (its
  conversations, contacts and card), and asks the device to remove the
  endpoint last. Its trusted set is the device's, kept in the endpoint
  overlay (ADR-0028), and goes with the endpoint at the device's remove
  (D1); a Remove that did not finish leaves it unchanged. A failure at
  any step after the key path, before the device has removed the
  endpoint, is a Remove that did not finish, whatever the device
  answered: a refusal, and `Internal` too, since the client cannot tell
  a write that failed from a configuration left ahead of the running
  device (ADR-0028, D1). The first five items below say what follows
  such a failure; the sixth says how the client reads a lost answer,
  and the last what leaves nothing of the identity behind:
  - The snackbar is `IdentityRemoveUnfinishedRows` when the
    identity's rows were deleted before the failure, and
    `IdentityRemoveUnfinished` when they were not (a failure at the
    Keystore alias), so that it says what was removed (ADR-0061
    decision 7). Neither is ever `IdentityChangeFailed`, and neither
    has an Undo. It names the identity by
    the name the Remove dialog showed. The dialog has closed and focus
    is on the overflow button of the identity's page, as after a refused
    Remove (cards §5.2). `IdentityRemoveUnfinishedRows` never says that
    the identity is still here, nor that Remove again will finish it:
    it shows also where the device may yet remove the endpoint a moment
    later, or only at its own next start (the items below).
  - The identity stays listed, and its page shows the missing key
    (`KeyMissing`, identity-key §4.1). If its rows were deleted before
    the failure, it has no card, conversations or contacts, and the
    list and its page name it by its alias, as they name any identity
    with no card (§6.2). From the deletion of its rows it receives
    nothing (no message to it is kept or shown) until Remove finishes,
    a list that answered does not hold its id, or the app's process
    ends (ADR-0061 decision 7). If Remove failed before its rows were
    deleted, the rows stay, and the identity still receives messages,
    like any other identity whose key is missing (identity-key §1).
  - After a restart of the app, the list and its page show the same as
    before, because nothing stores that a Remove began, and the
    identity receives messages again, like any other identity whose key
    is missing; the last item takes them with it when it goes.
  - Remove stays offered on that page. If the person chooses Remove
    again, it completes once the device removes the endpoint, because
    the key path now holds nothing to take. The Remove dialog is the
    same as before: its key item (`IdentityRemoveKey`) still says that
    no one can use this identity again.
  - **The configuration is left ahead of the running device** (the
    desktop's daemon wrote it and could not apply it, ADR-0028): the
    device still serves the endpoint and lists its id until its own
    next start, so each Remove again until then ends as above, with
    `IdentityRemoveUnfinishedRows`. The client cannot tell this case
    from a write that failed, and the snackbar does not name it. At the
    device's next start the endpoint is gone, and the identity leaves
    the list by the last item.
  - **The device's answer to the remove is lost** (the connection
    drops, or the answer never comes): the client reads the endpoint
    list back (`admin.endpoints.list`), as Add does (§6.2). If the
    identity's id is absent from the list, Remove is done, as when the
    device answers done (cards §5.2). If the id is present, or the list
    does not answer, the Remove is not known to have finished, since
    the remove may still be applied after that list answered (D1), and
    it ends as in the items above. If Remove is chosen again for an id
    that the device no longer holds, the device answers
    `EndpointUnknown` (D1), and Remove ends as done once the list shows
    the id absent.
  - **An identity that cannot come back leaves with nothing of it
    left** (ADR-0061 decision 7, A 2026-11-24): at the app's start,
    after every reconnection to the device, and whenever
    `admin.endpoints.list` answers, the client deletes the store rows of
    an identity whose id that list does not hold and whose key path
    holds no key file. Nothing is deleted on a list that failed, and an
    identity whose key file is still there keeps its rows, since it can
    be configured again. The identity leaves Your identities, the
    switcher and the switch, as one that Remove ended as done leaves
    them (cards §5.2), with no snackbar of its own: the person already
    chose Remove, and `IdentityRemoveUnfinishedRows` did not say it was
    staying. One case has one: when it was in front, the default comes
    in front with `ConfirmIdentityRemovedSwitched`, as after a Remove
    done (cards §5.2), since the identity in use never changes without
    the person being told which one it now is. Where its page or one
    of its screens was shown, the app lands as after a Remove done
    (cards §5.2, "Where it lands"); focus is F22's (§11).

**Turn off this identity** (the first identity) stands as cards §5.2
says. One addition: its trust stays. Turned off, it admits nothing
(its endpoint is disabled), and turned on again it trusts the same
peers.

## 8. Advertise off

Every identity's endpoint is unlisted, from its creation, and the first
one's from the client's first start of this release (ADR-0061 d. 5):
the endpoint directory lists none of them to a trusted peer. **There is
no control for it**, on any screen, and none is offered as disabled: a
switch the person cannot move is a question the system already
answered. What the person sees:

- on Your identities, under the shared-device statement,
  `IdentitiesUnlisted`: "Other devices cannot get a list of your
  identities from this device. A peer that already knows one of your
  identities can still send messages to that identity if that identity
  trusts the peer.";
- the Starting and Could not start states (§5.2), which are the only
  moments the setting is visible as work.

## 9. Notifications under several identities

**What the build has** (`NetworkService.kt` on `main`): one notification
of new messages, a count and no content, on a private channel; the
service's ongoing notification ("Stay reachable is on"), and a notice
posted when Stay reachable restarts without the app's network access
("Network access is off for this app"). Neither names an identity, and
both stay as they are. This note keeps the count form and adds the
identity.

**After the unlock**, with two or more identities held:

- one notification per identity that has new messages, its title the
  count as today, its sub-text (shown in the header) the identity's
  name; all of them in one notification group whose summary names the
  identities in its lines (`NotificationIdentityLine`, "{identity}: 3 new
  messages"), one line each;
- opening an identity's notification switches to it visibly first
  (cards §5.7.5's transition, "Opening as {identity}"), then shows its
  Chats; opening the notification of the identity in front shows its
  Chats with no transition;
- opening the summary, when every new message belongs to one identity,
  does what opening that identity's notification does; when they belong
  to two or more, it opens Chats of the identity in front with the
  switcher open over it, switching nothing, and the person picks, since
  no one identity is the notification's (`human-client-ui.md` §13: a
  switch is one explicit act). Ruled (§16 Q7 a): this is ADR-0061 d. 9
  as amended 2026-11-04, for every notification with no one identity
  (the lock screen's entry, a group's summary, the desktop's redacted
  form); the switcher names the identities with news.

**On the lock screen**, whatever the person's preview setting
(`human-client-ui.md` §10):

- **one entry only**, the same whatever the number of identities, the
  identities that have news and their counts: the app's name and
  `MessagesWaitingLocked` ("New messages"), no count, no name;
- opening it asks for the unlock, then does what opening the summary
  does above. Only after the unlock is any identity shown.

**Alternative named:** one group per identity (cards §5.7.5 said so).
Rejected: on the lock screen each group shows as its own redacted entry,
so the number of entries tells someone holding the locked phone how many
identities have news and which messages belong together. The identity's
name is not the only thing a person may be hiding; that they have a
second one can be. The mechanism (a group whose children are hidden on
the lock screen and whose summary carries a public form) is
rust-ui-dev's; what is checked is what the lock screen shows (§14).

**With one identity held**: one notification as today, naming no
identity, and on the lock screen **the same entry as with two or
more**: the app's own public form with `MessagesWaitingLocked`, in
place of the system's redaction `main` gets today by giving none. With
the system's redaction for one identity and the app's form for two,
the entry itself would tell a viewer of the locked phone that a second
identity exists.

**The desktop** has no system notification today (`human-client-ui.md`
§10 makes one optional). This note specifies none. Ruled (§16 Q6): a
desktop notification has one form and cannot tell a locked screen from
an unlocked one, so a later one is always the redacted form — the
count, no name, no content — and the identity is shown after the open,
by the summary's rule above. This replaces contact-cards §5.7.5's
desktop title that led with the identity's name, and its rule that
notifications of different identities are never folded into one
summary.

## 10. The contact page: a route's actions

Plan §30 step 2: Move puts a second route under a contact, and lands
with a control that deletes a route from a contact, beside it. The
command line already has the action (`contacts route delete`): it
removes one route of a contact and the key recorded for it, refuses a
contact's only route, and leaves the contact with its other routes
(`apps/human-cli/README.md`).

**Placement.** Each route row (cards §6.3) keeps **Open conversation**
as its one visible button, and gains a route menu button at its end
(⋮, 48 dp, accessible name `RouteMenu`: "Options for {title}"),
which holds, in order:

1. **Move to another contact** (`MoveRoute`), offered as cards §6.3
   says;
2. **Remove from this contact** (`RemoveRoute`), offered only while the
   contact holds two or more routes.

On a contact with one route the menu holds Move alone (when offered),
and the menu button is absent when it would hold nothing. The contact's
**Delete contact** stays in the app bar's overflow.

**Alternatives named.** (a) Two inline buttons, Move and Remove, beside
Open conversation. Rejected: at 200 % text on a 360 dp phone three
buttons do not fit on a row and wrap into an order that no longer reads
as one row's; and a destructive control next to the row's main one is
one slip away. (b) Remove shown on a contact's only route, doing Delete
contact. Rejected: one control whose result depends on a count the
person may not have noticed; and it hides the stronger act behind the
weaker word.

**Remove from this contact** runs at once, with no dialog, since it
deletes nothing the person cannot have back:

- the route leaves the contact, with its recorded key; the contact keeps
  its name, photo and other routes;
- the route's conversation stays; its title is the short PeerId again
  (with the route, the short EndpointId, beside it when
  `human-client-ui.md` §4 needs it: while a second conversation with
  that peer on another route exists), since it is under
  no contact of this identity; a later card on it is a new proposal;
- trust is not touched (the snackbar says so only when the route is trusted:
  `ConfirmRouteRemovedTrustedAs`, "… {identity} still trusts {peer}.";
  with one identity `ConfirmRouteRemovedTrusted`, "… You still trust
  {peer}.");
- `ConfirmRouteRemoved` ("Conversation removed from {contact}") with
  **Undo**, which puts the route back under the contact with its key.
  The Undo is withdrawn, its snackbar closed, as soon as the route comes
  under another contact (an Accept, Details' Add as a contact, a Move
  into another contact), as soon as the contact is deleted, or as soon
  as a route of the same PeerId comes under the contact (a Move), since
  the reverse would break "one route per PeerId". A kill inside the
  window leaves the route removed, as the snackbar said
  (`design/2026-10-16-message-swipe.md`'s rule). Snackbar lifetime is
  cards §6.2's. The proposals in the session are recomputed
  (cards §5.4.5).
- Focus: on the next route row's Open conversation, else the previous
  one's; the contact always has one left.

## 11. Keyboard and focus

Every new control is reached by Tab on the desktop and by a hardware
keyboard or switch access on Android, in the reading order the screen
shows, and has a non-gesture form (none of these screens has a
gesture). TalkBack does not reach the app yet (plan §21's carried
accessibility reach): every control here is a labelled one so that it
can, and the TalkBack rows of §14 are blocked until it does.

**Keys, on every screen of this note:**

- Tab and Shift+Tab follow the reading order below; arrows move within
  a menu, a sheet's choice rows and a dialog's buttons, as the toolkit's
  menus already do, and do not leave them. The reading order is the
  logical one: a right-to-left window mirrors where the controls stand,
  but their Tab order is not mirrored.
- Enter and Space press the focused control. In a text field, Enter
  presses the form's primary (Create on the add form, Save on a card)
  and Space types a space.
- Escape (desktop) and Back (Android) close the open menu, sheet or
  dialog with nothing changed, and focus returns to its opener; with
  none open, they leave the page as its back arrow does, through the
  unsaved-changes dialog when cards §5.2 asks for it.
- **No shortcut switches identity.** Switching is reached by Tab to the
  switch and Enter. **Alternative named:** Ctrl+1…9 to switch. Rejected:
  a switch made without the switcher in view is the act the owner's rule
  forbids by name, since the person may not see which identity a
  number is.

**Reading order of each new or changed screen:**

| screen | Tab order |
|---|---|
| Chats (two or more identities) | back or app title, the switch, the app options ⋮, then the list as today |
| switcher (sheet or dialog) | its title (a focus stop, so the reading starts there), the identity rows, Your identities |
| Your identities | back arrow, title, the shared-device statement, `IdentitiesUnlisted`, the rows in order (a row is one stop; its Try again, when shown, the next), Add an identity or `IdentityLimit` |
| add form | back arrow, title, photo buttons, Name, Family name, `IdentityNewNote`, the line above Create when one shows, Create |
| an identity's page | as cards §5.2, then `IdentityAlias` and its Copy, then the overflow |
| Trust | back arrow, title and subtitle, `TrustScope`, then as today |
| Peers | back arrow, title and subtitle, Show my address, Add an address, `PeersShared`, `PeersNoTrust`, the rows as today |
| a contact | as cards §6.3; on each route row, Open conversation, then the route menu button |
| route menu | Move, Remove |
| Remove dialog (identity) | title, the list of what goes, Cancel, Remove |

**Focus table.** One row per case; prose elsewhere points here.

| # | case | focus goes to | check |
|---|---|---|---|
| F1 | the switcher opens | its title | keyboard |
| F2 | the switcher closes with nothing changed | the switch | keyboard |
| F3 | a switch by the switcher or Switch back | the switch, naming the new identity (cards §5.7.3) | keyboard |
| F4 | Your identities opens | the title | keyboard |
| F5 | the add form opens | the Name field | keyboard |
| F6 | Create ends Done | the new identity's row | keyboard |
| F7 | Create refused, failed or unknown | the line above Create (`IdentityAddFailed`, `IdentityAddUnknown`) | keyboard |
| F8 | Create at the bound | `IdentityLimit` | keyboard |
| F9 | Create done, card not saved | the new row; `IdentityCardNotSaved` is read with it | keyboard |
| F10 | the identity Remove dialog opens | Cancel | keyboard |
| F11 | Remove done | as cards §5.2 ("Where it lands") | keyboard |
| F12 | the route menu opens | its first item | keyboard |
| F13 | the route menu closes with nothing chosen | the route menu button | keyboard |
| F14 | Remove from this contact | the next route row's Open conversation, else the previous | keyboard |
| F15 | its Undo | the restored row's Open conversation | keyboard |
| F16 | its Undo withdrawn while focused | the snackbar's place is taken by the page: focus to the page title | keyboard |
| F17 | Try again on Could not start | stays on Try again, disabled, while Starting (§5.2); on success, the row (or the composer's field in a conversation); on a second failure, stays on Try again, enabled again | keyboard |
| F18 | a notification of another identity opens the app | after the transition, the conversation list's title of that identity | device |
| F19 | the lock-screen entry or the summary opened with news in two or more identities, switcher shown | the switcher's title | device |
| F20 | Move done (cards §6.3) | the moved-to contact page's title | keyboard |
| F21 | the identity in front becomes Starting or Could not start while its send control has focus (§5.2) | the composer's field | keyboard, with the daemon restarted |
| F22 | an identity leaves the list after an unfinished Remove (§7's last item) | when that identity's page or one of its screens was shown, the screen and focus that F11 gives after Remove done (cards §5.2, "Where it lands"); otherwise unchanged, the focused control still being shown | keyboard, with acceptance 13's late-remove run |

## 12. Accessibility beyond focus

- **Names in words.** Every "as" line, subtitle and scope line is text
  in the accessibility tree, read with its control: the send control's
  name is `SendAs`, and with two or more identities held each Trust
  control's `TrustAs` ("Trust as {identity}: {peer}"): on the Trust
  page and in its confirmation, whose visible label is then
  `TrustPeerAs` ("Trust as {identity}") in place of `TrustPeer`; and a
  Peers row's Trust (N8), whose visible label `TrustThisPeer` ("Trust")
  it already begins with.
- **Announcements**: `ConfirmIdentityAdded`, the route-removed snackbar
  in each of its forms (`ConfirmRouteRemoved`,
  `ConfirmRouteRemovedTrusted`, `ConfirmRouteRemovedTrustedAs`) and
  the switch's snackbar are announced once through the live region
  (WCAG 4.1.3); the transition's `SwitchingTo` too.
- **Targets**: the switch, each switcher row, the route menu button and
  Try again are at least 48 dp high on Android, 24 px on the desktop
  (WCAG 2.5.8).
- **Contrast**: the check mark of the identity in front, the avatar ring
  on the switch and the `error` fill of Remove hold 3:1 against every
  surface they sit on (`surface`, `surface-container`, the sheet's
  `surface-container-low`), and text 4.5:1 (WCAG 1.4.3, 1.4.11). Each
  is one row of §14's contrast case.
- **Reduced motion**: the transition and the list's cross-fade keep
  their duration and lose their movement (cards §5.7.3, §5.7.5).
- **Long names and other scripts**: a name is isolated plain text
  (cards §3); the switch ellipsises at the end, never to less than the
  avatar and three graphemes; every "as" line wraps rather than truncates, so
  the whole name is always read; at 200 % text each line still fits
  the 360 dp width by wrapping.
- **The Chats bar when the switch does not fit beside the title**
  (rust-ui-dev-02's question, 2026-11-21: the desktop's 280 px list
  pane cut "Conversations" to "Conversa…" beside the switch). The bar
  keeps one row when the whole title, the switch at its minimum and
  the ⋮ fit on it at the current text size. The switch's minimum here
  is the avatar, three graphemes and the ▾. This minimum is measured
  for each identity held, and the widest of those minimums counts,
  whichever identity is in front. So adding or removing an identity
  can change the bar's rows, and switching identity cannot. When the
  title, the switch at its minimum and the ⋮ do not all fit, the bar
  takes two rows. The first row holds the title and the network-state
  line under it, across the whole width. The second row holds the
  switch at its start and the ⋮ at its end, the switch's name taking
  the width between them and ellipsising as above. Whether the bar
  has one row or two depends on the width, the text size, the title
  and the widest minimum among the identities held, never on which
  identity is in front, so switching identity never reflows the bar.
  The order stays §11's for Chats: the title, the switch, the ⋮, in
  reading order and in Tab order, in both
  layouts. Neither the title nor the switch ellipsises to make room for
  the other. In the two-row bar the title wraps at word boundaries, and
  a single word wider than its row breaks inside the word: no part of
  the title is ever ellipsised or hidden, and its accessible name is
  always whole. (This rule was first written as "a single word wider
  than its row ellipsises". It was changed on 2026-11-23, for two
  reasons. rust-ui-dev-02 found that the toolkit cannot ellipsise one
  word of a wrapped text. And an ellipsis hides part of the heading
  from a person who enlarged the text to read it (WCAG 1.4.4), while a
  break inside the word keeps all of it, which reflow allows (WCAG
  1.4.10). The cost: at 200 % text on the 280 px pane, "Conversations"
  may show split between two letters, with no hyphen.) The build sets
  each row's height: at least the height that the Targets item above
  gives, and more when the row's content needs it. With one identity
  held there is no switch, and the bar is one row as today.
  **Alternatives named:**
  - (a) Wrap the title beside the switch. Rejected: "Conversations" is
    one word, so at the default text size every desktop with two or
    more identities would show the title broken inside the word, on
    two lines in a third of the bar's width. The two-row bar breaks a word
    only when the word is wider than the whole row, which can happen,
    for example, at 200 % text on the 280 px pane.
  - (b) Drop the switch to the avatar alone. Rejected: §5 names the
    identity in front in words. Two identities without photos show the
    same placeholder, so the bar would no longer say who acts.
  - (c) Let the title ellipsise, as first built. Rejected: the desktop's
    list pane is 280 px at every wide width, so every desktop with two
    or more identities would show a cut heading at the default text
    size.
  - Material's medium top app bar (the controls on the first row, the
    title under them). Rejected: it puts the heading after the switch
    and the ⋮ in reading order, and §11's order and every focus return
    to the heading would change with it.
  (Hypothesis, from the platform convention that a top app bar's title
  is read before its controls; falsified if people at 280 px miss the ⋮
  on the second row more often than on the first.)

## 13. Strings

The text of every string this note shows, as language-culture-en wrote
it (en-US). Key names are suggestions; rust-ui-dev names them.
Placeholders are cards §10's, and also `{alias}` (an identity's alias),
`{short}` (`ShortEndpoint`'s text), `{tail}` (the last eight
characters of an EndpointId), `{contact}` (a contact's name) and
`{count}` (a number in digits). A key ending in `As` is the form shown
with two or more identities held; the key without it is the form with
one, unchanged unless this section says so. A text shown with two or
more identities never says "you trust" or "this profile" for a trust
that is now an identity's: it names `{identity}`, the identity in
front.

**New keys.**

| key | text | where | must say |
|---|---|---|---|
| `ProposalAs` | If you accept, the card is saved in the contacts of {identity}. | Accept's context, its last sentence, in every case but S (N4) | the contact is saved to the identity in front |
| `TrustStepAs` | Trust as {identity} | add-a-peer trust step's button (N9) | "Trust as {identity}" |
| `TrustScope` | The peers listed here can exchange messages with {identity}. Each of your other identities trusts its own peers. | Trust, under the title | peers trusted here write to this identity; the other identities trust their own |
| `TrustPeerAs` | Trust as {identity} | the Trust page's trust control and its confirmation's primary, in place of `TrustPeer` ("Trust this peer") | "Trust as {identity}" |
| `TrustAs` | Trust as {identity}: {peer} | the accessible name of the Trust page's trust controls (`TrustPeerAs`) and of a Peers row's Trust (`TrustThisPeer`) | trust {peer} as {identity}; begins with the button's visible label |
| `RemoveTrustAs` | Remove trust as {identity}: {peer} | each Remove trust button's accessible name, Trust | untrust {peer} as {identity}; begins with the visible label |
| `PeersShared` | These addresses are saved on this device, for all your identities. | Peers, under Add | addresses are the device's, for every identity |
| `PeerStateTrustedAs` | Trusted by {identity}: this device tries to connect to this address | a Peers row, in place of `PeerStateTrusted` | trusted by {identity}; the device dials it |
| `PeerStateNeedsTrustAs` | Not trusted by {identity} | a Peers row | not trusted by {identity}; no claim about connecting |
| `ChannelTrustNode` | A peer can post in this channel if any of your identities trusts it, or if it is in this device's own list of trusted peers. | a channel's Details | anyone any of the person's identities trusts, or the device's own list holds, can post in the channel; names no identity and no peer |
| `NoReachIdentity` | {identity} does not trust any peer yet. To exchange messages as {identity}, trust a peer. | Chats, the no-reach notice's second form | {identity} trusts no one yet; trust a peer while using it |
| `IdentityStarting` / `IdentityStartingOne` | {identity} is connecting and cannot send yet. / Connecting. You cannot send yet. | row, switcher, composer | the identity is starting and cannot send yet |
| `IdentityNotHidden` / `IdentityNotHiddenOne` | {identity} is not connected. This device could not make sure that {identity} stays unlisted. / You are not connected. This device could not make sure that you stay unlisted. | row, switcher, composer | not connected: could not make sure it stays unlisted |
| `ShortEndpoint` | …{tail} | Your identities, a row's second line; the switcher's rows; `DetailsIdentity` | the short id, built as `ShortPeer` |
| `IdentityIdLabel` | Shown to the people you write to as {short} | that line's accessible name | read as what others see |
| `IdentitiesUnlisted` | Other devices cannot get a list of your identities from this device. A peer that already knows one of your identities can still send messages to that identity if that identity trusts the peer. | Your identities | identities are not listed to other devices; someone who knows one can still write to it if it trusts them |
| `AddIdentityTitle` | New identity | the add form | a new identity |
| `IdentityNewNote` | A new identity has no contacts and trusts no peer. No one has its card until you share it. | the add form | it trusts no one and no one has its card until shared |
| `CreateIdentity` | Create | the add form's primary | create |
| `IdentityAlias` | Name to use on the command line: {alias} | an identity's page | the name used in commands, with the alias; the alias last, nothing touching it |
| `ConfirmIdentityAdded` | {name} was created. It does not trust any peer yet. | snackbar | {name} added; it trusts no one yet |
| `IdentityAddUnknown` | InterWeave could not check whether {name} was created. Choose Create to try again. If {name} was already created, Create does not create a second one. | the add form, above Create | could not tell whether it was created; Create tries again and cannot make it twice |
| `IdentityCardNotSaved` | The name and photo were not saved. Open this identity to try again. | a row | name and photo not saved; open it to try again |
| `IdentityRemoveKey` | The key of {identity}. No one can use this identity again. | Remove dialog, an item of `IdentityRemoveLoses` | its key, so it can never be used again |
| `IdentityRemoveUnfinished` | Could not finish removing {identity}. Its key was removed, but the identity is still here. Choose Remove again to finish. | snackbar after a Remove that failed after its key path was taken and before its rows were deleted (§7) | the key is gone, the identity is still there; Remove chosen again finishes it |
| `IdentityRemoveUnfinishedRows` | Could not finish removing {identity}. Its key, conversations, contacts and card were removed. If {identity} is still listed, choose Remove again. | snackbar after a Remove that failed after its rows were deleted, or whose answer from the device was lost, and before the device was known to have removed the endpoint (§7) | the key, conversations, contacts and card are gone; the identity may still be listed, or may leave the list a moment later or at the device's next start, so the text never says it is still here or that Remove again will finish it; while it is listed, Remove again is the way on |
| `IdentityRemoveOthersKeep` | People who saved {identity} as a contact still have its name and photo. Their messages to it reach no one. To them, an identity that you add later is a different one. | Remove dialog | others keep its name and photo, their messages reach no one, a new identity is a different one to them |
| `MessagesWaitingLocked` | New messages | lock screen | new messages, no count, no name; the same text as `MessagesChannel`, so the key may be that one |
| `NotificationIdentityLine` | {identity}: 1 new message / {identity}: {count} new messages | the group summary's lines (§9), one an identity; the first for a count of one, the second for any other | the identity and its count; plural forms language-culture's |
| `RouteMenu` | Options for {title} | a route row's menu button | options for the route's title |
| `RemoveRoute` | Remove from this contact | route menu | remove from this contact |
| `ConfirmRouteRemoved` | Conversation removed from {contact} | snackbar, with Undo, when the route is not trusted | removed the route from {contact} |
| `ConfirmRouteRemovedTrusted` / `ConfirmRouteRemovedTrustedAs` | Conversation removed from {contact}. You still trust {peer}. / Conversation removed from {contact}. {identity} still trusts {peer}. | the same snackbar when the route is trusted, with one identity / with two or more | removed, and trust is unchanged; each case one whole template, never a clause added to another |

**Existing trust strings, their form with two or more identities.**
Each is shown in place of the key it names, with two or more
identities held; with one, that key's text stands. Each As form fills
its placeholders as the key it replaces does: in `ConfirmAllowAs`,
`ConfirmRevokeAs`, `PeerTrustedAs` and `PeerUntrustedAs`, `{peer}` is
the exact PeerId, whole, as in `ConfirmAllow`, `ConfirmRevoke`,
`PeerTrusted` and `PeerUntrusted` (`labels.rs`), never the short
PeerId of cards §10, since the confirmation is where the person checks
every character before trusting.

| key | in place of | text |
|---|---|---|
| `NoTrustedPeerAs` | `NoTrustedPeer` | No peer is trusted by {identity} |
| `ConfirmAllowAs` | `ConfirmAllow` | Do you want {identity} to trust {peer}? This peer can then exchange messages with {identity} until you remove trust from it. |
| `ConfirmRevokeAs` | `ConfirmRevoke` | Do you want {identity} to stop trusting {peer}? Messages that this peer sends to {identity} are then refused, until {identity} trusts it again. |
| `PeerTrustedAs` | `PeerTrusted` | {peer} is trusted by {identity}. |
| `PeerUntrustedAs` | `PeerUntrusted` | {peer} is no longer trusted by {identity}. |
| `AlreadyTrustedAs` | `AlreadyTrusted` | That peer is already trusted by {identity}. |
| `PeersNoTrustAs` | `PeersNoTrust` | An address lets this device reach a peer. Adding an address does not mean that {identity} trusts the peer. Trust is a separate step. |
| `PeersTrustUnreadAs` | `PeersTrustUnread` | The list of peers that {identity} trusts could not be read, so this page does not show whether {identity} trusts each peer. |
| `TrustStepTitleAs` | `TrustStepTitle` | Do you want {identity} to trust {peer}? |
| `TrustStepBodyAs` | `TrustStepBody` | Trust lets this peer exchange messages with {identity}. Before you choose Trust as {identity}, compare this PeerId with the PeerId on the other person's screen. |
| `TrustStepDoneAs` | `TrustStepDone` | Trusted by {identity} |
| `TrustStepLaterAs` | `TrustStepLater` | Not trusted by {identity} yet |
| `TrustStepWaitingAs` | `TrustStepWaiting` | InterWeave is adding {peer} to the peers that {identity} trusts. This is not finished yet. |
| `TrustStepLaterUnknownAs` | `TrustStepLaterUnknown` | You did not choose Trust as {identity}. InterWeave could not check whether {identity} already trusts this peer. |
| `TrustStepUnknownAs` | `TrustStepUnknown` | InterWeave could not check whether {identity} trusts {peer}. Choose Trust as {identity} to try again. Trusting a peer twice does no harm. |
| `TrustStepNotMadeAs` | `TrustStepNotMade` | {identity} still does not trust {peer}. Your choice of Trust as {identity} was not saved. Choose Trust as {identity} to try again. |
| `TrustStepSupersededAs` | `TrustStepSuperseded` | {identity} does not trust {peer}. Your choice of Trust as {identity} was not saved. Another program or window on this device might have changed whether {identity} trusts {peer} at about the same time. If you still want {identity} to trust {peer}, choose Trust as {identity} again. |
| `TrustStepSupersededUnreadAs` | `TrustStepSupersededUnread` | Your choice of Trust as {identity} for {peer} was not saved. Another program or window on this device might have changed whether {identity} trusts {peer} at about the same time. InterWeave could not check whether {identity} trusts {peer}. To see whether {identity} trusts {peer}, open the trust settings. |
| `ConfirmPeerTrustedAs` | `ConfirmPeerTrusted` | Peer trusted by {identity} |
| `ConfirmAddressRemovedTrustedAs` | `ConfirmAddressRemovedTrusted` | Address removed. {identity} still trusts this peer. |
| `ConnectBodyAs` | `ConnectBody` | InterWeave can reach a person as {identity} only when this device knows their address and {identity} trusts them. Each of you shows your address, and the other person adds it. |
| `ConnectDoneTrustedAs` | `ConnectDoneTrusted` | You added an address, and {identity} trusts this peer. |
| `ConnectDoneNeedsTrustAs` | `ConnectDoneNeedsTrust` | You added an address. {identity} does not trust this peer yet. |
| `ConnectDoneTrustUnknownAs` | `ConnectDoneTrustUnknown` | You added an address. InterWeave could not check whether {identity} trusts this peer. |
| `ContactDeleteConfirmAs` | `ContactDeleteConfirm` | Delete {name}? Your conversations with them stay, and whether {identity} trusts their PeerId does not change. |
| `MoveRouteConfirmAs` | `MoveRouteConfirm` (cards §10) | Move this conversation from {name} to {other}? Moving does not change whether {identity} trusts {peer}. |
| `ContactRouteTrustedAs` / `ContactRouteNotTrustedAs` | `ContactRouteTrusted` / `ContactRouteNotTrusted` (cards §10) | Trusted by {identity} / Not trusted by {identity} |

`TrustStepBodyAs` leaves out "connects to this address only after you
trust this PeerId": in front of one identity the device may already
connect, because another identity trusts the peer, and saying so would
tell the person that (§4). `ConfirmRevokeAs` leaves out "Its
connections to this profile will close now" for the same reason (§7:
the peer leaves the device's admission set only when no other set
holds it). `ContactRouteNotTrustedAs` and `PeerStateNeedsTrustAs` have
the same text, and so do `ContactRouteTrustedAs` and
`TrustStepDoneAs`; each pair says one thing.

**Changed.**

| key | text | what changed |
|---|---|---|
| `IdentitiesShareDevice` | First half as cards §10, each platform's, up to and not including "Any peer you trust who knows an identity's label"; then: Each identity trusts its own peers. A peer that any of your identities trusts can connect to this device. That peer reaches only the identities that trust it. In a channel, a peer can post if any of your identities trusts it or if it is in this device's own list of trusted peers, whichever of your identities reads the channel. | the second half, for ADR-0067 (§6.1) |
| `IdentityRemoveLoses` | This deletes from this device: / The conversations of {identity}, with their unread and kept messages and the messages waiting to be sent / The contacts of {identity} / The list of peers that {identity} trusts / The card of {identity}, with its name and photo / `IdentityRemoveKey` / People who write to {identity} will no longer reach you. This cannot be undone. | its items are §7's, in its order, the trusted peers and the key added; one item a line, each "/" a line break |
| `IdentityAddFailed` | The identity could not be created. Nothing was changed. Choose Create to try again. | it stands above Create now, not Save (§6.2), and names the button |
| `PeerStateNeedsTrust` | unchanged | kept only with one identity held |
| `TrustExplanation` | The peers listed here can exchange messages with you. A change made here stays until you change it again. | from the Stage 22 release: no "Only", no claim about this profile (§5.1, §16 Q9) |
| `ConfirmRevoke` | Do you want to stop trusting {peer}? Messages that this peer sends to you are then refused, until you trust it again. | from the Stage 22 release: it states the route's refusal only, no claim that connections close or about channels (§5.1, §16 Q9, Q10); `{peer}` the exact PeerId, whole, as today |

**Retired**, since nothing shows them any more: `IdentityRemoveLabel`
(§7); `TrustAllIdentities` (cards §5.7.2's "trust is the device's",
false since ADR-0067); the route-label field's `IdentityRouteField`,
`IdentityRouteHelp`, `IdentityRouteNeeded`,
`IdentityRouteBadCharacters`, `IdentityRouteTooLong`,
`IdentityRouteTaken`, and `IdentityRouteLabel` (§6.1, §6.2);
`TrustFromConfiguration` (§5.1, from the Stage 22 release);
`ContactRouteRemovedTrust`, a clause added to another string, replaced
by the two whole templates of `ConfirmRouteRemovedTrusted`.

## 14. Acceptance

Each case is observable on the running build. "Two identities" means the
first and one added. Cases that need D1, D2 or D3 say so; they cannot
pass before it lands.

1. With one identity held, no switch, no "as" line, no identity subtitle,
   no `TrustScope`, `PeersShared` or `ChannelTrustNode`
   appears on any screen of §5's table. *(`human-client-ui.md` §13 bullet 13.)*
2. With two identities held, each screen N1–N12 names the identity in
   front as its row says, in words; switching changes every one of them
   at once. *(`human-client-ui.md` §13 bullet 13; exit gate.)*
3. With two identities, Trust in front of A lists A's trusted peers
   only; trusting P there makes P appear in A's list and not in B's;
   a message from P to B's endpoint is refused at the route while one to
   A's is delivered. Needs D2. *(ADR-0067 d. 6.)*
4. In front of A, with P trusted only by B, P's Peers row reads
   `PeerStateNeedsTrustAs` naming A and nothing about connecting; no
   screen in front of A says that another identity trusts P. Needs D2.
5. An identity's trust rows carry no configuration origin. Needs D2 and
   #553 (§16 Q5).
6. In front of an identity whose set is empty while the address list is
   not, Chats shows `NoReachIdentity` with an action opening Trust. Two
   more runs, with the address list empty: (b) B trusting a peer and
   (c) B's set empty too: in both, Chats in front of A shows add-a-peer
   §5.6's notice, the same screen whatever B holds. With one identity
   held, the notice is add-a-peer §5.6's, unchanged. Needs D2 and #553
   (§16 Q2, Q8).
7. Add: the form has no id, label or advertise field; Create makes an
   identity whose endpoint has advertise off and an empty trusted set,
   observed from the daemon (`admin.endpoints.list`, trust list for
   that endpoint); the new row is focused and the identity in front is
   unchanged. Needs D1, D2. *(`human-client-ui.md` §13 bullet 18; exit gate.)*
8. Add's alias follows §6.2's rule for each of its four examples and
   for a name of only punctuation (`identity`); renaming the card
   leaves the alias. Needs D3 (#553).
9. Add at the bound: with 64 endpoints, Create is not offered and
   `IdentityLimit` is focused; the daemon's refusal by name shows the
   same if the list was stale. Needs D1. *(Exit gate, "the add refused
   at the profile's 64-endpoint bound".)*
10. Add's lost answer, two runs, each ending with exactly one identity
    for the form's key: (a) the daemon stopped after it has written the
    add and before it answers, then started again: `IdentityAddUnknown`
    shows, Create reads the list, finds the id, writes the card; (b) the
    daemon stopped before it writes the add, then started again: Create
    reads the list, does not find the id, and adds with the same key. In
    both, no second EndpointId exists, and after Back on
    `IdentityAddUnknown` with the daemon down, the key file stays until
    a list that answers. Needs D1.
11. Add with the card write made to fail: the identity exists, named by
    its alias, with `IdentityCardNotSaved`. Needs D1.
12. Add offers no configured endpoint that is not an identity (an
    agent's), and lists none as an identity. *(`human-client-ui.md` §13 bullet 18.)*
13. Remove: the dialog lists §7's items in order, with the key and
    `IdentityRemoveOthersKeep`; first focus on Cancel; after Remove the
    endpoint is gone from the overlay, the key file and alias are gone,
    its conversations, pending outbound (tried no further), contacts,
    trusted set and card are gone, and a may-have-reached count was
    shown when one existed. Not offered on the first identity or the
    default. With the unlink of the key file made to fail (identity-key
    acceptance 14's fifth run): the snackbar is `IdentityChangeFailed`'s
    Remove text, the identity is still listed, and its conversations,
    contacts, trusted set, card and key are all still there. With the
    key file deleted before Remove is chosen (the page shows
    `KeyMissing`), Remove completes: the identity and everything that
    the first part of this item lists are gone, and no snackbar shows
    `IdentityChangeFailed`. With a seam that fails Remove after the key
    file is unlinked, in one run on each client at the device's
    administrative remove (a refusal) and, on Android only, in one more
    run at the Keystore alias's retirement: the snackbar is
    `IdentityRemoveUnfinishedRows` in the run at the administrative
    remove and `IdentityRemoveUnfinished` in the run at the alias,
    never `IdentityChangeFailed`; in each run focus is
    on the overflow button; the key file is gone; the identity is still
    listed, and its page shows `KeyMissing`, and still does after the
    app is restarted; in the run at the administrative remove, its
    conversations, contacts and card are gone and it is named by its
    alias, its trusted set is unchanged (D2), and a message sent to it
    before the restart is not shown;
    Remove chosen again completes as above, with no snackbar of
    failure. With a seam that drops the connection after the device has
    removed the endpoint and before the device's answer arrives: the
    list is read back, and Remove ends as done, with no snackbar of
    failure. With a seam that drops the connection before the device
    applies the remove: the snackbar is `IdentityRemoveUnfinishedRows`,
    as in the refusal run. With a seam that holds the remove until after
    the read-back has answered and then applies it (the late-remove
    run): the snackbar is `IdentityRemoveUnfinishedRows`; at the next
    list that answers (Your identities opened again) the identity is
    gone with no snackbar, and focus is as F22 says; with it in front
    when the remove was applied, the default is in front with
    `ConfirmIdentityRemovedSwitched`. On the desktop, with the daemon's
    configuration write made to install and its apply and restore made
    to fail (`Internal`): the snackbar is
    `IdentityRemoveUnfinishedRows`, and Remove chosen again ends the
    same way; after the app is restarted with the daemon still running,
    a message sent to the identity is admitted as to any identity whose
    key is missing; after the daemon is restarted, the identity is gone
    from the list, and the store holds no row of it, that message's
    included. In a run that takes an identity's endpoint out of the
    configuration by other hands with its key file left in place, its
    rows are still there after the next list that answers. Needs D1. *(`human-client-ui.md` §13 bullets 17,
    36; exit gate.)*
14. Turn off the first identity once another is on: listed as off, its
    trusted set unchanged when turned on again; the composed-profile
    check passes. *(Exit gate.)*
15. Advertise: no screen offers a control for it; with `advertise: true`
    authored and the assertion made to fail, the identity shows
    `IdentityNotHidden` with Try again and no lease is granted; made to
    block, it shows `IdentityStarting` and no lease is granted until it
    returns; after a daemon restart with the delta removed, the re-lease
    asserts again with the same two results; a trusted peer's directory
    query lists no identity of this device. Needs D1. *(`human-client-ui.md` §13 bullet 19;
    exit gate.)*
16. Notifications, Android, two identities with new messages in both:
    after the unlock, one notification per identity, each naming it in
    its header, in one group; opening B's from A's screens shows the
    transition naming B before any of B's content. On the lock screen,
    with previews on and with them off: one entry, `MessagesWaitingLocked`,
    no name and no count, and the same entry with news in one identity
    or both. Opening it after the unlock: with news in B only, the
    transition names B, then B's Chats; with news in both, the switcher
    over A's Chats, naming the identities with news, nothing switched
    (#553, §16 Q7 a). Device row (Pixel
    9a). *(`human-client-ui.md` §13 bullets 13, 16; exit gate.)*
17. With one identity held, the notification after the unlock is as on
    `main`, and the lock-screen entry is the same `MessagesWaitingLocked`
    entry as with two: compared side by side on the device, the two
    lock screens look the same. Device row.
18. Contact page: on a contact with two routes, each route's menu holds
    Move and Remove from this contact; on a contact with one route, the
    menu holds no Remove, and is absent when Move is not offered either.
19. Remove from this contact: the route and its recorded key leave the
    contact, the conversation stays titled by the short PeerId, trust is
    unchanged, the snackbar says so; Undo puts route and key back; the
    Undo is withdrawn on each of §10's three events; a kill inside the
    window leaves the route removed. The command line's `contacts route
    delete` and this control leave the store the same. *(Plan §30 step
    2.)*
20. Keyboard: every control of §11's reading-order table is reached by
    Tab in that order on the desktop, and by a hardware keyboard on the
    Pixel; Escape and Back close each menu, sheet and dialog with
    focus on the opener; no key switches identity without the switcher.
21. Focus table: each row F1–F21 holds, checked as its row says.
22. Contrast: one row per mark and surface named in §12, each measured
    at or above its ratio, light and dark.
23. TalkBack: each "as" line is read with its control, and each
    announcement of §12 is spoken once. **Blocked** until the app's
    accessibility tree reaches TalkBack (plan §21); a precondition, not
    a pass.
24. With two identities, a direct message reaching B from a route only A
    holds as a contact is titled by the short PeerId in B's Chats, never
    by A's name; B's Accept of a card on that route creates B's own
    contact and leaves A's unchanged. *(`human-client-ui.md` §13 bullet 15; exit gate.)*
25. With one identity held, on the Stage 22 release: Trust's
    explanation is §13's changed `TrustExplanation`, and revoking a peer
    shows §13's changed `ConfirmRevoke`; with that peer also in the
    device's own list (`transportctl trust allow`), after the revoke its
    connection stays, the identity's route refuses its direct message,
    and its post in a channel the person reads is still shown, which the
    confirmation's text does not contradict. With two identities, the
    same with `ConfirmRevokeAs`. Needs D2. *(§16 Q9, Q10.)*
26. With two identities held, on the desktop's wide layout (the 280 px
    list pane), Chats' bar has two rows: "Conversations" whole on the
    first row, with the network-state line under it; the switch and the
    ⋮ on the second row. The same holds with a 40-grapheme name in
    front and with a 3-grapheme name in front, and switching between
    those two names leaves the bar's height and the title's place
    unchanged. At 200 % text every letter of the title is shown, on one
    line or wrapped (inside the word when the word is wider than its row),
    never ellipsised, and its accessible name is "Conversations"
    either way. On a 360 dp phone at 100 % text, the
    test first measures, in the build, the width of the whole title, of
    the switch at its minimum (the avatar, three graphemes and the ▾,
    the widest of those minimums over the identities held) and of the
    ⋮, with the bar's paddings and gaps, and records which
    layout the sum asks for: one row when it is at most the bar's
    width, two rows otherwise. The bar then has that layout, and keeps
    it with a 40-grapheme name and with a 3-grapheme name in front.
    The widest minimum is checked on the desktop's narrow layout at
    100 % text, with three identities: the first, named "iii…", and
    two added ones, named "WWW…" and "ill…", neither of them the
    default. The three-grapheme minimum of "WWW" differs in width
    from those of "iii" and "ill". The window width is one where the
    title, the minimum of "iii" or of "ill", and the ⋮ fit on one row,
    and the title, the minimum of "WWW" and the ⋮ do not. In two runs,
    one with "WWW…" in front and one with "iii…" in front, the bar has
    two rows. Once "WWW…" is removed, two identities are left and the
    switch is still shown, and the bar has one row.
    Tab from the title reaches the switch, then the ⋮, in both
    layouts, and in a right-to-left window too, where the controls
    stand mirrored. *(§12 "The Chats bar when the switch does
    not fit"; §11 Keys.)*

## 15. Deliberately left out

- **Replacing an identity whose key is lost** (ADR-0067 d. 5: a new
  identity, never a new key under the old name). It is
  `design/2026-10-24-identity-key.md`'s own revision: part 1 (the
  interim, no Make a new key) on 2026-11-22, and part 2 (the
  replacement, in that note's §5) the same day, as a proposal to be
  built once an identity can be created and removed; nothing here
  builds "make a new key".
- **Per-conversation notifications** and any inline reply: the build
  has a count notification, and cards §5.7.5's rule (an inline reply
  names the identity it replies as) stands for later.
- **A desktop system notification** (§9; its rule ruled, §16 Q6).
- **Editing the alias in the graphical clients** (§6.2): the command
  line's.
- **Trusting a peer for several identities at once.** Each identity
  trusts as itself; a "trust for all" control would be the cross-
  identity act the owner's rule forbids.
- **Turning off an added identity** (cards §11).
- **The recovery screens' word for the device's key.** The build's
  `IdCreate`, `IdRestore`, `IdC1Point1`, `IdCreatedTitle`,
  `IdRestoredTitle`, `IdQReasonKey`, `IdQLost` and `IdStorageFailed`
  (`labels.rs`) still call it "identity", which with two identities
  reads as one of them; `design/2026-10-19-android-identity.md` already
  says PeerId in all eight, so that gap is the build's, rust-ui-dev's.
  `IdR1NewKeys` ("each identity gets a new key") contradicted ADR-0067
  d. 5 in that note and in the build alike; that note revised it on
  2026-11-22 with the identity-key note's part 1 (job j159), the words
  language-culture's, and the build's string is rust-ui-dev's.

## 16. Questions

Asked of architect-cto (QUESTION 01a1a69a) and ruled the same day
(REPLY 01a1a69f); the text is PR #553. Each
section above states the ruling where it applies:

- **Q1** `human-client-ui.md` §13's identity-settings criterion against ADR-0067 d. 6:
  amended to §6.1's sentence with a channel clause.
- **Q2** the no-reach rule's trust set: the effective set of the
  identity in front (§5.1).
- **Q3** how a route is shown now the alias stays local: the short
  EndpointId, built as `ShortPeer` (§6.1).
- **Q4** the alias's grammar: the retired label grammar, unique in the
  profile (§1 D3, §6.2).
- **Q5** an origin on an identity's trust rows: none (§5.1).
- **Q6** a desktop notification: always the redacted form (§9).

Asked after rounds 1 and 2 of #552's review (QUESTIONs 01a1a6a8 and
01a1a6b0) and ruled the same day (REPLYs 01a1a6bf-fd48 and
01a1a6bf-ff3f), on #553:

- **Q7** (a) a notification with no one identity opens the switcher
  when two or more identities have news, and switches to the one when
  one has (§9); (b) what counterparties keep of a removed identity is
  said in Remove's dialog alone (§6.2, §7).
- **Q8** the no-reach notice's form depends on the set of the identity
  in front and the address list alone; the Q2 ruling's "or another
  identity's set" is withdrawn (§5.1).

Asked of architect-cto after language-culture-en's pass (j242,
QUESTION 01a1a71f-2302) and ruled the same day (REPLY 01a1a727), from
ADR-0067 d. 6 as it stands, no amendment:

- **Q9** with one identity held, from the Stage 22 release, the first
  identity's set is seeded once from the device's own list (ADR-0067
  d. 6), and the operator's `transportctl trust allow` and revoke write
  the device's list only. Two one-identity strings on `main`, as asked, are
  true only while the two lists hold the same peers: `TrustExplanation`
  ("Only the peers listed here can exchange messages with this
  profile", while a peer only the device's list holds can post in a
  channel) and `ConfirmRevoke` ("Its connections to this profile will
  close now", while the device's list still admits the peer). Asked:
  can the two lists differ with one identity held? Ruled: yes, from the
  seed's first write on either side, so both take a form without the
  claim from the Stage 22 release, each keeping its own key, since
  `TrustScope`'s "each of your other identities" is false with one
  (§5.1, §13). Until that release both are true as on `main`, where the
  client's trust acts write the node's list.

Asked of architect-cto after round 8 of #552's review (QUESTION
01a1a736) and ruled the same day (REPLY 01a1a738):

- **Q10** the revoke confirmations' "no longer exchange messages".
  `ConfirmRevoke` (one identity, as Q9 prescribes its shape) and
  `ConfirmRevokeAs` say the revoked peer "can then no longer exchange
  messages with" the person or {identity}. A peer the device's own
  list still holds (or, with two or more, another identity's set)
  stays in the admission set and can still post in a channel the
  person reads, which Q9 counts as exchanging messages when it removes
  "Only" from `TrustExplanation`. Asked: may the string claim only
  what the identity's route now refuses (its direct messages), or no
  exclusion at all? Ruled: the first. Both confirmations state only
  that the peer's messages to this identity are refused until it is
  trusted again, nothing about channels, connections, the device's
  list or another identity's set (§5.1, §13); the Q9 ruling's shape
  for `ConfirmRevoke` is withdrawn. Worded by language-culture-en
  (REPLY 01a1a73b); §14 case 25 checks the channel post against it.
  architect-cto aligns `human-client-glossary.md`'s Remove trust row
  and `human-client-ui.md`'s eviction warning on their own branch.

Answered by language-culture-en (REPLY 01a1a71c, their j242): the
strings are §13's, and "identity" stays the person's word for a user
identity, the device's being "this device's PeerId" (contact-cards §12
L1); the build's recovery strings that still call the device's key an
identity, and `IdR1NewKeys`, are §15's. `TrustPeerAs` ("Trust as
{identity}", added after that reply) was accepted as final in REPLY
01a1a72b-9f52; the one-identity `TrustExplanation` of §16 Q9 was
worded in REPLY 01a1a730-ee4c, and `ConfirmRevoke`, first worded
there, was replaced under Q10 (REPLY 01a1a73b).
