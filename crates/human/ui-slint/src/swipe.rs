// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The view's half of a row's swipe (`design/2026-10-16-message-swipe.md`):
//! the rows a swipe can move as rendered, the pointer callbacks that
//! drive [`Gesture`], and the texts a swipe and a snackbar show.
//!
//! A committed swipe is recorded as the same input a row button makes --
//! the item and the action as rendered -- so the model judges both alike,
//! at take time (§8's re-check at commit).

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Instant;

use interweave_human_ui_model::{
    Confirmation, ConversationKey, ItemKey, RowAction, SwipeOffer, UiText, placeholder_en,
};
use slint::ComponentHandle as _;

use crate::generated::{AppWindow, SwipeKind};
use crate::gesture::{Gesture, Side};
use crate::{By, Input, Shared, enqueue};

/// What a rendered row's swipe does, by the row's swipe key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Swipeable {
    /// A message row and its offer.
    Item(ItemKey, SwipeOffer),
    /// A conversation row that may be hidden.
    Conversation(ConversationKey),
}

impl Swipeable {
    /// The action a swipe toward `side` of the screen does, if any.
    fn toward(&self, side: Side, rtl: bool) -> Option<RowAction> {
        // Toward the end of a line is right, left to right (§3).
        let toward_end = (side == Side::Right) != rtl;
        match self {
            Self::Item(_, offer) => {
                if toward_end {
                    offer.toward_end
                } else {
                    offer.toward_start
                }
            }
            Self::Conversation(_) => (!toward_end).then_some(RowAction::Hide),
        }
    }
}

/// The swipe in progress: its row's key and the gesture.
pub(crate) struct Tracked {
    key: String,
    gesture: Gesture,
    width: f32,
}

/// A committed swipe whose action takes its row away: the row is held
/// slid out until the render that removes it, or springs back if the row
/// is still there once the model has answered (a refused Cancel, §8).
#[derive(Debug, Clone)]
pub(crate) struct Committed {
    pub(crate) key: String,
    /// The model's outcome id when it was committed: a newer one is the
    /// answer.
    pub(crate) outcome: Option<u64>,
    pub(crate) at: Instant,
}

/// Milliseconds on a monotonic clock, for a gesture's samples.
pub(crate) fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    let start = *START.get_or_init(Instant::now);
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// The window's lengths are logical pixels, which a gesture reads as is.
#[allow(clippy::too_many_arguments)]
pub(crate) fn moved(
    shared: &Rc<RefCell<Shared>>,
    window: &slint::Weak<AppWindow>,
    key: &str,
    pressed: (f32, f32),
    current: (f32, f32),
    width: f32,
    window_width: f32,
) {
    let now = now_ms();
    let step = {
        let mut s = shared.borrow_mut();
        let Some(target) = s.swipes.get(key).cloned() else {
            return;
        };
        let rtl = s.touch.rtl;
        let fresh = s.tracked.as_ref().is_none_or(|t| t.key != key);
        if fresh {
            let left = target.toward(Side::Left, rtl).is_some();
            let right = target.toward(Side::Right, rtl).is_some();
            let gesture = Gesture::press(&s.touch, window_width, pressed, width, left, right);
            s.tracked = Some(Tracked {
                key: key.to_owned(),
                gesture,
                width,
            });
        }
        let Some(tracked) = s.tracked.as_mut() else {
            return;
        };
        let step = tracked.gesture.moved(current, now);
        let tracking = tracked.gesture.tracking();
        (step, tracking, s.haptic.clone())
    };
    let (step, tracking, haptic) = step;
    let Some(window) = window.upgrade() else {
        return;
    };
    if step.started {
        // A new swipe takes over from a row still held slid out.
        shared.borrow_mut().committed = None;
        window.set_swipe_key(key.into());
        window.set_swipe_tracking(true);
    }
    if tracking {
        window.set_swipe_offset(step.offset);
        window.set_swipe_armed(step.armed);
    }
    if step.tick
        && let Some(haptic) = haptic
    {
        haptic();
    }
}

