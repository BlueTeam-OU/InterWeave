// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Several user identities held by one client (plan §30 step 1): which
//! one is in front, what changes it, and the words every acting screen
//! uses to name it (design/2026-10-21-contact-cards.md §5.7, as
//! design/2026-11-04-identities.md §4, §5, §5.2, §5.3, §6.1 and §9 amend
//! it). Presentation state, no I/O: the composition root hands it the held
//! identities and the person's acts, and renders what it returns.
//!
//! An identity is held here by an opaque [`IdentityKey`]: the text of the
//! identity's derived `EndpointId` (ADR-0067 decision 2), which the root
//! hands over. The model only compares keys and shows the last eight
//! characters of one (`ShortPeer`'s template, identities §6.1;
//! `a_key_is_shown_by_its_last_eight_characters`), so it holds the text
//! and not the transport type: presentation state has no use for parsing
//! an id and no way to refuse one.
//!
//! With one identity held nothing of the naming is shown (ADR-0061 d. 3,
//! identities §4): every text below answers `None`, or its `One` form
//! where the design gives one
//! (`with_one_held_nothing_names_an_identity`).

use unicode_segmentation::UnicodeSegmentation as _;

use crate::identity_add::AddValues;
use crate::labels::{HostText, UiText, by_count, fill, isolated, placeholder_en};

/// How many identities a profile holds at most: the profile's endpoint
/// bound (`interweave-profile-config`'s `MAX_ENDPOINTS`), which this crate
/// does not depend on; `interweave-human-app-core` asserts the two equal at
/// compile time. [`Identities::held_changed`] refuses a longer list
/// and Add is refused at it (`more_than_the_bound_is_refused_and_add_stops_at_it`).
pub const IDENTITY_CAP: usize = 64;

/// The least time the transition before another identity's notification
/// opens is held (contact-cards §5.7.5: one second, a hypothesis).
pub const SWITCHING_HOLD_MS: u64 = 1_000;

/// An identity as the composition root names it. Opaque: it is compared,
/// and its last eight characters are shown, and nothing else is done with
/// it (`a_key_is_shown_by_its_last_eight_characters`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IdentityKey(String);

impl IdentityKey {
    /// The key the root hands over, taken as it is.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// The key as it was handed over.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The short form a row shows (`ShortPeer`'s template, identities
    /// §6.1): "…" and the key's last eight characters, counted by
    /// character; a shorter key whole after the "…".
    #[must_use]
    pub fn short(&self) -> String {
        let count = self.0.chars().count();
        let tail: String = self.0.chars().skip(count.saturating_sub(8)).collect();
        fill(placeholder_en::text(UiText::ShortPeer), &[("tail", &tail)])
    }
}

/// Whether an identity can act now (contact-cards §5.7.6, identities
/// §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdentityState {
    /// It holds its lease: it sends and receives.
    Ready,
    /// Asserting its advertise setting or taking its lease.
    /// `after_try_again` while a Try again runs: Try again stays in its
    /// place, disabled, with a progress indicator (§5.2).
    Starting {
        /// Whether this start is a Try again's.
        after_try_again: bool,
    },
    /// The advertise assertion failed, so no lease was taken (§5.2,
    /// `IdentityNotHidden`), with Try again.
    NotHidden,
    /// No lease, or its endpoint is turned off (cards §5.7.6).
    Unavailable,
}

/// One identity as the root reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldIdentity {
    /// The identity.
    pub key: IdentityKey,
    /// Its card's name, once it has a card.
    pub card_name: Option<String>,
    /// Its alias, local, the name it takes with no card (identities §6.1);
    /// empty for an identity with none, a first identity authored without
    /// one, which is then named by its short key ([`Self::name`]).
    pub alias: String,
    /// Whether it can act now.
    pub state: IdentityState,
    /// The first identity (`human`), which Remove is not offered on.
    pub is_first: bool,
    /// The default identity, which Remove is not offered on.
    pub is_default: bool,
    /// Its unread items.
    pub unread: u32,
    /// Whether a message of it was not sent.
    pub not_sent: bool,
}

impl HeldIdentity {
    /// The name it is shown by: its card's name, else its alias
    /// (identities §6.1), else its short key, `ShortEndpoint` as `ShortPeer`
    /// is built ([`IdentityKey::short`]): nothing is invented in an alias's
    /// place. An empty card name or alias is none.
    #[must_use]
    pub fn name(&self) -> std::borrow::Cow<'_, str> {
        match &self.card_name {
            Some(name) if !name.is_empty() => name.into(),
            _ if !self.alias.is_empty() => self.alias.as_str().into(),
            _ => self.key.short().into(),
        }
    }
}

/// An interface text and the values it is filled with, so a view fills
/// its own translation of it. Names are already isolated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said {
    /// The template.
    pub text: UiText,
    /// Its placeholders' values, by name.
    pub values: Vec<(&'static str, String)>,
}

impl Said {
    pub(crate) const fn of(text: UiText) -> Self {
        Self {
            text,
            values: Vec::new(),
        }
    }

    pub(crate) fn with(mut self, name: &'static str, value: String) -> Self {
        self.values.push((name, value));
        self
    }

    fn naming(text: UiText, identity: &HeldIdentity) -> Self {
        Self::of(text).with("identity", isolated(&identity.name()))
    }

    /// The placeholder English, filled.
    #[must_use]
    pub fn english(&self) -> String {
        let values: Vec<(&str, &str)> = self.values.iter().map(|(n, v)| (*n, v.as_str())).collect();
        fill(placeholder_en::text(self.text), &values)
    }
}

/// Why a held list was refused; the model keeps what it held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldProblem {
    /// More than [`IDENTITY_CAP`].
    OverBound,
    /// Two rows with one key: which one is in front would be ambiguous.
    DuplicateKey,
}

/// What a held list changed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HeldChange {
    /// The identity that came in front because the one in front is no
    /// longer held (or none was): Remove's snackbar and focus name it,
    /// not a switch's (cards §5.7.3, "When the identities change").
    pub came_in_front: Option<IdentityKey>,
    /// The identity in front stopped being able to send: focus on the
    /// send control moves to the composer's field (identities §5.2).
    pub send_disabled: bool,
    /// A notification's transition was under way to an identity no
    /// longer held: it leaves, and nothing is revealed
    /// (`a_held_change_during_the_transition_ends_it_if_its_identity_is_gone`).
    pub transition_left: bool,
}

/// A page of the second pane on a wide layout, for what a switch closes
/// (cards §5.7.3, identities §5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pane {
    /// A conversation.
    Conversation,
    /// Contacts.
    Contacts,
    /// A contact's page.
    Contact,
    /// An identity's page.
    IdentityPage,
    /// Trust, the identity's since ADR-0067.
    Trust,
    /// Peers, whose trust is the identity's.
    Peers,
    /// Your identities, which belongs to no identity.
    YourIdentities,
}

impl Pane {
    /// Every pane, for the tests.
    pub const ALL: &'static [Self] = &[
        Self::Conversation,
        Self::Contacts,
        Self::Contact,
        Self::IdentityPage,
        Self::Trust,
        Self::Peers,
        Self::YourIdentities,
    ];

    /// Whether a change of the identity in front closes it: everything
    /// of one identity does, Your identities stays and its check mark
    /// moves (`a_switch_closes_every_pane_of_one_identity`).
    #[must_use]
    pub const fn closes_on_switch(self) -> bool {
        match self {
            Self::Conversation
            | Self::Contacts
            | Self::Contact
            | Self::IdentityPage
            | Self::Trust
            | Self::Peers => true,
            Self::YourIdentities => false,
        }
    }
}

/// The snackbar a switch shows (cards §5.7.3): which identity is now in
/// use, with Switch back; announced. It replaces any snackbar shown
/// before the switch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchSnackbar {
    /// `ConfirmIdentitySwitched`.
    pub said: Said,
    /// `SwitchBack`.
    pub action: UiText,
}

/// A change of the identity in front, done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switched {
    /// Now in front.
    pub to: IdentityKey,
    /// In front before, which Switch back returns to.
    pub previous: IdentityKey,
    /// Its snackbar.
    pub snackbar: SwitchSnackbar,
    /// Focus goes to the switch, which names the new identity: after the
    /// switcher or Switch back, not after a notification's transition,
    /// whose Chats takes focus as it opens.
    pub focus_switch: bool,
    /// Whether a notification opened it: its Switch back shows the
    /// previous identity's Chats.
    pub from_notification: bool,
}

/// The transition before another identity's notification opens (cards
/// §5.7.5): that identity's avatar and `SwitchingTo`, announced, held
/// [`SWITCHING_HOLD_MS`] and until its Chats is ready. The identity in
/// front has changed already, so the panes of the old one close.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    /// The identity it opens as.
    pub to: IdentityKey,
    /// `SwitchingTo`.
    pub said: Said,
}

/// What an act that may change the identity in front did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontOutcome {
    /// Nothing changed: the identity chosen is the one in front, or is
    /// not held (`choosing_the_row_in_front_changes_nothing`), or a
    /// transition shows (`nothing_switches_while_the_transition_shows`).
    /// A switcher closes.
    Unchanged,
    /// An unsaved card edit is open: `CardDiscardChanges` is asked first,
    /// and [`Identities::discard_confirmed`] or
    /// [`Identities::discard_cancelled`] answers it.
    AskDiscard,
    /// The identity in front changed.
    Switched(Switched),
    /// A notification of the identity in front: its Chats shows, with no
    /// transition and no switch.
    ShowChats,
    /// A notification of another identity: the transition begins; the
    /// switch completes in [`Identities::opening_ready`] or
    /// [`Identities::tick`].
    Opening(Transition),
    /// A notification with no one identity, two or more of which have
    /// news: Chats of the identity in front with the switcher open over
    /// it, switching nothing (identities §9, §16 Q7 a;
    /// `a_notification_of_no_one_identity_switches_only_to_the_one_with_news`).
    OpenSwitcher,
    /// A row of Your identities was chosen: that identity's page opens
    /// ([`Identities::identity_page`]), switching nothing (cards §5.1).
    OpenPage(IdentityKey),
    /// Your identities' Add an identity: the root opens the add form,
    /// whose back arrow returns to the list (identities §6.2).
    OpenAdd,
}

/// Which notification was opened (identities §9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationOpened {
    /// One identity's notification.
    Identity(IdentityKey),
    /// A notification that names no one identity -- a group's summary,
    /// the lock screen's entry after the unlock, the desktop's redacted
    /// form -- with the identities that have news.
    NoOneIdentity(Vec<IdentityKey>),
}

/// A person's act on the identity switch, the switcher or the switch's
/// snackbar, as a view returns it (cards §5.7.1, §5.7.3; identities §11),
/// applied by [`Identities::act`].
///
/// A type of its own rather than [`crate::Intent`] variants: every root
/// matches `Intent` exhaustively, so a variant there would change each
/// of them, while these are carried out by app-core's model side alone
/// (`ModelSide::take_identity_acts`; plan §30 step 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityIntent {
    /// The switch was activated: the switcher opens.
    OpenSwitcher,
    /// Escape, Back or the scrim: the switcher closes with nothing
    /// changed.
    CloseSwitcher,
    /// A switcher row was chosen, by the identity it showed.
    Choose(IdentityKey),
    /// Switch back on the switch's snackbar, by the snackbar's id.
    SwitchBack(u64),
    /// The switch's snackbar closed without its action: its time, a
    /// swipe, or another snackbar taking its place.
    SnackbarClosed(u64),
    /// Your identities at the switcher's bottom, or in the app options:
    /// the switcher closes and, with two or more held, the list opens,
    /// switching nothing.
    OpenIdentities,
    /// Try again on a Could not start identity (identities §5.2): the
    /// root runs the assertion and the lease again; the model waits for
    /// the state the root reads back.
    TryAgain(IdentityKey),
    /// Your identities was left by its back arrow, Escape or Back.
    CloseIdentities,
    /// A row of Your identities was chosen, by the identity it showed.
    OpenRow(IdentityKey),
    /// Your identities' Add an identity.
    Add,
    /// An identity's page was left by its back arrow, Escape or Back:
    /// Your identities shows again, on that identity's row.
    ClosePage,
}

/// Where an act asks focus to go (identities §11's focus table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityFocus {
    /// The switch: the switcher closed with nothing changed (F2), or a
    /// switch by the switcher or Switch back (F3).
    Switch,
    /// The switcher's title: it opened (F1).
    SwitcherTitle,
    /// Your identities' title: it opened (F4), or Done named a row that
    /// is not held yet (the plan's answer to Q-U5).
    IdentitiesTitle,
    /// A row of Your identities, the one [`Identities::focus_row`]
    /// names: Create ended Done (F6), with its card not saved too (F9).
    Row,
    /// `IdentityLimit` under Add: Add was chosen at the bound (F8).
    Limit,
}

/// The switch's snackbar while it is up, by its id: a Switch back or a
/// close names this one, so an answer to one already replaced does
/// nothing (`switch_back_and_a_close_name_their_own_snackbar`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownSwitchSnackbar {
    /// Its id, new for every switch.
    pub id: u64,
    /// What it says.
    pub snackbar: SwitchSnackbar,
}

/// The switch's badge (cards §5.7.1), over the identities other than the
/// one in front. It counts and names nothing
/// (`the_badge_counts_the_others_and_names_nothing`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Badge {
    /// Their unread items together; 0 shows a dot with no count.
    pub unread: u32,
    /// Whether a message of any of them was not sent.
    pub not_sent: bool,
}

/// A name's first three graphemes, or the whole of a shorter name.
fn least(name: &str) -> String {
    name.graphemes(true).take(3).collect()
}

