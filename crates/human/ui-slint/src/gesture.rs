// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A row's swipe, decided here rather than by the toolkit
//! (`design/2026-10-16-message-swipe.md` §4).
//!
//! Slint's `SwipeGestureHandler` is the row's container because it is the
//! one element that takes a drag away from the controls inside it, but its
//! own rule starts a swipe when dx > |dy| / 2, so a drag 45-63 degrees off
//! horizontal would swipe. The rule here is the spec's: the row tracks
//! only once the horizontal movement passes the system's touch slop AND is
//! larger than the vertical movement at that moment; a touch whose
//! vertical movement passes the slop first is a scroll for the rest of its
//! life. The handler reports positions; this decides what they mean.
//!
//! Lengths are logical pixels, which Slint maps 1:1 to dp on Android, and
//! times milliseconds on a monotonic clock the caller supplies, so a test
//! replays a touch exactly.

/// The platform's values the gesture reads (§4, §7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchSettings {
    /// The system's touch slop, in logical pixels: Android's
    /// `ViewConfiguration.getScaledTouchSlop`, converted.
    pub slop: f32,
    /// The system's gesture insets at the window's left and right edges
    /// (`WindowInsets.systemGestures`): a touch that starts inside one is
    /// the system's back gesture, never a row's.
    pub edge_left: f32,
    /// The right edge's.
    pub edge_right: f32,
    /// A right-to-left layout: the directions mirror (§3).
    pub rtl: bool,
    /// The system's animator duration scale is 0: nothing animates, the
    /// row still follows the finger (§7).
    pub reduced_motion: bool,
    /// How long a snackbar stays up, in milliseconds: the platform's
    /// recommended timeout for content with controls, never under 6 s,
    /// never over 2 minutes (§6).
    pub snackbar_ms: u64,
    /// The platform's 12- or 24-hour setting (Android's
    /// `DateFormat.is24HourFormat`), which every shown time follows
    /// (design/2026-10-16-android-look.md section 5.2).
    pub hour24: bool,
    /// The platform's long-press time in milliseconds (Android's
    /// `ViewConfiguration.getLongPressTimeout`): a press held this long
    /// within the slop opens a message's menu (swipe spec section 4).
    pub long_press_ms: u64,
}

impl Default for TouchSettings {
    /// A desktop's: no gesture insets, left to right, motion on, 24-hour
    /// times.
    fn default() -> Self {
        Self {
            slop: 8.0,
            edge_left: 0.0,
            edge_right: 0.0,
            rtl: false,
            reduced_motion: false,
            snackbar_ms: SNACKBAR_MIN_MS,
            hour24: true,
            long_press_ms: 500,
        }
    }
}

/// A snackbar's shortest time up (§6).
pub const SNACKBAR_MIN_MS: u64 = 6_000;
/// Its longest, whatever the platform recommends: the longest the
/// platform's own setting offers (§6).
pub const SNACKBAR_MAX_MS: u64 = 120_000;

/// The snackbar's time up for a platform recommendation of `recommended`
/// milliseconds: at least 6 s, at most 2 minutes.
#[must_use]
pub fn snackbar_ms(recommended: u64) -> u64 {
    recommended.clamp(SNACKBAR_MIN_MS, SNACKBAR_MAX_MS)
}

/// The share of the row's width a drag must cover to arm (§4).
pub(crate) const ARM_FRACTION: f32 = 0.4;
/// The least distance that arms, whatever the row's width.
pub(crate) const ARM_MIN: f32 = 96.0;
/// A fling must have moved the row at least this far from touch-down.
pub(crate) const FLING_MIN: f32 = 48.0;
/// A fling is released within this long of the first move the row sees.
/// A time window rather than a speed: on Slint 1.18.1 a press inside a
/// scrolling list reaches the row about 100 ms late, so a fling's early
/// moves are never seen and a speed over them cannot be measured (§4,
/// measured on a Pixel 9a).
pub(crate) const FLING_WINDOW_MS: u64 = 100;

/// A side of the screen the row moved toward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    /// Toward the left edge.
    Left,
    /// Toward the right edge.
    Right,
}

