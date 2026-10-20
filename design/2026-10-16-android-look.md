# The Android client's look — Material 3, in InterWeave's colours

Status: specification for rust-ui-dev to build (`crates/human/ui-slint`,
`crates/human/ui-model` where a screen needs data it lacks,
`apps/human-android`). Written by ux-designer, 2026-10-16. It answers to
`architecture/docs/architecture/human-client-ui.md`. Where the two
disagree, that one wins and this file is wrong. The swipe actions follow
`design/2026-10-16-message-swipe.md`; this file changes how they look,
never what they do.

## 1. What the owner asked, and how it was read

The owner, in their words (2026-10-16):

> improve a bit that ux with colours - look like a prebeta not like an
> android application

Read as: the app should look and behave like an Android app a person
already knows. That means Material 3 components (top app bar, list
items, menus, a snackbar, a rounded text field), arranged the way
Android messengers arrange them, and a colour scheme instead of grey
boxes with outlines. The words, the screens' content, and every action
stay as they are, apart from the owner's three choices below.

The owner saw a mockup of this direction (published as a page on
2026-10-16, "InterWeave Android Look") and chose, on 2026-10-16:

| question | owner's choice |
|---|---|
| messages as bubbles, or full-width rows restyled | bubbles |
| an avatar per conversation, made from the peer id | id avatars |
| the Chats subtitle: the last message, or "Direct conversation" | the last message |

## 2. The person's goal

A person opening InterWeave on their phone wants to see at once which
conversations have something new, open one, tell their own messages
from the other side's, and act on a message, with the habits they bring
from every other messenger. Today every surface is the same grey, every
control is a thin outlined box, and the screen's structure has to be
read rather than seen.

## 3. Colour

The scheme is built from the icon's palette (`assets/README.md`: navy
`#232c63`, teal `#2ec4b6`, amber `#ffb547`). Teal is the primary colour,
amber marks what is new, and the dark scheme's surfaces are a navy-tinted
near-black. Colour and icons are brand-comms's. That role has no holder
placed in this repository yet, so, as the remit says, this spec names
the roles and the contrast each must hold, and rust-ui-dev adds them to
`Palette` in `app.slint`. The values are proposals, measured to meet
those contrasts: rust-ui-dev or a later brand-comms holder may change a
value if every pair it is in still meets its need in the table below.

Each role exists in both schemes; the platform's dark or light setting
picks one, as `Palette.dark` does today.

| role | used for | dark | light |
|---|---|---|---|
| `surface` | the window's background | `#11152a` | `#f8f9ff` |
| `surface-container` | the Chats pane on a wide screen, banners | `#171c33` | `#eef0fa` |
| `surface-container-high` | a received bubble, the text field | `#232a47` | `#e6e9f5` |
| `on-surface` | text on any surface | `#e3e6f3` | `#161b2e` |
| `on-surface-variant` | secondary text: times, statuses, subtitles | `#b9bfd6` | `#4a5068` |
| `outline` | outlined field borders | `#6b7396` | `#767c95` |
| `primary` | Send, the focus ring, links, icons in menu items | `#5fdccd` | `#006a61` |
| `on-primary` | text and icons on `primary` | `#00201d` | `#ffffff` |
| `primary-container` | a sent bubble, the selected conversation | `#004d46` | `#9ff2e5` |
| `on-primary-container` | text on `primary-container` | `#b8fff4` | `#00201d` |
| `tertiary` | the unread badge | `#ffb547` | `#ffb547` |
| `on-tertiary` | the badge's number | `#2a1800` | `#2a1800` |
| `unread-dot` | the unread dot beside a status | `#ffb547` | `#7c5200` |
| `inverse-surface` | the snackbar | `#e3e6f3` | `#2a3050` |
| `inverse-on-surface` | the snackbar's text | `#2a3050` | `#eef0fb` |
| `inverse-primary` | the snackbar's Undo | `#006a61` | `#5fdccd` |
| `error` | refused text, today's `refused-text` | `#ffb4ab` | `#ba1a1a` |
| `focus-ring-on-inverse` | the focus ring on the snackbar | `#006a61` | `#5fdccd` |
| `focus-ring-on-notice` | the focus ring on a notice banner | `#006a61` | `#006a61` |

The swipe roles (`swipe-keep`, `swipe-remove`, `swipe-retry`,
`swipe-on-fill`) keep their values and their rules (swipe spec §7).
`swipe-neutral` must hold 3:1 against the row's own background (swipe
spec §7), and that background is now a bubble or the selected
conversation, not only `surface`. Today's `#707070` measures 2.84 on a
received bubble and 1.97 on a sent one in dark. So in dark it becomes
`#9aa1bb`, and its icon and label take a new role, `swipe-on-neutral`
(`#11152a`), since white on that grey would fail. Light keeps `#707070`
with white. This narrows swipe spec §7's single `swipe-on-fill` for the
neutral fill in dark; the swipe spec follows when it is next changed. `notice-background` and `notice-text` keep theirs too:
the storage notice must stand out from everything else on screen.

