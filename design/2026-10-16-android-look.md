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
items, chips, a snackbar, a rounded text field), arranged the way
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
placed in this repository yet, so the values below are the roles
rust-ui-dev adds to `Palette` in `app.slint`, as the remit says. A
brand-comms holder may later change a value, keeping its contrast.

Each role exists in both schemes; the platform's dark or light setting
picks one, as `Palette.dark` does today.

| role | used for | dark | light |
|---|---|---|---|
| `surface` | the window's background | `#11152a` | `#f8f9ff` |
| `surface-container` | the Chats pane on a wide screen, banners | `#171c33` | `#eef0fa` |
| `surface-container-high` | a received bubble, the text field | `#232a47` | `#e6e9f5` |
| `on-surface` | text on any surface | `#e3e6f3` | `#161b2e` |
| `on-surface-variant` | secondary text: times, statuses, subtitles | `#b9bfd6` | `#4a5068` |
| `outline` | chip and field borders | `#6b7396` | `#767c95` |
| `primary` | Send, the focus ring, links, icons in chips | `#5fdccd` | `#006a61` |
| `on-primary` | text and icons on `primary` | `#00201d` | `#ffffff` |
| `primary-container` | a sent bubble, the selected conversation | `#004d46` | `#9ff2e5` |
| `on-primary-container` | text on `primary-container` | `#b8fff4` | `#00201d` |
| `tertiary` | the unread badge | `#ffb547` | `#ffb547` |
| `on-tertiary` | the badge's number | `#2a1800` | `#2a1800` |
| `inverse-surface` | the snackbar | `#e3e6f3` | `#2a3050` |
| `inverse-on-surface` | the snackbar's text | `#2a3050` | `#eef0fb` |
| `inverse-primary` | the snackbar's Undo | `#006a61` | `#5fdccd` |
| `error` | refused text, today's `refused-text` | `#ffb4ab` | `#ba1a1a` |

The swipe roles (`swipe-neutral`, `swipe-keep`, `swipe-remove`,
`swipe-retry`, `swipe-on-fill`) keep their values and their rules
(swipe spec §7). `notice-background` and `notice-text` keep theirs too:
the storage notice must stand out from everything else on screen.

**Contrast**, measured with WCAG 2.2's formula (dark, then light):

| pair | need | dark | light |
|---|---|---|---|
| `on-surface` on `surface` | 4.5 | 14.5 | 16.2 |
| `on-surface-variant` on `surface` | 4.5 | 9.9 | 7.6 |
| `on-surface` on `surface-container-high` | 4.5 | 11.3 | 14.1 |
| `on-surface-variant` on `surface-container-high` | 4.5 | 7.7 | 6.6 |
| `on-primary-container` on `primary-container` | 4.5 | 8.7 | 13.3 |
| `on-primary` on `primary` | 4.5 | 10.3 | 6.5 |
| `primary` as text or ring on `surface` | 4.5 | 10.8 | 6.2 |
| `primary` ring on `surface-container-high` | 3 | 8.4 | 5.4 |
| `primary` ring on `primary-container` | 3 | 5.9 | 5.0 |
| `on-tertiary` on `tertiary` | 4.5 | 9.7 | 9.7 |
| `inverse-on-surface` on `inverse-surface` | 4.5 | 10.3 | 11.3 |
| `inverse-primary` on `inverse-surface` | 4.5 | 5.2 | 7.7 |
| `error` on `surface` | 4.5 | 10.6 | 6.2 |
| `outline` on `surface` (a chip's edge) | 3 | 3.9 | 3.9 |
| `swipe-neutral` on `surface` (swipe spec §7) | 3 | 3.6 | 4.7 |

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
| body large | 16 / 24 | regular | a conversation's name in the list; message text |
| body medium | 14 / 20 | regular | the list's subtitle (last message) |
| label large | 14 / 20 | medium | chips, buttons, the snackbar's Undo |
| body small | 12 / 16 | regular | times, statuses, the app bar's subtitle |

Today `app.slint` sets 12px and 16px sizes, and whether they follow the
system font size was not checked. Every size here follows it, up to
200 % (§8).

## 5. The Chats screen

From the top:

- **Top app bar**, 64 dp: the title "Conversations" on the left. On the right an
  icon button for Trust settings (a shield, labelled "Trust settings")
  replaces today's full-width outlined button at the bottom of the list.
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
    pending, a clock icon takes the badge's place.
- **The selected conversation** (a wide screen shows the list and the
  conversation side by side) takes `primary-container` as its fill.
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
- When the owner later lets people name contacts, the avatar takes the
  name's initials. That is outside this spec.

### 5.2 The last message

- The last message the conversation holds in this session, received or
  sent, as plain text: its first line with the markdown marks removed
  (no bold, no link syntax; a link shows its text). The preview of a
  message the person sent starts with "You: " (§9).
- **Showing a preview never reads a message.** An unread message
  previewed in the list stays unread, keeps its durable copy and its
  badge count, until the person opens the conversation. Today a drag on
  a Chats row reads the conversation's messages unseen (swipe review
  item 17). A preview must not add a second way to do that.