/// What a touch is, once it has said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Within the slop either way.
    Undecided,
    /// The row follows the finger.
    Tracking,
    /// Vertical first, or a direction the row does not offer: never a
    /// swipe for the rest of this touch.
    Ignored,
}

/// What one movement did to the row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Step {
    /// Where the row is drawn, from its rest position: 0 unless tracking.
    pub(crate) offset: f32,
    /// The touch began tracking with this movement: the list stops
    /// scrolling for it.
    pub(crate) started: bool,
    /// The row is past the threshold.
    pub(crate) armed: bool,
    /// It crossed the threshold outward with this movement: the one haptic
    /// tick (§4). Never on the way back.
    pub(crate) tick: bool,
}

/// One touch on one row.
#[derive(Debug, Clone)]
pub(crate) struct Gesture {
    start: (f32, f32),
    width: f32,
    left: bool,
    right: bool,
    slop: f32,
    phase: Phase,
    offset: f32,
    armed: bool,
    /// When the row saw its first move: a fling's window opens there.
    first_move: Option<u64>,
}

impl Gesture {
    /// A press at `at` (window coordinates), on a row `width` wide that
    /// may move `left` and `right` on screen. A press inside a gesture
    /// inset is the system's: the row never tracks it. `at` is where the
    /// finger touched down, as Slint reports it, which a fling's distance
    /// counts from.
    #[must_use]
    pub(crate) fn press(
        settings: &TouchSettings,
        window_width: f32,
        at: (f32, f32),
        width: f32,
        left: bool,
        right: bool,
    ) -> Self {
        let in_inset = at.0 < settings.edge_left || at.0 > window_width - settings.edge_right;
        Self {
            start: at,
            width,
            left,
            right,
            slop: settings.slop,
            phase: if in_inset || !(left || right) {
                Phase::Ignored
            } else {
                Phase::Undecided
            },
            offset: 0.0,
            armed: false,
            first_move: None,
        }
    }

    /// The finger moved to `at` (window coordinates) at `now` ms.
    pub(crate) fn moved(&mut self, at: (f32, f32), now: u64) -> Step {
        let (dx, dy) = (at.0 - self.start.0, at.1 - self.start.1);
        let mut started = false;
        if self.phase == Phase::Undecided {
            if dx.abs() > self.slop && dx.abs() > dy.abs() {
                // A direction the row does not offer does not move it, and
                // the touch is not taken from the list for it (§3).
                if (dx > 0.0 && self.right) || (dx < 0.0 && self.left) {
                    self.phase = Phase::Tracking;
                    started = true;
                } else {
                    self.phase = Phase::Ignored;
                }
            } else if dy.abs() > self.slop {
                self.phase = Phase::Ignored;
            }
        }
        self.first_move.get_or_insert(now);
        if self.phase != Phase::Tracking {
            return Step {
                offset: 0.0,
                started,
                armed: false,
                tick: false,
            };
        }
        // 1:1 in an allowed direction; held at rest the other way.
        self.offset = if dx > 0.0 {
            if self.right { dx } else { 0.0 }
        } else if self.left {
            dx
        } else {
            0.0
        };
        let was = self.armed;
        self.armed = self.offset.abs() >= self.threshold();
        Step {
            offset: self.offset,
            started,
            armed: self.armed,
            tick: self.armed && !was,
        }
    }

    /// The finger lifted at `now` ms: the side to commit toward, or `None`
    /// to spring back. A release while armed commits; so does a fling,
    /// armed or not: a release within [`FLING_WINDOW_MS`] of the first move
    /// the row saw, the row having moved at least [`FLING_MIN`] from
    /// touch-down in an allowed direction (§4). A single late move of that
    /// distance is a fling, since the distance counts from touch-down.
    #[must_use]
    pub(crate) fn release(&self, now: u64) -> Option<Side> {
        if self.phase != Phase::Tracking {
            return None;
        }
        // The offset is held at rest the way the row does not move, so its
        // sign is the allowed direction it moved in.
        let side = if self.offset > 0.0 {
            Side::Right
        } else {
            Side::Left
        };
        if self.armed {
            return Some(side);
        }
        let quick = self
            .first_move
            .is_some_and(|first| now.saturating_sub(first) <= FLING_WINDOW_MS);
        (quick && self.offset.abs() >= FLING_MIN).then_some(side)
    }