**Contrast**, measured with WCAG 2.2's formula (dark, then light):

| pair | need | dark | light |
|---|---|---|---|
| `on-surface` on `surface` | 4.5 | 14.5 | 16.2 |
| `on-surface-variant` on `surface` | 4.5 | 9.9 | 7.6 |
| `on-surface` on `surface-container-high` | 4.5 | 11.3 | 14.1 |
| `on-surface-variant` on `surface-container-high` | 4.5 | 7.7 | 6.6 |
| `on-surface-variant` on `surface-container` (the wide-screen pane) | 4.5 | 9.2 | 7.0 |
| `on-surface-variant` on `primary-container` (status, time, reply line in a sent bubble) | 4.5 | 5.3 | 6.2 |
| `on-surface` on `primary-container` (the selected row's name) | 4.5 | 7.9 | 13.3 |
| `error` on `primary-container` (a failed send's words) | 4.5 | 5.8 | 5.0 |
| `on-primary-container` on `primary-container` | 4.5 | 8.7 | 13.3 |
| `on-primary` on `primary` | 4.5 | 10.3 | 6.5 |
| `primary` as text or ring on `surface` | 4.5 | 10.8 | 6.2 |
| `primary` ring on `surface-container-high` | 3 | 8.4 | 5.4 |
| `primary` as a channel author's text on `surface-container-high` | 4.5 | 8.4 | 5.4 |
| `on-primary-container` as link text on `primary-container` | 4.5 | 8.7 | 13.3 |
| `on-surface` code text on `surface` | 4.5 | 14.5 | 16.2 |
| `primary` ring on `primary-container` | 3 | 5.9 | 5.0 |
| `primary` selection bar on `primary-container` | 3 | 5.9 | 5.0 |
| `primary` ring on `surface-container` | 3 | 10.1 | 5.7 |
| `focus-ring-on-inverse` on `inverse-surface` | 3 | 5.2 | 7.7 |
| `focus-ring-on-notice` on `notice-background` | 3 | 5.9 | 5.9 |
| `on-tertiary` on `tertiary` | 4.5 | 9.7 | 9.7 |
| `unread-dot` on `surface-container-high` / `primary-container` (in a bubble) | 3 | 8.0 / 5.6 | 5.7 / 5.3 |
| `inverse-on-surface` on `inverse-surface` | 4.5 | 10.3 | 11.3 |
| `inverse-primary` on `inverse-surface` | 4.5 | 5.2 | 7.7 |
| `error` on `surface` | 4.5 | 10.6 | 6.2 |
| `outline` on `surface` (an outlined field's edge) | 3 | 3.9 | 3.9 |
| `swipe-neutral` on `surface` (swipe spec §7) | 3 | 7.0 | 4.7 |
| `swipe-neutral` on `surface-container` | 3 | 6.6 | 4.4 |
| `swipe-neutral` on `surface-container-high` (a received bubble) | 3 | 5.5 | 4.1 |
| `swipe-neutral` on `primary-container` (a sent bubble, the selection) | 3 | 3.8 | 3.9 |
| `swipe-on-neutral` (dark) or white (light) on `swipe-neutral` | 4.5 | 7.0 | 5.0 |

Colour is never the only sign of anything. Unread is a number and a
bold name as well as amber. Sent and received differ by side as well as
fill. A status has its words beside its icon.

## 4. Type

The platform's sans-serif (Roboto on the Pixel), in Material 3's scale.
Sizes are in sp, so they follow the system's font size:

| style | size / line | weight | used for |
|---|---|---|---|
| title large | 22 / 28 | regular | the Chats app bar's title |
| title medium | 16 / 24 | medium | a conversation's name in its app bar |
| title small | 14 / 20 | medium | a channel message's author (§6) |
| body large | 16 / 24 | regular | a conversation's name in the list; message text |
| body medium | 14 / 20 | regular | the list's subtitle (last message); Chats' connectivity subtitle (Android review F7) |
| label large | 14 / 20 | medium | menu items, buttons, the snackbar's Undo |
| body small | 12 / 16 | regular | times, statuses |
| markdown heading, level 1 / 2 / 3 and below | 20 / 28, 18 / 24, 16 / 24 | bold | headings inside a message |
| table header | as body large | bold | a table's header row inside a message |
| code | 14 / 20 | regular, the code font | code lines inside a message |

Today `app.slint` sets 12px, 14px (message text) and 16px, and
markdown headings from 14px up to 22px; whether they follow the system
font size was not checked. Every size here follows it, up to
200 % (§8).

## 5. The Chats screen

From the top:

- **Top app bar**, 64 dp: the title "Conversations" on the left, and
  under it the device's connectivity as a one-line subtitle with its
  icon (Android review F7). On the right an overflow button (⋮,
  labelled `AppOptions`, never "More options") whose menu holds
  "Trust" (Android review F12). It replaces today's full-width outlined
  button at the bottom of the list, and the shield of an earlier
  revision. The menu opens with focus on its first item; Escape, Back
  or a tap outside closes it with focus back on ⋮. "Trust" opens Trust
  with focus on its title, and Back from Trust returns focus to ⋮.
- **The list.** Each conversation is a Material list item, at least
  72 dp tall, with 16 dp side padding:
  - an **avatar**, 40 dp, at the start (§5.1);
  - the **name** in body large (today's "...x6gY6Eri / human"), bold
    when the conversation has unread messages;
  - the **last message** under it in body medium, one line, cut with an
    ellipsis (§5.2);
  - at the end, the **time** of the last message in body small and,
    under it, the **unread badge**: the count in `on-tertiary` on a
    `tertiary` pill, 20 dp tall. With no unread messages there is no
    badge. When the last message is the person's own and still
    pending and nothing is unread, a clock icon takes the badge's
    place; with unread messages the badge shows, never the clock.
- **The selected conversation** (a wide screen shows the list and the
  conversation side by side) takes `primary-container` as its fill and
  a 4 dp bar in `primary` along its start edge, so it is not shown by
  colour alone: the fill measures only 1.7 (dark) and 1.1 (light)
  against the list pane.
- **Swipe to hide** follows swipe spec §10. Its fill follows the list
  item's full width.

### 5.1 The avatar

- A circle with two characters of the peer id, the last two before any
  "/" endpoint (the same characters the list already shows last in
  "...x6gY6Eri"), in label large.
- Its colour is one of three pairs, picked from the peer id. The same
  id always gets the same pair, on every launch and both platforms:
  - teal: fill `#004d46`, text `#b8fff4` (dark); `#9ff2e5`, `#00201d` (light);
  - amber: fill `#5c3d00`, text `#ffdea8` (dark); `#ffdea8`, `#2a1800` (light);
  - navy: fill `#2e3a74`, text `#dde1ff` (dark); `#dde1ff`, `#141a3a` (light).

  The pick is the sum of the bytes of the peer id's canonical text
  form (UTF-8), modulo 3. Each pair holds at
  least 4.5:1 for its text.
- **It is never a trust signal.** Two peers can share an avatar. The
  avatar is decoration beside the name, which stays the identity
  (`human-client-ui.md` §3: labels and avatars are not authenticated by
  the transport). TalkBack reads the name and never the avatar.
- **A channel** has no peer id. Its avatar is a "#" on the navy pair,
  the same for every channel; its title already begins with "#" as text
  (`#name`), so direct and channel differ in text, never by colour alone
  (U2a, `app.slint`).
- When the owner later lets people name contacts, the avatar takes the
  name's initials. That is outside this spec.

### 5.2 The last message

- The last message the conversation holds in this session, received or
  sent, as plain text: its first line with the markdown marks removed
  (no bold, no link syntax; a link shows its text). The preview of a
  message the person sent starts with "You: " (§9).
- **Showing a preview never reads a message.** An unread message
  previewed in the list stays unread, keeps its durable copy and its
  badge count, until the person opens the conversation. On main
  761d72b3 a drag on a Chats row reads the conversation's messages
  unseen (found on the Pixel 9a, 2026-10-16, running swipe spec §11
  item 17). A preview must not add a second way to do that.
- A conversation is listed only while the app holds one of its
  messages, so every row has a last message to show. After a restart,
  a conversation whose messages were all read and not kept is not
  listed at all. (If contacts later list a conversation with no
  message, its subtitle will need words then.)
- Every time on these screens is the local time the message reached
  this app (`local_at`), never the time a sender asserts
  (`sent_at_ms`), which a peer chooses. Day separators and the
  one-minute grouping use it too.
- The time is the last message's, computed in the phone's time zone:
  the time of day for today, "Yesterday", the day of the week within
  the last seven days (today and the six calendar days before it),
  else the month and day (and the year when it is not this year). A
  time later than now (a clock set back) counts as today, never as a
  future day. The time of day follows the phone's 12- or 24-hour
  setting (`DateFormat.is24HourFormat`), read again when it changes
  while the app is open: a person sets it once for the whole phone.
  The words (weekdays, months, the date and time templates) come from
  `labels.rs` in English; other locales' formats wait for the
  localisation contract to carry a second locale (ruled 2026-10-17 on
  rust-ui-dev's question: the app has no locale formatter, and one call
  into Android's per row was judged too costly for now).

## 6. A conversation

From the top:

- **Top app bar**, 64 dp:
  - a back arrow (48 dp target, labelled "Back to conversations"), which
    replaces today's outlined "Back to conversations" box;
  - the avatar, 36 dp;
  - the name in title medium, alone and centred in the bar's height.
    No subtitle: the connectivity line is the device's and moves to
    Chats, and the path line is left out for now (Android review F7).
    When the device is offline, a notice banner under the app bar says
    so (F7);
  - an overflow menu (⋮, "More options") holding "Hide conversation"
    when the conversation is offered for hiding (swipe spec §10), and
    "Details". "Details" opens a dialog with the full peer id, or a
    channel's id, with "Copy PeerId" (or "Copy channel ID") and Close
    (Android review F11); the copy gives
    its exact canonical form (`human-client-ui.md` §11).
  The peer id is no longer shown on the conversation screen itself.
- **Messages as bubbles.**
  - Received: on the start side, `surface-container-high`, 18 dp
    corners with the corner nearest the sender 6 dp. Sent: on the end
    side, `primary-container`, the mirrored corner 6 dp.
  - A bubble is at most 80 % of the list's width. The text wraps inside
    it.
  - Under the text, inside the bubble, right-aligned: the status with
    its icon, then " · " and the time, in body small:
    every status label (`LabelKey`) has an icon:

    | status | icon |
    |---|---|
    | Unread | a dot in `unread-dot` |
    | Unread, kept (`UnreadAlsoKept`) | the dot, then the filled bookmark |
    | Read, not kept | none |
    | Kept | a filled bookmark |
    | Sending; Not confirmed, trying again | a clock |
    | Not sent, send again or cancel; May have reached the peer, send again or cancel | an error icon, the icon and words in `error` |
    | Accepted by remote transport | a single check |
    | Accepted by local transport (a channel's send) | an outlined check |
    | Canceled; Canceled, may have reached the peer | a crossed circle |

    The words stay those of `human-client-ui.md` §5. A check never
    means read (§5 there).
  - Consecutive messages from the same side within a minute sit 2 dp
    apart. A change of side, 8 dp. A day change shows a centred
    separator in body small, naming the day: "Today", "Yesterday", the
    day of the week within the last seven days, else the month and day
    (and the year when it is not this year), with days counted as in
    §5.2. For a screen reader, the separator is an item of its own. It
    is read before the messages of its day.
  - In a **direct** conversation the author line ("...x6gY6Eri", "You")
    is removed: there are two parties and the bubble's side shows which.
    In a **channel**, received bubbles come from several peers, so each
    keeps its author as the bubble's first line, in title small
    (14 / 20, medium) in `primary`, with the author's full PeerId as its
    accessible description as today (U5a). Consecutive bubbles from the
    same author within a minute show it once.
    TalkBack reads the author first in both (§7).
- **What else a bubble holds**, each kept from today:
  - the **reply line** ("In reply to an earlier message", or its
    "not shown here" form, `human-client-ui.md` §15) at the top inside
    the bubble, in body small and `on-surface-variant`, after a 3 dp
    start bar in the bubble's text colour;
  - **links** as text in `primary` in a received bubble and in
    `on-primary-container` in a sent one, underlined, each a 48 dp
    target as today's link buttons are;
  - **code** lines on `surface` with `on-surface` text, in the code
    font; **quote** bars and **rules** in the bubble's text colour.
    Today's `Palette.pane` and `divider` become `surface-container` and
    `outline` wherever else they are used;
  - the **route** ("Route: {route}") is no longer drawn on each bubble.
    A direct conversation is keyed by its route, so every message in it
    has the same route; a channel message has none. Where the route is
    shown is `human-client-ui.md` §4's: on the conversation's Details
    while one conversation with the peer exists, and on every title for
    that peer once a second route exists.
- **The "Messages" heading** is no longer drawn: the bubbles show where
  the messages are. It stays as a heading for screen-reader navigation.
- **Row actions in the bubble's menu** (revised 2026-10-19 by
  `design/2026-10-19-android-review.md` F2; until then they were
  outlined chips under every bubble, visible at rest, which on the
  device took more room than the messages):
  - a Material menu anchored to the bubble, opened as the swipe spec's
    §7 says; each item 48 dp high, with a 24 dp icon in
    `on-surface-variant` before its label:

    | action | icon |
    |---|---|
    | Keep | bookmark add |
    | Dismiss | visibility off |
    | Stop keeping | bookmark remove |
    | Send again | refresh |
    | Cancel | close |
    | Show source | code |

  - Nothing shows under a bubble at rest. While its menu is open, the
    bubble keeps a pressed state layer (`on-surface` at 10 %) so the
    person sees which message the menu is about.
- **The swipe on a bubble.** The fill is drawn under the bubble's
  shape, across the list's full width, with the same colours, icon and
  label as today. Only its shape follows the bubble.
- **Composer.** A filled text field, 48 dp, fully rounded, on
  `surface-container-high`, with the placeholder "Message". Beside it a
  round 48 dp Send button: the send icon in `on-primary` on `primary`,
  labelled "Send". It is dimmed (38 % opacity) and inactive while the
  field is empty.
  - The field always takes focus and the keyboard on a tap, in every
    conversation the person can open, the untrusted peer's included:
    trust gates the send, never the composer (architect-cto-01,
    REPLY 01a159de-729d). Send always commits the message as a
    pending row first (`RETENTION.md` §2); a peer this profile does
    not trust is learned at the first attempt. The row then shows
    the needs-attention status, "Not sent, send again or cancel",
    with the reason `PeerNotTrusted` ("This peer is not trusted for
    this profile."), and its menu offers Send again and Cancel as
    swipe spec §7 gives them. Trust is changed only from Trust
    (`human-client-ui.md` §8); nothing in the conversation changes it.
  - The composer is disabled for one cause only: the app's network
    access is withheld (the `NetworkDenied` notice shows, `network
    denied` in the model). It then looks disabled (38 % opacity, no
    cursor), takes no focus, and the `NetworkDenied` notice above it
    says why. If the field held focus when access was withheld,
    focus goes to that notice. Offline, by contrast, the composer
    stays enabled: what is written waits (F7's banner). It never
    looks enabled while it takes no input (Android review F16).
- **The storage notice** and the other notices keep
  `notice-background`, as a full-width banner with 12 dp corners under
  the app bar.

## 7. Snackbar, focus, motion

- **The snackbar** becomes Material's: `inverse-surface`, 4 dp corners,
  at least 48 dp tall, 12 dp from the screen's sides, above the
  composer in a conversation and 12 dp above the bottom of the screen's
  safe area on the Chats list. The text is in `inverse-on-surface` and Undo is a text
  button in `inverse-primary`, with a 48 × 48 dp target. Its behaviour
  follows swipe spec §6.
- **Focus.** Every focusable control draws a 3 dp ring 2 dp outside its
  edge when it holds keyboard focus. The ring sits on the control's
  container, so its colour depends on that container: `primary` on the
  surfaces and bubbles, `focus-ring-on-inverse` on the snackbar,
  `focus-ring-on-notice` on a notice banner (§3, each 3:1 or more). Today the Chats list
  draws none and a conversation's buttons only thicken a grey border
  from 1 to 2 px, which is hard to see. The ring follows the control's
  shape: round for the back arrow and Send, rounded for bubbles, menu items
  and list items.
- **TalkBack.** A bubble is read as one item: author ("You" for one's
  own), the text, the status, the time. Its label is `UiText::Item`
  (§9), on every platform, so the desktop's screen readers read the
  same order. A day separator is an item
  of its own, read before the messages of its day (§6), so the day is
  heard there and not on every bubble. A bubble's default action opens
  its menu (swipe spec §7).
  Swipe spec §7's labels are unchanged. A conversation row in the list
  is read as one item too: its name (the row's label), then its
  description: the kind (direct or channel, in words), the unread count
  when there is one, the last message as the list shows it, then its
  time. The
  preview is what a sighted person reads first after the name, so a
  TalkBack user hears it in the same place (§9's templates). This needs a new
  description template in place of today's `ConversationDescription`
  ("{kind}, {unread}"); language-culture writes it. TalkBack is not
  reachable on Android today (Slint 1.18.1's Android backend has no
  accessibility bridge, rust-ui-dev's finding of 2026-10-16), so this
  reading order is the target once it is.
- **Motion.** Nothing new animates beyond Material's own press ripple.
  Under "Remove animations" there is no ripple: the press shows as a
  flat state layer (`on-surface` at 10 %).
- **RTL.** Everything mirrors: received bubbles on the right, sent on
  the left, avatars on the right in the list, the back arrow pointing
  right. The snackbar mirrors too: text at the start, Undo at the end
  (on main 761d72b3 it did not, swipe spec §11 item 16's run of
  2026-10-16).

## 8. Edge cases

- **System font size 200 %.** The list item grows in height and nothing
  is cut, apart from the subtitle's one line. A menu item's label
  wraps rather than being cut. The app bar's title takes one line, cut
  with an ellipsis; Chats' connectivity subtitle takes one line and
  its short texts fit without one (Android review F7). Every control keeps its 48 dp target.
- **A very long message.** The bubble wraps. It never widens the list.
- **A message with no text** (only a part this client cannot show). The
  bubble shows the existing placeholder text, in `on-surface-variant`.
- **Two peers with the same two characters.** They share the letters,
  and possibly the colour. The names tell them apart (§5.1).
- **Light scheme on an OLED phone in the dark.** Nothing to do. The
  app follows the system's setting and never picks its own scheme.
- **A conversation selected and then hidden** on a wide screen. The
  selection goes with it, as today.
- **Back with the list and a conversation side by side.** Back leaves
  the app, as it does from Chats: the list is already on screen, so
  there is no screen of the app to return to. This is the platform's
  default for a list and its detail shown together (Material 3's
  adaptive list-detail layout, which goes back only when the panes
  shown would change). The selection and any draft are kept, and the
  conversation is open again when the person comes back, in either
  orientation.
- **A display cutout in landscape.** The cutout's inset sits at one
  side. The surface next to it reaches under it: the list pane's when
  the list shows, the conversation's when the list has stepped aside
  for the keyboard. Content, touch targets and the selected row's bar
  stay inside the safe area. No strip of the window's own background
  shows there (S2 of the Android review: edge-to-edge).
- **A message's status line longer than the bubble.** A pending
  message's status ("May have reached the peer, send again or cancel")
  can be wider than the bubble. The line wraps inside the bubble; the
  time stays whole, on the line's last row, and never runs past the
  bubble's edge, at 100 % and 200 %.

## 9. Words

New and changed text a person reads. language-culture finalised the
English (2026-10-16). rust-ui-dev names the keys and commits the values
to `labels.rs`; the names below are suggestions, apart from the keys
that exist.

| key | text | where |
|---|---|---|
| `Conversations` (exists) | Conversations | the list's app bar title, in place of today's heading: the same key and the same text |
| `Composer` (exists) | Message | the composer's placeholder. The placeholder is the text field's label, so it is the same key. A second key with the same text would fail the `labels.rs` test that every `UiText` is distinct |
| `OwnLastMessage` | You: {message} | the list's subtitle when the last message is one's own. `{message}` is the preview text. A template, not a prefix: `labels.rs` fills a template by placeholder and never joins strings, so a translation can reorder it |
| `Details` | Details | the conversation's overflow menu |
| `MoreOptions` | More options | the conversation's overflow button's label, Android's own words for it. Chats' overflow has `AppOptions` |
| `AppOptions` (new) | App options | the Chats app bar's overflow button's label (Android review F12). It must say the menu is the app's, not a conversation's, since both buttons show side by side on a wide screen. A proposal; the words are language-culture's |
| `NetworkDenied` (exists) | Network access is off for this app. You can read your messages, but you cannot send or receive messages until you allow network access. | the notice above a disabled composer (§6), and the target of focus if the field held it |
| `PeerNotTrusted` (exists, `ErrorClass`) | This peer is not trusted for this profile. | the reason on a pending row whose send was refused for trust (§6, Composer) |
| `TrustHeading` (exists) | Trust | the Chats overflow menu's entry that opens Trust (Android review F12); the shield's label `TrustSettings` has no use left on Chats |
| `StateShortDirect`, `StateShortRelayed`, `StateShortPartial`, `StateShortOffline`, `StateShortUnknown` (#313, not yet on main) | Online, direct · Online, relayed · Online, limited · Offline · Status not known | Chats' connectivity subtitle, one per `human-client-ui.md` §7 state; the full state text is the subtitle's accessible description (Android review F7) |
| `OfflineBanner` (#313, not yet on main) | language-culture's | the notice banner under a conversation's app bar while the device is offline (F7) |
| `DetailsPeerId`, `DetailsChannelId`, `DetailsRoute`, `CopyPeerId`, `CopyChannelId`, `Close`, `PeerIdCopied`, `ChannelIdCopied` (#313, not yet on main) | PeerId · Channel ID · Route named by the peer, not verified: "{route}" · Copy PeerId · Copy channel ID · Close · PeerId copied · Channel ID copied | the Details dialog and its copy confirmation (Android review F11); they replace `ConversationId`'s "Identifier, select to copy" |
| `Today`, `Yesterday` | Today, Yesterday | day separators; `Yesterday` also in list times (for today the list shows the time of day) |
| weekdays (seven keys, `Monday` to `Sunday`) | Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday | day separators and list times within the last seven days |
| months (twelve keys, `January` to `December`) | Jan, Feb, Mar, Apr, May, Jun, Jul, Aug, Sep, Oct, Nov, Dec | inside the date templates only |
| `DateThisYear` | {month} {day} | day separators and list times before the last seven days, this year: "Sep 30" |
| `DateOtherYear` | {month} {day}, {year} | the same, another year: "Sep 30, 2025" |
| `Time24` | {hour}:{minute} | the time of day, phone set to 24-hour: "01:18", "13:05" |
| `TimeAm` | {hour}:{minute} AM | the time of day, phone set to 12-hour, before noon: "1:18 AM", "12:05 AM" just after midnight |
| `TimePm` | {hour}:{minute} PM | the same, from noon: "1:05 PM", "12:30 PM" |
| `ConversationDescription` (exists, new value) | {kind}, {unread}, {preview}, {time} | a list row's description as TalkBack reads it (§7), with unread messages. `{kind}` is `DirectConversation` or `ChannelConversation` as today, `{unread}` is `UnreadCount` ("{count} unread"), `{preview}` the subtitle as shown (with "You: " for one's own), `{time}` the list's time |
| `ConversationDescriptionRead` | {kind}, {preview}, {time} | the same, with nothing unread |
| `Item` (exists, new value) | {author}: {body}. {status}, {time} | a message bubble's label, as TalkBack reads it (§7), on every platform: "You: See you at six. Kept, 01:18". `{time}` is the time of day that the bubble shows, whatever day the message is from. The day is read once, from the separator before that day's messages, not on every bubble |

The weekday, month, date and time values are final (language-culture,
2026-10-17). What the code fills in, and what a translation may change:
- `{day}`, `{year}`, `{hour}` and `{minute}` are numbers in ASCII
  digits. `{day}` has no leading zero. `{minute}` always has two
  digits. In `Time24`, `{hour}` has two digits (00–23). In `TimeAm`
  and `TimePm` it has no leading zero (1–12): 12 is the hour from
  midnight to 1 AM, and from noon to 1 PM.
- `{month}` is one of the twelve month values. Months are short (three
  letters, no period) because the list's time column is narrow; a
  screen reader reads "Sep 30" as the full date. Weekdays stay whole:
  one word fits, and a short weekday is harder to read.
- Each date and time is one whole template, never assembled: a
  translation may reorder the parts ("30 Sep"), and may put its AM or
  PM word before the time. That is why AM and PM are two templates,
  not one template with an `{am_pm}` word.
- A numeric date ("9/30") is not used: it reads differently in
  different countries.

A row's description is two templates, not one with optional parts,
because `labels.rs` never joins strings: each case is a whole sentence
a translation can reorder. The commas give TalkBack its pauses. The
row's accessible label stays its name (`row.title`, as today), so the
description does not repeat it. The description keeps `{kind}`: the
subtitle no longer says "Direct conversation" or "Channel", and a "#"
read aloud is not a kind (U2a, §7). The two values are final: with
the name first, TalkBack reads a row as, for example, "…x6gY6Eri /
human, Direct conversation, 2 unread, You: See you at six, 01:18".

A bubble's label, `Item`, puts `{body}` between the author and the
rest, in §7's reading order. `{body}` can be long, hold several lines
and end with any punctuation, so the template marks both of its ends:
a colon after the author, as in the list's "You: {message}", and a
full stop after the body, so a screen reader pauses before the status
even when the body ends with no punctuation. A body that ends in "?"
or "!" then shows "?." to a braille reader; that costs less than a
status run into the last word. The status and the time are short and
are separated by a comma, as in the list's descriptions.

The title stays "Conversations", not "Chats". The app calls each item
a conversation ("Hide conversation", "Conversation hidden", "Back to
conversations"), and one noun for one thing is easier to read than
two. "Chats" in this file and the swipe spec names the screen, not a
word the person sees.

The swipe spec's focus table (§11 item 21, added by InterWeave #286,
not yet on main) names the `Conversations` heading as a focus target.
With this change the "Conversations" title is that target. If this
spec lands first, the sentence waits for #286.

## 10. Deliberately left out

- **Search** and a **New chat** button. The mockup showed them for
  placement only. Neither exists, and adding them is a product decision.
- **Dynamic colour** (Material You, colours from the wallpaper).
  InterWeave keeps its own colours, so the app is recognisable.
- **Contact names and photos, and new channel features.** Separate
  work. Channels that exist today keep what they show (§5.1, §6).
- **The desktop client.** It shares `Palette`, so it changes colour with
  this. Its layout is not specified here.
- **Haptics and sounds.** No change.

## 11. Acceptance

On the Pixel 9a, dark and light scheme, at 100 % and 200 % font size:

1. Every pair in §3's contrast table meets its need, measured from a
   screenshot, in both schemes. The avatar pairs (§5.1) hold 4.5:1.
2. Chats: the app bar's title is "Conversations", with the device's
   connectivity under it (an icon, none when the state is unknown,
   and a short text, read whole at 200 % on 320 dp). With a keyboard:
   ⋮ opens its menu with focus on the first item, Escape returns focus
   to ⋮, "Trust" opens Trust with focus on its title, and Back from
   Trust returns focus to ⋮. Its overflow button is labelled `AppOptions` and
   its menu holds "Trust", which opens Trust; no shield shows. The
   bottom outlined button is gone.
3. Chats: each row shows the avatar, the name, the last message, the
   time and, with unread messages, the badge with
   their count. The name is bold exactly when the badge shows. With
   nothing unread and one's own pending last message, the clock shows
   in the badge's place.
4. The avatar shows the last two characters of the peer id before any
   "/", in the pair the byte sum modulo 3 picks (checked by a ui-model
   test over fixed ids). The same id has the same avatar after a
   restart and on desktop. A channel's avatar is "#".
5. **A preview reads nothing**: a conversation with 2 unread messages
   shows its last one in the list. After a restart without opening it,
   it still has both messages unread.
6. The preview is the first line with markdown removed, a link showing
   its text. One's own reads "You: …". The time reads as §5.2 says for
   a message from today, yesterday, this week, earlier this year and a
   past year, and comes from `local_at` (a ui-model test with a
   peer-asserted time far off). With the phone set to 12-hour the time
   of day shows AM or PM, and switching the setting while the app is
   open changes the list's times.
7. A conversation's app bar has the back arrow, avatar and name, and no
   subtitle. Offline, a notice banner under it says so; online or
   unknown, none shows. The overflow button is labelled "More options"
   and holds Details, and Hide conversation exactly when it is
   offered. Details is a dialog with the peer id, "Copy PeerId" and
   Close, and the copy gives the exact canonical id.
8. Received messages are on the start side and sent on the end side,
   with §6's fills, corners and width (at most 80 %). In an RTL locale
   the sides swap. Same-side messages within a minute sit 2 dp apart,
   and a day change shows its separator as §6 says: a message from two
   to six days before today under its weekday, an older one under its
   date.
9. In a channel, each received bubble shows its author, and the
   author's description is the full PeerId. In a direct conversation
   no bubble shows an author line.
10. Each status in §6's table shows its icon and its words. Accepted
    shows a check and never the words "Read" or "Seen".
11. A reply shows its reply line in the bubble, a link opens as today's
    link button does, and code, quote and rule lines keep their look in
    both bubble colours. No bubble shows a route line.
12. Every menu item, link, and the Send and back buttons measure at least
    48 × 48 dp to touch. Send is dimmed and does nothing while the field
    is empty.
13. Every focusable control shows the 3 dp ring under Tab: on the Chats
    list, in a conversation, on the snackbar's Undo and on a notice
    banner's buttons, in the colour §7 gives its container, and the
    ring measures 3:1 or more against it.
14. The selected conversation on a wide screen shows its start bar as
    well as its fill.
15. The snackbar is Material's (§7), sits above the composer, or at the
    bottom on the Chats list, and in an RTL locale has its text at the
    start and Undo at the end.
16. At 200 % font size nothing overlaps or is cut, apart from the
    one-line subtitles' ellipsis.
17. Every swipe spec §11 item still passes. Only the swipe's look has
    changed; the neutral fill holds §3's contrasts.
18. Under "Remove animations" no ripple plays.
19. TalkBack reads a bubble as author, text, status, time; a day
    separator as its own item, before the messages of its day; and a
    conversation row as name, kind, unread count, last message, time,
    the name once, and never the avatar: a person's
    check, as swipe spec §11 item 12, blocked until TalkBack reaches the
    app.
20. With the list and a conversation side by side, Back leaves the app.
    Back in the app afterwards shows the same conversation selected,
    with its draft.
21. In landscape, nothing but the adjacent pane's surface shows in the
    cutout's inset, with the keyboard up and down.
22. A pending message in a phone-wide portrait window shows its whole
    time inside the bubble, at 100 % and 200 %.
23. **The composer** (§6), in every conversation on the phone, the one
    with a peer Trust does not list included: a tap on the field opens
    the keyboard and shows the cursor (`dumpsys input_method`
    `mInputShown=true`). Sending there to an untrusted peer leaves a
    pending row whose status reads "Not sent, send again or cancel"
    with the not-trusted reason. With the app's network access
    withheld, the field is drawn at 38 % with no cursor, a tap opens
    no keyboard, and the `NetworkDenied` notice shows above it.
