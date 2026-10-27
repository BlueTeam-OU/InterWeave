// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Onboarding's third step, Connect (`design/2026-10-22-add-a-peer.md`
//! §5.1): shown once after a new or restored identity, while this device
//! knows no address and trusts no peer. Pure: the platform says whether
//! Connect is owed, the root asks [`crate::UiModel::connect_turn`] what to
//! do about it, and the add and Show pages it opens are Peers' own.
//!
//! What the person did here is held for this visit only: Connect opened
//! again after a restart starts with no line (§5.1).

use interweave_transport_api::TransportIdentity;

use crate::peers::ConnectAdd;
use crate::trust::TrustSettings;

/// What a start does about an owed Connect, said once
/// ([`crate::UiModel::connect_turn`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectGate {
    /// Both reads succeeded, and the device knows no address and trusts
    /// no peer: Connect opens over Chats.
    Open,
    /// Connect is no longer owed: either list had an entry, or the person
    /// left Connect. The platform forgets that it was owed.
    Settled,
    /// A read failed: Chats with no Connect this start, still owed, so
    /// the next start that reads both decides again (§5.1).
    Deferred,
}

/// Connect's line: what is still to do wins (§5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectLine {
    /// An added peer is known not to be trusted: `ConnectDoneNeedsTrust`,
    /// with Trust.
    NeedsTrust,
    /// An added peer's trust is not known: `ConnectDoneTrustUnknown`, with
    /// Trust.
    TrustUnknown,
    /// Every added peer is trusted: `ConnectDoneTrusted`.
    Trusted,
    /// Show drew this device's addresses and nothing was added:
    /// `ConnectDoneShown`.
    Shown,
}

impl ConnectLine {
    /// Whether the line offers Trust beside it.
    #[must_use]
    pub const fn offers_trust(self) -> bool {
        matches!(self, Self::NeedsTrust | Self::TrustUnknown)
    }
}

/// Connect's controls: where focus returns to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectFocus {
    /// The heading, as Connect opens (§8).
    Title,
    /// Add an address.
    Add,
    /// Show my address.
    Show,
    /// Trust, beside the line.
    Trust,
    /// Continue, the primary once something was done.
    Continue,
}

/// What the person did on Connect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectInput {
    /// Add an address: the add page, opened from Connect.
    Add,
    /// Show my address.
    Show,
    /// Trust, beside the line: step two for the peers still to trust.
    Trust,
    /// Not now, Continue, or Back: Chats.
    Leave,
}

/// Connect while it is open.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Connect {
    /// Each peer this visit's adds added, its trust as last decided.
    added: Vec<ConnectAdd>,
    /// Show drew this device's addresses during this visit.
    shown: bool,
    /// The control that opened the add page or Show, for focus on return,
    /// and the line as it read then: one that reads otherwise on the
    /// return was changed under the page, and is said then.
    opener: Option<(ConnectFocus, Option<ConnectLine>)>,
    focus: Option<ConnectFocus>,
    focuses: u64,
    /// Bumped at each return whose page did something §5.1 counts, its
    /// words unchanged included, or under which the line changed; and at
    /// each late read that changes the line with no page up.
    said: u64,
}

impl Connect {
    pub(crate) fn opened() -> Self {
        let mut connect = Self::default();
        connect.ask_focus(ConnectFocus::Title);
        connect
    }

    /// The line, if anything was done (§5.1's precedence).
    #[must_use]
    pub fn line(&self) -> Option<ConnectLine> {
        if self.added.iter().any(|a| a.trusted == Some(false)) {
            Some(ConnectLine::NeedsTrust)
        } else if self.added.iter().any(|a| a.trusted.is_none()) {
            Some(ConnectLine::TrustUnknown)
        } else if !self.added.is_empty() {
            Some(ConnectLine::Trusted)
        } else if self.shown {
            Some(ConnectLine::Shown)
        } else {
            None
        }
    }

    /// The peers Trust opens step two for: each added peer not known to
    /// be trusted, in the order they were added.
    #[must_use]
    pub fn to_trust(&self) -> Vec<TransportIdentity> {
        self.added
            .iter()
            .filter(|a| a.trusted != Some(true))
            .map(|a| a.peer.clone())
            .collect()
    }

