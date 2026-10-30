// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The human client's UI model (plan §17 (6)): presentation state only,
//! pure and synchronous, between the transport facade and the views. Its
//! surface was agreed with the client's role before it was built (relay
//! seqs 10630, 10639, 10642; amended 10707, 10710, 10713): the
//! composition root feeds it what the
//! facade and the store did, a view renders what it returns, and a
//! person's actions come back as [`Intent`]s -- a view never reaches the
//! facade, the store, or IPC.
//!
//! It names the client's vocabulary (`interweave-human-client-api`) and
//! nothing that reaches a store, a transport or a toolkit (plan §17 P2).

#![forbid(unsafe_code)]

pub mod body;
mod connect;
mod identity_flow;
mod labels;
mod look;
mod model;
mod order;
mod peers;
mod picker;
mod presence;
pub mod preview;
mod store_recovery;
mod trust;
mod waiting;

pub use connect::{Connect, ConnectFocus, ConnectGate, ConnectInput, ConnectLine};
pub use identity_flow::{
    Effect, IdentityFlow, Notice, RecoveryReason, ScreenId, Start, StepFailed, Words,
};
/// The store row an intent names, as the model hands it to a view.
pub use interweave_human_core::RowId;
pub use labels::{
    ErrorClass, HostText, LabelKey, RuntimeHost, UiText, attention_class, by_count, fill, isolated,
    outbound_label, placeholder_en, send_error_class, send_problem_class, session_problem_class,
    short_peer, visible_destination,
};
pub use look::{Avatar, AvatarHue, Day, LocalTime, Weekday, local_time};
pub use model::cards::{
    AcceptLine, CardAnswer, CardOutcome, CardSwipe, ContactRef, ContactView, IdentityView,
    KeyProblem, KeyStep, KeyView, NameProblem, OwnKey, OwnPhotoStep, Photo, PhotoBasis,
    PhotoChange, PhotoEdit, Proposal, ProposalContext, ProposalItem, ProposalNote,
    READ_PROPOSAL_CAP, RouteView, SentCard, ValueChanges, name_problem,
};
pub use model::directory::{ContactsState, DirectoryContact, PHOTO_CAP, PhotoAsk, PhotoState};
pub use model::summaries::{NewestRow, StoredSummary};
pub use model::window::{
    Anchor, Edge, HELD_ROW_CAP, MarkKind, MarkPlace, PageAsk, PageEnd, PageMark, PageOrder,
    PageRows, WindowEvent, WindowInput, WindowMark,
};
pub use model::{
    Announce, Composer, Confirmation, ContactChange, ConversationKey, ConversationSummary,
    DEDUP_CAP, Direction, FocusTarget, HELD_UPDATE_CAP, Intent, ItemDiagnostics, ItemKey,
    ItemStatus, LastMessage, ListedInbound, ListedOutbound, MessageItem, Outcome, Released, Reply,
    Retention, RouteLabel, RowAction, SESSION_ITEM_CAP, SessionNotice, Snackbar, SwipeOffer, Table,
    Trust, UiModel, UndoClosed, ViewEvent,
};
pub use order::{compare_names, contact_order};
pub use peers::{
    ADDRESS_BOUND, AddNote, AddResult, Barred, BlockHeld, BlockProblem, BlockState, BlockView,
    CODE_LINES, ConnectAdd, LineSelection, LineSource, ListProblem, PeerRow, Peers, PeersFocus,
    PeersInput, PeersSnack, Refusal, RowReadBack, RowState, ScanControl, ScanVerdict, ShowProblem,
    ShowState, TYPED_LINES, TrustLine, is_address_list, lines_of,
};
pub use picker::{MAX_CHIPS, PHRASE_WORDS, PhraseEntry, Suggestions, WordPicker};
pub use presence::{Admit, PresenceGate, Prompted, WINDOW_MS};
pub use store_recovery::{
    DiscardConfirmation, DiscardKind, Effect as StoreEffect, Phase as StorePhase, StoreRecovery,
    StoreScreen,
};
pub use trust::{EntryProblem, OutcomeFrom, TrustChange, TrustInput, TrustOutcome, TrustSettings};
pub use waiting::{age_text, announced_change, next_age_change, waiting_text};