/// The identity switch on Chats' app bar (cards §5.7.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchView {
    /// The identity in front's name, shown.
    pub name: String,
    /// The name's first three graphemes, or the whole of a shorter name:
    /// the switch ellipsises its name, never below the avatar and these
    /// (cards §5.7.1, identities §12;
    /// `the_switch_keeps_three_graphemes_of_the_name`, and drawn,
    /// `a_wide_name_never_ellipsises_the_switch_below_its_three_graphemes`
    /// in ui-slint's tests).
    pub least: String,
    /// Every held identity's `least`, in the held order, the one in front
    /// among them: the Chats bar decides between one row and two by the
    /// widest, so the row count never depends on the name in front and a
    /// switch never reflows the bar (identities §12 "The Chats bar when
    /// the switch does not fit";
    /// `every_held_identitys_least_is_offered_whoever_is_in_front`).
    pub leasts: Vec<String>,
    /// Its badge, if any.
    pub badge: Option<Badge>,
    /// Its accessible name, sentence by sentence: `IdentitySwitch`, then
    /// the badge's unread text, then `IdentityOthersNotSentInOther` or
    /// `IdentityOthersNotSentInOthers`, by the same one-other test.
    pub accessible: Vec<Said>,
}

/// Try again on a Could not start identity (identities §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TryAgainControl {
    /// `TryAgain`.
    pub text: UiText,
    /// Whether it can be chosen: not while its own retry runs.
    pub enabled: bool,
    /// The progress indicator in it, while its retry runs.
    pub progress: bool,
}

/// One row of the switcher (cards §5.7.1, identities §6.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitcherRow {
    /// The identity it switches to.
    pub key: IdentityKey,
    /// Its name.
    pub name: String,
    /// Its second line, the short `EndpointId`.
    pub second_line: String,
    /// That line's accessible name, `IdentityIdLabel`.
    pub second_line_label: Said,
    /// Its unread items.
    pub unread: u32,
    /// Their count, spoken, when there are any.
    pub unread_said: Option<Said>,
    /// `IdentityNotSent`, when a message of it was not sent.
    pub not_sent: Option<UiText>,
    /// Its state's line, when it cannot act now.
    pub state: Option<Said>,
    /// Try again, on a Could not start identity.
    pub try_again: Option<TryAgainControl>,
    /// It is the identity in front.
    pub chosen: bool,
}

/// The switcher (cards §5.7.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switcher {
    /// `IdentitySwitchTitle`.
    pub title: UiText,
    /// Every held identity, in the order held.
    pub rows: Vec<SwitcherRow>,
    /// The text button at the bottom, `IdentitiesTitle`.
    pub identities: UiText,
}

/// The send control (cards §5.7.2, identities §5.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendControl {
    /// It sends; `name` is its accessible name, `SendAs`, with two or
    /// more held.
    Enabled {
        /// `SendAs`, or `None` for the control's own name.
        name: Option<Said>,
    },
    /// It does not send; `reason` is the state's text, which the control
    /// carries as its accessible description.
    Disabled {
        /// `SendAs` with two or more held, as when enabled: the control's
        /// name does not change with its state
        /// (`the_composer_names_the_identity_or_its_state`).
        name: Option<Said>,
        /// Why, or `None` where the design gives no text (an unavailable
        /// identity with one held: the session's own refusal says why).
        reason: Option<Said>,
    },
}

/// A conversation's composer line (cards §5.7.2 N2, §5.7.6, identities
/// §5.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposerLine {
    /// The line above the send control: `ComposerAs`, or the state's text
    /// in its place.
    pub line: Option<Said>,
    /// The send control.
    pub send: SendControl,
    /// Try again at the end of the line.
    pub try_again: Option<TryAgainControl>,
}

/// One row of Your identities (cards §5.1, identities §6.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityRow {
    /// The identity whose page it opens.
    pub key: IdentityKey,
    /// Its name: its card's, else its alias.
    pub name: String,
    /// Its second line, the short `EndpointId`.
    pub second_line: String,
    /// That line's accessible name, `IdentityIdLabel`.
    pub second_line_label: Said,
    /// `IdentityDefault`, on the default identity's row alone.
    pub default: Option<UiText>,
    /// It is the identity in front: the check mark.
    pub in_front: bool,
    /// `IdentityInFront` in its accessible description, with the check
    /// mark.
    pub description: Option<UiText>,
    /// Its state's line, when it cannot act now, where the switcher's row
    /// shows it.
    pub state: Option<Said>,
    /// Try again, on a Could not start identity.
    pub try_again: Option<TryAgainControl>,
    /// `IdentityCardNotSaved`: Done made it and did not write its card.
    pub card_not_saved: Option<UiText>,
}

/// Your identities' primary and what stands under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddControl {
    /// `AddIdentity`.
    pub text: UiText,
    /// `IdentityLimit` under it, at the bound: Add then opens nothing.
    pub limit: Option<UiText>,
    /// Why Add is disabled, when it is: `StorageDegraded`, the notice the
    /// app already shows above it while the store cannot write (cards
    /// §5.1, "Store degraded"). Add then opens nothing.
    pub disabled: Option<UiText>,
}

/// The alias line under an identity's card fields (identities §6.2,
/// "The alias" and "Its Copy").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasLine {
    /// `IdentityAlias`, the alias its last value, bare: the alias grammar
    /// is lowercase ASCII, and nothing touches it, so the line ends with
    /// the alias a person copies by hand
    /// (`the_alias_line_ends_with_the_alias_and_is_absent_without_one`).
    pub said: Said,
    /// The alias alone: what Copy puts on the clipboard, without the
    /// line's words.
    pub alias: String,
    /// Copy's visible label, `CopyAlias`.
    pub copy: UiText,
    /// Copy's accessible name, `CopyAliasName`.
    pub copy_name: UiText,
    /// The snackbar once copied, where the platform confirms nothing
    /// itself: `AliasCopied`.
    pub copied: UiText,
}

/// The page of an identity, opened from its row of Your identities
/// (cards §5.2; identities §6.2 step 4), apart from its card's fields,
/// which are that identity's own model's. It has no overflow while two
/// or more are held: Make default and Remove land together, later
/// (architect-cto's ruling of 2026-11-28, plan §30; Q-A1 (a)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityPageView {
    /// Whose page it is.
    pub key: IdentityKey,
    /// The app bar's subtitle: that identity's name (N10).
    pub subtitle: String,
    /// `IdentityNotInFront` under the app bar, when it is not the
    /// identity in front.
    pub not_in_front: Option<Said>,
    /// The alias line and its Copy, for an identity with an alias.
    pub alias: Option<AliasLine>,
    /// The back arrow's accessible name, `BackToIdentities`.
    pub back: UiText,
    /// The add form's values that Done did not write: the editor opens
    /// holding them as unsaved changes, for this session (F9).
    pub unsaved: Option<AddValues>,
}

/// Your identities (cards §5.1, identities §6.1), top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentitiesPage {
    /// `IdentitiesTitle`.
    pub title: UiText,
    /// The shared-device statement, in the root's host's words.
    pub statement: HostText,
    /// `IdentitiesUnlisted`.
    pub unlisted: UiText,
    /// Every held identity, in the order held.
    pub rows: Vec<IdentityRow>,
    /// Add an identity, on a root that can add.
    pub add: Option<AddControl>,
}

/// The lock screen's one entry (identities §9): the app's name and
/// `MessagesChannel` ("New messages", the text `MessagesWaitingLocked`
/// asks for). It has no field for a name or a count, whatever the number
/// of identities and of those with news
/// (`the_lock_screen_entry_is_one_and_names_nothing`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockedEntry {
    /// Its text.
    pub text: UiText,
}

/// An act held while `CardDiscardChanges` is asked.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Pending {
    Switch(IdentityKey),
    Open(IdentityKey),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Opening {
    to: IdentityKey,
    previous: IdentityKey,
    since: u64,
    ready: bool,
}

/// The identities held, the one in front, and the one before it.
#[derive(Debug, Clone, Default)]
pub struct Identities {
    held: Vec<HeldIdentity>,
    front: Option<IdentityKey>,
    previous: Option<IdentityKey>,
    pending: Option<Pending>,
    opening: Option<Opening>,
    /// The switcher is open over Chats.
    switcher_open: bool,
    /// The switch's snackbar, while up.
    snackbar: Option<ShownSwitchSnackbar>,
    /// The last snackbar's id.
    snackbar_seq: u64,
    /// The newest focus request, by its own count.
    focus: Option<(u64, IdentityFocus)>,
    /// The row the newest [`IdentityFocus::Row`] request names.
    focus_row: Option<IdentityKey>,
    /// Your identities is shown.
    list_open: bool,
    /// Whether Add is drawn and whether it opens the form.
    add_gate: AddGate,
    /// The identities whose card Done did not write, each with the form's
    /// values, until a held list gives each a card's name: kept for the
    /// session.
    cards_not_saved: Vec<(IdentityKey, AddValues)>,
    /// The identity whose page is shown, opened from its row.
    page: Option<IdentityKey>,
}

/// What Your identities' Add depends on besides the bound.
#[derive(Debug, Clone, Copy, Default)]
struct AddGate {
    /// The root can add an identity: false until it says so.
    adds: bool,
    /// The store cannot write now: Add is disabled (cards §5.1, "Store
    /// degraded").
    store_degraded: bool,
}

impl Identities {
    /// None held yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The identities held, in the order the root handed them.
    #[must_use]
    pub fn held(&self) -> &[HeldIdentity] {
        &self.held
    }

    /// The identity in front.
    #[must_use]
    pub const fn front(&self) -> Option<&IdentityKey> {
        self.front.as_ref()
    }

    /// The identity Switch back returns to.
    #[must_use]
    pub const fn previous(&self) -> Option<&IdentityKey> {
        self.previous.as_ref()
    }

    /// Whether the identities are named at all: with two or more held
    /// (ADR-0061 d. 3, identities §4).
    #[must_use]
    pub const fn shows_identity(&self) -> bool {
        self.held.len() >= 2
    }

    fn get(&self, key: &IdentityKey) -> Option<&HeldIdentity> {
        self.held.iter().find(|h| &h.key == key)
    }

    fn front_held(&self) -> Option<&HeldIdentity> {
        self.front.as_ref().and_then(|k| self.get(k))
    }

    /// The identity that comes in front when none is: the default, else
    /// the first identity, else the first held.
    fn fallback(&self) -> Option<IdentityKey> {
        self.held
            .iter()
            .find(|h| h.is_default)
            .or_else(|| self.held.iter().find(|h| h.is_first))
            .or_else(|| self.held.first())
            .map(|h| h.key.clone())
    }

    /// The root read the held identities again. The one in front stays
    /// while it is held; when it is gone, the default comes in front
    /// (cards §5.7.3 item 4, `the_front_gone_brings_the_default_in_front`);
    /// an identity added switches nothing
    /// (`a_second_identity_added_switches_nothing`).
    ///
    /// # Errors
    ///
    /// A list longer than [`IDENTITY_CAP`], or with a key twice, is
    /// refused and the model keeps what it held.
    pub fn held_changed(&mut self, held: Vec<HeldIdentity>) -> Result<HeldChange, HeldProblem> {
        if held.len() > IDENTITY_CAP {
            return Err(HeldProblem::OverBound);
        }
        for (i, h) in held.iter().enumerate() {
            if held[..i].iter().any(|o| o.key == h.key) {
                return Err(HeldProblem::DuplicateKey);
            }
        }
        let was_sending = self
            .front_held()
            .map(|h| (h.key.clone(), h.state == IdentityState::Ready));
        let was_named = self.shows_identity();
        self.held = held;
        // A card saved since: its row no longer says that it was not.
        let held = &self.held;
        self.cards_not_saved.retain(|(key, _)| {
            !held
                .iter()
                .any(|h| &h.key == key && h.card_name.as_deref().is_some_and(|n| !n.is_empty()))
        });
        let mut change = HeldChange::default();
        if self.front_held().is_none() {
            self.front = self.fallback();
            change.came_in_front.clone_from(&self.front);
            // Not a switch: the identity the snackbar names is no longer
            // in front, and Switch back would find its target already
            // there or beside the point; Remove's own snackbar and focus
            // are the root's (cards §5.7.3, identities §7;
            // `the_front_gone_brings_the_default_in_front`).
            self.previous = None;
            self.snackbar = None;
        } else if let (Some((key, true)), Some(now)) = (was_sending, self.front_held()) {
            change.send_disabled = key == now.key && now.state != IdentityState::Ready;
        }
        if self
            .previous
            .as_ref()
            .is_some_and(|p| self.get(p).is_none())
        {
            self.previous = None;
        }
        if self
            .opening
            .as_ref()
            .is_some_and(|o| self.get(&o.to).is_none())
        {
            self.opening = None;
            change.transition_left = true;
        }
        // Back to one, nothing of the switch shows (ADR-0061 d. 3); with
        // the identity Switch back returns to gone, its snackbar would
        // offer an act that does nothing
        // (`going_back_to_one_closes_the_switcher_and_its_snackbar`).
        if !self.shows_identity() {
            self.switcher_open = false;
        }
        // Back to one, there is no list. Only on the way down: Done may
        // open it before the new identity is held, which then shows it
        // (`done_before_the_new_identity_is_held_shows_the_list_when_it_is`).
        if was_named && !self.shows_identity() {
            self.list_open = false;
        }
        // A page of an identity no longer held has nothing to show, and
        // with one held the card editor is that identity's page.
        if !self.shows_identity() || self.page.as_ref().is_some_and(|p| self.get(p).is_none()) {
            self.page = None;
        }
        if !self.shows_identity() || self.previous.is_none() {
            self.snackbar = None;
        }
        Ok(change)
    }

