// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The window manager's jobs these cases need: giving the client's
//! window the input focus, as a desktop does when the person clicks it
//! or switches to it, and raising it above the others. The test display
//! has no window manager, so nothing else would, and a read follows the
//! window's focus (STATE.md: a message is read when it is shown in a
//! focused window). And the person's own hand: a key and a click, through
//! XTEST.

use std::time::Duration;

use x11rb::connection::Connection as _;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, InputFocus, MapState, Window};
use x11rb::rust_connection::RustConnection;

use crate::common::PATIENCE;

/// The display has one input focus, and a case that reads needs it for
/// its whole run: two such cases in parallel take it from each other, and
/// each then waits for a read that never comes. Held for the case's run.
/// A client the other cases start does not take it: with no window
/// manager, nothing answers a window's request to be activated. It can
/// cover the case's window, though: every window opens at the display's
/// origin and the one mapped last is on top, and a click goes to the
/// window under the pointer while a read of a window's pixels returns
/// what covers it. So [`luma_of`] and [`click`] raise the window first
/// ([`raise`]); one mapped between the raise and the read or the click
/// still covers it, which only a run of one case at a time excludes
/// (nextest's `end-to-end` group, `.config/nextest.toml`).
static FOCUS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// The display's focus, for this case alone until the guard drops.
pub(crate) async fn exclusive() -> tokio::sync::MutexGuard<'static, ()> {
    FOCUS.lock().await
}

/// Give the input focus to the viewable top-level window of process
/// `pid` on `$DISPLAY`, once it exists.
pub(crate) async fn focus(pid: u32, log: impl Fn() -> String) {
    let (conn, screen) = x11rb::connect(None).expect("the test display answers");
    let root = conn.setup().roots[screen].root;
    let wm_pid = conn
        .intern_atom(false, b"_NET_WM_PID")
        .expect("an atom request")
        .reply()
        .expect("the _NET_WM_PID atom")
        .atom;
    let deadline = tokio::time::Instant::now() + PATIENCE;
    let window = loop {
        if let Some(window) = window_of(&conn, root, wm_pid, pid) {
            break window;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "no viewable window of pid {pid} on the display: {}",
            log()
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    conn.set_input_focus(InputFocus::PARENT, window, x11rb::CURRENT_TIME)
        .expect("a focus request")
        .check()
        .expect("the display gives the window the focus");
}

/// Put `window` above its siblings, as a window manager does for the
/// window a person clicks, so nothing another case opened covers it. A
/// part it uncovers is drawn again by the client in its next frame.
fn raise(conn: &RustConnection, window: Window) {
    use x11rb::protocol::xproto::{ConfigureWindowAux, StackMode};
    conn.configure_window(
        window,
        &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
    )
    .expect("a stacking request")
    .check()
    .expect("the display raises the window");
}

/// A viewable child of `root` whose `_NET_WM_PID` is `pid`.
fn window_of(conn: &RustConnection, root: Window, wm_pid: u32, pid: u32) -> Option<Window> {
    let tree = conn.query_tree(root).ok()?.reply().ok()?;
    tree.children.into_iter().find(|&window| {
        let viewable = conn
            .get_window_attributes(window)
            .ok()
            .and_then(|c| c.reply().ok())
            .is_some_and(|a| a.map_state == MapState::VIEWABLE);
        let owner = conn
            .get_property(false, window, wm_pid, AtomEnum::CARDINAL, 0, 1)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|p| p.value32().and_then(|mut v| v.next()));
        viewable && owner == Some(pid)
    })
}

/// What the viewable top-level window of process `pid` shows, as a
/// person sees it: its width, its height and its pixels' luma, row by
/// row. The test display's visual is a 24-bit true-colour one, in which
/// a pixel is four bytes, blue first.
pub(crate) fn luma_of(pid: u32) -> (usize, usize, Vec<u8>) {
    use x11rb::protocol::xproto::ImageFormat;
    let (conn, screen) = x11rb::connect(None).expect("the test display answers");
    let root = conn.setup().roots[screen].root;
    let wm_pid = conn
        .intern_atom(false, b"_NET_WM_PID")
        .expect("an atom request")
        .reply()
        .expect("the _NET_WM_PID atom")
        .atom;
    let window = window_of(&conn, root, wm_pid, pid).expect("the client's window");
    raise(&conn, window);
    let geometry = conn
        .get_geometry(window)
        .expect("a geometry request")
        .reply()
        .expect("the window's geometry");
    let image = conn
        .get_image(
            ImageFormat::Z_PIXMAP,
            window,
            0,
            0,
            geometry.width,
            geometry.height,
            !0,
        )
        .expect("an image request")
        .reply()
        .expect("the window's pixels");
    let (width, height) = (usize::from(geometry.width), usize::from(geometry.height));
    assert_eq!(
        image.data.len(),
        width * height * 4,
        "four bytes a pixel at depth {}",
        image.depth
    );
    let luma = image
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[b, g, r, _]| {
            let (b, g, r) = (u32::from(b), u32::from(g), u32::from(r));
            u8::try_from((299 * r + 587 * g + 114 * b) / 1000).unwrap_or(u8::MAX)
        })
        .collect();
    (width, height, luma)
}

/// Tab pressed and released on the test display's keyboard, through
/// XTEST: what a person's key does, to the window holding the input
/// focus ([`focus`] gives it), past every toolkit hook a test backend
/// would skip.
pub(crate) fn tab() {
    Hand::new().key(TAB);
}

/// The first button pressed and released at `(x, y)` of the viewable
/// top-level window of process `pid`, through XTEST: a person's click,
/// which reaches the window's own event first, as no accessible action
/// does.
pub(crate) fn click(pid: u32, x: i32, y: i32) {
    Hand::new().click(pid, x, y);
}