    /// Whether the row is following the finger.
    #[must_use]
    pub(crate) fn tracking(&self) -> bool {
        self.phase == Phase::Tracking
    }

    /// How far the row must move to arm: 40 % of its width, at least 96.
    fn threshold(&self) -> f32 {
        (self.width * ARM_FRACTION).max(ARM_MIN)
    }
}

#[cfg(test)]
// An offset at rest is exactly 0.0, set rather than computed.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const ROW: f32 = 360.0;
    const WINDOW: f32 = 400.0;

    fn phone() -> TouchSettings {
        TouchSettings {
            slop: 8.0,
            edge_left: 24.0,
            edge_right: 24.0,
            ..TouchSettings::default()
        }
    }

    fn press(left: bool, right: bool) -> Gesture {
        Gesture::press(&phone(), WINDOW, (200.0, 100.0), ROW, left, right)
    }

    /// Move along a straight line from the press, `steps` samples `ms`
    /// apart, ending `(dx, dy)` away.
    fn drag(g: &mut Gesture, dx: f32, dy: f32, steps: u32, ms: u64) -> Step {
        let mut last = None;
        for i in 1..=steps {
            #[allow(clippy::cast_precision_loss)]
            let f = i as f32 / steps as f32;
            last = Some(g.moved((200.0 + dx * f, 100.0 + dy * f), u64::from(i) * ms));
        }
        last.expect("at least one step")
    }

    #[test]
    fn a_horizontal_drag_tracks_1_to_1_once_past_the_slop() {
        let mut g = press(true, true);
        let early = g.moved((206.0, 100.0), 10);
        assert_eq!(early.offset, 0.0, "within the slop: at rest");
        let step = g.moved((230.0, 104.0), 20);
        assert!(step.started && g.tracking());
        assert!((step.offset - 30.0).abs() < f32::EPSILON, "1:1");
    }

    #[test]
    fn a_drag_more_vertical_than_horizontal_never_swipes() {
        // 55-60 degrees off horizontal: Slint's own handler would take it
        // (dx > |dy| / 2); this rule leaves it to the list (item 11).
        for degrees in [46.0_f32, 55.0, 60.0, 63.0] {
            let mut g = press(true, true);
            let (dy, dx) = degrees.to_radians().sin_cos();
            let step = drag(&mut g, dx * 200.0, dy * 200.0, 40, 8);
            assert!(!g.tracking(), "{degrees} degrees tracked");
            assert_eq!(step.offset, 0.0);
            assert_eq!(g.release(400), None);
        }
    }

    #[test]
    fn vertical_first_is_a_scroll_for_the_rest_of_the_touch() {
        let mut g = press(true, true);
        g.moved((200.0, 112.0), 10);
        let step = g.moved((300.0, 112.0), 20);
        assert!(!g.tracking(), "it never becomes a swipe");
        assert_eq!(step.offset, 0.0);
    }

    #[test]
    fn a_direction_with_no_action_does_not_move_and_is_not_taken() {
        let mut g = press(true, false);
        let step = drag(&mut g, 200.0, 0.0, 10, 10);
        assert!(!g.tracking());
        assert_eq!(step.offset, 0.0);
        assert_eq!(g.release(100), None);
    }

    #[test]
    fn a_row_with_no_action_never_tracks() {
        let mut g = press(false, false);
        drag(&mut g, -200.0, 0.0, 10, 10);
        assert!(!g.tracking());
    }

    #[test]
    fn tracking_holds_at_rest_when_dragged_back_past_its_start() {
        let mut g = press(false, true);
        g.moved((240.0, 100.0), 10);
        let step = g.moved((150.0, 100.0), 20);
        assert_eq!(step.offset, 0.0, "the row never moves the other way");
    }

    #[test]
    fn arming_ticks_once_per_outward_crossing_and_never_on_the_way_back() {
        let mut g = press(true, true);
        // 40 % of 360 is 144.
        let ticks: Vec<bool> = [50.0, 140.0, 150.0, 160.0, 100.0, 150.0]
            .into_iter()
            .enumerate()
            .map(|(i, dx)| {
                g.moved(
                    (200.0 + dx, 100.0),
                    10 * (u64::try_from(i).expect("small") + 1),
                )
                .tick
            })
            .collect();
        assert_eq!(ticks, [false, false, true, false, false, true]);
    }

    #[test]
    fn the_threshold_is_never_under_96_on_a_narrow_row() {
        let mut g = Gesture::press(&phone(), WINDOW, (200.0, 100.0), 160.0, true, true);
        assert!(!g.moved((290.0, 100.0), 10).armed, "40 % would be 64");
        assert!(g.moved((297.0, 100.0), 20).armed);
    }

    #[test]
    fn release_commits_only_while_armed() {
        let mut g = press(true, true);
        drag(&mut g, 150.0, 0.0, 30, 20);
        assert_eq!(g.release(620), Some(Side::Right));
        // Item 2: to 30 % and released, slowly.
        let mut g = press(true, true);
        drag(&mut g, 108.0, 0.0, 30, 20);
        assert_eq!(g.release(620), None);
        // To past the threshold and back to 30 %: disarmed.
        let mut g = press(true, true);
        drag(&mut g, 150.0, 0.0, 15, 20);
        let back = g.moved((308.0, 100.0), 400);
        assert!(!back.armed && !back.tick);
        assert_eq!(g.release(1_000), None);
    }

    #[test]
    fn a_fling_of_75_in_150_ms_from_touch_down_commits_unarmed() {
        // Item 3, as the Pixel delivers it: the press reaches the row about
        // 100 ms late, so the row sees the last 50 ms of a 150 ms fling.
        let mut g = press(true, true);
        g.moved((250.0, 100.0), 100);
        let step = g.moved((275.0, 100.0), 140);
        assert!(!step.armed);
        assert_eq!(g.release(150), Some(Side::Right));
    }

    #[test]
    fn a_single_late_move_of_48_from_touch_down_is_a_fling() {
        let mut g = press(true, true);
        g.moved((248.0, 100.0), 120);
        assert_eq!(g.release(220), Some(Side::Right), "100 ms after the move");
        let mut g = press(true, true);
        g.moved((152.0, 100.0), 120);
        assert_eq!(g.release(130), Some(Side::Left), "to the left too");
    }

    #[test]
    fn a_fling_needs_its_distance_and_its_window() {
        // Quick but under 48 from touch-down.
        let mut g = press(true, true);
        drag(&mut g, 47.0, 0.0, 4, 10);
        assert_eq!(g.release(45), None);
        // Far enough, released 101 ms after the first move.
        let mut g = press(true, true);
        g.moved((260.0, 100.0), 10);
        assert_eq!(g.release(111), None);
        // A long drag ending fast is not a fling: the window is from the
        // first move, not the last.
        let mut g = press(true, true);
        drag(&mut g, 120.0, 0.0, 10, 30);
        assert_eq!(g.release(305), None);
        // A direction the row does not offer: no fling that way.
        let mut g = press(false, true);
        g.moved((140.0, 100.0), 10);
        assert_eq!(g.release(20), None);
    }

    #[test]
    fn a_release_with_no_move_seen_does_nothing() {
        // A fling that ended before the row saw a move (§4).
        let g = press(true, true);
        assert_eq!(g.release(90), None);
    }

    #[test]
    fn a_touch_that_starts_in_a_gesture_inset_is_the_systems() {
        let mut g = Gesture::press(&phone(), WINDOW, (10.0, 100.0), ROW, true, true);
        g.moved((200.0, 100.0), 50);
        assert!(!g.tracking(), "the left edge's back gesture");
        let mut g = Gesture::press(&phone(), WINDOW, (390.0, 100.0), ROW, true, true);
        g.moved((200.0, 100.0), 50);
        assert!(!g.tracking(), "the right edge's");
    }

    #[test]
    fn the_snackbar_is_up_6_seconds_at_least_and_2_minutes_at_most() {
        assert_eq!(snackbar_ms(0), 6_000);
        assert_eq!(snackbar_ms(20_000), 20_000);
        assert_eq!(snackbar_ms(600_000), 120_000);
    }
}