    /// The app started (cards §5.7.3, "When the app starts"): the
    /// identity last in front comes in front if it is held, else the
    /// default. `last` is `None` when the setting cannot be read.
    pub fn start(&mut self, last: Option<IdentityKey>) -> Option<&IdentityKey> {
        self.front = match last {
            Some(key) if self.get(&key).is_some() => Some(key),
            _ => self.fallback(),
        };
        self.previous = None;
        self.pending = None;
        self.opening = None;
        self.switcher_open = false;
        self.snackbar = None;
        self.page = None;
        self.front.as_ref()
    }

    /// The person chose `to` in the switcher. `unsaved_edit` is whether an
    /// identity's page with changes is open.
    pub fn switch(&mut self, to: &IdentityKey, unsaved_edit: bool) -> FrontOutcome {
        if self.opening.is_some() || self.get(to).is_none() || self.front.as_ref() == Some(to) {
            return FrontOutcome::Unchanged;
        }
        if unsaved_edit {
            self.pending = Some(Pending::Switch(to.clone()));
            return FrontOutcome::AskDiscard;
        }
        self.switch_now(to.clone())
    }

    /// Switch back on a switch's snackbar: the same rule, to the previous
    /// identity.
    pub fn switch_back(&mut self, unsaved_edit: bool) -> FrontOutcome {
        match self.previous.clone() {
            Some(previous) => self.switch(&previous, unsaved_edit),
            None => FrontOutcome::Unchanged,
        }
    }

    fn switch_now(&mut self, to: IdentityKey) -> FrontOutcome {
        let Some(previous) = self.front.replace(to.clone()) else {
            return FrontOutcome::Unchanged;
        };
        self.previous = Some(previous.clone());
        // An identity's page is one identity's pane, which a switch
        // closes (`Pane::IdentityPage`; `a_switch_closes_an_identitys_page`).
        self.page = None;
        let switched = self.switched(to, previous, true, false);
        self.shown(&switched);
        FrontOutcome::Switched(switched)
    }

    /// A switch done: its snackbar replaces the one up, and focus goes
    /// to the switch unless a notification's Chats takes it.
    fn shown(&mut self, switched: &Switched) {
        self.snackbar_seq = self.snackbar_seq.wrapping_add(1);
        self.snackbar = Some(ShownSwitchSnackbar {
            id: self.snackbar_seq,
            snackbar: switched.snackbar.clone(),
        });
        if switched.focus_switch {
            self.ask_focus(IdentityFocus::Switch);
        }
    }

    fn ask_focus(&mut self, to: IdentityFocus) {
        let seq = self.focus.map_or(1, |(seq, _)| seq.wrapping_add(1));
        self.focus = Some((seq, to));
    }

    /// The person's act on the switch, the switcher or the switch's
    /// snackbar. `unsaved_edit` is whether an identity's page with
    /// changes is open, as [`Identities::switch`] takes it.
    ///
    /// The switcher opens with two or more held and no transition
    /// showing (`the_switcher_opens_with_two_held_and_closes_to_the_switch`);
    /// a choice is taken from the open switcher alone, and closes it
    /// (`a_choice_is_taken_from_the_open_switcher_alone`); choosing the
    /// row in front closes it with nothing changed
    /// (`choosing_the_chosen_row_closes_the_switcher_with_nothing_changed`).
    /// Your identities, its rows and Add, and Try again switch nothing:
    /// the list opens and closes here
    /// (`your_identities_opens_from_the_switcher_and_focuses_its_title`),
    /// and the root opens a row's page or the add form, or runs the
    /// retry.
    pub fn act(&mut self, intent: IdentityIntent, unsaved_edit: bool) -> FrontOutcome {
        match intent {
            IdentityIntent::OpenSwitcher => {
                if self.shows_identity() && self.opening.is_none() && !self.switcher_open {
                    self.switcher_open = true;
                    self.ask_focus(IdentityFocus::SwitcherTitle);
                }
                FrontOutcome::Unchanged
            }
            IdentityIntent::CloseSwitcher => {
                if self.switcher_open {
                    self.switcher_open = false;
                    self.ask_focus(IdentityFocus::Switch);
                }
                FrontOutcome::Unchanged
            }
            IdentityIntent::Choose(to) => {
                if !self.switcher_open {
                    return FrontOutcome::Unchanged;
                }
                self.switcher_open = false;
                let outcome = self.switch(&to, unsaved_edit);
                // A switch asked its own focus; CardDiscardChanges takes
                // focus as it opens.
                if outcome == FrontOutcome::Unchanged {
                    self.ask_focus(IdentityFocus::Switch);
                }
                outcome
            }
            IdentityIntent::SwitchBack(id) => {
                if self.snackbar.as_ref().map(|s| s.id) != Some(id) {
                    return FrontOutcome::Unchanged;
                }
                self.snackbar = None;
                self.switch_back(unsaved_edit)
            }
            IdentityIntent::SnackbarClosed(id) => {
                if self.snackbar.as_ref().map(|s| s.id) == Some(id) {
                    self.snackbar = None;
                }
                FrontOutcome::Unchanged
            }
            IdentityIntent::OpenIdentities => {
                self.switcher_open = false;
                // With one identity there is no list: the root opens that
                // identity's card (cards §5.1).
                if self.shows_identity() {
                    self.list_open = true;
                    self.ask_focus(IdentityFocus::IdentitiesTitle);
                }
                FrontOutcome::Unchanged
            }
            IdentityIntent::TryAgain(_) => FrontOutcome::Unchanged,
            IdentityIntent::CloseIdentities => {
                self.list_open = false;
                self.page = None;
                FrontOutcome::Unchanged
            }
            IdentityIntent::OpenRow(key) => {
                if self.identities_open() && self.get(&key).is_some() {
                    // The page opens over the list; the identity in front
                    // stays (`a_row_opens_that_identitys_page_and_switches_nothing`).
                    self.page = Some(key.clone());
                    FrontOutcome::OpenPage(key)
                } else {
                    FrontOutcome::Unchanged
                }
            }
            IdentityIntent::ClosePage => {
                // Back on the row it was opened from
                // (`back_from_a_rows_page_returns_to_the_list_on_that_row`).
                if let Some(key) = self.page.take()
                    && self.identities_open()
                {
                    self.focus_row = Some(key);
                    self.ask_focus(IdentityFocus::Row);
                }
                FrontOutcome::Unchanged
            }
            IdentityIntent::Add => {
                // Disabled while the store cannot write: it opens nothing
                // and moves no focus
                // (`add_is_disabled_while_the_store_is_degraded`).
                if !self.identities_open() || !self.add_gate.adds || self.add_gate.store_degraded {
                    return FrontOutcome::Unchanged;
                }
                // Interim, Q-A2 (a), until the endpoint list's read lands
                // (the plan's PR 4): the list knows no bound but the
                // identities it holds, so below 64 held Add always opens
                // the form, and an add refused `bound` shows
                // `IdentityLimit` there in Create's place. No endpoint
                // list is read here.
                if self.can_add() {
                    FrontOutcome::OpenAdd
                } else {
                    self.ask_focus(IdentityFocus::Limit);
                    FrontOutcome::Unchanged
                }
            }
        }
    }

    /// Whether Your identities is shown: opened, and two or more held.
    #[must_use]
    pub const fn identities_open(&self) -> bool {
        self.list_open && self.shows_identity()
    }

    /// The root says whether it can add an identity: Add is drawn only
    /// where it can (`add_is_drawn_only_on_a_root_that_can_add`).
    pub const fn set_adds(&mut self, adds: bool) {
        self.add_gate.adds = adds;
    }

    /// The store can write, or cannot, as the session says: while it
    /// cannot, Add is disabled with the degraded notice as its reason
    /// (`add_is_disabled_while_the_store_is_degraded`).
    pub const fn set_store_degraded(&mut self, degraded: bool) {
        self.add_gate.store_degraded = degraded;
    }

    /// A Create ended Done with `key` (identities §6.2 step 4): Your
    /// identities opens on its row (F6). `kept` is the form's values when
    /// its card was not saved: the row then says so (F9) and its page
    /// opens holding them, until a held list gives it a card's name;
    /// `None` when the card was saved. A row not held yet is not waited
    /// for: the title takes focus.
    pub fn added(&mut self, key: &IdentityKey, kept: Option<AddValues>) {
        self.switcher_open = false;
        self.list_open = true;
        if let Some(values) = kept {
            if let Some((_, held)) = self.cards_not_saved.iter_mut().find(|(k, _)| k == key) {
                *held = values;
            } else {
                // As many as a profile holds; a mark past that is dropped,
                // the oldest first, so the list stays bounded.
                if self.cards_not_saved.len() >= IDENTITY_CAP {
                    self.cards_not_saved.remove(0);
                }
                self.cards_not_saved.push((key.clone(), values));
            }
        }
        if self.get(key).is_some() {
            self.focus_row = Some(key.clone());
            self.ask_focus(IdentityFocus::Row);
        } else {
            self.ask_focus(IdentityFocus::IdentitiesTitle);
        }
    }

    /// Your identities, while it is shown.
    #[must_use]
    pub fn your_identities(&self) -> Option<IdentitiesPage> {
        if !self.identities_open() {
            return None;
        }
        let rows = self
            .held
            .iter()
            .map(|h| {
                let in_front = self.front.as_ref() == Some(&h.key);
                IdentityRow {
                    key: h.key.clone(),
                    name: h.name().into_owned(),
                    second_line: h.key.short(),
                    second_line_label: Self::id_label_of(&h.key),
                    default: h.is_default.then_some(UiText::IdentityDefault),
                    in_front,
                    description: in_front.then_some(UiText::IdentityInFront),
                    state: self.state_line(h),
                    try_again: try_again(h.state),
                    card_not_saved: self
                        .kept_values(&h.key)
                        .map(|_| UiText::IdentityCardNotSaved),
                }
            })
            .collect();
        Some(IdentitiesPage {
            title: UiText::IdentitiesTitle,
            statement: HostText::IdentitiesShareDevice,
            unlisted: UiText::IdentitiesUnlisted,
            rows,
            add: self.add_gate.adds.then(|| AddControl {
                text: UiText::AddIdentity,
                limit: (!self.can_add()).then_some(UiText::IdentityLimit),
                disabled: self
                    .add_gate
                    .store_degraded
                    .then_some(UiText::StorageDegraded),
            }),
        })
    }