    /// Where focus is asked to go, and a count that changes with each ask.
    #[must_use]
    pub const fn focus(&self) -> (Option<ConnectFocus>, u64) {
        (self.focus, self.focuses)
    }

    pub(crate) fn opening(&mut self, from: ConnectFocus) {
        self.opener = Some((from, self.line()));
    }

    /// A count bumped each time the line is to be said, once per bump,
    /// after focus lands (§5.1, §8): at a return whose page did something
    /// §5.1 counts -- an add counted, a Show that drew this device's
    /// addresses, or a trust decided in step two -- its words unchanged
    /// included; at a return whose line reads otherwise than when its
    /// page opened, a late read having changed it meanwhile; and at a
    /// late read that changes the line's words with no page up. A return
    /// that did nothing counted with the line as it was -- a refusal, a
    /// failure, an unknown add, Back from an add with nothing added, a
    /// Show that drew nothing -- leaves it unchanged, an earlier line
    /// still showing included, and so does a read that leaves the words
    /// as they were.
    #[must_use]
    pub const fn said(&self) -> u64 {
        self.said
    }

    /// A page Connect opened has closed: what its adds came to, `None`
    /// when it did nothing §5.1 counts, whether Show drew this device's
    /// addresses, and focus back on the control that opened it --
    /// Continue in Trust's place once the line no longer offers Trust.
    /// The adds' trust is read from `trust` at once, so a return's line is
    /// already what the list confirms and no later turn says it again.
    pub(crate) fn returned(
        &mut self,
        adds: Option<Vec<ConnectAdd>>,
        shown: bool,
        trust: &TrustSettings,
    ) {
        let counted = adds.is_some() || shown;
        let page = self.opener.take();
        // A sync with no page closing and nothing to hand over: a late
        // read is `trust_read`'s, at the turn that draws it.
        if !counted && page.is_none() {
            return;
        }
        for add in adds.into_iter().flatten() {
            self.added.retain(|a| a.peer != add.peer);
            self.added.push(add);
        }
        self.shown |= shown;
        self.confirm(trust);
        // Said for what the page did, or for a line changed under it.
        if counted || page.is_some_and(|(_, then)| then != self.line()) {
            self.said += 1;
        }
        let Some((opener, _)) = page else {
            return;
        };
        let offers_trust = self.line().is_some_and(ConnectLine::offers_trust);
        self.ask_focus(match opener {
            ConnectFocus::Trust if !offers_trust => ConnectFocus::Continue,
            other => other,
        });
    }

    /// Each added peer's trust as the list now confirms it, so the line
    /// and its Trust follow an answer that came after the page closed --
    /// a Trust whose answer was lost, read back later, or answered as
    /// made with no list read back. A list is
    /// confirmed only once no change of it is lost
    /// ([`TrustSettings::confirmed_list`]), so a Trust still lost leaves
    /// its peer not known here (§5.3). A read that changes the line's
    /// words with no page up is said once ([`Self::said`]); under a page,
    /// the return says it.
    pub(crate) fn trust_read(&mut self, trust: &TrustSettings) {
        let before = self.line();
        self.confirm(trust);
        if self.opener.is_none() && self.line() != before {
            self.said += 1;
        }
    }

    fn confirm(&mut self, trust: &TrustSettings) {
        for add in &mut self.added {
            match trust.confirmed_list() {
                Some(list) => add.trusted = Some(list.allows(&add.peer)),
                // A late answer saying the Trust was made, its list not
                // read back: known and trusted, as Peers' row reads it
                // (§5.1, §5.3).
                None if trust.trusted_here(&add.peer) => add.trusted = Some(true),
                None => {}
            }
        }
    }

    fn ask_focus(&mut self, focus: ConnectFocus) {
        self.focus = Some(focus);
        self.focuses += 1;
    }
}

/// Whether Connect is owed, open, or done with for this start.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum ConnectState {
    /// Not owed: no new identity, or already decided on an earlier start.
    #[default]
    NotOwed,
    /// Owed, and not yet decided this start.
    Owed,
    /// Open over Chats.
    Open(Connect),
    /// Left this start: its settling still to be said.
    Left,
    /// Decided for this start; nothing more is said.
    Decided,
}
