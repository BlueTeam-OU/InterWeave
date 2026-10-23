// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What a view may say: closed enums of stable keys, never English text,
//! so a translation is a table and a new label is a compile error in
//! every exhaustive match below (agreed items 5, 6).

use interweave_human_client_api::{
    Connectivity, OutboundStatus, SendError, SendProblem, SessionProblem, TrustProblem,
};
use interweave_transport_api::PeerPath;

use crate::trust::EntryProblem;

/// One list makes the enum, [`LabelKey::ALL`] and the keys, so a label
/// cannot exist outside the list P6's test walks (review F4): there is no
/// second list to forget.
macro_rules! labels {
    ($($(#[$doc:meta])* $variant:ident => $key:literal,)+) => {
        /// Every status label a view shows: closed, and P6's test walks
        /// every one of them.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum LabelKey {
            $($(#[$doc])* $variant,)+
        }

        impl LabelKey {
            /// Every label, from the same list as the enum.
            pub const ALL: &'static [Self] = &[$(Self::$variant,)+];

            /// The label's stable key, which a translation table maps to
            /// text.
            #[must_use]
            pub const fn key(self) -> &'static str {
                match self {
                    $(Self::$variant => $key,)+
                }
            }
        }
    };
}

labels! {
    /// Outbound, being sent.
    Sending => "status.sending",
    /// Outbound, the last attempt may have been received: "not
    /// confirmed", never "failed".
    NotConfirmed => "status.not_confirmed",
    /// Outbound, waiting for the person, nothing went out that may have
    /// reached the remote.
    NeedsAttention => "status.needs_attention",
    /// Outbound, waiting for the person, and an earlier attempt may
    /// already have been received.
    NeedsAttentionMayHaveBeenReceived => "status.needs_attention.may_have_been_received",
    /// Outbound, the remote transport's queue accepted it (`AcceptedV2`):
    /// not read, not seen, not processed.
    AcceptedByRemoteTransport => "status.accepted_by_remote_transport",
    /// Outbound, published by the local transport: not delivered to any
    /// recipient in particular.
    PublishedLocally => "status.published_locally",
    /// Outbound, cancelled; nothing that went out may have been received.
    Cancelled => "status.cancelled",
    /// Outbound, cancelled after an attempt that may have been received.
    CancelledMayHaveBeenReceived => "status.cancelled.may_have_been_received",
    /// Inbound, not yet read here.
    Unread => "status.unread",
    /// Inbound, read here and not kept: gone after a restart.
    ReadNotKept => "status.read_not_kept",
    /// Inbound, read here and kept.
    Kept => "status.kept",
}

impl LabelKey {
    /// Whether the label describes an outbound message's delivery: the
    /// labels P6 holds to "never read, seen or processed".
    #[must_use]
    pub const fn is_delivery(self) -> bool {
        !matches!(self, Self::Unread | Self::ReadNotKept | Self::Kept)
    }
}

/// Every other piece of person-facing text a view shows: closed, from one
/// list, so [`placeholder_en`] covers each and a test walks them all.
macro_rules! ui_texts {
    ($($(#[$doc:meta])* $variant:ident => $english:literal,)+) => {
        /// A piece of interface text that is not a status label or an
        /// error class: a control's name, a heading, a template.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum UiText {
            $($(#[$doc])* $variant,)+
        }

        impl UiText {
            /// Every one, from the same list as the enum.
            pub const ALL: &'static [Self] = &[$(Self::$variant,)+];
        }

        /// The placeholder English for `text`.
        const fn ui_text_en(text: UiText) -> &'static str {
            match text {
                $(UiText::$variant => $english,)+
            }
        }
    };
}

ui_texts! {
    /// The window's title.
    AppTitle => "InterWeave",
    /// A `PeerId` shortened for display: `{tail}` is its last eight
    /// characters, verbatim.
    ShortPeer => "…{tail}",
    /// A direct conversation's title: `{peer}` the short `PeerId`,
    /// `{route}` the route label the peer asserts.
    DirectTitle => "{peer} / {route}",
    /// The heading of the conversation list.
    Conversations => "Conversations",
    /// The heading of a conversation's messages.
    Messages => "Messages",
    /// Chats' subtitle (#304 F7), one line under "Conversations", short
    /// enough for 320 dp at 200 %; its accessible description is the
    /// state's full text. Online, reachable directly.
    StateShortDirect => "Online, direct",
    /// Online, reachable inbound through a relay.
    StateShortRelayed => "Online, relayed",
    /// Online, some peers might not reach this profile.
    StateShortPartial => "Online, limited",
    /// Offline, whichever host: the banner and the description say why.
    StateShortOffline => "Offline",
    /// Not known yet: never read as offline.
    StateShortUnknown => "Status not known",
    /// Details (#304 F11): the label above a direct conversation's whole
    /// `PeerId`.
    DetailsPeerId => "PeerId",
    /// Details: the label above a channel's whole id.
    DetailsChannelId => "Channel ID",
    /// Details: the route a direct conversation goes to, as the peer named
    /// it and nothing verified. `{route}` is the routing label, quoted as
    /// a label, never read as a role.
    DetailsRoute => "Route named by the peer, not verified: “{route}”",
    /// Details' action: the whole `PeerId` to the clipboard.
    CopyPeerId => "Copy PeerId",
    /// Details' action for a channel: its whole id to the clipboard.
    CopyChannelId => "Copy channel ID",
    /// Details' other action. Also the DESKTOP start-up refusal
    /// screen's: it ends the screen, and the app with it.
    Close => "Close",
    /// The snackbar after Copy where the platform confirms nothing itself
    /// (below Android 13, and the desktop).
    PeerIdCopied => "PeerId copied",
    /// The same, for a channel's id.
    ChannelIdCopied => "Channel ID copied",
    /// Shown when no conversation is selected.
    NoConversation => "Select a conversation",
    /// What a direct conversation is, in its row's description.
    DirectConversation => "Direct conversation",
    /// What a channel conversation is, in its row's description.
    ChannelConversation => "Channel",
    /// A conversation's unread count. `{count}` is a number, never zero:
    /// a conversation with nothing unread is described without one
    /// (`ConversationDescriptionRead`).
    UnreadCount => "{count} unread",
    /// A conversation row's description as a screen reader reads it after
    /// its name, with unread messages (design/2026-10-16-android-look.md
    /// sections 7 and 9): `{kind}` is what the conversation is, `{unread}`
    /// its unread count's text, `{preview}` the last message as the list
    /// shows it, `{time}` the list's time. Three whole templates rather than
    /// one with optional parts, so a translation can reorder each.
    ConversationDescription => "{kind}, {unread}, {preview}, {time}",
    /// The same, with nothing unread.
    ConversationDescriptionRead => "{kind}, {preview}, {time}",
    /// The list's preview of one's own last message: `{message}` is the
    /// preview text. A template, not a prefix.
    OwnLastMessage => "You: {message}",
    /// The overflow button in a conversation's app bar: Android's own
    /// words for it.
    MoreOptions => "More options",
    /// The overflow menu's entry that shows the conversation's full
    /// identifier, to read or copy.
    Details => "Details",
    /// A day separator in a conversation, for today.
    Today => "Today",
    /// The list's time, and a day separator, for yesterday.
    Yesterday => "Yesterday",
    /// A weekday: the list's time, and a day separator, within the last
    /// seven days.
    Monday => "Monday",
    /// A weekday.
    Tuesday => "Tuesday",
    /// A weekday.
    Wednesday => "Wednesday",
    /// A weekday.
    Thursday => "Thursday",
    /// A weekday.
    Friday => "Friday",
    /// A weekday.
    Saturday => "Saturday",
    /// A weekday.
    Sunday => "Sunday",
    /// A month's short name, in a date.
    January => "Jan",
    /// A month's short name.
    February => "Feb",
    /// A month's short name.
    March => "Mar",
    /// A month's short name.
    April => "Apr",
    /// A month's short name.
    May => "May",
    /// A month's short name.
    June => "Jun",
    /// A month's short name.
    July => "Jul",
    /// A month's short name.
    August => "Aug",
    /// A month's short name.
    September => "Sep",
    /// A month's short name.
    October => "Oct",
    /// A month's short name.
    November => "Nov",
    /// A month's short name.
    December => "Dec",
    /// A date earlier than the last seven days, in the current year:
    /// `{month}` a month's short name, `{day}` the day's number in ASCII
    /// digits with no leading zero. Never a numeric date, which reads
    /// differently in different countries.
    DateThisYear => "{month} {day}",
    /// The same, in another year: `{year}` the four-digit year.
    DateOtherYear => "{month} {day}, {year}",
    /// A clock time on a phone set to 24 hours: `{hour}` 00 to 23 and
    /// `{minute}` 00 to 59, each two digits, in ASCII digits.
    Time24 => "{hour}:{minute}",
    /// A clock time before noon on a phone set to 12 hours: `{hour}` 1 to
    /// 12 with no leading zero (12 for the hour after midnight),
    /// `{minute}` two digits. A whole template, with `TimePm`, so a
    /// translation may put its word before the time.
    TimeAm => "{hour}:{minute} AM",
    /// The same, from noon (12 for the hour after noon).
    TimePm => "{hour}:{minute} PM",
    /// The author of this client's own messages.
    You => "You",
    /// A message item as a screen reader reads it, one order on every
    /// platform (design/2026-10-16-android-look.md section 7): `{author}`
    /// short ("You" for one's own), the `{body}`, its `{status}`, then
    /// `{time}`, the clock time it reached this app, as the bubble shows
    /// it; the day is said once per run by the day separator.
    Item => "{author}: {body}. {status}, {time}",
    /// The same for a message that needs the person and says why:
    /// `{reason}` is its error class's text, a full sentence, read last
    /// so the status and time keep `Item`'s order (Android review F16).
    ItemWithReason => "{author}: {body}. {status}, {time}. {reason}",
    /// A link's control, after the message body: `{destination}` is the
    /// link's full destination as `visible_destination` shows it -- every
    /// character as sent, a hidden one as its code point -- so the person
    /// sees where it goes before activating it.
    OpenLink => "Open link: {destination}",
    /// An image the body references, in its place: never fetched.
    /// `{alt}` is the image's alt text, verbatim, never empty
    /// ([`UiText::ImageNotShownWithoutAlt`] stands in for that).
    ImageNotShown => "[Image not shown: {alt}]",
    /// The same, for an image whose alt text is empty or only whitespace:
    /// named as an image, with no empty name after a colon.
    ImageNotShownWithoutAlt => "[Image not shown]",
    /// Show a message's source as received, in place of the drawn body.
    ShowSource => "Show source",
    /// Return from the source to the drawn body.
    ShowFormatted => "Show formatted",
    /// The window's one announcement, read by a screen reader as it
    /// changes: a message arrived in the open conversation. `{author}`
    /// is the short `PeerId`; never the text, which is read by moving to
    /// the message.
    AnnounceArrival => "New message from {author}.",
    /// More than one arrived in the open conversation at once. `{count}`
    /// is a number.
    AnnounceArrivals => "New messages in this conversation: {count}.",
    /// One of this client's messages in the open conversation changed
    /// status. `{status}` is the status's text.
    AnnounceOwnStatus => "Your message: {status}.",
    /// The same for a status that comes with a reason: `{reason}` is its
    /// error class's text, a full sentence with its own period.
    AnnounceOwnStatusReason => "Your message: {status}. {reason}",
    /// More than one of them changed status at once. `{count}` is a
    /// number.
    AnnounceOwnStatuses => "Your messages changed status: {count}.",
    /// Messages arrived in one other conversation. `{conversation}` is
    /// its title.
    AnnounceElsewhere => "New messages in {conversation}.",
    /// Messages arrived in several other conversations. `{count}` is how
    /// many conversations.
    AnnounceElsewhereMany => "New messages in other conversations: {count}.",
    /// Two announcements made at once, `{first}` before `{rest}`.
    AnnounceBoth => "{first} {rest}",
    /// The open direct conversation's route indicator, once the runtime
    /// has said: the connection to the peer is its own (`human-client-ui.md`
    /// §7). Shown in place, never as a new event.
    PathDirect => "Connected directly",
    /// The same, through a relay's circuit.
    PathRelayed => "Connected through a relay",
    /// An unread message whose content is also kept (U2b).
    UnreadAlsoKept => "Unread, kept",
    /// A reply whose target is shown in this conversation.
    ReplyPresent => "In reply to an earlier message",
    /// A reply whose target is not held here: neutral, never a doubt.
    ReplyUnavailable => "In reply to a message not shown here",
    /// The composer's text field.
    Composer => "Message",
    /// The composer's send control.
    Send => "Send",
    /// Why the last send made no row. `{reason}` is an error class's
    /// text.
    NotSent => "Not sent: {reason}",
    /// Retry a pending message now.
    Retry => "Send again",
    /// Cancel a pending message.
    Cancel => "Cancel",
    /// Keep a read message.
    Keep => "Keep",
    /// Remove Keep from a kept message.
    Unkeep => "Stop keeping",
    /// Dismiss a read, unkept message for the rest of this session: a
    /// row button, and the label beside the icon a swipe uncovers. Nothing
    /// is deleted from anywhere -- its stored copy went when it was read --
    /// and it does not come back in this session (the owner's ruling,
    /// 2026-10-16: a message is dismissed, a conversation hidden).
    Dismiss => "Dismiss",
    /// The label beside the icon a conversation row's swipe uncovers: the
    /// conversation leaves the list.
    HideShort => "Hide",
    /// The conversation screen's header control that hides the whole
    /// conversation from the list, offered only when it holds nothing
    /// unread, kept or pending. A short button label.
    HideConversation => "Hide conversation",
    /// The snackbar after Keep, a few words above the composer for about
    /// six seconds, beside an Undo button; also announced.
    ConfirmKept => "Kept",
    /// The snackbar after Dismiss: the message left the list.
    ConfirmDismissed => "Dismissed",
    /// The snackbar after the person removed Keep: the stored copy is
    /// already deleted when this shows.
    ConfirmUnkept => "No longer kept",
    /// The snackbar on the conversation list after a conversation was
    /// hidden from it.
    ConfirmConversationHidden => "Conversation hidden",
    /// The snackbar's button that reverses what it confirms. A touch
    /// target of its own, so one word.
    Undo => "Undo",
    /// The snackbar after a Cancel: the message will not be sent again.
    ConfirmSendingCanceled => "Sending canceled",
    /// The same, when an earlier attempt may have been received.
    ConfirmSendingCanceledMayHaveReached => "Sending canceled, may have reached the peer",
    /// Announced to a screen reader after an Undo put things back as they
    /// were before the action.
    ActionUndone => "Action undone",
    /// The session is re-opening on its own.
    Reconnecting => "Reconnecting",
    /// Storage cannot hold new messages.
    StorageDegraded => "This device cannot store new messages now. You will not receive new messages until storage is available again.",
    /// The session was refused. `{reason}` is an error class's text.
    Refused => "Could not open the session: {reason}",
    /// Leave a refused session.
    TryAgain => "Try again",
    /// Re-check storage now.
    RecheckStorage => "Check storage again",
    /// The platform gives this app no network access: the person's
    /// messages stay readable, and nothing can be sent until it is
    /// allowed. Android only (human-client-android.md, "Runtime
    /// permissions and the network-denied state").
    NetworkDenied => "Network access is off for this app. You can read your messages, but you cannot send or receive messages until you allow network access.",
    /// Ask for the network access the platform withholds.
    AllowNetwork => "Allow network access",
    /// ANDROID: the profile's key needs the person and their last
    /// authentication is too old, so nothing runs until they
    /// authenticate; nothing stored is lost. Never says the app will
    /// connect by itself (#309 acceptance 10).
    AuthenticationNeeded => "InterWeave is offline because it is locked. To unlock it, tap Unlock and enter your phone's PIN, pattern or password. Your conversations are saved on this phone and appear again after you unlock InterWeave.",
    /// ANDROID: the notice's action, raising the system's prompt again.
    Authenticate => "Unlock",
    /// ANDROID: the title of the system's authentication prompt when it
    /// unlocks the app for a key operation or a launch (not S1's, which
    /// is `IdS1Title`).
    UnlockPromptTitle => "Unlock InterWeave",
    /// The Chats bar's overflow button, its accessible name and tooltip:
    /// the app's options, Trust among them (#304 F12; ux-designer-01
    /// 34023aa1). Named apart from a conversation's More options, which
    /// shows beside it on a wide window.
    AppOptions => "App options",
    /// The control that leaves the trust settings for the conversations.
    BackToConversations => "Back to conversations",
    /// The trust settings' heading.
    TrustHeading => "Trust",
    /// Under the heading: what the list decides, and that a change made
    /// here lasts until it is changed again (ADR-0028 A 2026-10-07: the
    /// daemon keeps it in its state). The person's state: about to add or
    /// remove a peer; read once, so it may run to two sentences.
    TrustExplanation => "Only the peers listed here can exchange messages with this profile. A change made here stays until you change it again.",
    /// The label of this profile's own `PeerId`, shown in full and
    /// selectable to copy.
    OwnPeerId => "This profile's PeerId, select to copy",
    /// The heading of the trusted peers' list.
    TrustedPeers => "Trusted peers",
    /// Beside a listed peer that the profile's configuration lists: why it
    /// is there.
    TrustFromConfiguration => "From this profile's configuration",
    /// Beside a listed peer that was added in these settings.
    TrustAddedHere => "Added here",
    /// Shown in place of the list while it is read.
    TrustReading => "Reading the list of trusted peers",
    /// Shown in place of the list when it holds no peer.
    NoTrustedPeer => "No peer is trusted",
    /// The field a person types or pastes a `PeerId` into.
    PeerIdToTrust => "PeerId to trust",
    /// The control that proposes trusting the typed `PeerId`.
    TrustPeer => "Trust this peer",
    /// The control on a listed peer that proposes removing its trust.
    RemoveTrust => "Remove trust",
    /// The confirmation of an allow. `{peer}` is the exact `PeerId`,
    /// whole: the person must be able to check every character.
    ConfirmAllow => "Trust {peer} for this profile? It will be able to exchange messages with this profile until you remove trust from it.",
    /// The confirmation of a removal. `{peer}` is the exact `PeerId`,
    /// whole. The warning is human-client-ui.md section 8's: connections
    /// close at once.
    ConfirmRevoke => "Remove trust from {peer}? Its connections to this profile will close now, and it cannot exchange messages with this profile until it is trusted again.",
    /// Carry out the change on show.
    ConfirmChange => "Confirm",
    /// Drop the change on show.
    CancelChange => "Do not change",
    /// Said once the daemon allowed a peer. `{peer}` the exact `PeerId`.
    PeerTrusted => "{peer} is trusted.",
    /// Said once the daemon revoked a peer. `{peer}` the exact `PeerId`.
    PeerUntrusted => "{peer} is no longer trusted.",
    /// The typed text is not a `PeerId`.
    NotAPeerId => "That is not a PeerId. A PeerId starts with 12D3KooW or Qm. Check that you copied all of it.",
    /// The typed `PeerId` is this profile's own.
    OwnIdentity => "That is this profile's own PeerId. You do not need to trust it.",
    /// The typed `PeerId` is already listed.
    AlreadyTrusted => "That peer is already trusted.",
    /// Anything else went wrong, before anything was changed. No raw code
    /// is kept, so the text points nowhere for details.
    TrustFailed => "Trust could not be read or changed. Nothing was changed.",
    /// ANDROID: the notification channel of the Stay-reachable mode, in
    /// the system's settings. The glossary's mode name; never a promise.
    ReachableChannel => "Stay reachable",
    /// ANDROID: the title of the notification that shows while the
    /// network service runs under Stay reachable (ADR-0041).
    ReachableTitle => "Stay reachable is on",
    /// ANDROID: its text. Says the app tries, never that it is reachable:
    /// the platform may still stop it (ADR-0041, Operational implications).
    ReachableBody => "InterWeave tries to stay reachable while this notification is shown.",
    /// ANDROID: the notification channel of new messages.
    MessagesChannel => "New messages",
    /// ANDROID: the notice for one message no window has read. Never its
    /// sender or its content.
    NewMessage => "New message",
    /// ANDROID: the notice for more than one. `{count}` is a number.
    NewMessages => "New messages: {count}",
    /// ANDROID: the one notice a Service set to stay reachable posts
    /// when it restarts without network access, and then ends: the
    /// person chose reachability and it no longer holds
    /// (human-client-android.md, "Runtime permissions and the
    /// network-denied state", step 5).
    NetworkOffTitle => "InterWeave cannot stay reachable",
    /// ANDROID: its text: what is off, and that opening the app is how to
    /// turn it back on.
    NetworkOffBody => "Network access is off for this app. Open InterWeave and tap “Allow network access”.",
    /// The list a made or possibly made change left to read again could
    /// not be read: it may not show that change. Never says that nothing
    /// changed. Opening the settings again reads it.
    TrustNotReadAgain => "The list of trusted peers could not be read again, so it may not show the latest change. Open the trust settings again to read it.",
    /// ANDROID, identity screens: W's title.
    IdWelcomeTitle => "Welcome to InterWeave",
    /// ANDROID, identity screens: W's body, two lines.
    IdWelcomeBody => "Exchange messages with the people you trust, directly from this phone. There is no account to create.",
    /// ANDROID, identity screens: W's notice while the void-paper marker
    /// is set (§3.1), three lines.
    IdVoidPaper => "The words shown last time were never used. They are not your identity, so do not use that paper as your backup.",
    /// ANDROID, identity screens: W's primary.
    IdCreate => "Create a new identity",
    /// ANDROID, identity screens: W's secondary.
    IdRestore => "Restore my identity",
    /// ANDROID, identity screens: C1's title.
    IdC1Title => "About your recovery phrase",
    /// ANDROID, identity screens: C1, first point.
    IdC1Point1 => "These 24 words are the key to your identity. With them, you can restore it on another phone.",
    /// ANDROID, identity screens: C1, second point.
    IdC1Point2 => "Write them on paper, in order, with your PeerId. Store the paper in a safe place.",
    /// ANDROID, identity screens: C1, third point: "can be you", without
    /// alarm.
    IdC1Point3 => "Anyone who has these words can act as you in InterWeave. Do not show them to anyone.",
    /// ANDROID, identity screens: C1, fourth point (human-client-ui.md
    /// §9).
    IdC1Point4 => "The words bring back your identity only, not your contacts or your settings.",
    /// ANDROID, identity screens: C1, fifth point (§11 Q3).
    IdC1Point5 => "Your identity exists only after you finish, by checking two of the words.",
    /// ANDROID, identity screens: C1's primary.
    IdShowWords => "Show my words",
    /// ANDROID, identity screens: C2's and S2's title.
    IdWordsTitle => "Your 24 words",
    /// ANDROID, identity screens: The `PeerId`'s label (§5) on C2 and S2,
    /// where the person writes.
    IdPeerIdLabel => "Your PeerId. Write it on the same paper as your words. It is not a password, and it is safe to share.",
    /// ANDROID, identity screens: The `PeerId`'s label on C4 and R4, where
    /// the person has already written it.
    IdPeerIdLabelShown => "Your PeerId. It is the one on your paper. It is not a password, and it is safe to share.",
    /// ANDROID, identity screens: C2's primary.
    IdWrittenDown => "I wrote down the words and PeerId",
    /// ANDROID, identity screens: C3's title.
    IdCheckTitle => "Check your words",
    /// ANDROID, identity screens: C3's body, one line.
    IdCheckBody => "Enter two words from your paper.",
    /// ANDROID, identity screens: The picker's position (§4), and C3's
    /// heading of each entry.
    IdWordPosition => "Word {n} of 24",
    /// ANDROID, identity screens: The suggestions row while more than
    /// four words match (§4).
    IdWordMatches => "{n} words fit these letters",
    /// ANDROID, identity screens: C3's inline error.
    IdCheckWrong => "This does not match word {n}. Check word {n} on your paper.",
    /// ANDROID, identity screens: C3's action after an error.
    IdShowAgain => "Show my words again",
    /// ANDROID, identity screens: C4's title.
    IdCreatedTitle => "Identity created",
    /// ANDROID, identity screens: R4's title.
    IdRestoredTitle => "Identity restored",
    /// ANDROID, identity screens: C4's and R4's body.
    IdCreatedBody => "When you continue, the app connects to the network with this identity.",
    /// ANDROID, identity screens: C4's and R4's primary.
    IdContinue => "Continue",
    /// ANDROID, identity screens: R1's title.
    IdR1Title => "Restore your identity",
    /// ANDROID, identity screens: R1's body, three lines.
    IdR1Body => "You need your 24 words, and your PeerId from your paper or from a contact's app. The app stays offline until the restore is done.",
    /// ANDROID, identity screens: R1's primary.
    IdEnterWords => "Enter my words",
    /// ANDROID, identity screens: R2's primary.
    IdCheckWords => "Check my words",
    /// ANDROID, identity screens: R1's line under the body, two lines.
    IdR1Settings => "The words bring back your identity only. Your contacts and settings are not part of them.",
    /// ANDROID, identity screens: R1's and, when it offers a restore,
    /// K1's line before the phrase is taken: an identity still running
    /// on another device would be on two (`human-client-ui.md` section
    /// 9, ADR-0043), and the phone cannot tell. A statement, not a
    /// question.
    IdActiveElsewhere => "If another device still uses this PeerId, a restore here puts the same PeerId on two devices. One PeerId must never be on two devices, and this phone cannot check whether another device uses it.",
    /// ANDROID, identity screens: R3, two lines under the question.
    IdR3NoPeerId => "If you have no PeerId on paper, ask a contact to read yours from their app. Restore only if it matches.",
    /// ANDROID, identity screens: R3's question, two lines.
    IdR3Question => "Is this the PeerId that you wrote down? If it is not, a word may be wrong.",
    /// ANDROID, identity screens: R3's primary.
    IdR3Yes => "Yes, restore it",
    /// ANDROID, identity screens: R3's secondary.
    IdR3No => "No, check my words",
    /// ANDROID, identity screens: K1's title.
    IdQTitle => "This phone needs your recovery phrase",
    /// ANDROID, identity screens: K1 for `KeyInvalidated` and `KeyMissing`.
    IdQReasonKey => "This phone's screen lock or security changed, so the app cannot open your identity. Restore it with your recovery phrase.",
    /// ANDROID, identity screens: K1 for Record, Authentication and
    /// `WrongIdentity`.
    IdQReasonRecord => "The app cannot read your identity on this phone. Restore it with your recovery phrase.",
    /// ANDROID, identity screens: K1's primary.
    IdEnterPhrase => "Enter my recovery phrase",
    /// ANDROID, identity screens: K1's plain text below its primary
    /// (§3.4), three lines.
    IdQLost => "If your words are lost, clear this app's data in the phone's settings. That starts a new identity, which each contact must trust again.",
    /// ANDROID, identity screens: The label of the expected `PeerId`, on K1
    /// and in `IdOtherIdentity`'s banner.
    IdExpectedPeerId => "PeerId this phone expects",
    /// ANDROID, identity screens: The label of the derived `PeerId` in
    /// `IdOtherIdentity`'s banner.
    IdDerivedPeerId => "PeerId from your words",
    /// ANDROID, identity screens: The Settings item (§3.5).
    IdShowPhraseItem => "Show recovery phrase",
    /// ANDROID, identity screens: S1's title.
    IdS1Title => "Show your recovery phrase",
    /// ANDROID, identity screens: S1's body, two lines.
    IdS1Body => "You will be asked to unlock your phone first. The app goes offline while your words are shown.",
    /// ANDROID, identity screens: S1's primary: raises the system's
    /// prompt, then shows the words.
    IdS1Unlock => "Unlock and show my words",
    /// ANDROID, identity screens: The progress state (§3.5, §6).
    IdGoingOffline => "Going offline…",
    /// ANDROID, identity screens: S2's primary.
    IdDone => "Done",
    /// ANDROID, identity screens: §6's checksum banner.
    IdChecksumFailed => "These words do not form a recovery phrase. A word may be wrong or in the wrong place. Check them against your paper.",
    /// ANDROID, identity screens: §6's other-identity banner, above the
    /// two labelled `PeerIds`.
    IdOtherIdentity => "These words open a different identity, so nothing on this phone was changed. Compare the two PeerIds below with your paper.",
    /// ANDROID, identity screens: §6's busy notice, with `TryAgain`.
    IdBusy => "The app could not go offline. Wait a moment, then try again.",
    /// ANDROID, identity screens: §6's storage notice, with `TryAgain`.
    IdStorageFailed => "The app could not save your identity on this phone. Nothing was changed.",
    /// ANDROID, identity screens: §7's dialog title.
    IdDiscardTitle => "Delete the words you entered?",
    /// ANDROID, identity screens: §7's dialog: the action that drops the
    /// words.
    IdDiscard => "Delete words",
    /// ANDROID, identity screens: §7's dialog: the default action.
    IdContinueEntering => "Continue entering",
    /// ANDROID, identity screens: The keypad's delete key (its accessible
    /// label).
    IdKeyDelete => "Delete last letter",
    /// ANDROID, identity screens: The keypad's clear key (its accessible
    /// label).
    IdKeyClear => "Clear this word",
    /// ANDROID, identity screens: The grid's button at large font (§4).
    IdReviewWords => "Show all my words",
    /// How long a waiting message has waited, from when it was written,
    /// rounded down (#304 F10): under a minute.
    AgeJustNow => "just now",
    /// One minute.
    AgeOneMinute => "1 minute ago",
    /// 2 to 59 minutes; `{count}` is the number.
    AgeMinutes => "{count} minutes ago",
    /// One hour.
    AgeOneHour => "1 hour ago",
    /// 2 to 23 hours; `{count}` is the number.
    AgeHours => "{count} hours ago",
    /// One day.
    AgeOneDay => "1 day ago",
    /// 2 days or more; `{count}` is the number.
    AgeDays => "{count} days ago",
    /// A waiting message nothing has reached the peer of yet, with no
    /// problem known (F10 A0). `{age}` is one of the Age texts. No
    /// waiting status ends with a period: `AnnounceOwnStatus` adds its own.
    WaitingSending => "Still sending, started {age}",
    /// A0 after an attempt that reached no path to the peer, or knew no
    /// address for it (A1). The phone's own network may be fine.
    WaitingSendingNoPath => "Still sending, started {age}. Last attempt: the peer could not be reached",
    /// A0 after the peer answered with the coarse no-route class (A2).
    WaitingSendingRoute => "Still sending, started {age}. Last attempt: the peer was reached but could not accept the message",
    /// A0 after a busy answer (A3).
    WaitingSendingBusy => "Still sending, started {age}. Last attempt: the network or the peer was busy",
    /// An earlier attempt may have reached the peer, no problem known
    /// (B0).
    WaitingMaybe => "May have reached the peer. Still sending, started {age}",
    /// B0 with A1's problem (B1).
    WaitingMaybeNoPath => "May have reached the peer. Still sending, started {age}. Last attempt: the peer could not be reached",
    /// B0 with A2's problem (B2).
    WaitingMaybeRoute => "May have reached the peer. Still sending, started {age}. Last attempt: the peer was reached but could not accept the message",
    /// B0 with A3's problem (B3).
    WaitingMaybeBusy => "May have reached the peer. Still sending, started {age}. Last attempt: the network or the peer was busy",
    /// DESKTOP, start-up refusal screen: its heading, over every reason.
    RefusalHeading => "InterWeave could not start",
    /// DESKTOP, start-up refusal screen: the label of the file or folder
    /// a refusal is about, shown whole below the reason.
    RefusalPath => "File or folder",
    /// DESKTOP, start-up refusal screen: said after Try again, when the
    /// answer is still a refusal.
    RefusalStillRefused => "InterWeave tried again, and it still cannot start.",
    /// DESKTOP, start-up refusal screen: the profile's format marker
    /// names a newer build's format (ADR-0028 A 2026-10-20). The path is
    /// the marker file.
    RefusalProfileNewer => "A newer version of InterWeave has used this profile, and this version cannot open it. Nothing was changed. To use this profile, install and start the newest version of InterWeave that you have used, or a newer one.",
    /// DESKTOP, start-up refusal screen: the profile's configuration, or
    /// its format marker, could not be read or used. The path is the
    /// file.
    RefusalProfileSettings => "InterWeave could not use the settings in the file below. Check the file, then start InterWeave again.",
    /// DESKTOP, start-up refusal screen: the profile's folders could not
    /// be worked out, or two of them are one. No path.
    RefusalProfileFolders => "InterWeave could not set up the folders for this profile. Check where this profile stores its files, then start InterWeave again.",
    /// DESKTOP, start-up refusal screen: another window holds this
    /// profile. No path.
    RefusalAlreadyOpen => "InterWeave is already open for this profile. Use that window. It may be behind other windows.",
    /// DESKTOP, start-up refusal screen: a file or folder the app keeps
    /// private, found reachable by others before the store is opened. It
    /// can be a file (the lock file, the configuration) or a folder (the
    /// state or profile folders); the path below shows which. Nothing
    /// offers Try again.
    RefusalNotPrivate => "Other accounts on this computer can read or change the file or folder below. InterWeave needs it to be private to you. Nothing was changed. Make it private to you, then start InterWeave again.",
    /// DESKTOP, start-up refusal screen: as `RefusalNotPrivate`, for the
    /// message store, where Try again is offered.
    RefusalStoreNotPrivate => "Other accounts on this computer can read or change the file or folder below, where InterWeave stores your messages. InterWeave needs it to be private to you. Nothing was changed. Make it private to you, then try again.",
    /// DESKTOP, start-up refusal screen: a file the app needs before the
    /// store (the lock) could not be opened. Nothing offers Try again.
    RefusalUnavailable => "InterWeave could not open a file that it needs. Check that the computer has free disk space, and that you can write to the folder below. Then start InterWeave again.",
    /// DESKTOP, start-up refusal screen: the message store could not be
    /// opened now; Try again is offered.
    RefusalStoreUnavailable => "InterWeave could not open the file that holds your messages. Check that the computer has free disk space, and that you can write to the file and its folder. Then try again.",
    /// DESKTOP, start-up refusal screen: the message store needs
    /// recovery -- corrupt, written by a newer version, its migration
    /// failed, or its key is missing or does not open it (ADR-0060
    /// decision 6). Try again and Delete stored data are offered. The
    /// stored data opens only with its key on this device, so no backup
    /// copy can be put back (design/2026-10-21-contact-cards.md §10).
    RefusalStoreRecovery => "InterWeave cannot read its stored data, which includes your messages and contacts. Nothing was changed. The data opens only with its key on this device, so there is no copy of it that you can put back. You can try again, close InterWeave, or choose Delete stored data. InterWeave shows what you lose before it deletes anything. If a newer version of InterWeave wrote this data, install and start the newest version that you have used, or a newer one. Restoring this device's PeerId from its recovery phrase does not bring back this data.",
    /// ANDROID, the store's recovery screen (R): its heading.
    StoreRecoveryTitle => "InterWeave cannot read its stored data",
    /// ANDROID, the store's recovery screen (R): why, and the way back.
    /// No newer-version advice: Android installs no older build over a
    /// newer one.
    StoreRecoveryBody => "InterWeave cannot read its stored data, which includes your messages and contacts. Nothing was changed. The data opens only with its key on this phone, so there is no copy of it that you can put back. You can try again, close InterWeave, or choose Delete stored data. InterWeave shows what you lose before it deletes anything. Restoring this device's PeerId from its recovery phrase does not bring back this data.",
    /// ANDROID, the key-lost screen (K): the Keystore invalidated the
    /// store key (ADR-0060 decision 6). Its heading.
    StoreKeyLostTitle => "The key to your stored data no longer works",
    /// ANDROID, the key-lost screen (K): why; the identity is untouched.
    StoreKeyLostBody => "This phone no longer accepts the key that protects InterWeave's stored data. This can happen after some changes to the screen lock. The data cannot be read again. This device's PeerId and recovery phrase are not affected. To use InterWeave again, choose Delete stored data. InterWeave shows what you lose before it deletes anything.",
    /// R and K: the action that builds the discard's confirmation. The
    /// ellipsis says a further step follows.
    DiscardStore => "Delete stored data…",
    /// While the discard's confirmation is built (the complete copy
    /// opened read-only under the key), past 300 ms.
    StoreChecking => "Checking the stored data…",
    /// The full discard's dialog title.
    DiscardStoreTitle => "Delete all stored data?",
    /// The full discard's list, its lead-in, one identity held.
    DiscardStoreLosesOne => "This deletes from this device:",
    /// The full discard's list, its lead-in, two or more identities held:
    /// it is every identity's data (ADR-0061 decision 9).
    DiscardStoreLosesMany => "This deletes from this device, for all your identities:",
    /// The full discard's list: an item.
    DiscardLosesContacts => "Your contacts and their routes",
    /// The full discard's list: an item.
    DiscardLosesPending => "Messages waiting to be sent",
    /// The full discard's list: an item.
    DiscardLosesMessages => "Unread and kept messages",
    /// The full discard's list: an item.
    DiscardLosesConversations => "Your conversation list",
    /// The full discard's list: an item, one identity held.
    DiscardLosesCardOne => "The name and photo on your card",
    /// The full discard's list: an item, two or more identities held.
    DiscardLosesCardMany => "The names and photos of your identities",
    /// The full discard's list: an item.
    DiscardLosesSettings => "The app's settings, other than your network settings",
    /// The full discard's list: the duplicate-suppression pairs go too.
    DiscardLosesShownAgain => "The record of the messages you already received (one of them might show again)",
    /// The full discard: what is kept, one identity held.
    DiscardStoreKeepsOne => "This device's PeerId and recovery phrase, the peers you trust and your network settings are not affected.",
    /// The full discard: what is kept, two or more identities held.
    DiscardStoreKeepsMany => "This device's PeerId and recovery phrase, the peers you trust and your network settings are not affected. All your identities are kept. Each one shows its label as its name until you give it a new name.",
    /// The full discard: RETENTION.md's disclaimer (ADR-0060 decision 6).
    DiscardStoreRemains => "Earlier copies that your device made outside InterWeave might still be on it.",
    /// The full discard's primary action.
    DiscardStoreConfirm => "Delete stored data",
    /// The partial discard's dialog title: a complete copy exists.
    DiscardPartialTitle => "Delete the unfinished file?",
    /// The partial discard: nothing the person has is lost.
    DiscardPartialBody => "An update to how your data is protected was interrupted. A complete copy of your data exists. This deletes only the unfinished file. Then InterWeave opens with your data.",
    /// The partial discard's primary action.
    DiscardPartialConfirm => "Delete the unfinished file",
    /// While the confirmed discard runs.
    StoreDiscarding => "Deleting…",
    /// Chats' notice after a full discard, shown once.
    StoreDiscarded => "The stored data was deleted. This device's PeerId and the peers you trust did not change.",
    /// Chats' notice after the partial discard, shown once.
    StoreRestored => "Your data was restored from the complete copy.",
    /// R and K after a discard that failed.
    StoreDiscardFailed => "The stored data could not be deleted. Try again.",
    /// A conversation's title once its route belongs to a contact who
    /// shares a name with another: `{name}` is the contact's name as the
    /// person accepted or edited it, `{peer}` the short `PeerId` that tells
    /// the two apart (human-client-ui.md §13). Plain text, never markdown.
    ContactTitle => "{name} ({peer})",
    /// CARD PROPOSAL, in a conversation, before Accept: what the
    /// counterparty says about themselves. `{name}` is the card's name and
    /// surname as plain text; the full `PeerId` is shown beside it, always.
    CardProposal => "This peer says they are {name}",
    /// CARD PROPOSAL: the control that writes the contact. Its context line
    /// (one of the two below) says what it will do before it runs.
    CardAccept => "Accept",
    /// CARD PROPOSAL: the control that discards the card, writing nothing.
    CardDecline => "Decline",
    /// CARD PROPOSAL, Accept's context line when no contact holds the
    /// card's route.
    CardWillCreate => "If you accept, a new contact is saved.",
    /// CARD PROPOSAL, Accept's context line when a contact holds the
    /// route: `{contact}` is that contact's name. The person's own edits
    /// stay, and the line says so.
    CardWillUpdate => "If you accept, your contact {contact} is updated. If you chose a name or photo yourself, it does not change.",
    /// CARD PROPOSAL, the same-name note: `{contact}` is a contact of this
    /// identity with the same name on another `PeerId`.
    CardSameName => "One of your contacts is named {contact} and has a different PeerId. Compare the two PeerIds before you accept.",
    /// CARD PROPOSAL: the card came from another identity of a device one
    /// of this identity's contacts is on; `{contact}` is that contact.
    CardSharesNode => "This card comes from the same device as your contact {contact}.",
    /// CARD PROPOSAL, a key change, before Accept: the words human-client-
    /// ui.md §3 asks for. `{old}` and `{new}` are the recorded and the
    /// presented fingerprints, 16 hex characters each, in that order.
    CardKeyChange => "The identity key in this conversation has changed. The person might have installed InterWeave again, or this might be someone else. Previous key fingerprint: {old}. New key fingerprint: {new}.",
    /// CARD PROPOSAL: the card's key is one a contact of this identity
    /// holds on another `PeerId`; `{contact}` is that contact.
    CardKeyMatches => "This card has the same identity key as your contact {contact}, but it comes from a different PeerId.",
    /// A conversation's action: send this identity's own card to the
    /// counterparty. Offered in a direct conversation only, never sent
    /// on its own.
    ShareCard => "Share a card",
    /// The identity and contact pages: the label beside an identity key's
    /// fingerprint, 16 lowercase hex characters.
    Fingerprint => "Key fingerprint",
    /// PEERS BY ADDRESS (ADR-0062; design/2026-10-22-add-a-peer.md
    /// §9, every value below language-culture-en's except where marked):
    /// the Chats menu item, the page's title and the no-reach notice's
    /// action.
    PeersTitle => "Peers",
    /// Peers and Connect: opens the add page.
    AddAddress => "Add an address",
    /// Peers and Connect: opens the show page.
    ShowMyAddress => "Show my address",
    /// Under Peers' two actions: an address is not trust.
    PeersNoTrust => "An address lets this device reach a peer. Adding an address does not mean that you trust the peer. Trust is a separate step.",
    /// In `PeersNoTrust`'s place while the trust list is unread or not
    /// confirmed, with or without Try again: no row's trust is claimed.
    PeersTrustUnread => "The list of trusted peers could not be read, so this page does not show whether each peer is trusted.",
    /// A row's state: its peer is trusted, so this device dials it --
    /// never that the peer is reachable.
    PeerStateTrusted => "Trusted: this device tries to connect to this address",
    /// A row's state: its peer is not trusted.
    PeerStateNeedsTrust => "Not trusted: this device does not connect to this address",
    /// A row's state: its peer is this device, whatever trust says.
    PeerStateThisDevice => "Own address: this address belongs to this device and cannot be used to reach another device",
    /// A row's state: the configuration turns the list off.
    PeerStateProviderOff => "Not used: this device's configuration turns this list off",
    /// A row's origin: the device's configuration.
    PeerFromConfig => "From this device's configuration",
    /// A row's menu, its accessible name; `{peer}` the short `PeerId`.
    PeerMenu => "Options for {peer}",
    /// A row's menu item.
    CopyAddress => "Copy address",
    /// An added row's menu item.
    RemoveAddress => "Remove",
    /// A configuration row's menu, in place of Remove.
    PeerConfigNotRemovable => "Set in this device's configuration, so it cannot be removed here.",
    /// Peers with no address.
    PeersEmpty => "No addresses yet. Add the address of someone you want to reach, or show yours so they can add you.",
    /// Under Add an address at the bound; `{max}` the bound.
    PeersLimit => "This device can hold at most {max} addresses. Remove one to add another.",
    /// Under Add an address when the configuration turns the list off.
    PeersListOff => "This device's configuration turns the list of addresses off.",
    /// The snackbar after removing a row whose list is off: no Undo.
    ConfirmAddressRemovedFinal => "Address removed. It cannot be added again while this device's configuration turns the list off.",
    /// The snackbar after a removal, with Undo.
    ConfirmAddressRemoved => "Address removed",
    /// The same, when the removed address's peer is still trusted.
    ConfirmAddressRemovedTrusted => "Address removed. You still trust this peer.",
    /// The snackbar after a failed Undo, with Try again.
    UndoFailed => "The address could not be added back. It is still removed.",
    /// The add page's field.
    AddressField => "Address",
    /// Under the field.
    AddressHelp => "An address starts like /ip4/192.168.1.20/tcp/4001/p2p/ and ends with a PeerId. Ask the other person to show theirs.",
    /// Android: opens the scanner.
    ScanCode => "Scan a code",
    /// Desktop: opens a file dialog for an image with a code.
    OpenCodeImage => "Open an image…",
    /// Desktop: reads an image with a code from the clipboard.
    PasteCodeImage => "Paste an image",
    /// The add page's primary.
    AddAction => "Add",
    /// Under the field: the add stored nothing. `{line}` the line,
    /// `{reason}` one Refused* value.
    AddRefused => "Not added: {line}. {reason}",
    /// The refusal's first drawn text, above the refused line, when it
    /// names one, and the reason; with no line, it and the reason read
    /// together are the label.
    AddRefusedLead => "Not added:", // PLACEHOLDER (en)
    /// A refusal: the grammar.
    RefusedGrammar => "This is not an InterWeave address.",
    /// A refusal: a special-use address.
    RefusedSpecialUse => "This address cannot be reached from another device.",
    /// A refusal: a line naming this device's own `PeerId`, refused
    /// before sending, and the runtime's `own_peer`.
    RefusedOwnAddress => "This is this device's own address, so it cannot be used to reach another device. Add the other person's address instead.",
    /// A refusal: a DNS name in a scanned code.
    RefusedDnsFromScan => "This address uses a name instead of numbers, and a scanned code cannot add such an address. If you know that the address is correct, type it in the field.",
    /// A refusal: a code over its bound; `{max}` the code's bound.
    RefusedTooMany => "This code has more than {max} addresses.",
    /// A refusal: the device's bound; `{max}` that bound.
    RefusedLimit => "This device already holds {max} addresses.",
    /// A refusal: the list is off.
    RefusedListOff => "This device's configuration turns the list of addresses off, so no address can be added.",
    /// Step one's result, one line; `{peer}` the short `PeerId`.
    AddedLineOne => "Added 1 address for {peer}.",
    /// Step one's result, `{n}` lines.
    AddedLineOther => "Added {n} addresses for {peer}.",
    /// Step one's result: the one line was already there.
    AddedUnchangedOne => "Nothing was added. This device already has this address.",
    /// Step one's result: every line was already there.
    AddedUnchangedOther => "Nothing was added. This device already has these addresses.",
    /// Step two's block title; `{peer}` the short `PeerId`.
    TrustStepTitle => "Do you trust {peer}?",
    /// Step two's block body.
    TrustStepBody => "InterWeave connects to this address only after you trust this PeerId. Trust lets this peer exchange messages with you. Before you choose Trust, compare this PeerId with the one on the other person's screen.",
    /// Step two: both ways.
    TrustStepBoth => "The other person also has to add your address and trust you. Each device connects only to peers that it trusts, so nothing connects until both of you have done this.",
    /// Step two's Trust, and a row's.
    TrustThisPeer => "Trust",
    /// Step two's Not now, and Connect's.
    NotNow => "Not now",
    /// A block's state once trusted.
    TrustStepDone => "Trusted",
    /// A block's state after Not now.
    TrustStepLater => "Not trusted yet",
    /// A block's state while its Trust awaits its answer, beside a
    /// progress mark: claims no outcome.
    TrustStepWaiting => "InterWeave is trusting {peer}. This is not finished yet.",
    /// A block's state after Not now or Done while its peer's trust is
    /// not known: claims no trust state.
    TrustStepLaterUnknown => "You did not choose Trust. InterWeave could not check whether this peer is already trusted.",
    /// The snackbar when every block was trusted.
    ConfirmPeerTrusted => "Peer trusted",
    /// The snackbar when an address was added.
    ConfirmAddressAdded => "Address added",
    /// Announced only, before focus moves, when focus was asked for a
    /// row a later read no longer has, after the person's own act on it:
    /// claims nothing about who removed it or about trust.
    PeerRowGone => "The address you acted on is no longer in the list of addresses.",
    /// Chats' notice: the list and the trust set are both empty.
    NoReachNotice => "InterWeave cannot reach anyone yet. Add a peer's address, then trust the peer.",
    /// Under the scan controls: a scan pressed with an address in the
    /// field.
    ScanAfterField => "The field already has an address. Choose Add, or clear the field. Then try again.",
    /// Under the field: Add with nothing in it.
    AddressNeeded => "The field is empty, so nothing was added. Type or paste an address, then choose Add.",
    /// Under Add: the call failed, nothing was changed.
    AddFailed => "The address could not be saved. Nothing was changed. Try again.",
    /// Under Add: the answer was lost and the list could not be read back;
    /// adding again is safe. One line.
    AddUnknownOne => "InterWeave could not check whether the address was saved. Choose Add to try again. If an address is already saved, it is not saved again.",
    /// The same, several lines.
    AddUnknownOther => "InterWeave could not check whether the addresses were saved. Choose Add to try again. If an address is already saved, it is not saved again.",
    /// The snackbar after a removal not made, with Try again.
    RemoveFailed => "The address could not be removed. It is still in the list.",
    /// The snackbar after a removal whose answer was lost and whose list
    /// could not be read back, with Try again: says neither.
    RemoveUnknown => "InterWeave could not check whether the address was removed. Removing it again does no harm.",
    /// The snackbar after an Undo likewise.
    UndoUnknown => "InterWeave could not check whether the address was added back. Adding it back again does no harm.",
    /// Step one's result after a read back that lists one line: never
    /// that this add made it.
    AddedCheckedOne => "This device has this address.",
    /// The same, `{n}` lines.
    AddedCheckedOther => "This device has these {n} addresses.",
    /// Step one's result for several peers; `{count}` the peers.
    AddedLineSeveral => "Added {n} addresses in total, for {count} peers.",
    /// Under the field: typed lines past the bound; `{n}` the lines,
    /// `{max}` the bound.
    RefusedTooManyLines => "The field has {n} addresses. You can add at most {max} at one time. Delete some from the field, then choose Add.",
    /// Peers, in place of the list: anything but the runtime unreachable.
    PeersReadFailed => "The list of addresses could not be read.",
    /// A step-two block: the trust answer was lost and could not be
    /// checked; `{peer}` the short `PeerId`.
    TrustStepUnknown => "InterWeave could not check whether {peer} is trusted. Choose Trust to try again. Trusting a peer twice does no harm.",
    /// A step-two block: the trust read back after a lost answer does not
    /// list the peer.
    TrustStepNotMade => "{peer} is still not trusted. Your choice of Trust was not saved. Choose Trust to try again.",
    /// Under Add: a daemon with the one-address add only did not take
    /// several lines; `{n}` the lines. Daemon only: the embedded runtime
    /// is this app's own version.
    AddSeveralIncompatible => "The transport daemon is an older version that can add only one address at a time, so these {n} addresses were not added. Put one address in the field, then choose Add. Or update InterWeave and the transport daemon to the same version.",
    /// Desktop: an image with no code in it.
    ImageNoCode => "No code was found in this image.",
    /// A code that carries no address.
    ScanNotAddress => "This code does not contain an InterWeave address.",
}

/// Where the transport runtime runs, which decides how a text that names
/// it reads (relay 01a12495, after language-culture-en's 01a12491): a
/// daemon of its own on the desktop, the app's own network service on
/// Android, which has no daemon (human-client-android.md, "Deployment
/// decision"). Chosen once by each app's composition root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeHost {
    /// A separate process the person starts: the desktop's transport
    /// daemon (ADR-0040).
    Daemon,
    /// The runtime inside the app's own foreground service (ADR-0041).
    Embedded,
}

impl RuntimeHost {
    /// Both, for the tests that walk every text.
    pub const ALL: &'static [Self] = &[Self::Daemon, Self::Embedded];
}

/// The interface texts that name where the runtime runs: one key per
/// situation, a text per [`RuntimeHost`], so a view cannot show the
/// desktop's daemon on a phone -- it reads them only through
/// [`placeholder_en::host_text`], which asks for the host.
macro_rules! host_texts {
    ($($(#[$doc:meta])* $variant:ident => { daemon: $daemon:literal, embedded: $embedded:literal $(,)? },)+) => {
        /// An interface text whose words depend on where the runtime runs.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum HostText {
            $($(#[$doc])* $variant,)+
        }

        impl HostText {
            /// Every one, from the same list as the enum.
            pub const ALL: &'static [Self] = &[$(Self::$variant,)+];
        }

        /// The placeholder English for `text` on `host`.
        const fn host_text_en(host: RuntimeHost, text: HostText) -> &'static str {
            match (host, text) {
                $(
                    (RuntimeHost::Daemon, HostText::$variant) => $daemon,
                    (RuntimeHost::Embedded, HostText::$variant) => $embedded,
                )+
            }
        }
    };
}

host_texts! {
    /// PEERS BY ADDRESS (design/2026-10-22-add-a-peer.md §9), in place of
    /// the list: the runtime cannot be reached.
    PeersReadUnavailable => {
        daemon: "The list of addresses cannot be read, because the transport daemon cannot be reached.",
        embedded: "The list of addresses cannot be read, because the network service is not running.",
    },
    /// Peers and the add page: this app may not administer the addresses.
    PeersNotPermitted => {
        daemon: "This app is not allowed to read or change the list of addresses on this transport daemon.",
        embedded: "This app is not allowed to read or change the list of addresses.",
    },
    /// In `PeersNoTrust`'s place: this app may not read the trust list, so
    /// no row's trust is shown; no Try again.
    PeersTrustNotPermitted => {
        daemon: "This app is not allowed to read the list of trusted peers on this transport daemon, so this page does not show whether each peer is trusted.",
        embedded: "This app is not allowed to read the list of trusted peers, so this page does not show whether each peer is trusted.",
    },
    /// Peers and the add page: the runtime predates the methods.
    PeersIncompatible => {
        daemon: "The transport daemon is a different version from this app, so the list of addresses cannot be read or changed here. To use Peers, update InterWeave and the transport daemon to the same version.",
        embedded: "This version of the app cannot read or change the list of addresses.",
    },
    /// Nothing runs the transport for this profile; the window connects
    /// once it starts. On the desktop the person starts the daemon; on
    /// Android the app starts its own service.
    NotRunning => {
        daemon: "The transport daemon for this profile is not running. This window will connect when the daemon starts.",
        embedded: "The network service is not running. The app will connect when the service starts.",
    },
    /// Trust could not be read or changed: the runtime is not reachable.
    TrustUnavailable => {
        daemon: "The transport daemon cannot be reached. Nothing was changed.",
        embedded: "The network service is not running. Nothing was changed.",
    },
    /// This client may not administer trust on this runtime. Not expected
    /// on Android, where the settings hold the in-process admin port; not
    /// yet shown unreachable there (relay 01a12495).
    TrustNotPermitted => {
        daemon: "This app is not allowed to change trust on this transport daemon. Nothing was changed.",
        embedded: "This app is not allowed to change trust. Nothing was changed.",
    },
    /// The runtime refused the change: its own identity, or the list is
    /// full.
    TrustRefused => {
        daemon: "The transport daemon refused this change. Nothing was changed.",
        embedded: "The network service refused this change. Nothing was changed.",
    },
    /// The runtime and the app are not the same release, so they share no
    /// version trust needs: what helps is bringing both to one release,
    /// not trying again. Not expected on Android, one package holding
    /// both; not yet shown unreachable there.
    TrustIncompatible => {
        daemon: "The transport daemon is a different version from this app, so trust cannot be read or changed here. Nothing was changed.",
        embedded: "This version of the app cannot read or change trust. Nothing was changed.",
    },
    /// The runtime did not confirm a change, which may have been made: the
    /// list is read again. `{peer}` the exact `PeerId`, whole. Never says
    /// that nothing changed (TRANSPORT.md's outcome-unknown class).
    TrustUnconfirmed => {
        daemon: "The transport daemon did not confirm the change for {peer}. The change may have been made. The list of trusted peers is being read again.",
        embedded: "The network service did not confirm the change for {peer}. The change may have been made. The list of trusted peers is being read again.",
    },
    /// A conversation's banner while the device is offline (#304 F7): no
    /// dismiss, gone when the state leaves offline.
    OfflineBanner => {
        daemon: "InterWeave is offline: the transport daemon is not running. What you write now is sent when the transport daemon starts.",
        embedded: "InterWeave is offline: the network service is not running. What you write now is sent when the network service starts.",
    },
    /// A0 after the local runtime was unavailable (F10 A4).
    WaitingSendingUnavailable => {
        daemon: "Still sending, started {age}. Last attempt: the transport daemon was not available",
        embedded: "Still sending, started {age}. Last attempt: the network service was not available",
    },
    /// B0 with A4's problem (B4).
    WaitingMaybeUnavailable => {
        daemon: "May have reached the peer. Still sending, started {age}. Last attempt: the transport daemon was not available",
        embedded: "May have reached the peer. Still sending, started {age}. Last attempt: the network service was not available",
    },
}

/// The English the client shows.
///
/// Every value here was read by language-culture (relay message
/// 01a12001-199d-75d9-8e8d-cceb01ea9bbf): person-facing copy is never
/// self-authored, so a value added later ships as a placeholder its
/// author drafts and language-culture finalises. A drafted value carries
/// a line comment at the end of its line, or on the line above it when
/// the value wraps: the word PLACEHOLDER in capitals, a space, and the
/// locale in parentheses, `(en)`. This paragraph spells the mark out
/// rather than writing it, so a search for the mark finds only values.
/// The commit that takes language-culture's final text removes it, so
/// that search lists every value still a draft. The module keeps its
/// Stage 14 name, from when these words shipped unreviewed under
/// architect-cto's ruling on relay message
/// 01a0fe85-6b39-7d6e-8b2a-f0cc4280c7a8. What every value must pass is
/// structural, and the tests below hold it: no delivery label reads as
/// read, seen, processed or delivered (P6), `Unknown` connectivity never
/// reads as offline, and every template is filled by placeholder, never
/// assembled by concatenation, so a translation may reorder it.
pub mod placeholder_en {
    use super::{
        Connectivity, EntryProblem, ErrorClass, HostText, LabelKey, PeerPath, RuntimeHost,
        TrustProblem, UiText,
    };

    /// A status label.
    #[must_use]
    pub const fn label(key: LabelKey) -> &'static str {
        match key {
            LabelKey::Sending => "Sending",
            LabelKey::NotConfirmed => "Not confirmed, trying again",
            LabelKey::NeedsAttention => "Not sent, send again or cancel",
            LabelKey::NeedsAttentionMayHaveBeenReceived => {
                "May have reached the peer, send again or cancel"
            }
            LabelKey::AcceptedByRemoteTransport => "Accepted by remote transport",
            LabelKey::PublishedLocally => "Accepted by local transport",
            LabelKey::Cancelled => "Canceled",
            LabelKey::CancelledMayHaveBeenReceived => "Canceled, may have reached the peer",
            LabelKey::Unread => "Unread",
            LabelKey::ReadNotKept => "Read, not kept",
            LabelKey::Kept => "Kept",
        }
    }

    /// An error class's message (`human-client-ui.md` §12), as it reads
    /// where the runtime runs on `host`.
    #[must_use]
    pub const fn error(host: RuntimeHost, class: ErrorClass) -> &'static str {
        match class {
            ErrorClass::TransportUnavailable => match host {
                RuntimeHost::Daemon => "The transport daemon cannot be reached.",
                RuntimeHost::Embedded => "The network service is not running.",
            },
            ErrorClass::PeerNotTrusted => "This peer is not trusted for this profile.",
            ErrorClass::RouteUnavailable => "This route is not available now.",
            ErrorClass::NoNetworkPath => "This peer cannot be reached over the network now.",
            ErrorClass::Busy => "The transport is temporarily busy.",
            ErrorClass::Incompatible => "The two sides have no protocol version in common.",
            ErrorClass::TooLarge => "The message is too large to send.",
            ErrorClass::NotConfigured => "This route or channel is not configured here.",
            ErrorClass::InvalidMessage => "The message cannot be sent in this form.",
            ErrorClass::StorageUnavailable => "The message cannot be stored on this device.",
            ErrorClass::AlreadyPending => "This message is already waiting to be sent.",
            ErrorClass::EndpointInUse => "Another app or window is already using this endpoint.",
            ErrorClass::EndpointNotAvailable => "This endpoint is not available to this app.",
            ErrorClass::Internal => "Something went wrong.",
        }
    }

    /// A direct conversation's route indicator (`human-client-ui.md` §7).
    #[must_use]
    pub const fn path(path: PeerPath) -> &'static str {
        text(match path {
            PeerPath::Direct => UiText::PathDirect,
            PeerPath::Relayed => UiText::PathRelayed,
        })
    }

    /// Connectivity as `human-client-ui.md` §7 normalizes it, as it
    /// reads where the runtime runs on `host`.
    #[must_use]
    pub const fn connectivity(host: RuntimeHost, state: Connectivity) -> &'static str {
        match state {
            Connectivity::OnlineDirect => "Online, reachable directly",
            Connectivity::OnlineRelay => "Online, reachable through a relay",
            Connectivity::OnlinePartial => "Online, some peers might not reach you",
            Connectivity::Offline => match host {
                RuntimeHost::Daemon => "Offline, transport daemon not running",
                RuntimeHost::Embedded => "Offline, network service not running",
            },
            Connectivity::Unknown => "Network status not known yet",
        }
    }

    /// Any other interface text.
    #[must_use]
    pub const fn text(text: UiText) -> &'static str {
        super::ui_text_en(text)
    }

    /// An interface text that names where the runtime runs, as it reads
    /// on `host`.
    #[must_use]
    pub const fn host_text(host: RuntimeHost, text: HostText) -> &'static str {
        super::host_text_en(host, text)
    }

    /// Why trust was not read or changed, as it reads on `host`.
    #[must_use]
    pub const fn trust_problem(host: RuntimeHost, problem: TrustProblem) -> &'static str {
        match problem {
            TrustProblem::Unavailable => host_text(host, HostText::TrustUnavailable),
            TrustProblem::NotPermitted => host_text(host, HostText::TrustNotPermitted),
            TrustProblem::Refused => host_text(host, HostText::TrustRefused),
            TrustProblem::Incompatible => host_text(host, HostText::TrustIncompatible),
            TrustProblem::Internal => text(UiText::TrustFailed),
        }
    }

    /// Why a typed `PeerId` was not proposed.
    #[must_use]
    pub const fn entry_problem(problem: EntryProblem) -> &'static str {
        text(match problem {
            EntryProblem::NotAPeerId => UiText::NotAPeerId,
            EntryProblem::OwnIdentity => UiText::OwnIdentity,
            EntryProblem::AlreadyTrusted => UiText::AlreadyTrusted,
        })
    }
}

