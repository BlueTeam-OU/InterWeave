// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Onboarding's Connect, drawn (`design/2026-10-22-add-a-peer.md` §5.1):
//! from `ui-model`'s [`Connect`], over the panes until the person leaves
//! it. The add page and Show it opens are Peers' own, drawn in Peers'
//! place while they are up, and Connect comes back when they close.

use std::cell::RefCell;
use std::rc::Rc;

use interweave_human_ui_model::{
    Connect, ConnectFocus, ConnectInput, ConnectLine, UiModel, UiText, placeholder_en,
};
use slint::SharedString;

use crate::generated::AppWindow;
use crate::{Input, Page, Shared, View, enqueue};

/// Connect's presses: each becomes an input, taken as the others are.
pub(crate) fn wire(window: &AppWindow, shared: &Rc<RefCell<Shared>>) {
    let set = |t| SharedString::from(placeholder_en::text(t));
    window.set_connect_title(set(UiText::ConnectTitle));
    window.set_connect_body(set(UiText::ConnectBody));
    window.set_continue_label(set(UiText::IdContinue));
    let on = |s: &Rc<RefCell<Shared>>, input: ConnectInput| {
        let s = Rc::clone(s);
        move || enqueue(&s, Input::Connect(input))
    };
    window.on_connect_add(on(shared, ConnectInput::Add));
    window.on_connect_show(on(shared, ConnectInput::Show));
    window.on_connect_trust(on(shared, ConnectInput::Trust));
    window.on_connect_leave(on(shared, ConnectInput::Leave));
}

/// What of Connect the window last drew.
#[derive(Default)]
pub(crate) struct ConnectDrawn {
    /// The focus request last carried out, by the model's count.
    focus_seen: u64,
    /// The model's count of returns whose line is said, as last said.
    said_seen: u64,
}

const fn line_text(line: ConnectLine) -> UiText {
    match line {
        ConnectLine::NeedsTrust => UiText::ConnectDoneNeedsTrust,
        ConnectLine::TrustUnknown => UiText::ConnectDoneTrustUnknown,
        ConnectLine::Trusted => UiText::ConnectDoneTrusted,
        ConnectLine::Shown => UiText::ConnectDoneShown,
    }
}

/// The window's `focus-key` for each of Connect's controls.
pub(crate) const fn focus_key(focus: ConnectFocus) -> &'static str {
    match focus {
        ConnectFocus::Title => "connect-title",
        ConnectFocus::Add => "connect-add",
        ConnectFocus::Show => "connect-show",
        ConnectFocus::Trust => "connect-trust",
        ConnectFocus::Continue => "connect-continue",
    }
}

/// Where focus goes once Connect is left: Chats' title (§8).
const CHATS_TITLE: &str = "heading";

/// Whether a page Connect opened is up: the add page or Show.
pub(crate) fn page_up(model: &UiModel) -> bool {
    model.peers().add_open() || model.peers().show().is_some()
}

impl View {
    /// Connect, or nothing once it is left: its page set before Peers is
    /// drawn, since Peers draws the pages Connect opened. On a return
    /// from a page that did something §5.1 counts ([`Connect::said`]),
    /// its line is returned for the render's one announcement -- the same
    /// words again included: focus goes back to the control that opened
    /// the page, so the line would otherwise go unheard. A return that
    /// did nothing counted says nothing, an earlier line still showing
    /// included (`design/2026-10-22-add-a-peer.md` §5.1).
    pub(crate) fn render_connect(&mut self, model: &UiModel) -> Option<String> {
        let Some(connect) = model.connect() else {
            if self.page == Page::Connect {
                self.page = Page::Conversation;
                self.window.set_connect_open(false);
                self.window.set_list_shown(true);
                self.focus_key_after_draw(CHATS_TITLE.to_owned());
            }
            return None;
        };
        if self.page != Page::Connect {
            self.page = Page::Connect;
            self.peer_lists.connect = ConnectDrawn {
                said_seen: connect.said(),
                ..ConnectDrawn::default()
            };
            // The pages Connect opens are drawn in the pane, which a
            // narrow window shows only off the list; Connect covers both.
            self.window.set_list_shown(false);
        }
        self.window.set_connect_open(true);
        let line = connect.line();
        self.window.set_connect_line(
            line.map_or("", |l| placeholder_en::text(line_text(l)))
                .into(),
        );
        self.window
            .set_connect_line_trust(line.is_some_and(ConnectLine::offers_trust));
        // Nothing of Connect's is on screen while a page it opened is:
        // its requests and its line wait for that page to close.
        if page_up(model) {
            return None;
        }
        self.connect_focus(connect);
        let said = std::mem::replace(&mut self.peer_lists.connect.said_seen, connect.said());
        line.filter(|_| said != connect.said())
            .map(|l| placeholder_en::text(line_text(l)).to_owned())
    }

    fn connect_focus(&mut self, connect: &Connect) {
        let (focus, focuses) = connect.focus();
        if focuses == self.peer_lists.connect.focus_seen {
            return;
        }
        self.peer_lists.connect.focus_seen = focuses;
        if let Some(focus) = focus {
            // Most land on a control this same render draws again.
            self.focus_key_after_draw(focus_key(focus).to_owned());
        }
    }
}
