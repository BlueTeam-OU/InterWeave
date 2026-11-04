// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! A text field's focus ring as a person sees it (#549): Tab onto the
//! field draws it, and a click on the field takes it away while the
//! field keeps the focus. Slint 1.18 shows a press on a text field to no
//! element around it, so only each window's winit filter can end
//! keyboard mode there, and the testing backend never calls that filter.
//! The key and the click here are the display's (XTEST), not accessible
//! actions, and the ring is read from the window's pixels.
//!
//! What this does not prove: a touch, which the filter takes too but
//! XTEST cannot make, and any window but the one a case drives.

use std::time::{Duration, Instant};

use crate::a11y::{Element, Window};
use crate::common::PATIENCE;
use crate::display;

/// How many Tabs a case may press looking for its field.
const MOST_TABS: usize = 40;

/// The luma of the middle line of the focus ring's 3 px stroke around a
/// control drawn at `extents` in process `pid`'s window, along its four
/// straight sides without their rounded corners. The ring is placed
/// `FocusMode.ring-reach` (5 px) outside the control, so that line runs
/// 4 px out.
fn band(pid: u32, (x, y, w, h): (i32, i32, i32, i32)) -> Vec<u8> {
    const OUT: i32 = 4;
    // Past the ring's corner radius, 9 px at a field's 4 px.
    const CORNER: i32 = 10;
    let (width, height, luma) = display::luma_of(pid);
    let at = |px: i32, py: i32| {
        let (px, py) = (
            usize::try_from(px).expect("inside the window"),
            usize::try_from(py).expect("inside the window"),
        );
        assert!(px < width && py < height, "({px}, {py}) inside the window");
        luma[py * width + px]
    };
    let mut band = Vec::new();
    for py in y + CORNER..y + h - CORNER {
        band.extend([at(x - OUT, py), at(x + w - 1 + OUT, py)]);
    }
    for px in x + CORNER..x + w - CORNER {
        band.extend([at(px, y - OUT), at(px, y + h - 1 + OUT)]);
    }
    assert!(!band.is_empty(), "a field larger than its ring's corners");
    band
}

/// How many of `band`'s pixels differ from `from`'s.
fn changed(band: &[u8], from: &[u8]) -> usize {
    band.iter().zip(from).filter(|(a, b)| a != b).count()
}

/// Read the band around `extents` until `done` takes it, within
/// [`PATIENCE`].
fn until(
    pid: u32,
    extents: (i32, i32, i32, i32),
    what: &str,
    mut done: impl FnMut(&[u8]) -> bool,
    log: &impl Fn() -> String,
) -> Vec<u8> {
    let deadline = Instant::now() + PATIENCE;
    loop {
        let band = band(pid, extents);
        if done(&band) {
            return band;
        }
        assert!(Instant::now() < deadline, "{what}, never\n{}", log());
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// On `screen`, process `pid`'s window with the input focus and no
/// keyboard use yet: Tab until `field` holds the focus, its ring drawn,
/// then a click on it takes the ring away and leaves it the focus.
pub(crate) async fn tab_then_click(
    screen: &Window<'_>,
    field: &Element,
    pid: u32,
    log: impl Fn() -> String,
) {
    let extents = screen.extents(field).await;
    let mut last = Vec::new();
    // No ring before any key: nothing has used the keyboard yet.
    let unringed = until(
        pid,
        extents,
        "the screen drawn and still",
        |band| {
            let still = band == last.as_slice();
            last = band.to_vec();
            still
        },
        &log,
    );
    // One Tab at least: a field that holds the focus already, as a
    // screen's first focus, draws no ring until a key is used.
    let mut tabs = 0;
    loop {
        assert!(tabs < MOST_TABS, "{MOST_TABS} Tabs never reached the field");
        display::tab();
        tabs += 1;
        // The key's turn handled before focus is asked again.
        tokio::time::sleep(Duration::from_millis(100)).await;
        if screen.focused(field).await {
            break;
        }
    }
    until(
        pid,
        extents,
        "the ring drawn",
        |band| changed(band, &unringed) * 10 >= band.len() * 9,
        &log,
    );

    display::click(pid, extents.0 + extents.2 / 2, extents.1 + extents.3 / 2);
    until(
        pid,
        extents,
        "the ring taken away",
        |band| band == unringed.as_slice(),
        &log,
    );
    assert!(
        screen.focused(field).await,
        "the click kept the focus on the field, so only keyboard mode ended"
    );
}