    /// The form's values Done kept for `key`, its card not saved.
    fn kept_values(&self, key: &IdentityKey) -> Option<&AddValues> {
        self.cards_not_saved
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, values)| values)
    }

    /// The page of the identity opened from its row, while it is shown:
    /// with two or more held, the identity held.
    #[must_use]
    pub fn identity_page(&self) -> Option<IdentityPageView> {
        let h = self.named(self.page.as_ref()?)?;
        Some(IdentityPageView {
            key: h.key.clone(),
            subtitle: h.name().into_owned(),
            not_in_front: self.not_in_front_line(&h.key),
            alias: (!h.alias.is_empty()).then(|| AliasLine {
                // Bare, not isolated: an isolate's closing mark would be
                // the line's last character, after the alias.
                said: Said::of(UiText::IdentityAlias).with("alias", h.alias.clone()),
                alias: h.alias.clone(),
                copy: UiText::CopyAlias,
                copy_name: UiText::CopyAliasName,
                copied: UiText::AliasCopied,
            }),
            back: UiText::BackToIdentities,
            unsaved: self.kept_values(&h.key).cloned(),
        })
    }

    /// Whether the switcher is open: with two or more held alone.
    #[must_use]
    pub const fn switcher_open(&self) -> bool {
        self.switcher_open && self.shows_identity()
    }

    /// The switch's snackbar, while it is up.
    #[must_use]
    pub const fn switch_snackbar(&self) -> Option<&ShownSwitchSnackbar> {
        self.snackbar.as_ref()
    }

    /// The newest focus request, by its count: a view moves focus once
    /// per count.
    #[must_use]
    pub const fn focus_request(&self) -> Option<(u64, IdentityFocus)> {
        self.focus
    }

    /// The row an [`IdentityFocus::Row`] request names.
    #[must_use]
    pub const fn focus_row(&self) -> Option<&IdentityKey> {
        self.focus_row.as_ref()
    }

    fn switched(
        &self,
        to: IdentityKey,
        previous: IdentityKey,
        focus_switch: bool,
        from_notification: bool,
    ) -> Switched {
        let said = self.get(&to).map_or_else(
            || Said::of(UiText::ConfirmIdentitySwitched),
            |h| Said::naming(UiText::ConfirmIdentitySwitched, h),
        );
        Switched {
            to,
            previous,
            snackbar: SwitchSnackbar {
                said,
                action: UiText::SwitchBack,
            },
            focus_switch,
            from_notification,
        }
    }

    /// `CardDiscardChanges` answered Close without saving: the held act
    /// goes on, at `now` (ms).
    pub fn discard_confirmed(&mut self, now: u64) -> FrontOutcome {
        match self.pending.take() {
            Some(Pending::Switch(to)) => self.switch(&to, false),
            Some(Pending::Open(to)) => {
                self.open_notification(NotificationOpened::Identity(to), false, now)
            }
            None => FrontOutcome::Unchanged,
        }
    }

    /// `CardDiscardChanges` answered Continue editing: the held act is
    /// dropped and everything stays as it was.
    pub fn discard_cancelled(&mut self) {
        self.pending = None;
    }

    /// A notification was opened at `now` (ms), after the unlock
    /// (identities §9).
    pub fn open_notification(
        &mut self,
        opened: NotificationOpened,
        unsaved_edit: bool,
        now: u64,
    ) -> FrontOutcome {
        let to = match opened {
            NotificationOpened::Identity(key) => key,
            NotificationOpened::NoOneIdentity(with_news) => {
                let mut held: Vec<IdentityKey> = Vec::new();
                for key in with_news {
                    if self.get(&key).is_some() && !held.contains(&key) {
                        held.push(key);
                    }
                }
                match held.len() {
                    0 => return FrontOutcome::ShowChats,
                    1 => held.remove(0),
                    _ => return FrontOutcome::OpenSwitcher,
                }
            }
        };
        if self.opening.is_some() || self.get(&to).is_none() || self.front.as_ref() == Some(&to) {
            return FrontOutcome::ShowChats;
        }
        if unsaved_edit {
            self.pending = Some(Pending::Open(to));
            return FrontOutcome::AskDiscard;
        }
        let Some(previous) = self.front.replace(to.clone()) else {
            return FrontOutcome::ShowChats;
        };
        self.previous = Some(previous.clone());
        self.switcher_open = false;
        self.page = None;
        self.opening = Some(Opening {
            to: to.clone(),
            previous,
            since: now,
            ready: false,
        });
        let said = self.get(&to).map_or_else(
            || Said::of(UiText::SwitchingTo),
            |h| Said::naming(UiText::SwitchingTo, h),
        );
        FrontOutcome::Opening(Transition { to, said })
    }

    /// The opened identity's Chats is ready at `now` (ms): the transition
    /// leaves if it has been held long enough.
    pub fn opening_ready(&mut self, now: u64) -> Option<Switched> {
        if let Some(opening) = &mut self.opening {
            opening.ready = true;
        }
        self.tick(now)
    }

    /// Time passed to `now` (ms): the transition leaves once it has been
    /// held [`SWITCHING_HOLD_MS`] and its Chats is ready, and the switch's
    /// snackbar shows (`the_transition_holds_a_second_and_until_ready`).
    pub fn tick(&mut self, now: u64) -> Option<Switched> {
        let opening = self.opening.as_ref()?;
        if !opening.ready || now.saturating_sub(opening.since) < SWITCHING_HOLD_MS {
            return None;
        }
        let opening = self.opening.take()?;
        let switched = self.switched(opening.to, opening.previous, false, true);
        self.shown(&switched);
        Some(switched)
    }

    /// When the transition's hold next ends, for the root's timer: `None`
    /// while it waits on its Chats, or with no transition.
    #[must_use]
    pub fn next_change(&self, now: u64) -> Option<u64> {
        let opening = self.opening.as_ref()?;
        let end = opening.since.saturating_add(SWITCHING_HOLD_MS);
        (opening.ready && end > now).then_some(end)
    }

    /// Whether a notification's transition is under way.
    #[must_use]
    pub const fn is_opening(&self) -> bool {
        self.opening.is_some()
    }

    /// The lock screen's entry while any identity has news (identities
    /// §9).
    #[must_use]
    pub const fn locked_entry() -> LockedEntry {
        LockedEntry {
            text: UiText::MessagesChannel,
        }
    }

    /// An identity's notification sub-text, its name, with two or more
    /// held (cards §5.7.5); with one, a notification names no identity.
    #[must_use]
    pub fn notification_subtext(&self, key: &IdentityKey) -> Option<String> {
        self.named(key).map(|h| h.name().into_owned())
    }

    /// The group summary's lines (identities §9), one an identity with
    /// news, in the order given; none with one held.
    #[must_use]
    pub fn summary_lines(&self, news: &[(IdentityKey, u32)]) -> Vec<Said> {
        news.iter()
            .filter(|(_, count)| *count > 0)
            .filter_map(|(key, count)| {
                let h = self.named(key)?;
                let text = by_count(
                    u64::from(*count),
                    UiText::NotificationIdentityLineOne,
                    UiText::NotificationIdentityLine,
                );
                Some(Said::naming(text, h).with("count", count.to_string()))
            })
            .collect()
    }

    /// `key`'s identity, when identities are named at all.
    fn named(&self, key: &IdentityKey) -> Option<&HeldIdentity> {
        if self.shows_identity() {
            self.get(key)
        } else {
            None
        }
    }

    fn front_named(&self) -> Option<&HeldIdentity> {
        self.front.as_ref().and_then(|k| self.named(k))
    }

    /// The switch's badge, over every identity but the one in front.
    #[must_use]
    pub fn badge(&self) -> Option<Badge> {
        if !self.shows_identity() {
            return None;
        }
        let others = self
            .held
            .iter()
            .filter(|h| Some(&h.key) != self.front.as_ref());
        let badge = others.fold(
            Badge {
                unread: 0,
                not_sent: false,
            },
            |b, h| Badge {
                unread: b.unread.saturating_add(h.unread),
                not_sent: b.not_sent || h.not_sent,
            },
        );
        (badge.unread > 0 || badge.not_sent).then_some(badge)
    }

    /// The identity switch, with two or more held.
    #[must_use]
    pub fn switch_view(&self) -> Option<SwitchView> {
        let front = self.front_named()?;
        let badge = self.badge();
        let mut accessible = vec![Said::naming(UiText::IdentitySwitch, front)];
        if let Some(b) = badge {
            let one_other = self.held.len() == 2;
            if b.unread > 0 {
                let text = match (b.unread == 1, one_other) {
                    (true, true) => UiText::IdentityOthersUnreadOneInOther,
                    (false, true) => UiText::IdentityOthersUnreadInOther,
                    (true, false) => UiText::IdentityOthersUnreadOneInOthers,
                    (false, false) => UiText::IdentityOthersUnreadInOthers,
                };
                accessible.push(Said::of(text).with("n", b.unread.to_string()));
            }
            if b.not_sent {
                accessible.push(Said::of(if one_other {
                    UiText::IdentityOthersNotSentInOther
                } else {
                    UiText::IdentityOthersNotSentInOthers
                }));
            }
        }
        Some(SwitchView {
            name: front.name().into_owned(),
            least: least(&front.name()),
            leasts: self.held.iter().map(|h| least(&h.name())).collect(),
            badge,
            accessible,
        })
    }

    /// The switcher, with two or more held.
    #[must_use]
    pub fn switcher(&self) -> Option<Switcher> {
        if !self.shows_identity() {
            return None;
        }
        let rows = self
            .held
            .iter()
            .map(|h| SwitcherRow {
                key: h.key.clone(),
                name: h.name().into_owned(),
                second_line: h.key.short(),
                second_line_label: Self::id_label_of(&h.key),
                unread: h.unread,
                unread_said: (h.unread > 0).then(|| {
                    Said::of(by_count(
                        u64::from(h.unread),
                        UiText::IdentityRowUnreadOne,
                        UiText::IdentityRowUnread,
                    ))
                    .with("n", h.unread.to_string())
                }),
                not_sent: h.not_sent.then_some(UiText::IdentityNotSent),
                state: self.state_line(h),
                try_again: try_again(h.state),
                chosen: self.front.as_ref() == Some(&h.key),
            })
            .collect();
        Some(Switcher {
            title: UiText::IdentitySwitchTitle,
            rows,
            identities: UiText::IdentitiesTitle,
        })
    }

    fn id_label_of(key: &IdentityKey) -> Said {
        Said::of(UiText::IdentityIdLabel).with("short", key.short())
    }

    /// The accessible name of an identity's short `EndpointId` line, with
    /// two or more held.
    #[must_use]
    pub fn id_label(&self, key: &IdentityKey) -> Option<Said> {
        self.named(key).map(|h| Self::id_label_of(&h.key))
    }

    /// The state's line of an identity that cannot act now, in the form
    /// for the number held.
    fn state_line(&self, h: &HeldIdentity) -> Option<Said> {
        let named = self.shows_identity();
        match h.state {
            IdentityState::Starting { .. } if named => {
                Some(Said::naming(UiText::IdentityStarting, h))
            }
            IdentityState::Starting { .. } => Some(Said::of(UiText::IdentityStartingOne)),
            IdentityState::NotHidden if named => Some(Said::naming(UiText::IdentityNotHidden, h)),
            IdentityState::NotHidden => Some(Said::of(UiText::IdentityNotHiddenOne)),
            IdentityState::Unavailable if named => {
                Some(Said::naming(UiText::IdentityUnavailable, h))
            }
            // The design gives no one-identity form: the session's own
            // refusal says why it cannot send.
            IdentityState::Ready | IdentityState::Unavailable => None,
        }
    }

    /// The composer line of a conversation of `conversation`'s identity,
    /// or `None` when it is not held.
    #[must_use]
    pub fn composer(&self, conversation: &IdentityKey) -> Option<ComposerLine> {
        let h = self.get(conversation)?;
        let named = self.shows_identity();
        Some(if h.state == IdentityState::Ready {
            ComposerLine {
                line: named.then(|| Said::naming(UiText::ComposerAs, h)),
                send: SendControl::Enabled {
                    name: named.then(|| Said::naming(UiText::SendAs, h)),
                },
                try_again: None,
            }
        } else {
            let line = self.state_line(h);
            ComposerLine {
                send: SendControl::Disabled {
                    name: named.then(|| Said::naming(UiText::SendAs, h)),
                    reason: line.clone(),
                },
                line,
                try_again: try_again(h.state),
            }
        })
    }

    /// `NewConversationAs`, at the new-conversation flow's top.
    #[must_use]
    pub fn new_conversation_line(&self) -> Option<Said> {
        self.front_named()
            .map(|h| Said::naming(UiText::NewConversationAs, h))
    }

    /// `ShareCardAs`, under the share sheet's title.
    #[must_use]
    pub fn share_card_line(&self) -> Option<Said> {
        self.front_named()
            .map(|h| Said::naming(UiText::ShareCardAs, h))
    }

    /// `ProposalAs`, the last sentence of Accept's context; the caller
    /// leaves it out in case S, where Accept is not shown (N4).
    #[must_use]
    pub fn proposal_line(&self) -> Option<Said> {
        self.front_named()
            .map(|h| Said::naming(UiText::ProposalAs, h))
    }

    /// The app bar's subtitle of Contacts, a contact, Trust and Peers
    /// (N6-N8): the identity in front's name.
    #[must_use]
    pub fn subtitle(&self) -> Option<String> {
        self.front_named().map(|h| h.name().into_owned())
    }

    /// `IdentityNotInFront` under an identity's page's app bar, when
    /// `page` is not the identity in front.
    #[must_use]
    pub fn not_in_front_line(&self, page: &IdentityKey) -> Option<Said> {
        let front = self.front_named()?;
        let page = self.named(page)?;
        (front.key != page.key).then(|| {
            Said::naming(UiText::IdentityNotInFront, page).with("front", isolated(&front.name()))
        })
    }

    /// Whether Add opens the form: below [`IDENTITY_CAP`] held. Interim,
    /// Q-A2 (a): the identities held are the one count the list has until
    /// the endpoint list's read lands, which counts every endpoint of the
    /// profile (cards §5.1, "At the bound").
    #[must_use]
    pub const fn can_add(&self) -> bool {
        self.held.len() < IDENTITY_CAP
    }

    /// Whether Remove is offered on `key`: absent on the first identity
    /// and on the default (plan §30's exit gate).
    #[must_use]
    pub fn can_remove(&self, key: &IdentityKey) -> bool {
        self.get(key).is_some_and(|h| !h.is_first && !h.is_default)
    }
}

const fn try_again(state: IdentityState) -> Option<TryAgainControl> {
    match state {
        IdentityState::NotHidden => Some(TryAgainControl {
            text: UiText::TryAgain,
            enabled: true,
            progress: false,
        }),
        IdentityState::Starting {
            after_try_again: true,
        } => Some(TryAgainControl {
            text: UiText::TryAgain,
            enabled: false,
            progress: true,
        }),
        IdentityState::Starting {
            after_try_again: false,
        }
        | IdentityState::Ready
        | IdentityState::Unavailable => None,
    }
}

#[cfg(test)]
mod tests {
    use interweave_test_support::endpoint_keys::endpoint_id;

    use super::*;

    /// The one place a test key is made: the derived `EndpointId` the root
    /// would hand over, of the tag's deterministic test key.
    fn key(tag: &str) -> IdentityKey {
        IdentityKey::new(endpoint_id(tag).as_str())
    }

    fn ident(tag: &str) -> HeldIdentity {
        HeldIdentity {
            key: key(tag),
            card_name: Some(format!("Name {tag}")),
            alias: format!("alias-{tag}"),
            state: IdentityState::Ready,
            is_first: false,
            is_default: false,
            unread: 0,
            not_sent: false,
        }
    }

    /// `human` first and default, then the others as given.
    fn holding(tags: &[&str]) -> Vec<HeldIdentity> {
        tags.iter()
            .enumerate()
            .map(|(i, t)| HeldIdentity {
                is_first: i == 0,
                is_default: i == 0,
                ..ident(t)
            })
            .collect()
    }

    fn model(tags: &[&str]) -> Identities {
        let mut m = Identities::new();
        m.held_changed(holding(tags)).ok();
        m.start(None);
        m
    }

    fn switched(outcome: FrontOutcome) -> Switched {
        match outcome {
            FrontOutcome::Switched(s) => s,
            other => panic!("not switched: {other:?}"),
        }
    }

    fn name(tag: &str) -> String {
        isolated(&format!("Name {tag}"))
    }