- With no message held (all were read, not kept, and a restart removed
  them), the subtitle is "No messages", in `on-surface-variant`.
- The time is the last message's: "01:18" today, "Yesterday", the day
  of the week within the last seven days, else the date, in the phone's
  locale format.

## 6. A conversation

From the top:

- **Top app bar**, 64 dp:
  - a back arrow (48 dp target, labelled "Back to conversations"), which
    replaces today's outlined "Back to conversations" box;
  - the avatar, 36 dp;
  - the name in title medium and, under it, the connectivity line
    ("Online, some peers might not reach you") in body small;
  - an overflow menu (⋮) holding "Hide conversation" when the
    conversation is offered for hiding (swipe spec §10), and "Details".
    "Details" shows the full peer id, copyable in its exact canonical
    form (`human-client-ui.md` §11).
  The peer id is no longer shown on the conversation screen itself.
- **Messages as bubbles.**
  - Received: on the start side, `surface-container-high`, 18 dp
    corners with the corner nearest the sender 6 dp. Sent: on the end
    side, `primary-container`, the mirrored corner 6 dp.
  - A bubble is at most 80 % of the list's width. The text wraps inside
    it.
  - Under the text, inside the bubble, right-aligned: the status with
    its icon, then " · " and the time, in body small:
    - Kept: a filled bookmark and "Kept";
    - Read, not kept: no icon, "Read, not kept";
    - pending ("Sending", "Not confirmed, trying again"): a clock;
    - accepted by remote transport: a single check;
    - failed: an error icon, in `error`.
    The words stay those of `human-client-ui.md` §5. A check never
    means read (§5 there).
  - Consecutive messages from the same side within a minute sit 2 dp
    apart. A change of side, 8 dp. A day change shows a centred "Today",
    "Yesterday" or the date, in body small.
  - The author line ("...x6gY6Eri", "You") is removed: the bubble's
    side shows who wrote it.
    TalkBack still reads the author first (§7).
- **Row actions as chips**, under the bubble, on the bubble's side:
  - outlined chips, 32 dp drawn and 48 dp to touch (the touch area
    extends 8 dp above and below), 8 dp apart, each with an 18 dp icon
    in `primary` before its label:

    | action | icon |
    |---|---|
    | Keep | bookmark add |
    | Dismiss | visibility off |
    | Stop keeping | bookmark remove |
    | Send again | refresh |
    | Cancel | close |
    | Show source | code |

  - They stay visible on every row. They are the swipe's equivalent
    (swipe spec §7) and its primary control.
- **The swipe on a bubble.** The fill is drawn under the bubble's
  shape, across the list's full width, with the same colours, icon and
  label as today. Only its shape follows the bubble.
- **Composer.** A filled text field, 48 dp, fully rounded, on
  `surface-container-high`, with the placeholder "Message". Beside it a
  round 48 dp Send button: the send icon in `on-primary` on `primary`,
  labelled "Send". It is dimmed (38 % opacity) and inactive while the
  field is empty.
- **The storage notice** and the other notices keep
  `notice-background`, as a full-width banner with 12 dp corners under
  the app bar.

## 7. Snackbar, focus, motion

- **The snackbar** becomes Material's: `inverse-surface`, 4 dp corners,
  at least 48 dp tall, 12 dp from the screen's sides, above the
  composer. The text is in `inverse-on-surface` and Undo is a text
  button in `inverse-primary`, with a 48 × 48 dp target. Its behaviour
  follows swipe spec §6.
- **Focus.** Every focusable control draws a 3 dp ring in `primary`, 2 dp
  outside its edge, when it holds keyboard focus. Today the Chats list
  draws none and a conversation's buttons only thicken a grey border
  from 1 to 2 px, which is hard to see. The ring follows the control's
  shape: round for the back arrow and Send, rounded for chips and list
  items.
- **TalkBack.** A bubble is read as one item: author ("You" for one's
  own), the text, the status, the time. The chips follow it in order.
  Swipe spec §7's labels are unchanged. A conversation row in the list
  is read as one item too: the name, the unread count when there is
  one, the last message as the list shows it, then its time. The
  preview is what a sighted person reads first after the name, so a
  TalkBack user hears it in the same place. This needs a new
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
  right. The snackbar mirrors too (swipe review item 16 found it did
  not).

## 8. Edge cases

- **System font size 200 %.** The list item grows in height and nothing
  is cut, apart from the subtitle's one line. A chip row wraps to a
  second line. The app bar's title and subtitle each take one line,
  cut with an ellipsis. Every control keeps its 48 dp target.
