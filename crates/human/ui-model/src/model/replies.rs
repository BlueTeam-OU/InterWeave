// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A reply whose target is not held (`design/2026-10-27-start-window.md`
//! §5.2): its line says whether the person can reach the target, so the
//! model asks the store whether the conversation stores it -- once when a
//! reply to it is built with the target not held, and once each time the
//! target leaves (released by the ceiling, dropped by the session bound,
//! dismissed once its snackbar closed) -- and never the transport.
//!
//! One record per target some held reply names, gone with the last such
//! reply, so the records are bounded by the items the model holds.

use std::collections::{HashMap, HashSet};

use super::{ConversationKey, Reply, UiModel};

/// What the model asks the store: whether `conversation` stores a row of
/// `target` the app can show, unread or kept.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReplyAsk {
    /// The conversation the reply is in.
    pub conversation: ConversationKey,
    /// The application id it replies to.
    pub target: String,
}

type Pair = (ConversationKey, String);

/// A target some held reply names.
#[derive(Debug, Default)]
struct Target {
    /// The held items replying to it.
    replies: usize,
    /// Whether the line reads present while no holder is held: the store's
    /// last answer, or until it comes, what the line said before the
    /// target left, so a release on screen changes no line (§10 item 17
    /// (b)).
    stored: bool,
    /// An ask is with the store.
    asking: bool,
    /// The target moved while the ask was out: its answer may predate the
    /// move, so it is asked again rather than shown.
    stale: bool,
}

/// The records, and the targets to look at at the next take.
#[derive(Debug, Default)]
pub(super) struct Replies {
    targets: HashMap<Pair, Target>,
    due: HashSet<Pair>,
}

impl Replies {
    pub(super) fn stored(&self, conversation: &ConversationKey, target: &str) -> bool {
        self.targets
            .get(&(conversation.clone(), target.to_owned()))
            .is_some_and(|t| t.stored)
    }
}

impl UiModel {
    /// A held item replying to `target` was built: the first reply to a
    /// target is looked at at the next take.
    pub(super) fn reply_built(&mut self, conversation: &ConversationKey, target: Option<&str>) {
        let Some(target) = target else {
            return;
        };
        let pair = (conversation.clone(), target.to_owned());
        let record = self.replies.targets.entry(pair.clone()).or_default();
        record.replies += 1;
        if record.replies == 1 {
            self.replies.due.insert(pair);
        }
    }

    /// A held item replying to `target` went: the record goes with the
    /// last one.
    pub(super) fn reply_dropped(&mut self, conversation: &ConversationKey, target: Option<&str>) {
        let Some(target) = target else {
            return;
        };
        let pair = (conversation.clone(), target.to_owned());
        if let Some(record) = self.replies.targets.get_mut(&pair) {
            record.replies -= 1;
            if record.replies == 0 {
                self.replies.targets.remove(&pair);
                self.replies.due.remove(&pair);
            }
        }
    }

    /// An item holding application id `id` is about to go: a reply to it
    /// keeps what its line says now until the store answers.
    pub(super) fn target_leaving(&mut self, conversation: &ConversationKey, id: &str) {
        let pair = (conversation.clone(), id.to_owned());
        if !self.replies.targets.contains_key(&pair) {
            return;
        }
        let shown = matches!(
            self.reply(conversation, Some(id)),
            Some(Reply::Present(_) | Reply::Stored(_))
        );
        if let Some(record) = self.replies.targets.get_mut(&pair) {
            record.stored = shown;
            if record.asking {
                record.stale = true;
            }
        }
        self.replies.due.insert(pair);
    }

    /// What the store is to be asked about replies' targets since the last
    /// take: each target a held reply names that no item holds -- shown or
    /// hidden, since a hidden one reads unavailable whatever the store
    /// holds (§5.2) -- and that has no ask out.
    pub fn take_reply_asks(&mut self) -> Vec<ReplyAsk> {
        let due: Vec<Pair> = self.replies.due.drain().collect();
        let mut asks = Vec::new();
        for pair in due {
            if self.by_app.get(&pair).is_some_and(|keys| !keys.is_empty()) {
                continue;
            }
            let Some(record) = self.replies.targets.get_mut(&pair) else {
                continue;
            };
            if record.asking {
                record.stale = true;
                continue;
            }
            record.asking = true;
            asks.push(ReplyAsk {
                conversation: pair.0,
                target: pair.1,
            });
        }
        asks
    }

    /// The store's answer to `ask`: the line changes in place, nothing is
    /// announced. An answer the target moved under is asked again.
    pub fn reply_target_answered(&mut self, ask: &ReplyAsk, stored: bool) {
        let pair = (ask.conversation.clone(), ask.target.clone());
        let Some(record) = self.replies.targets.get_mut(&pair) else {
            return;
        };
        record.asking = false;
        if std::mem::take(&mut record.stale) {
            self.replies.due.insert(pair);
            return;
        }
        record.stored = stored;
    }

    /// The store could not answer `ask`: the line keeps what it says, and
    /// the next move of the target asks again.
    pub fn reply_target_failed(&mut self, ask: &ReplyAsk) {
        let pair = (ask.conversation.clone(), ask.target.clone());
        if let Some(record) = self.replies.targets.get_mut(&pair) {
            record.asking = false;
            if std::mem::take(&mut record.stale) {
                self.replies.due.insert(pair);
            }
        }
    }

    /// How many targets the model keeps a record of: bounded by the held
    /// replies.
    #[must_use]
    pub fn reply_targets(&self) -> usize {
        self.replies.targets.len()
    }
}