    #[test]
    fn a_key_is_shown_by_its_last_eight_characters() {
        let id = endpoint_id("a").as_str().to_owned();
        assert_eq!(id.len(), 52, "the premise: a derived id");
        assert_eq!(
            key("a").short(),
            format!("…{}", &id[id.len() - 8..]),
            "a derived id, by its last eight characters"
        );
        assert_eq!(IdentityKey::new("abc").short(), "…abc");
        assert_eq!(IdentityKey::new("abcdefgh").short(), "…abcdefgh");
        // By character, so a key in any script cuts between characters.
        assert_eq!(IdentityKey::new("xxéééééééé").short(), "…éééééééé");
        assert_eq!(
            IdentityKey::new("x y").as_str(),
            "x y",
            "taken as handed over"
        );
    }

    #[test]
    fn a_name_is_the_cards_else_the_alias() {
        let mut h = ident("a");
        assert_eq!(h.name(), "Name a");
        h.card_name = None;
        assert_eq!(h.name(), "alias-a");
        h.card_name = Some(String::new());
        assert_eq!(h.name(), "alias-a");
    }

    /// With neither a card name nor an alias -- a first identity authored
    /// without one -- the name is the short key, nothing invented.
    /// Mutation: `name` answering the empty alias -- the name is empty.
    #[test]
    fn with_no_card_name_and_no_alias_the_name_is_the_short_key() {
        let mut h = ident("a");
        h.card_name = None;
        h.alias = String::new();
        assert_eq!(h.name(), h.key.short());
        h.card_name = Some(String::new());
        assert_eq!(h.name(), h.key.short());
        h.card_name = Some("Name a".to_owned());
        assert_eq!(h.name(), "Name a", "a card name before the key");
    }

    #[test]
    fn with_one_held_nothing_names_an_identity() {
        let mut m = model(&["a"]);
        let a = key("a");
        assert!(!m.shows_identity());
        assert_eq!(m.switch_view(), None);
        assert_eq!(m.switcher(), None);
        assert_eq!(m.badge(), None);
        assert_eq!(m.new_conversation_line(), None);
        assert_eq!(m.share_card_line(), None);
        assert_eq!(m.proposal_line(), None);
        assert_eq!(m.subtitle(), None);
        assert_eq!(m.not_in_front_line(&a), None);
        assert_eq!(m.id_label(&a), None);
        assert_eq!(m.notification_subtext(&a), None);
        assert!(m.summary_lines(&[(a.clone(), 3)]).is_empty());
        assert_eq!(
            m.composer(&a),
            Some(ComposerLine {
                line: None,
                send: SendControl::Enabled { name: None },
                try_again: None,
            })
        );
        let mut held = holding(&["a"]);
        held[0].state = IdentityState::Starting {
            after_try_again: false,
        };
        m.held_changed(held.clone()).ok();
        let line = m.composer(&a).and_then(|c| c.line);
        assert_eq!(line, Some(Said::of(UiText::IdentityStartingOne)));
        held[0].state = IdentityState::NotHidden;
        m.held_changed(held.clone()).ok();
        let c = m.composer(&a);
        assert_eq!(
            c.clone().and_then(|c| c.line),
            Some(Said::of(UiText::IdentityNotHiddenOne))
        );
        assert!(c.is_some_and(|c| c.try_again.is_some_and(|t| t.enabled)));
        held[0].state = IdentityState::Unavailable;
        m.held_changed(held).ok();
        assert_eq!(
            m.composer(&a),
            Some(ComposerLine {
                line: None,
                send: SendControl::Disabled {
                    name: None,
                    reason: None
                },
                try_again: None,
            })
        );
        // The control: the same with two held names it.
        let two = model(&["a", "b"]);
        assert!(two.shows_identity() && two.switch_view().is_some() && two.subtitle().is_some());
    }

    #[test]
    fn a_second_identity_added_switches_nothing() {
        let mut m = model(&["a"]);
        let change = m.held_changed(holding(&["a", "b"]));
        assert_eq!(change, Ok(HeldChange::default()));
        assert_eq!(m.front(), Some(&key("a")));
        assert!(m.shows_identity());
        assert_eq!(m.previous(), None);
    }

    #[test]
    fn the_front_gone_brings_the_default_in_front() {
        let mut m = model(&["a", "b", "c"]);
        switched(m.switch(&key("c"), false));
        let mut held = holding(&["a", "b"]);
        held[0].is_default = false;
        held[1].is_default = true;
        let change = m.held_changed(held);
        assert_eq!(change.map(|c| c.came_in_front), Ok(Some(key("b"))));
        assert_eq!(m.front(), Some(&key("b")));
        // Not a switch: no snackbar of the switch is up, and Switch back
        // has no target, though `a` is still held.
        assert_eq!(m.switch_snackbar(), None);
        assert_eq!(m.previous(), None);
    }

    #[test]
    fn the_front_gone_to_the_old_previous_closes_the_switch_snackbar() {
        let mut m = model(&["a", "b", "c"]);
        switched(m.switch(&key("c"), false));
        assert_eq!(m.previous(), Some(&key("a")));
        assert!(m.switch_snackbar().is_some());
        // `a`, the default and Switch back's target, comes in front.
        let change = m.held_changed(holding(&["a", "b"]));
        assert_eq!(change.map(|c| c.came_in_front), Ok(Some(key("a"))));
        assert_eq!(m.front(), Some(&key("a")));
        assert_eq!(m.switch_snackbar(), None);
        assert_eq!(m.previous(), None);
    }

    #[test]
    fn the_front_still_held_stays_whatever_the_default() {
        let mut m = model(&["a", "b"]);
        switched(m.switch(&key("b"), false));
        let change = m.held_changed(holding(&["a", "b", "c"]));
        assert_eq!(change, Ok(HeldChange::default()));
        assert_eq!(m.front(), Some(&key("b")));
    }

    #[test]
    fn with_no_default_the_first_identity_comes_in_front() {
        let mut m = Identities::new();
        let mut held = holding(&["a", "b"]);
        held[0].is_default = false;
        m.held_changed(held).ok();
        assert_eq!(m.start(None), Some(&key("a")));
    }

    #[test]
    fn the_app_starts_with_the_last_in_front_if_held_else_the_default() {
        let mut m = model(&["a", "b"]);
        assert_eq!(m.start(Some(key("b"))), Some(&key("b")));
        assert_eq!(m.start(Some(key("gone"))), Some(&key("a")));
        assert_eq!(m.start(None), Some(&key("a")));
    }

    #[test]
    fn more_than_the_bound_is_refused_and_add_stops_at_it() {
        let tags: Vec<String> = (0..=IDENTITY_CAP).map(|i| i.to_string()).collect();
        let refs: Vec<&str> = tags.iter().map(String::as_str).collect();
        let mut m = model(&["a", "b"]);
        assert_eq!(m.held_changed(holding(&refs)), Err(HeldProblem::OverBound));
        assert_eq!(m.held().len(), 2, "what it held is kept");
        assert!(m.can_add());
        assert!(m.held_changed(holding(&refs[..IDENTITY_CAP])).is_ok());
        assert_eq!(m.held().len(), IDENTITY_CAP);
        assert!(!m.can_add());
        assert!(m.held_changed(holding(&refs[..IDENTITY_CAP - 1])).is_ok());
        assert!(m.can_add());
    }

    #[test]
    fn a_key_held_twice_is_refused() {
        let mut m = model(&["a", "b"]);
        assert_eq!(
            m.held_changed(holding(&["a", "c", "a"])),
            Err(HeldProblem::DuplicateKey)
        );
        assert_eq!(m.held().len(), 2);
    }

    #[test]
    fn choosing_the_row_in_front_changes_nothing() {
        let mut m = model(&["a", "b"]);
        assert_eq!(m.switch(&key("a"), false), FrontOutcome::Unchanged);
        assert_eq!(m.switch(&key("a"), true), FrontOutcome::Unchanged);
        assert_eq!(m.switch(&key("gone"), false), FrontOutcome::Unchanged);
        assert_eq!(m.front(), Some(&key("a")));
        assert_eq!(m.previous(), None);
    }

    #[test]
    fn a_switch_names_the_new_identity_and_focuses_the_switch() {
        let mut m = model(&["a", "b"]);
        let s = switched(m.switch(&key("b"), false));
        assert_eq!(s.to, key("b"));
        assert_eq!(s.previous, key("a"));
        assert!(s.focus_switch && !s.from_notification);
        assert_eq!(s.snackbar.action, UiText::SwitchBack);
        assert_eq!(
            s.snackbar.said.english(),
            format!("You are now using {}", name("b"))
        );
        assert_eq!(m.front(), Some(&key("b")));
        assert_eq!(m.previous(), Some(&key("a")));
    }

    #[test]
    fn switch_back_returns_by_the_same_rule() {
        let mut m = model(&["a", "b", "c"]);
        switched(m.switch(&key("c"), false));
        let s = switched(m.switch_back(false));
        assert_eq!((s.to, s.previous), (key("a"), key("c")));
        let s = switched(m.switch_back(false));
        assert_eq!(s.to, key("c"));
        assert_eq!(m.switch_back(true), FrontOutcome::AskDiscard);
        assert_eq!(m.front(), Some(&key("c")));
        assert_eq!(
            Identities::new().switch_back(false),
            FrontOutcome::Unchanged
        );
    }

    #[test]
    fn an_unsaved_edit_asks_first_and_cancel_leaves_everything() {
        let mut m = model(&["a", "b"]);
        assert_eq!(m.switch(&key("b"), true), FrontOutcome::AskDiscard);
        assert_eq!(
            m.front(),
            Some(&key("a")),
            "nothing switched before the answer"
        );
        m.discard_cancelled();
        assert_eq!(m.front(), Some(&key("a")));
        assert_eq!(m.previous(), None);
        assert_eq!(
            m.discard_confirmed(0),
            FrontOutcome::Unchanged,
            "nothing held"
        );
        assert_eq!(m.switch(&key("b"), true), FrontOutcome::AskDiscard);
        let s = switched(m.discard_confirmed(0));
        assert_eq!(s.to, key("b"));
        assert_eq!(m.front(), Some(&key("b")));
    }

    #[test]
    fn a_switch_closes_every_pane_of_one_identity() {
        let stays: Vec<Pane> = Pane::ALL
            .iter()
            .copied()
            .filter(|p| !p.closes_on_switch())
            .collect();
        assert_eq!(stays, [Pane::YourIdentities]);
        assert!(Pane::Trust.closes_on_switch() && Pane::Peers.closes_on_switch());
    }

    #[test]
    fn the_badge_counts_the_others_and_names_nothing() {
        let mut held = holding(&["a", "b", "c"]);
        held[0].unread = 100;
        held[0].not_sent = true;
        held[1].unread = 2;
        held[2].unread = 3;
        let mut m = Identities::new();
        m.held_changed(held.clone()).ok();
        m.start(None);
        assert_eq!(
            m.badge(),
            Some(Badge {
                unread: 5,
                not_sent: false
            }),
            "the one in front's are not counted"
        );
        let v = m.switch_view();
        let accessible: Vec<String> = v
            .iter()
            .flat_map(|v| v.accessible.iter().map(Said::english))
            .collect();
        assert_eq!(
            accessible,
            [
                format!("Identity: {}. Switch identity", name("a")),
                "5 unread messages in your other identities".to_owned(),
            ]
        );
        for said in v.iter().flat_map(|v| v.accessible.iter().skip(1)) {
            assert!(said.values.iter().all(|(n, _)| *n == "n"), "names nothing");
        }
        // Nothing unread there but a message not sent: a dot, no count.
        held[1].unread = 0;
        held[2].unread = 0;
        held[2].not_sent = true;
        m.held_changed(held.clone()).ok();
        let v = m.switch_view();
        assert_eq!(
            v.as_ref().and_then(|v| v.badge),
            Some(Badge {
                unread: 0,
                not_sent: true
            })
        );
        assert_eq!(
            v.map(|v| v
                .accessible
                .iter()
                .skip(1)
                .map(|s| s.text)
                .collect::<Vec<_>>()),
            Some(vec![UiText::IdentityOthersNotSentInOthers])
        );
        // Both: the count and both strings.
        held[1].unread = 1;
        m.held_changed(held.clone()).ok();
        assert_eq!(
            m.switch_view().map(|v| v
                .accessible
                .iter()
                .skip(1)
                .map(|s| s.text)
                .collect::<Vec<_>>()),
            Some(vec![
                UiText::IdentityOthersUnreadOneInOthers,
                UiText::IdentityOthersNotSentInOthers
            ])
        );
        // Nothing there: no badge.
        held[1].unread = 0;
        held[2].not_sent = false;
        m.held_changed(held).ok();
        assert_eq!(m.badge(), None);
        assert_eq!(m.switch_view().map(|v| v.accessible.len()), Some(1));
    }

    #[test]
    fn the_badge_text_takes_one_other_identitys_form_with_two_held() {
        let mut held = holding(&["a", "b"]);
        held[1].unread = 1;
        let mut m = Identities::new();
        m.held_changed(held.clone()).ok();
        m.start(None);
        let texts = |m: &Identities| m.switch_view().map(|v| v.accessible[1].text);
        assert_eq!(texts(&m), Some(UiText::IdentityOthersUnreadOneInOther));
        held[1].unread = 4;
        m.held_changed(held.clone()).ok();
        assert_eq!(texts(&m), Some(UiText::IdentityOthersUnreadInOther));
        assert_eq!(
            m.switch_view().map(|v| v.accessible[1].english()),
            Some("4 unread messages in your other identity".to_owned())
        );
        // A message of the one other identity not sent is said in the
        // same form, never as "another" identity.
        held[1].unread = 0;
        held[1].not_sent = true;
        m.held_changed(held).ok();
        assert_eq!(texts(&m), Some(UiText::IdentityOthersNotSentInOther));
    }

    #[test]
    fn the_badge_sum_saturates() {
        let mut held = holding(&["a", "b", "c"]);
        held[1].unread = u32::MAX;
        held[2].unread = 9;
        let mut m = Identities::new();
        m.held_changed(held).ok();
        m.start(None);
        assert_eq!(m.badge().map(|b| b.unread), Some(u32::MAX));
    }