/// A `PeerId` as a view shows it in a list: its last eight characters,
/// verbatim, in the `ShortPeer` template. The full id is shown where a
/// person can read it whole (§11).
#[must_use]
pub fn short_peer(id: &str) -> String {
    let tail = &id[id.len().saturating_sub(8)..];
    fill(placeholder_en::text(UiText::ShortPeer), &[("tail", tail)])
}

/// A link's destination as a view shows it on the link's control: every
/// control character, and every character Unicode makes default-ignorable
/// -- drawn as nothing, or as a change of direction or joining -- is shown
/// as its code point, `<U+202E>`, and every other character as it is. A
/// remote sender can otherwise put a right-to-left override in a
/// destination so that the label reads as one address while the opener
/// is given another; with it shown, no directional control reorders the
/// label. Right-to-left LETTERS are laid out by the ordinary bidirectional
/// rules, as in any text. The destination itself is not changed: what
/// opens is what was sent.
#[must_use]
pub fn visible_destination(destination: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(destination.len());
    for c in destination.chars() {
        if is_hidden(c) {
            // Writing to a String cannot fail.
            let _ = write!(out, "<U+{:04X}>", u32::from(c));
        } else {
            out.push(c);
        }
    }
    out
}

/// Unicode's `Default_Ignorable_Code_Point` property
/// (`DerivedCoreProperties`, Unicode 16), with the interlinear annotation characters beside it:
/// what a renderer draws as nothing or applies rather than shows. A test
/// walks every range.
const HIDDEN: &[(char, char)] = &[
    ('\u{00AD}', '\u{00AD}'),
    ('\u{034F}', '\u{034F}'),
    ('\u{061C}', '\u{061C}'),
    ('\u{115F}', '\u{1160}'),
    ('\u{17B4}', '\u{17B5}'),
    ('\u{180B}', '\u{180F}'),
    ('\u{200B}', '\u{200F}'),
    ('\u{202A}', '\u{202E}'),
    ('\u{2060}', '\u{206F}'),
    ('\u{3164}', '\u{3164}'),
    ('\u{FE00}', '\u{FE0F}'),
    ('\u{FEFF}', '\u{FEFF}'),
    ('\u{FFA0}', '\u{FFA0}'),
    ('\u{FFF0}', '\u{FFFB}'),
    ('\u{1BCA0}', '\u{1BCA3}'),
    ('\u{1D173}', '\u{1D17A}'),
    ('\u{E0000}', '\u{E0FFF}'),
];