- **A very long message.** The bubble wraps. It never widens the list.
- **A message with no text** (only a part this client cannot show). The
  bubble shows the existing placeholder text, in `on-surface-variant`.
- **Two peers with the same two characters.** They share the letters,
  and possibly the colour. The names tell them apart (§5.1).
- **Light scheme on an OLED phone in the dark.** Nothing to do. The
  app follows the system's setting and never picks its own scheme.
- **A conversation selected and then hidden** on a wide screen. The
  selection goes with it, as today.

## 9. Words

New and changed text a person reads. language-culture finalised the
English (2026-10-16). rust-ui-dev names the keys and commits the values
to `labels.rs`; the names below are suggestions, apart from the keys
that exist.

| key | text | where |
|---|---|---|
| `Conversations` (exists) | Conversations | the list's app bar title, in place of today's heading: the same key and the same text |
| `Composer` (exists) | Message | the composer's placeholder. The placeholder is the text field's label, so it is the same key. A second key with the same text would fail the `labels.rs` test that every `UiText` is distinct |
| `NoMessages` | No messages | the list's subtitle when nothing is held |
| `OwnLastMessage` | You: {message} | the list's subtitle when the last message is one's own. `{message}` is the preview text. A template, not a prefix: `labels.rs` fills a template by placeholder and never joins strings, so a translation can reorder it |
| `Details` | Details | the conversation's overflow menu |
| `MoreOptions` | More options | the overflow button's label, Android's own words for it |
| `Today`, `Yesterday` | Today, Yesterday | day separators and list times |
| `ConversationDescription` (exists, new value) | {name}, {unread}, {preview}, {time} | a list row as TalkBack reads it (§7), with unread messages. `{unread}` is `UnreadCount` ("{count} unread"), `{preview}` the subtitle as shown (with "You: " for one's own), `{time}` the list's time |
| `ConversationDescriptionRead` | {name}, {preview}, {time} | the same, with nothing unread |
| `ConversationDescriptionEmpty` | {name}, {preview} | the same, with no message held: `{preview}` is "No messages", and there is no time |

A row's description is three templates, not one with optional parts,
because `labels.rs` never joins strings: each case is a whole sentence
a translation can reorder. The commas give TalkBack its pauses. The
old value's `{kind}` ("Direct conversation") goes: the subtitle that
showed it now shows the last message.

The title stays "Conversations", not "Chats". The app calls each item
a conversation ("Hide conversation", "Conversation hidden", "Back to
conversations"), and one noun for one thing is easier to read than
two. "Chats" in this file and the swipe spec names the screen, not a
word the person sees.

The swipe spec's focus table names the `Conversations` heading. With
this change the "Conversations" title is that focus target.

## 10. Deliberately left out

- **Search** and a **New chat** button. The mockup showed them for
  placement only. Neither exists, and adding them is a product decision.
- **Dynamic colour** (Material You, colours from the wallpaper).
  InterWeave keeps its own colours, so the app is recognisable.
- **Contact names, photos and channels.** Separate work.
- **The desktop client.** It shares `Palette`, so it changes colour with
  this. Its layout is not specified here.
- **Haptics and sounds.** No change.

## 11. Acceptance

On the Pixel 9a, dark and light scheme, at 100 % and 200 % font size:

1. Every pair in §3's contrast table holds as measured from a
   screenshot (the drawn colours equal §3's values).
2. Chats: the app bar's title is "Conversations" and Trust settings is an icon
   button in it. The bottom outlined button is gone.
3. Chats: each row shows the avatar, the name, the last message (or
   "No messages"), the time and, with unread messages, the badge with
   their count. The name is bold exactly when the badge shows.
4. The avatar for one peer id has the same letters and colour after a
   restart, and on desktop.
5. **A preview reads nothing**: a conversation with 2 unread messages
   shows its last one in the list. After a restart without opening it,
   it still has both messages unread.
6. A conversation's app bar has the back arrow, avatar, name and
   connectivity line. The peer id is in Details, and copying it gives
   the exact canonical id.
7. Received messages are on the start side, sent on the end side, with
   §6's fills. In an RTL locale the sides swap.
8. Each bubble's status shows its icon and its words. Accepted shows a
   single check and never the words "Read" or "Seen".
9. Every chip and the Send and back buttons measure at least
   48 × 48 dp to touch.
10. Every focusable control shows the 3 dp `primary` ring under Tab,
    on the Chats list and in a conversation.
11. At 200 % font size nothing overlaps or is cut, apart from the
    one-line subtitles' ellipsis.
12. Every swipe spec §11 item still passes. Only the swipe's look has
    changed.
13. Under "Remove animations" no ripple plays.
14. TalkBack reads a bubble as author, text, status, time, and a
    conversation row as name, unread count, last message, time (a
    person's check, as swipe spec §11 item 12; blocked until TalkBack
    reaches the app).