/// `XK_Tab`.
pub(crate) const TAB: u32 = 0xff09;
/// `XK_Escape`.
pub(crate) const ESCAPE: u32 = 0xff1b;
/// `XK_Return`.
pub(crate) const RETURN: u32 = 0xff0d;
/// `XK_Shift_L`.
const SHIFT: u32 = 0xffe1;

/// Now on the host's monotonic clock, in microseconds: the clock the
/// traced client stamps its lines with (`apps/human-desktop`'s
/// `trace.rs`), so a send and a line hold together.
pub(crate) fn mono_us() -> u64 {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
    // Both fields are non-negative for this clock (clock_gettime(2)).
    t.tv_sec.unsigned_abs() * 1_000_000 + t.tv_nsec.unsigned_abs() / 1_000
}

/// A person's hand on the test display: its keyboard and its pointer,
/// through XTEST, over one connection made beforehand, so a timed press
/// goes out when it is due. Each press is stamped with [`mono_us`] the
/// moment before it is sent, whatever the window then does with it: the
/// sending tool's time A5 times a Back from.
pub(crate) struct Hand {
    conn: RustConnection,
    root: Window,
    wm_pid: u32,
    low: u8,
    per: usize,
    keysyms: Vec<u32>,
}

impl Hand {
    /// The display's keyboard and pointer.
    pub(crate) fn new() -> Self {
        let (conn, screen) = x11rb::connect(None).expect("the test display answers");
        let root = conn.setup().roots[screen].root;
        let (low, high) = (conn.setup().min_keycode, conn.setup().max_keycode);
        let map = conn
            .get_keyboard_mapping(low, high - low + 1)
            .expect("a keymap request")
            .reply()
            .expect("the display's keymap");
        let wm_pid = conn
            .intern_atom(false, b"_NET_WM_PID")
            .expect("an atom request")
            .reply()
            .expect("the _NET_WM_PID atom")
            .atom;
        Self {
            conn,
            root,
            wm_pid,
            low,
            per: usize::from(map.keysyms_per_keycode),
            keysyms: map.keysyms,
        }
    }

    /// The key typing `keysym` pressed and released, to the window
    /// holding the input focus; when the press was sent, by [`mono_us`].
    pub(crate) fn key(&self, keysym: u32) -> u64 {
        use x11rb::protocol::xproto::{KEY_PRESS_EVENT, KEY_RELEASE_EVENT};
        let code = self.code(keysym);
        self.send([
            (KEY_PRESS_EVENT, code, 0, 0),
            (KEY_RELEASE_EVENT, code, 0, 0),
        ])
    }

    /// Shift held while Tab is pressed and released: a step back along
    /// the focus order.
    pub(crate) fn shift_tab(&self) {
        use x11rb::protocol::xproto::{KEY_PRESS_EVENT, KEY_RELEASE_EVENT};
        let (shift, tab) = (self.code(SHIFT), self.code(TAB));
        self.send([
            (KEY_PRESS_EVENT, shift, 0, 0),
            (KEY_PRESS_EVENT, tab, 0, 0),
            (KEY_RELEASE_EVENT, tab, 0, 0),
            (KEY_RELEASE_EVENT, shift, 0, 0),
        ]);
    }

    /// The keycode that types `keysym`.
    fn code(&self, keysym: u32) -> u8 {
        self.keysyms
            .chunks(self.per)
            .position(|syms| syms.contains(&keysym))
            .and_then(|at| u8::try_from(at).ok())
            .map_or_else(
                || panic!("a key that types keysym {keysym:#x}"),
                |at| self.low + at,
            )
    }

    /// The first button pressed and released at `(x, y)` of the viewable
    /// top-level window of process `pid`, raised first; when the press
    /// was sent, by [`mono_us`].
    pub(crate) fn click(&self, pid: u32, x: i32, y: i32) -> u64 {
        use x11rb::protocol::xproto::{
            BUTTON_PRESS_EVENT, BUTTON_RELEASE_EVENT, MOTION_NOTIFY_EVENT,
        };
        let (root_x, root_y) = self.on_screen(pid, x, y);
        self.send([(MOTION_NOTIFY_EVENT, 0, root_x, root_y)]);
        self.send([
            (BUTTON_PRESS_EVENT, 1, root_x, root_y),
            (BUTTON_RELEASE_EVENT, 1, root_x, root_y),
        ])
    }

    /// `(x, y)` of the viewable top-level window of process `pid`, raised
    /// first, on the screen.
    fn on_screen(&self, pid: u32, x: i32, y: i32) -> (i16, i16) {
        let window =
            window_of(&self.conn, self.root, self.wm_pid, pid).expect("the client's window");
        raise(&self.conn, window);
        let at = self
            .conn
            .translate_coordinates(window, self.root, 0, 0)
            .expect("a translation request")
            .reply()
            .expect("the window's place on the screen");
        let to = |c: i32, origin: i16| {
            i16::try_from(c + i32::from(origin)).expect("a point on the screen")
        };
        (to(x, at.dst_x), to(y, at.dst_y))
    }

    /// `events` faked in order, each checked as taken; the stamp read the
    /// moment before the first goes out.
    fn send<const N: usize>(&self, events: [(u8, u8, i16, i16); N]) -> u64 {
        use x11rb::protocol::xtest::ConnectionExt as _;
        let sent = mono_us();
        for (kind, detail, x, y) in events {
            self.conn
                .xtest_fake_input(kind, detail, x11rb::CURRENT_TIME, self.root, x, y, 0)
                .expect("an XTEST request")
                .check()
                .expect("the display takes the input");
        }
        sent
    }
}