/// The finger lifted on `key`'s row: commit or spring back.
pub(crate) fn ended(shared: &Rc<RefCell<Shared>>, window: &slint::Weak<AppWindow>, key: &str) {
    let now = now_ms();
    let (tracked, target, rtl) = {
        let mut s = shared.borrow_mut();
        let Some(tracked) = s.tracked.take_if(|t| t.key == key) else {
            return;
        };
        let target = s.swipes.get(key).cloned();
        (tracked, target, s.touch.rtl)
    };
    let Some(window) = window.upgrade() else {
        return;
    };
    if !tracked.gesture.tracking() {
        return;
    }
    window.set_swipe_tracking(false);
    window.set_swipe_armed(false);
    let side = tracked.gesture.release(now);
    let action = side.and_then(|side| target.as_ref()?.toward(side, rtl).map(|a| (side, a)));
    let Some(((side, action), target)) = action.zip(target) else {
        window.set_swipe_offset(0.0);
        return;
    };
    // Hide and Cancel take the row away: it carries on out and is held
    // there until the render that removes it. Everything else slides back
    // to rest as its result arrives (§5).
    if matches!(action, RowAction::Hide | RowAction::Cancel) {
        let out = if side == Side::Right {
            tracked.width
        } else {
            -tracked.width
        };
        window.set_swipe_offset(out);
        shared.borrow_mut().committed = Some(Committed {
            key: key.to_owned(),
            outcome: None,
            at: Instant::now(),
        });
        // Its bound passes with nothing else to turn the root: wake it, so
        // a render springs the row back if the model never took it away.
        let s = Rc::clone(shared);
        slint::Timer::single_shot(crate::COMMIT_HOLD + crate::LEAVING, move || {
            let wake = s.borrow().wake.clone();
            if let Some(wake) = wake {
                wake();
            }
        });
    } else {
        window.set_swipe_offset(0.0);
    }
    let input = match target {
        Swipeable::Item(item, _) => Input::Action(item, action, By::Swipe),
        Swipeable::Conversation(conversation) => Input::HideConversation(conversation),
    };
    enqueue(shared, input);
}

/// The touch on `key`'s row was taken away -- the list scrolled it, the
/// system cancelled it: the row springs back and nothing happens.
pub(crate) fn cancelled(shared: &Rc<RefCell<Shared>>, window: &slint::Weak<AppWindow>, key: &str) {
    let was = shared.borrow_mut().tracked.take_if(|t| t.key == key);
    if let (Some(tracked), Some(window)) = (was, window.upgrade())
        && tracked.gesture.tracking()
    {
        window.set_swipe_tracking(false);
        window.set_swipe_armed(false);
        window.set_swipe_offset(0.0);
    }
}

/// The kind a direction's action is drawn as.
pub(crate) const fn kind(action: Option<RowAction>) -> SwipeKind {
    match action {
        None => SwipeKind::None,
        Some(RowAction::Keep) => SwipeKind::Keep,
        Some(RowAction::Hide) => SwipeKind::Hide,
        Some(RowAction::Unkeep) => SwipeKind::Unkeep,
        Some(RowAction::Retry) => SwipeKind::Retry,
        Some(RowAction::Cancel) => SwipeKind::Cancel,
    }
}

/// A row action's text: its button's, and the label beside its icon.
pub(crate) const fn action_text(action: RowAction) -> &'static str {
    placeholder_en::text(match action {
        RowAction::Keep => UiText::Keep,
        RowAction::Hide => UiText::Dismiss,
        RowAction::Unkeep => UiText::Unkeep,
        RowAction::Retry => UiText::Retry,
        RowAction::Cancel => UiText::Cancel,
    })
}

/// What a snackbar says, and its announcement: what just happened, a
/// cancel saying whether an attempt may have reached the peer (§8).
pub(crate) fn confirmation_text(confirms: Confirmation) -> &'static str {
    match confirms {
        Confirmation::Kept => placeholder_en::text(UiText::ConfirmKept),
        Confirmation::Hidden => placeholder_en::text(UiText::ConfirmDismissed),
        Confirmation::Unkept => placeholder_en::text(UiText::ConfirmUnkept),
        Confirmation::ConversationHidden => placeholder_en::text(UiText::ConfirmConversationHidden),
        Confirmation::Cancelled {
            may_have_reached: false,
        } => placeholder_en::text(UiText::ConfirmSendingCanceled),
        Confirmation::Cancelled {
            may_have_reached: true,
        } => placeholder_en::text(UiText::ConfirmSendingCanceledMayHaveReached),
        Confirmation::IdentifierCopied { channel: false } => {
            placeholder_en::text(UiText::PeerIdCopied)
        }
        Confirmation::IdentifierCopied { channel: true } => {
            placeholder_en::text(UiText::ChannelIdCopied)
        }
    }
}

/// Keep `window` reachable from a callback without keeping it alive.
pub(crate) fn weak(window: &AppWindow) -> slint::Weak<AppWindow> {
    window.as_weak()
}