/// A control character, or one in [`HIDDEN`].
fn is_hidden(c: char) -> bool {
    c.is_control() || HIDDEN.iter().any(|&(low, high)| (low..=high).contains(&c))
}

/// `template` with each `{name}` replaced by its value, inserted verbatim
/// -- an id is never translated or reshaped (U4b). A name the template
/// does not hold is ignored; a placeholder left unfilled stays visible.
/// The test below holds that every template is filled by its own names;
/// that each CALL passes those names is held where the calls are, by
/// ui-slint's sweep of the rendered tree.
#[must_use]
pub fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let name = &after[..close];
        if let Some((_, value)) = values.iter().find(|(n, _)| *n == name) {
            out.push_str(value);
        } else {
            out.push_str(&rest[open..=open + 1 + close]);
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

/// The label for an outbound status.
#[must_use]
pub const fn outbound_label(status: &OutboundStatus) -> LabelKey {
    match status {
        OutboundStatus::Sending { .. } => LabelKey::Sending,
        OutboundStatus::Unconfirmed { .. } => LabelKey::NotConfirmed,
        OutboundStatus::NeedsAttention {
            may_have_reached: false,
            ..
        } => LabelKey::NeedsAttention,
        OutboundStatus::NeedsAttention {
            may_have_reached: true,
            ..
        } => LabelKey::NeedsAttentionMayHaveBeenReceived,
        OutboundStatus::Accepted { .. } => LabelKey::AcceptedByRemoteTransport,
        OutboundStatus::Published => LabelKey::PublishedLocally,
        OutboundStatus::Cancelled {
            may_have_reached: false,
        } => LabelKey::Cancelled,
        OutboundStatus::Cancelled {
            may_have_reached: true,
        } => LabelKey::CancelledMayHaveBeenReceived,
    }
}

/// A user-actionable error class: one stable key per message
/// `human-client-ui.md` §12 asks for. The raw code stays in diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorClass {
    /// The peer is not trusted for this profile.
    PeerNotTrusted,
    /// The selected route is currently unavailable.
    RouteUnavailable,
    /// No usable network path.
    NoNetworkPath,
    /// The remote or local transport is temporarily busy.
    Busy,
    /// The local transport is unavailable.
    TransportUnavailable,
    /// The two sides do not speak a common protocol version.
    Incompatible,
    /// The message is too large to send.
    TooLarge,
    /// This route or channel is not configured for this client.
    NotConfigured,
    /// The message is not one a receiver would accept.
    InvalidMessage,
    /// Local storage cannot hold it.
    StorageUnavailable,
    /// The same message is already pending.
    AlreadyPending,
    /// This local endpoint is already owned by another client or session.
    EndpointInUse,
    /// The endpoint is not available to this client.
    EndpointNotAvailable,
    /// Anything else; the raw code is in diagnostics.
    Internal,
}