    #[test]
    fn the_switcher_lists_every_identity_with_the_one_in_front_chosen() {
        let mut held = holding(&["a", "b"]);
        held[1].unread = 1;
        held[1].not_sent = true;
        held[1].card_name = None;
        held[1].state = IdentityState::NotHidden;
        let mut m = Identities::new();
        m.held_changed(held).ok();
        m.start(None);
        let s = m.switcher();
        assert_eq!(
            s.as_ref().map(|s| s.title),
            Some(UiText::IdentitySwitchTitle)
        );
        assert_eq!(
            s.as_ref().map(|s| s.identities),
            Some(UiText::IdentitiesTitle)
        );
        let rows = s.map(|s| s.rows).unwrap_or_default();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].chosen && !rows[1].chosen);
        assert_eq!(rows[0].unread_said, None);
        assert_eq!(rows[0].not_sent, None);
        assert_eq!(rows[0].state, None);
        let b = &rows[1];
        assert_eq!(b.name, "alias-b");
        assert_eq!(b.second_line, key("b").short());
        assert_eq!(
            b.second_line_label.english(),
            format!(
                "People who exchange messages with this identity see it as {}",
                key("b").short()
            )
        );
        assert_eq!(
            b.unread_said.as_ref().map(Said::english),
            Some("1 unread message".to_owned())
        );
        assert_eq!(b.not_sent, Some(UiText::IdentityNotSent));
        assert_eq!(
            b.state.as_ref().map(|s| s.text),
            Some(UiText::IdentityNotHidden)
        );
        assert!(b.try_again.is_some_and(|t| t.enabled && !t.progress));
        switched(m.switch(&key("b"), false));
        let rows = m.switcher().map(|s| s.rows).unwrap_or_default();
        assert!(!rows[0].chosen && rows[1].chosen, "the check mark moves");
    }

    #[test]
    fn the_composer_names_the_identity_or_its_state() {
        let mut held = holding(&["a", "b"]);
        let mut m = Identities::new();
        m.held_changed(held.clone()).ok();
        m.start(None);
        let a = key("a");
        let c = m.composer(&a);
        assert_eq!(
            c.as_ref().and_then(|c| c.line.as_ref()).map(Said::english),
            Some(format!("As {}", name("a")))
        );
        assert_eq!(
            c.map(|c| c.send),
            Some(SendControl::Enabled {
                name: Some(Said::naming(UiText::SendAs, &held[0]))
            })
        );
        for (state, text, retry) in [
            (
                IdentityState::Starting {
                    after_try_again: false,
                },
                UiText::IdentityStarting,
                None,
            ),
            (
                IdentityState::Starting {
                    after_try_again: true,
                },
                UiText::IdentityStarting,
                Some((false, true)),
            ),
            (
                IdentityState::NotHidden,
                UiText::IdentityNotHidden,
                Some((true, false)),
            ),
            (
                IdentityState::Unavailable,
                UiText::IdentityUnavailable,
                None,
            ),
        ] {
            held[0].state = state;
            m.held_changed(held.clone()).ok();
            let c = m.composer(&a);
            let line = c.as_ref().and_then(|c| c.line.clone());
            assert_eq!(line.as_ref().map(|l| l.text), Some(text), "{state:?}");
            assert_eq!(
                c.as_ref().map(|c| &c.send),
                Some(&SendControl::Disabled {
                    name: Some(Said::naming(UiText::SendAs, &held[0])),
                    reason: line
                }),
                "{state:?}: disabled, named as when enabled, with the line as its reason"
            );
            assert_eq!(
                c.and_then(|c| c.try_again).map(|t| (t.enabled, t.progress)),
                retry,
                "{state:?}"
            );
        }
        assert_eq!(m.composer(&key("gone")), None);
    }

    #[test]
    fn the_front_leaving_ready_disables_send() {
        let mut held = holding(&["a", "b"]);
        let mut m = Identities::new();
        m.held_changed(held.clone()).ok();
        m.start(None);
        held[1].state = IdentityState::NotHidden;
        assert_eq!(
            m.held_changed(held.clone()).map(|c| c.send_disabled),
            Ok(false),
            "another identity's state is not the front's"
        );
        held[0].state = IdentityState::Starting {
            after_try_again: false,
        };
        assert_eq!(
            m.held_changed(held.clone()).map(|c| c.send_disabled),
            Ok(true)
        );
        held[0].state = IdentityState::NotHidden;
        assert_eq!(
            m.held_changed(held).map(|c| c.send_disabled),
            Ok(false),
            "it could not send before either"
        );
    }

    #[test]
    fn each_acting_screen_names_the_identity_in_front() {
        let mut m = model(&["a", "b"]);
        switched(m.switch(&key("b"), false));
        let b = name("b");
        assert_eq!(
            m.new_conversation_line().map(|s| s.english()),
            Some(format!("You are starting this conversation as {b}."))
        );
        assert_eq!(
            m.share_card_line().map(|s| s.english()),
            Some(format!("as {b}"))
        );
        assert_eq!(
            m.proposal_line().map(|s| s.english()),
            Some(format!(
                "If you accept, the card is saved in the contacts of {b}."
            ))
        );
        assert_eq!(m.subtitle(), Some("Name b".to_owned()));
        assert_eq!(
            m.not_in_front_line(&key("a")).map(|s| s.english()),
            Some(format!(
                "You are using {b}. What you change on this page applies to {}.",
                name("a")
            ))
        );
        assert_eq!(m.not_in_front_line(&key("b")), None, "the page in front");
        assert_eq!(m.not_in_front_line(&key("gone")), None);
    }

    #[test]
    fn remove_is_absent_on_the_first_and_the_default() {
        let mut held = holding(&["a", "b", "c"]);
        held[0].is_default = false;
        held[1].is_default = true;
        let mut m = Identities::new();
        m.held_changed(held).ok();
        assert!(!m.can_remove(&key("a")), "the first");
        assert!(!m.can_remove(&key("b")), "the default");
        assert!(m.can_remove(&key("c")));
        assert!(!m.can_remove(&key("gone")));
    }

    #[test]
    fn a_notification_of_the_front_shows_its_chats_with_no_switch() {
        let mut m = model(&["a", "b"]);
        let out = m.open_notification(NotificationOpened::Identity(key("a")), true, 0);
        assert_eq!(out, FrontOutcome::ShowChats);
        assert!(!m.is_opening());
        assert_eq!(m.previous(), None);
    }

    #[test]
    fn the_transition_holds_a_second_and_until_ready() {
        let mut m = model(&["a", "b"]);
        let out = m.open_notification(NotificationOpened::Identity(key("b")), false, 1_000);
        let FrontOutcome::Opening(t) = out else {
            panic!("no transition: {out:?}");
        };
        assert_eq!(t.to, key("b"));
        assert_eq!(t.said.english(), format!("Switching to {}", name("b")));
        assert_eq!(m.front(), Some(&key("b")), "in front before anything shows");
        // Held a second, not yet ready.
        assert_eq!(m.tick(2_500), None);
        assert_eq!(m.next_change(2_500), None, "it waits on its Chats");
        // Ready, after the second: it leaves at once.
        let s = m.opening_ready(2_500);
        assert!(
            s.as_ref()
                .is_some_and(|s| s.from_notification && !s.focus_switch)
        );
        assert_eq!(s.map(|s| s.previous), Some(key("a")));
        assert!(!m.is_opening());
        // Ready before the second: held until it.
        m.open_notification(NotificationOpened::Identity(key("a")), false, 10_000);
        assert_eq!(m.opening_ready(10_200), None);
        assert_eq!(m.next_change(10_200), Some(11_000));
        assert_eq!(m.tick(10_999), None);
        let s = m.tick(11_000);
        assert_eq!(
            s.map(|s| (s.to, s.snackbar.action)),
            Some((key("a"), UiText::SwitchBack))
        );
        assert_eq!(m.tick(12_000), None, "once");
    }

    #[test]
    fn a_notification_switch_asks_about_an_unsaved_edit_first() {
        let mut m = model(&["a", "b"]);
        let out = m.open_notification(NotificationOpened::Identity(key("b")), true, 0);
        assert_eq!(out, FrontOutcome::AskDiscard);
        assert_eq!(m.front(), Some(&key("a")));
        m.discard_cancelled();
        assert!(!m.is_opening());
        assert_eq!(m.front(), Some(&key("a")));
        m.open_notification(NotificationOpened::Identity(key("b")), true, 0);
        assert!(matches!(m.discard_confirmed(5), FrontOutcome::Opening(_)));
        assert_eq!(m.front(), Some(&key("b")));
    }

    #[test]
    fn a_notification_of_no_one_identity_switches_only_to_the_one_with_news() {
        let mut m = model(&["a", "b", "c"]);
        let none = NotificationOpened::NoOneIdentity;
        assert_eq!(
            m.open_notification(none(vec![key("b"), key("c")]), false, 0),
            FrontOutcome::OpenSwitcher
        );
        assert_eq!(m.front(), Some(&key("a")), "switching nothing");
        assert_eq!(
            m.open_notification(none(vec![]), false, 0),
            FrontOutcome::ShowChats
        );
        assert_eq!(
            m.open_notification(none(vec![key("a")]), false, 0),
            FrontOutcome::ShowChats
        );
        assert!(matches!(
            m.open_notification(none(vec![key("c"), key("c"), key("gone")]), false, 0),
            FrontOutcome::Opening(t) if t.to == key("c")
        ));
    }

    #[test]
    fn a_held_change_during_the_transition_ends_it_if_its_identity_is_gone() {
        let mut m = model(&["a", "b"]);
        m.open_notification(NotificationOpened::Identity(key("b")), false, 0);
        let change = m.held_changed(holding(&["a"]));
        assert!(change.is_ok_and(|c| c.transition_left && c.came_in_front == Some(key("a"))));
        assert!(!m.is_opening());
        assert_eq!(m.opening_ready(5_000), None);
        assert_eq!(m.switch(&key("a"), false), FrontOutcome::Unchanged);
    }

    #[test]
    fn nothing_switches_while_the_transition_shows() {
        let mut m = model(&["a", "b", "c"]);
        m.open_notification(NotificationOpened::Identity(key("b")), false, 0);
        assert_eq!(m.switch(&key("c"), false), FrontOutcome::Unchanged);
        assert_eq!(m.switch_back(false), FrontOutcome::Unchanged);
        assert_eq!(m.front(), Some(&key("b")));
    }

    #[test]
    fn the_lock_screen_entry_is_one_and_names_nothing() {
        let entry = Identities::locked_entry();
        assert_eq!(entry.text, UiText::MessagesChannel);
        assert_eq!(placeholder_en::text(entry.text), "New messages");
        assert!(
            !placeholder_en::text(entry.text).contains('{'),
            "no name, no count"
        );
    }

    #[test]
    fn the_summary_names_each_identity_with_its_count() {
        let m = model(&["a", "b"]);
        let lines: Vec<String> = m
            .summary_lines(&[
                (key("a"), 1),
                (key("b"), 3),
                (key("gone"), 2),
                (key("b"), 0),
            ])
            .iter()
            .map(Said::english)
            .collect();
        assert_eq!(
            lines,
            [
                format!("{}: 1 new message", name("a")),
                format!("{}: 3 new messages", name("b"))
            ]
        );
        assert_eq!(m.notification_subtext(&key("b")), Some("Name b".to_owned()));
    }

    fn focus(m: &Identities) -> Option<IdentityFocus> {
        m.focus_request().map(|(_, f)| f)
    }

    #[test]
    fn the_switcher_opens_with_two_held_and_closes_to_the_switch() {
        let mut one = model(&["a"]);
        assert_eq!(
            one.act(IdentityIntent::OpenSwitcher, false),
            FrontOutcome::Unchanged
        );
        assert!(
            !one.switcher_open(),
            "one held: no switcher (ADR-0061 d. 3)"
        );
        assert_eq!(one.focus_request(), None);
        let mut none = Identities::new();
        none.act(IdentityIntent::OpenSwitcher, false);
        assert!(!none.switcher_open(), "none held");

        let mut m = model(&["a", "b"]);
        m.act(IdentityIntent::OpenSwitcher, false);
        assert!(m.switcher_open());
        assert_eq!(focus(&m), Some(IdentityFocus::SwitcherTitle), "F1");
        let asked = m.focus_request().map(|(seq, _)| seq);
        m.act(IdentityIntent::CloseSwitcher, false);
        assert!(!m.switcher_open());
        assert_eq!(focus(&m), Some(IdentityFocus::Switch), "F2");
        assert_ne!(
            m.focus_request().map(|(seq, _)| seq),
            asked,
            "a new request"
        );
        assert_eq!(m.front(), Some(&key("a")), "nothing changed");
        assert_eq!(m.previous(), None);
        assert_eq!(m.switch_snackbar(), None);
        // Closing what is not open asks nothing.
        let asked = m.focus_request();
        m.act(IdentityIntent::CloseSwitcher, false);
        assert_eq!(m.focus_request(), asked);
    }

    #[test]
    fn choosing_another_row_switches_shows_its_snackbar_and_focuses_the_switch() {
        let mut m = model(&["a", "b"]);
        m.act(IdentityIntent::OpenSwitcher, false);
        let s = switched(m.act(IdentityIntent::Choose(key("b")), false));
        assert_eq!(s.to, key("b"));
        assert!(!m.switcher_open(), "closed by the choice");
        assert_eq!(focus(&m), Some(IdentityFocus::Switch), "F3");
        let shown = m.switch_snackbar().cloned();
        assert_eq!(shown.as_ref().map(|s| &s.snackbar), Some(&s.snackbar));
        assert_eq!(
            shown.map(|s| s.snackbar.said.english()),
            Some(format!("You are now using {}", name("b")))
        );
        // Switch back on it: back to a, a snackbar of its own.
        let id = m.switch_snackbar().map(|s| s.id).unwrap_or_default();
        let back = switched(m.act(IdentityIntent::SwitchBack(id), false));
        assert_eq!(back.to, key("a"));
        assert_eq!(focus(&m), Some(IdentityFocus::Switch), "F3");
        assert!(
            m.switch_snackbar().is_some_and(|s| s.id != id),
            "a new snackbar replaces the one acted on"
        );
    }

    #[test]
    fn choosing_the_chosen_row_closes_the_switcher_with_nothing_changed() {
        let mut m = model(&["a", "b"]);
        m.act(IdentityIntent::OpenSwitcher, false);
        assert_eq!(
            m.act(IdentityIntent::Choose(key("a")), false),
            FrontOutcome::Unchanged
        );
        assert!(!m.switcher_open());
        assert_eq!(focus(&m), Some(IdentityFocus::Switch), "F2");
        assert_eq!(m.switch_snackbar(), None);
        assert_eq!(m.previous(), None);
    }

    #[test]
    fn a_choice_is_taken_from_the_open_switcher_alone() {
        let mut m = model(&["a", "b"]);
        assert_eq!(
            m.act(IdentityIntent::Choose(key("b")), false),
            FrontOutcome::Unchanged
        );
        assert_eq!(m.front(), Some(&key("a")));
        assert_eq!(m.switch_snackbar(), None);
        // An unsaved edit asks first, and the switcher is closed meanwhile.
        m.act(IdentityIntent::OpenSwitcher, false);
        assert_eq!(
            m.act(IdentityIntent::Choose(key("b")), true),
            FrontOutcome::AskDiscard
        );
        assert!(!m.switcher_open());
        assert_eq!(m.switch_snackbar(), None, "nothing switched yet");
        switched(m.discard_confirmed(0));
        assert!(
            m.switch_snackbar().is_some(),
            "the switch's snackbar once done"
        );
    }

    #[test]
    fn switch_back_and_a_close_name_their_own_snackbar() {
        let mut m = model(&["a", "b", "c"]);
        m.act(IdentityIntent::OpenSwitcher, false);
        switched(m.act(IdentityIntent::Choose(key("b")), false));
        let first = m.switch_snackbar().map(|s| s.id).unwrap_or_default();
        m.act(IdentityIntent::OpenSwitcher, false);
        switched(m.act(IdentityIntent::Choose(key("c")), false));
        let second = m.switch_snackbar().map(|s| s.id).unwrap_or_default();
        assert_ne!(first, second);
        assert_eq!(
            m.act(IdentityIntent::SwitchBack(first), false),
            FrontOutcome::Unchanged,
            "a replaced snackbar's Switch back"
        );
        assert_eq!(m.front(), Some(&key("c")));
        m.act(IdentityIntent::SnackbarClosed(first), false);
        assert_eq!(m.switch_snackbar().map(|s| s.id), Some(second));
        m.act(IdentityIntent::SnackbarClosed(second), false);
        assert_eq!(m.switch_snackbar(), None);
        assert_eq!(
            m.act(IdentityIntent::SwitchBack(second), false),
            FrontOutcome::Unchanged,
            "a closed snackbar's"
        );
        assert_eq!(m.front(), Some(&key("c")));
    }

    #[test]
    fn your_identities_and_try_again_switch_nothing() {
        let mut m = model(&["a", "b"]);
        m.act(IdentityIntent::OpenSwitcher, false);
        assert_eq!(
            m.act(IdentityIntent::OpenIdentities, false),
            FrontOutcome::Unchanged
        );
        assert!(!m.switcher_open());
        assert_eq!(
            m.act(IdentityIntent::TryAgain(key("b")), false),
            FrontOutcome::Unchanged
        );
        assert_eq!(m.front(), Some(&key("a")));
        assert_eq!(m.switch_snackbar(), None);
    }

    #[test]
    fn going_back_to_one_closes_the_switcher_and_its_snackbar() {
        let mut m = model(&["a", "b", "c"]);
        m.act(IdentityIntent::OpenSwitcher, false);
        switched(m.act(IdentityIntent::Choose(key("b")), false));
        m.act(IdentityIntent::OpenSwitcher, false);
        // The one Switch back returns to goes: so does its snackbar, and
        // Switch back has no target, while the front stays.
        assert_eq!(m.previous(), Some(&key("a")));
        m.held_changed(holding(&["b", "c"])).ok();
        assert!(m.switcher_open(), "two still held");
        assert_eq!(m.front(), Some(&key("b")));
        assert_eq!(m.switch_snackbar(), None);
        assert_eq!(m.previous(), None);
        m.held_changed(holding(&["b"])).ok();
        assert!(!m.switcher_open());
        // Held again, it does not come back open.
        m.held_changed(holding(&["b", "c"])).ok();
        assert!(!m.switcher_open());
    }

    #[test]
    fn the_transition_closes_the_switcher_and_its_switch_shows_the_snackbar_without_focus() {
        let mut m = model(&["a", "b"]);
        m.act(IdentityIntent::OpenSwitcher, false);
        let asked = m.focus_request();
        m.open_notification(NotificationOpened::Identity(key("b")), false, 0);
        assert!(!m.switcher_open());
        m.act(IdentityIntent::OpenSwitcher, false);
        assert!(!m.switcher_open(), "not over the transition");
        assert!(m.opening_ready(SWITCHING_HOLD_MS).is_some());
        assert!(m.switch_snackbar().is_some());
        assert_eq!(m.focus_request(), asked, "the Chats it opens takes focus");
    }

    #[test]
    fn the_switch_keeps_three_graphemes_of_the_name() {
        let mut held = holding(&["a", "b"]);
        // "e" with a combining acute is one grapheme of two characters.
        held[0].card_name = Some("e\u{301}xyz".to_owned());
        let mut m = Identities::new();
        m.held_changed(held.clone()).ok();
        m.start(None);
        assert_eq!(
            m.switch_view().map(|v| v.least),
            Some("e\u{301}xy".to_owned())
        );
        held[0].card_name = Some("Al".to_owned());
        m.held_changed(held).ok();
        assert_eq!(m.switch_view().map(|v| v.least), Some("Al".to_owned()));
    }

    #[test]
    fn every_held_identitys_least_is_offered_whoever_is_in_front() {
        let mut held = holding(&["a", "b", "c"]);
        held[0].card_name = Some("Al".to_owned());
        held[1].card_name = Some("Wolfgang".to_owned());
        held[2].card_name = Some("e\u{301}xyz".to_owned());
        let mut m = Identities::new();
        m.held_changed(held).ok();
        m.start(None);
        let every = vec!["Al".to_owned(), "Wol".to_owned(), "e\u{301}xy".to_owned()];
        assert_eq!(m.switch_view().map(|v| v.leasts), Some(every.clone()));
        m.act(IdentityIntent::OpenSwitcher, false);
        m.act(IdentityIntent::Choose(m.held()[1].key.clone()), false);
        let after = m.switch_view().expect("two or more held");
        assert_eq!(after.least, "Wol", "the premise: another in front");
        assert_eq!(after.leasts, every, "the same, whoever is in front");
    }

    fn page(m: &Identities) -> IdentitiesPage {
        m.your_identities().expect("Your identities shown")
    }

    fn row<'a>(page: &'a IdentitiesPage, tag: &str) -> &'a IdentityRow {
        page.rows
            .iter()
            .find(|r| r.key == key(tag))
            .expect("the row")
    }

    /// F4: Your identities at the switcher's bottom closes the switcher,
    /// opens the list -- it opened nothing before (identities §6.1, U3)
    /// -- and its title takes focus; nothing switches. Mutation: the
    /// act left as it was, closing the switcher alone.
    #[test]
    fn your_identities_opens_from_the_switcher_and_focuses_its_title() {
        let mut m = model(&["a", "b"]);
        m.act(IdentityIntent::OpenSwitcher, false);
        assert_eq!(
            m.act(IdentityIntent::OpenIdentities, false),
            FrontOutcome::Unchanged
        );
        assert!(!m.switcher_open());
        assert!(m.identities_open());
        assert_eq!(focus(&m), Some(IdentityFocus::IdentitiesTitle));
        assert_eq!(m.front(), Some(&key("a")), "nothing switched");
        let p = page(&m);
        assert_eq!(p.title, UiText::IdentitiesTitle);
        assert_eq!(p.statement, HostText::IdentitiesShareDevice);
        assert_eq!(p.unlisted, UiText::IdentitiesUnlisted);

        assert_eq!(
            m.act(IdentityIntent::CloseIdentities, false),
            FrontOutcome::Unchanged
        );
        assert!(m.your_identities().is_none(), "closed");
    }

    /// Cards §5.1: the list is shown while two or more are held; with
    /// one, the root opens that identity's card and the list opens
    /// nothing, then or when a second identity arrives unasked. Back to
    /// one, an open list closes. Mutation: the list opened whatever is
    /// held.
    #[test]
    fn with_one_identity_there_is_no_list() {
        let mut m = model(&["a"]);
        m.act(IdentityIntent::OpenIdentities, false);
        assert!(m.your_identities().is_none());
        assert_eq!(focus(&m), None, "no title to focus");
        m.held_changed(holding(&["a", "b"])).expect("held");
        assert!(m.your_identities().is_none(), "not opened by the arrival");

        m.act(IdentityIntent::OpenIdentities, false);
        assert!(m.your_identities().is_some());
        m.held_changed(holding(&["a"])).expect("held");
        assert!(m.your_identities().is_none());
        m.held_changed(holding(&["a", "b"])).expect("held");
        assert!(m.your_identities().is_none(), "closed on the way to one");
    }

    /// Cards §5.1 and identities §6.1: each row, in the order held,
    /// reads its name -- the card's, else the alias -- its short
    /// `EndpointId` with `IdentityIdLabel`, `IdentityDefault` on the
    /// default's row alone, and the check mark with `IdentityInFront` on
    /// the identity in front alone; a row that cannot act shows its
    /// state and Try again as the switcher's does. A row opens its page
    /// and switches nothing. Mutation: every row marked in use.
    #[test]
    fn a_row_reads_its_name_its_short_id_default_and_in_use() {
        let mut m = Identities::new();
        let mut held = holding(&["a", "b", "c"]);
        held[1].card_name = None;
        held[2].state = IdentityState::NotHidden;
        m.held_changed(held).expect("held");
        m.start(Some(key("c")));
        m.act(IdentityIntent::OpenIdentities, false);
        let p = page(&m);
        let keys: Vec<_> = p.rows.iter().map(|r| r.key.clone()).collect();
        assert_eq!(keys, [key("a"), key("b"), key("c")], "the held order");

        let first = row(&p, "a");
        assert_eq!(first.name, "Name a");
        assert_eq!(first.second_line, key("a").short());
        assert_eq!(first.second_line_label.text, UiText::IdentityIdLabel);
        assert_eq!(
            first.second_line_label.values,
            [("short", key("a").short())]
        );
        assert_eq!(first.default, Some(UiText::IdentityDefault));
        assert!(!first.in_front);
        assert_eq!(first.description, None);
        assert_eq!(first.state, None);

        let second = row(&p, "b");
        assert_eq!(second.name, "alias-b", "no card: the alias");
        assert_eq!(second.default, None);
        assert!(!second.in_front);

        let third = row(&p, "c");
        assert!(third.in_front);
        assert_eq!(third.description, Some(UiText::IdentityInFront));
        assert_eq!(
            third.state.as_ref().map(|s| s.text),
            Some(UiText::IdentityNotHidden)
        );
        assert_eq!(third.try_again.map(|t| t.enabled), Some(true));
        assert_eq!(p.rows.iter().filter(|r| r.in_front).count(), 1);

        assert_eq!(
            m.act(IdentityIntent::OpenRow(key("b")), false),
            FrontOutcome::OpenPage(key("b"))
        );
        assert_eq!(m.front(), Some(&key("c")), "opening switched nothing");
        assert_eq!(
            m.act(IdentityIntent::OpenRow(key("z")), false),
            FrontOutcome::Unchanged,
            "not held"
        );
    }

    /// Android composes its roster with no add (plan §1.1, "Root can
    /// add"): Add is drawn only where the root said it can add, and is
    /// not drawn until it says so. Mutation: a root that can add by
    /// default.
    #[test]
    fn add_is_drawn_only_on_a_root_that_can_add() {
        let mut m = model(&["a", "b"]);
        m.act(IdentityIntent::OpenIdentities, false);
        assert_eq!(page(&m).add, None);
        assert_eq!(m.act(IdentityIntent::Add, false), FrontOutcome::Unchanged);

        m.set_adds(true);
        assert_eq!(
            page(&m).add,
            Some(AddControl {
                text: UiText::AddIdentity,
                limit: None,
                disabled: None,
            })
        );
        assert_eq!(m.act(IdentityIntent::Add, false), FrontOutcome::OpenAdd);

        m.set_adds(false);
        assert_eq!(page(&m).add, None);
    }

    /// F8 and cards §5.1, "At the bound", in the interim of Q-A2 (a): the
    /// list counts the identities held, so with 63 held Add opens the
    /// form, and with 64 -- `IDENTITY_CAP`, the profile's bound -- it
    /// opens nothing, `IdentityLimit` stands under it and takes focus.
    /// Mutation: `<=` for `<` in `can_add`.
    #[test]
    fn at_the_bound_add_opens_nothing_and_the_limit_takes_focus() {
        let tags: Vec<String> = (0..IDENTITY_CAP).map(|n| format!("i{n}")).collect();
        let tags: Vec<&str> = tags.iter().map(String::as_str).collect();
        let mut m = model(&tags[..IDENTITY_CAP - 1]);
        m.set_adds(true);
        m.act(IdentityIntent::OpenIdentities, false);
        assert_eq!(page(&m).add.and_then(|a| a.limit), None);
        assert_eq!(m.act(IdentityIntent::Add, false), FrontOutcome::OpenAdd);

        m.held_changed(holding(&tags)).expect("64 held");
        assert_eq!(
            page(&m).add.and_then(|a| a.limit),
            Some(UiText::IdentityLimit)
        );
        assert_eq!(m.act(IdentityIntent::Add, false), FrontOutcome::Unchanged);
        assert_eq!(focus(&m), Some(IdentityFocus::Limit));
    }

    /// F6 and F9, identities §6.2's last outcome: Done opens the list on
    /// the new row; with its card not saved the row is named by its
    /// alias and says `IdentityCardNotSaved` until a held list gives it a
    /// card's name, whatever else that list changes. A row not held yet
    /// sends focus to the title (Q-U5). The marks are as many as a
    /// profile holds. Mutation: the mark kept when the card is saved.
    #[test]
    fn a_row_whose_card_was_not_saved_says_so_until_its_card_is_saved() {
        let mut m = model(&["a", "b"]);
        let mut held = holding(&["a", "b", "c"]);
        held[2].card_name = None;
        m.held_changed(held.clone()).expect("held");
        m.added(&key("c"), Some(AddValues::default()));
        assert_eq!(focus(&m), Some(IdentityFocus::Row));
        assert_eq!(m.focus_row(), Some(&key("c")));
        let p = page(&m);
        let c = row(&p, "c");
        assert_eq!(c.name, "alias-c");
        assert_eq!(c.card_not_saved, Some(UiText::IdentityCardNotSaved));
        assert_eq!(row(&p, "b").card_not_saved, None);

        held[2].unread = 3;
        m.held_changed(held.clone()).expect("held");
        assert_eq!(
            row(&page(&m), "c").card_not_saved,
            Some(UiText::IdentityCardNotSaved),
            "still no card"
        );
        held[2].card_name = Some("Name c".to_owned());
        m.held_changed(held).expect("held");
        assert_eq!(row(&page(&m), "c").card_not_saved, None, "saved");

        m.added(&key("b"), None);
        assert_eq!(row(&page(&m), "b").card_not_saved, None);
        assert_eq!(focus(&m), Some(IdentityFocus::Row));
        assert_eq!(m.focus_row(), Some(&key("b")));
        m.added(&key("not-held"), None);
        assert_eq!(focus(&m), Some(IdentityFocus::IdentitiesTitle));

        for n in 0..=IDENTITY_CAP {
            m.added(&key(&format!("x{n}")), Some(AddValues::default()));
        }
        assert_eq!(m.cards_not_saved.len(), IDENTITY_CAP);
        assert!(m.kept_values(&key("x0")).is_none(), "the oldest went");
    }

    /// With one identity held before, Done may come before the new
    /// identity is held: the list it opened shows once that identity is,
    /// a held list of one in between closing nothing (identities §6.2
    /// step 4, Q-U5). Mutation: any held list of one closing the list.
    #[test]
    fn done_before_the_new_identity_is_held_shows_the_list_when_it_is() {
        let mut m = model(&["a"]);
        m.added(&key("c"), Some(AddValues::default()));
        assert!(m.your_identities().is_none(), "one held: no list yet");
        assert_eq!(focus(&m), Some(IdentityFocus::IdentitiesTitle));
        let mut one = holding(&["a"]);
        one[0].unread = 1;
        m.held_changed(one).expect("held");
        let mut held = holding(&["a", "c"]);
        held[1].card_name = None;
        m.held_changed(held).expect("held");
        let p = page(&m);
        assert_eq!(
            row(&p, "c").card_not_saved,
            Some(UiText::IdentityCardNotSaved)
        );
    }

    /// Cards §5.7.3 and `Pane::YourIdentities`: Your identities belongs
    /// to no identity, so a switch leaves it open and its check mark
    /// moves to the new identity in front. Mutation: a switch closing
    /// the list.
    #[test]
    fn the_list_stays_across_a_switch_and_its_check_mark_moves() {
        let mut m = model(&["a", "b"]);
        m.act(IdentityIntent::OpenIdentities, false);
        assert!(row(&page(&m), "a").in_front);
        assert!(!Pane::YourIdentities.closes_on_switch());

        switched(m.switch(&key("b"), false));
        let p = page(&m);
        assert!(!row(&p, "a").in_front);
        assert!(row(&p, "b").in_front);
        assert_eq!(row(&p, "b").description, Some(UiText::IdentityInFront));
    }

    fn opened_page(m: &Identities) -> IdentityPageView {
        m.identity_page().expect("an identity's page shown")
    }

    /// Cards §5.1 and identities §6.2 step 4: a row of Your identities
    /// opens that identity's page, the identity in front unchanged and
    /// nothing switched; a row chosen with the list closed opens nothing.
    /// Mutation: opening the row switches to it.
    #[test]
    fn a_row_opens_that_identitys_page_and_switches_nothing() {
        let mut m = model(&["a", "b"]);
        m.act(IdentityIntent::OpenIdentities, false);
        assert_eq!(
            m.act(IdentityIntent::OpenRow(key("b")), false),
            FrontOutcome::OpenPage(key("b"))
        );
        assert_eq!(m.front(), Some(&key("a")), "nothing switched");
        assert_eq!(m.previous(), None);
        assert_eq!(m.switch_snackbar(), None);
        assert_eq!(opened_page(&m).key, key("b"));

        m.act(IdentityIntent::CloseIdentities, false);
        assert_eq!(m.identity_page(), None, "the list's close closes it");
        assert_eq!(
            m.act(IdentityIntent::OpenRow(key("b")), false),
            FrontOutcome::Unchanged,
            "no list, no row to choose"
        );
        assert_eq!(m.identity_page(), None);
    }

    /// Identities §5's N10 and cards §5.7.2: the page names its identity
    /// as its subtitle and, when it is not the identity in front, says so
    /// with `IdentityNotInFront`, naming both; the page of the identity in
    /// front does not. Mutation: the line given on the front's page too.
    #[test]
    fn a_page_not_in_front_says_whose_it_is_and_the_fronts_page_does_not() {
        let mut m = model(&["a", "b"]);
        m.act(IdentityIntent::OpenIdentities, false);
        m.act(IdentityIntent::OpenRow(key("b")), false);
        let p = opened_page(&m);
        assert_eq!(p.subtitle, "Name b");
        let line = p.not_in_front.expect("not in front");
        assert_eq!(line.text, UiText::IdentityNotInFront);
        assert_eq!(
            line.english(),
            format!(
                "You are using {}. What you change on this page applies to {}.",
                name("a"),
                name("b")
            )
        );
        assert_eq!(p.back, UiText::BackToIdentities);

        m.act(IdentityIntent::ClosePage, false);
        m.act(IdentityIntent::OpenRow(key("a")), false);
        let p = opened_page(&m);
        assert_eq!(p.subtitle, "Name a");
        assert_eq!(p.not_in_front, None, "the front's own page");
    }

    /// Identities §6.2, "The alias" and "Its Copy": the page shows
    /// `IdentityAlias` with the alias its last characters, bare -- no
    /// isolate or punctuation after it -- and Copy, labelled `CopyAlias`,
    /// named `CopyAliasName`, putting the alias alone on the clipboard
    /// and confirmed by `AliasCopied`; an identity with no alias (a first
    /// identity authored without one) has no line. Mutation: the alias
    /// isolated as a name is.
    #[test]
    fn the_alias_line_ends_with_the_alias_and_is_absent_without_one() {
        let mut held = holding(&["a", "b"]);
        held[1].alias = "anna-rossi".to_owned();
        held[0].alias = String::new();
        let mut m = Identities::new();
        m.held_changed(held).expect("held");
        m.start(None);
        m.act(IdentityIntent::OpenIdentities, false);
        m.act(IdentityIntent::OpenRow(key("b")), false);
        let alias = opened_page(&m).alias.expect("an alias line");
        assert_eq!(alias.said.text, UiText::IdentityAlias);
        let line = alias.said.english();
        assert!(line.ends_with(": anna-rossi"), "{line:?}");
        assert_eq!(alias.alias, "anna-rossi", "the alias alone");
        assert_eq!(alias.copy, UiText::CopyAlias);
        assert_eq!(alias.copy_name, UiText::CopyAliasName);
        assert_eq!(alias.copied, UiText::AliasCopied);
        assert!(
            placeholder_en::text(alias.copy_name).starts_with(placeholder_en::text(alias.copy)),
            "the accessible name begins with the visible label (WCAG 2.5.3)"
        );

        m.act(IdentityIntent::ClosePage, false);
        m.act(IdentityIntent::OpenRow(key("a")), false);
        assert_eq!(opened_page(&m).alias, None, "no alias, no line");
    }

    /// Identities §6.2: Back from a row's page returns to Your
    /// identities, focus on that row; Back with no page open moves
    /// nothing. Mutation: focus sent to the list's title.
    #[test]
    fn back_from_a_rows_page_returns_to_the_list_on_that_row() {
        let mut m = model(&["a", "b", "c"]);
        m.act(IdentityIntent::OpenIdentities, false);
        m.act(IdentityIntent::OpenRow(key("c")), false);
        assert_eq!(
            m.act(IdentityIntent::ClosePage, false),
            FrontOutcome::Unchanged
        );
        assert_eq!(m.identity_page(), None);
        assert!(m.identities_open(), "the list shows again");
        assert_eq!(focus(&m), Some(IdentityFocus::Row));
        assert_eq!(m.focus_row(), Some(&key("c")));

        let before = m.focus_request();
        m.act(IdentityIntent::ClosePage, false);
        assert_eq!(m.focus_request(), before, "no page, nothing moves");
    }

    /// F9 and cards §5.2: the page of a row whose card Done did not write
    /// opens holding the form's values, as unsaved changes; another
    /// row's page holds none, and once a held list gives that identity a
    /// card's name its page holds none either. Mutation: the values not
    /// kept by `added`.
    #[test]
    fn a_page_opened_from_a_card_not_saved_row_holds_the_forms_values_unsaved() {
        let mut m = model(&["a", "b"]);
        let mut held = holding(&["a", "b", "c"]);
        held[2].card_name = None;
        m.held_changed(held.clone()).expect("held");
        let values = AddValues {
            name: "Anna".to_owned(),
            surname: "Rossi".to_owned(),
            photo: false,
        };
        m.added(&key("c"), Some(values.clone()));
        m.act(IdentityIntent::OpenRow(key("c")), false);
        assert_eq!(opened_page(&m).unsaved, Some(values));
        m.act(IdentityIntent::ClosePage, false);
        m.act(IdentityIntent::OpenRow(key("b")), false);
        assert_eq!(opened_page(&m).unsaved, None, "another identity's");

        m.act(IdentityIntent::ClosePage, false);
        held[2].card_name = Some("Anna Rossi".to_owned());
        m.held_changed(held).expect("held");
        m.act(IdentityIntent::OpenRow(key("c")), false);
        assert_eq!(opened_page(&m).unsaved, None, "its card saved since");
    }

    /// Identities §5.3 and `Pane::IdentityPage`: an identity's page is one
    /// identity's pane, so a switch closes it, as does a notification's
    /// switch; the list under it stays. A page whose identity is no longer
    /// held closes too. Mutation: a switch leaving the page open.
    #[test]
    fn a_switch_closes_an_identitys_page() {
        assert!(Pane::IdentityPage.closes_on_switch());
        let mut m = model(&["a", "b", "c"]);
        m.act(IdentityIntent::OpenIdentities, false);
        m.act(IdentityIntent::OpenRow(key("b")), false);
        switched(m.switch(&key("c"), false));
        assert_eq!(m.identity_page(), None);
        assert!(m.identities_open(), "the list stays");

        m.act(IdentityIntent::OpenRow(key("b")), false);
        let opened = m.open_notification(NotificationOpened::Identity(key("a")), false, 0);
        assert!(matches!(opened, FrontOutcome::Opening(_)), "{opened:?}");
        assert_eq!(m.identity_page(), None, "a notification's switch");

        m.act(IdentityIntent::OpenRow(key("b")), false);
        m.held_changed(holding(&["a", "c"])).expect("held");
        assert_eq!(m.identity_page(), None, "b no longer held");
    }

    /// Cards §5.1, "Store degraded": while the store cannot write, Add is
    /// drawn disabled with the degraded notice, `StorageDegraded`, as its
    /// reason, and choosing it opens nothing and moves no focus; once the
    /// store writes again Add opens the form. Mutation: the fact ignored
    /// by `act` and by the view.
    #[test]
    fn add_is_disabled_while_the_store_is_degraded() {
        let mut m = model(&["a", "b"]);
        m.set_adds(true);
        m.act(IdentityIntent::OpenIdentities, false);
        m.set_store_degraded(true);
        assert_eq!(
            page(&m).add.and_then(|a| a.disabled),
            Some(UiText::StorageDegraded)
        );
        let before = m.focus_request();
        assert_eq!(m.act(IdentityIntent::Add, false), FrontOutcome::Unchanged);
        assert_eq!(m.focus_request(), before);

        m.set_store_degraded(false);
        assert_eq!(page(&m).add.and_then(|a| a.disabled), None);
        assert_eq!(m.act(IdentityIntent::Add, false), FrontOutcome::OpenAdd);
    }
}
