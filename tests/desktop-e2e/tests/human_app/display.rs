// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The window manager's one job these cases need: giving the client's
//! window the input focus, as a desktop does when the person clicks it
//! or switches to it. The test display has no window manager, so nothing
//! else would, and a read follows the window's focus (STATE.md: a message
//! is read when it is shown in a focused window).

use std::time::Duration;

use x11rb::connection::Connection as _;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, InputFocus, MapState, Window};
use x11rb::rust_connection::RustConnection;

use crate::common::PATIENCE;

/// The display has one input focus, and a case that reads needs it for
/// its whole run: two such cases in parallel take it from each other, and
/// each then waits for a read that never comes. Held for the case's run.
/// A client the other cases start does not take it: with no window
/// manager, nothing answers a window's request to be activated.
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