/// The class of a send problem a row reports.
#[must_use]
pub const fn send_problem_class(problem: SendProblem) -> ErrorClass {
    match problem {
        SendProblem::PeerUntrusted => ErrorClass::PeerNotTrusted,
        SendProblem::RouteUnavailable => ErrorClass::RouteUnavailable,
        SendProblem::NoNetworkPath => ErrorClass::NoNetworkPath,
        SendProblem::Busy => ErrorClass::Busy,
        SendProblem::ServiceUnavailable => ErrorClass::TransportUnavailable,
        SendProblem::Incompatible => ErrorClass::Incompatible,
        SendProblem::TooLarge => ErrorClass::TooLarge,
        SendProblem::NotConfigured => ErrorClass::NotConfigured,
        SendProblem::Internal => ErrorClass::Internal,
    }
}

/// Why a pending row needs the person, or `None` for a row that does
/// not: a needs-attention row shows its problem's class beside its status
/// (design/2026-10-16-android-look.md section 6, Composer), since the
/// facade will not retry it and the status alone does not say what to fix.
#[must_use]
pub const fn attention_class(status: &OutboundStatus) -> Option<ErrorClass> {
    match status {
        OutboundStatus::NeedsAttention { problem, .. } => Some(send_problem_class(*problem)),
        _ => None,
    }
}

/// The class of a send the facade refused with no row.
#[must_use]
pub const fn send_error_class(error: &SendError) -> ErrorClass {
    match error {
        SendError::TooLarge => ErrorClass::TooLarge,
        SendError::InvalidEnvelope => ErrorClass::InvalidMessage,
        SendError::NotConfigured => ErrorClass::NotConfigured,
        SendError::StorageUnavailable => ErrorClass::StorageUnavailable,
        SendError::AlreadyPending => ErrorClass::AlreadyPending,
    }
}

/// The class of a session refusal.
#[must_use]
pub const fn session_problem_class(problem: SessionProblem) -> ErrorClass {
    match problem {
        SessionProblem::EndpointInUse => ErrorClass::EndpointInUse,
        SessionProblem::NotAvailableToThisClient => ErrorClass::EndpointNotAvailable,
        SessionProblem::Internal => ErrorClass::Internal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The network-off notice tells the person which button to tap by its
    /// label: the two texts are one wording, so an edit to the button's
    /// label that leaves the notice behind fails here.
    #[test]
    fn the_network_off_notice_names_the_button_as_it_reads() {
        let button = placeholder_en::text(UiText::AllowNetwork);
        assert!(
            placeholder_en::text(UiText::NetworkOffBody).contains(button),
            "{:?} does not name {button:?}",
            placeholder_en::text(UiText::NetworkOffBody)
        );
    }

    #[test]
    fn every_label_appears_once_in_all() {
        let mut keys: Vec<&str> = LabelKey::ALL.iter().map(|l| l.key()).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), LabelKey::ALL.len(), "no label listed twice");
    }

    /// P6: no delivery label reads as read, seen or processed. Checked on
    /// the stable key AND the variant's name, since either is what a
    /// translator starts from.
    #[test]
    fn no_delivery_label_reads_as_read_seen_or_processed() {
        for label in LabelKey::ALL.iter().copied().filter(|l| l.is_delivery()) {
            let words = format!("{label:?} {}", label.key()).to_ascii_lowercase();
            for forbidden in ["read", "seen", "processed", "delivered"] {
                assert!(!words.contains(forbidden), "{label:?}: {forbidden}");
            }
        }
    }

    /// P6 on the words a person reads, not only the keys: no delivery
    /// label's English reads as read, seen, processed or delivered.
    #[test]
    fn no_delivery_text_reads_as_read_seen_or_processed() {
        for label in LabelKey::ALL.iter().copied().filter(|l| l.is_delivery()) {
            let words = placeholder_en::label(label).to_ascii_lowercase();
            for forbidden in ["read", "seen", "processed", "delivered"] {
                assert!(!words.contains(forbidden), "{label:?}: {words}");
            }
        }
    }

    #[test]
    fn unknown_connectivity_never_reads_as_offline() {
        for &host in RuntimeHost::ALL {
            let unknown = placeholder_en::connectivity(host, Connectivity::Unknown);
            assert!(
                !unknown.to_ascii_lowercase().contains("offline"),
                "{unknown}"
            );
            assert!(
                placeholder_en::connectivity(host, Connectivity::Offline)
                    .to_ascii_lowercase()
                    .contains("offline"),
                "the control: Offline does say so on {host:?}"
            );
        }
    }

    /// Every error class. The exhaustive match fails to compile on a
    /// class added to the enum until it is named there; the array must
    /// gain it too, which nothing enforces.
    fn every_class() -> [ErrorClass; 14] {
        const fn listed(class: ErrorClass) -> ErrorClass {
            match class {
                ErrorClass::PeerNotTrusted
                | ErrorClass::RouteUnavailable
                | ErrorClass::NoNetworkPath
                | ErrorClass::Busy
                | ErrorClass::TransportUnavailable
                | ErrorClass::Incompatible
                | ErrorClass::TooLarge
                | ErrorClass::NotConfigured
                | ErrorClass::InvalidMessage
                | ErrorClass::StorageUnavailable
                | ErrorClass::AlreadyPending
                | ErrorClass::EndpointInUse
                | ErrorClass::EndpointNotAvailable
                | ErrorClass::Internal => class,
            }
        }
        [
            ErrorClass::PeerNotTrusted,
            ErrorClass::RouteUnavailable,
            ErrorClass::NoNetworkPath,
            ErrorClass::Busy,
            ErrorClass::TransportUnavailable,
            ErrorClass::Incompatible,
            ErrorClass::TooLarge,
            ErrorClass::NotConfigured,
            ErrorClass::InvalidMessage,
            ErrorClass::StorageUnavailable,
            ErrorClass::AlreadyPending,
            ErrorClass::EndpointInUse,
            ErrorClass::EndpointNotAvailable,
            ErrorClass::Internal,
        ]
        .map(listed)
    }

    /// Every class, state and host text, as `host` reads it: what the
    /// platform-value tests walk. A `Connectivity` variant fails to
    /// compile until `state` names it, and an `ErrorClass` one until
    /// `every_class` does; each array must gain it too, which nothing
    /// enforces.
    fn host_dependent(host: RuntimeHost) -> Vec<&'static str> {
        const fn state(state: Connectivity) -> Connectivity {
            match state {
                Connectivity::OnlineDirect
                | Connectivity::OnlineRelay
                | Connectivity::OnlinePartial
                | Connectivity::Offline
                | Connectivity::Unknown => state,
            }
        }
        let classes = every_class();
        let states = [
            Connectivity::OnlineDirect,
            Connectivity::OnlineRelay,
            Connectivity::OnlinePartial,
            Connectivity::Offline,
            Connectivity::Unknown,
        ];
        let mut texts: Vec<&str> = classes
            .iter()
            .map(|c| placeholder_en::error(host, *c))
            .collect();
        texts.extend(
            states
                .iter()
                .map(|s| placeholder_en::connectivity(host, state(*s))),
        );
        texts.extend(
            HostText::ALL
                .iter()
                .map(|t| placeholder_en::host_text(host, *t)),
        );
        texts
    }

    /// The punctuation the status templates are written around: a
    /// status fills `{status}` before a comma or a period the template
    /// adds, so no status ends with one; an error class fills `{reason}`
    /// last, so each is a sentence with its own period (`ItemWithReason`,
    /// `AnnounceOwnStatusReason`, `NotSent`, `Refused`).
    #[test]
    fn statuses_end_bare_and_every_reason_ends_with_a_period() {
        // Listed by hand: a waiting text added later is not walked here
        // until it is added.
        let waiting = [
            UiText::WaitingSending,
            UiText::WaitingSendingNoPath,
            UiText::WaitingSendingRoute,
            UiText::WaitingSendingBusy,
            UiText::WaitingMaybe,
            UiText::WaitingMaybeNoPath,
            UiText::WaitingMaybeRoute,
            UiText::WaitingMaybeBusy,
            UiText::UnreadAlsoKept,
        ]
        .map(placeholder_en::text);
        for &host in RuntimeHost::ALL {
            let unavailable = [
                HostText::WaitingSendingUnavailable,
                HostText::WaitingMaybeUnavailable,
            ]
            .map(|t| placeholder_en::host_text(host, t));
            let statuses = LabelKey::ALL
                .iter()
                .map(|l| placeholder_en::label(*l))
                .chain(waiting)
                .chain(unavailable);
            for status in statuses {
                assert!(!status.ends_with('.'), "{host:?}: {status}");
            }
            for class in every_class() {
                let reason = placeholder_en::error(host, class);
                assert!(
                    reason.ends_with('.') && !reason.ends_with(".."),
                    "{host:?} {class:?}: {reason}"
                );
            }
        }
    }

    /// Android has no daemon (relay 01a12495): no text it can show names
    /// one. The desktop's do, the control that the walk reaches them.
    #[test]
    fn no_embedded_text_names_a_daemon() {
        for text in host_dependent(RuntimeHost::Embedded) {
            assert!(!text.to_ascii_lowercase().contains("daemon"), "{text}");
        }
        assert!(
            host_dependent(RuntimeHost::Daemon)
                .iter()
                .filter(|t| t.contains("transport daemon"))
                .count()
                >= 8,
            "the control: the desktop names its daemon"
        );
    }

    /// Every template is filled by name, and filling leaves no
    /// placeholder behind: a template is one string a translation may
    /// reorder, never pieces joined in code (U4b).
    #[test]
    fn every_template_is_filled_by_name() {
        let values = [
            ("count", "3"),
            ("route", "human"),
            ("author", "…abcd1234"),
            ("status", "Unread"),
            ("body", "hi"),
            ("reason", "busy"),
            ("kind", "Channel"),
            ("unread", "2 unread"),
            ("preview", "You: hi"),
            ("time", "01:18"),
            ("message", "hi"),
            ("month", "Oct"),
            ("day", "9"),
            ("year", "2025"),
            ("hour", "01"),
            ("minute", "18"),
            ("tail", "abcd1234"),
            ("peer", "…abcd1234"),
            ("destination", "https://example.org/a"),
            ("alt", "a cat"),
            ("conversation", "…abcd1234 / human"),
            ("first", "New message from …abcd1234."),
            ("rest", "Your message: Sent."),
            ("n", "7"),
            ("age", "3 hours ago"),
            ("name", "Ada Lovelace"),
            ("contact", "Ada"),
            ("old", "0123456789abcdef"),
            ("new", "fedcba9876543210"),
            ("max", "64"),
            ("count", "2"),
            ("line", "/ip4/192.168.1.20/tcp/4001/p2p/x"),
        ];
        for text in UiText::ALL {
            let filled = fill(placeholder_en::text(*text), &values);
            assert!(!filled.contains('{'), "{text:?}: {filled}");
        }
        for &host in RuntimeHost::ALL {
            for text in HostText::ALL {
                let filled = fill(placeholder_en::host_text(host, *text), &values);
                assert!(!filled.contains('{'), "{host:?} {text:?}: {filled}");
            }
        }
        assert_eq!(fill("route: {route}", &[("route", "a{b}")]), "route: a{b}");
        assert_eq!(
            fill("{missing}", &[]),
            "{missing}",
            "an unfilled one stays visible"
        );
    }

    #[test]
    fn a_destinations_hidden_characters_are_shown_and_the_rest_kept() {
        assert_eq!(
            visible_destination("https://evil.example/#\u{202E}elpmaxe.knab//:sptth"),
            "https://evil.example/#<U+202E>elpmaxe.knab//:sptth",
            "an override reads as its code point"
        );
        // Named apart from the table, at least one from each of its ranges,
        // so a range dropped from it or narrowed at an end fails here.
        let mut every: Vec<char> = vec![
            '\u{0007}',
            '\u{009F}',
            '\u{00AD}',
            '\u{034F}',
            '\u{061C}',
            '\u{115F}',
            '\u{1160}',
            '\u{17B4}',
            '\u{17B5}',
            '\u{180B}',
            '\u{180F}',
            '\u{200B}',
            '\u{200E}',
            '\u{200F}',
            '\u{202A}',
            '\u{202D}',
            '\u{2060}',
            '\u{2066}',
            '\u{2069}',
            '\u{206F}',
            '\u{3164}',
            '\u{FE00}',
            '\u{FE0F}',
            '\u{FEFF}',
            '\u{FFA0}',
            '\u{FFF0}',
            '\u{FFF9}',
            '\u{FFFB}',
            '\u{1BCA0}',
            '\u{1BCA3}',
            '\u{1D173}',
            '\u{1D17A}',
            '\u{E0000}',
            '\u{E0041}',
            '\u{E0100}',
            '\u{E0FFF}',
        ];
        for &(low, high) in HIDDEN {
            every.extend([low, high]);
            if let Some(middle) = char::from_u32(u32::midpoint(u32::from(low), u32::from(high))) {
                every.push(middle);
            }
        }
        for hidden in every {
            let shown = visible_destination(&format!("https://a.example/{hidden}x"));
            assert!(
                !shown.contains(hidden) && shown.contains("<U+"),
                "{:04X} is shown: {shown}",
                u32::from(hidden)
            );
        }
        let ordinary = "https://bücher.example/straße?q=日本&x=1#a-b_c~d";
        assert_eq!(
            visible_destination(ordinary),
            ordinary,
            "letters of any script stay as they are"
        );
    }

    #[test]
    fn a_short_peer_is_the_ids_tail_verbatim() {
        assert_eq!(short_peer("12D3KooWABCDEFGH12345678"), "…12345678");
        assert_eq!(short_peer("abc"), "…abc");
    }

    /// Pairs whose English is one word and whose meaning is not:
    /// another language may need two words, so each keeps its key.
    const ONE_ENGLISH_WORD: &[(UiText, UiText)] = &[
        // The trust settings' heading, a noun; step two's button, a verb.
        (UiText::TrustHeading, UiText::TrustThisPeer),
    ];

    #[test]
    fn every_ui_text_appears_once_in_all() {
        for (a, b) in ONE_ENGLISH_WORD {
            assert_eq!(placeholder_en::text(*a), placeholder_en::text(*b));
        }
        for &host in RuntimeHost::ALL {
            let mut texts: Vec<&str> = UiText::ALL
                .iter()
                .filter(|t| !ONE_ENGLISH_WORD.iter().any(|(_, b)| b == *t))
                .map(|t| placeholder_en::text(*t))
                .chain(
                    HostText::ALL
                        .iter()
                        .map(|t| placeholder_en::host_text(host, *t)),
                )
                .collect();
            texts.sort_unstable();
            let n = texts.len();
            texts.dedup();
            assert_eq!(
                texts.len(),
                n,
                "no two interface texts read the same on {host:?}"
            );
        }
    }

    #[test]
    fn every_trust_problem_reads_apart() {
        // An exhaustive match: a variant added to `TrustProblem` fails to
        // compile here until `listed` names it. The compiler does not see the
        // array below; add the variant there too, and raise the count.
        const fn listed(problem: TrustProblem) -> TrustProblem {
            match problem {
                TrustProblem::Unavailable
                | TrustProblem::NotPermitted
                | TrustProblem::Refused
                | TrustProblem::Incompatible
                | TrustProblem::Internal => problem,
            }
        }
        for &host in RuntimeHost::ALL {
            let mut texts: Vec<&str> = [
                TrustProblem::Unavailable,
                TrustProblem::NotPermitted,
                TrustProblem::Refused,
                TrustProblem::Incompatible,
                TrustProblem::Internal,
            ]
            .map(|p| placeholder_en::trust_problem(host, listed(p)))
            .to_vec();
            texts.sort_unstable();
            texts.dedup();
            assert_eq!(
                texts.len(),
                5,
                "each problem says its own cause on {host:?}"
            );
        }
    }

    #[test]
    fn may_have_reached_gets_its_own_labels() {
        assert_ne!(
            outbound_label(&OutboundStatus::Cancelled {
                may_have_reached: true
            }),
            outbound_label(&OutboundStatus::Cancelled {
                may_have_reached: false
            })
        );
        assert_ne!(
            outbound_label(&OutboundStatus::NeedsAttention {
                problem: SendProblem::Busy,
                may_have_reached: true
            }),
            outbound_label(&OutboundStatus::NeedsAttention {
                problem: SendProblem::Busy,
                may_have_reached: false
            })
        );
    }

    #[test]
    fn endpoint_in_use_keeps_its_own_class() {
        assert_eq!(
            session_problem_class(SessionProblem::EndpointInUse),
            ErrorClass::EndpointInUse
        );
        assert_ne!(
            session_problem_class(SessionProblem::NotAvailableToThisClient),
            ErrorClass::EndpointInUse
        );
    }
}
