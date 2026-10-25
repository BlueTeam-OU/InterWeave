// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The desktop's scan: an image file the person opens, or the image on
//! the clipboard (`design/2026-10-22-add-a-peer.md` §5.5; no webcam, the
//! owner's word of 2026-10-22). Off the window's thread, since the dialog
//! blocks until the person answers and a large image takes a moment to
//! read; the result goes back as the [`PeersInput`] the model takes.
//!
//! A file is held to [`crate::code`]'s bounds before it is decoded: its
//! bytes, its sides and the decoder's allocation. The clipboard's image
//! is not read at all: the clipboard crate decodes an image whole, under
//! no bound of this client's, before handing it over, so a small image
//! with huge declared sides could take hundreds of MiB (#409 F1). A
//! Paste press ends as a cancelled scan, whatever the window offers,
//! until the clipboard's encoded bytes can be read and taken through
//! [`crate::code::read_image`].

use std::io::Read as _;
use std::sync::{Arc, Mutex};

use interweave_human_ui_model::{PeersInput, ScanControl};

use crate::code::{self, CodeRead, MAX_IMAGE_BYTES};

/// Read the image `control` names on a thread of its own and hand `done`
/// what it came to: the code's text, no code, or the dialog cancelled.
/// The camera is the Android app's, and is not read here.
pub(crate) fn read(
    control: ScanControl,
    filter: String,
    done: impl FnOnce(PeersInput) + Send + 'static,
) {
    let read = move || match control {
        ScanControl::OpenImage => from_file(&filter),
        ScanControl::PasteImage | ScanControl::Camera => PeersInput::ScannerClosed,
    };
    // Whoever takes `done` first calls it: the thread with its result, or
    // this function when no thread could be made, ending the scan as a
    // cancelled one rather than leaving it open on a result that never
    // comes.
    let done = Arc::new(Mutex::new(Some(done)));
    let held = Arc::clone(&done);
    let spawned = std::thread::Builder::new()
        .name("code-image".to_owned())
        .spawn(move || {
            let input = read();
            if let Some(done) = held.lock().ok().and_then(|mut d| d.take()) {
                done(input);
            }
        });
    if spawned.is_err()
        && let Some(done) = done.lock().ok().and_then(|mut d| d.take())
    {
        done(PeersInput::ScannerClosed);
    }
}

fn from_file(filter: &str) -> PeersInput {
    let Some(path) = rfd::FileDialog::new()
        .add_filter(filter, &["png", "jpg", "jpeg"])
        .pick_file()
    else {
        return PeersInput::ScannerClosed;
    };
    // A file that cannot be read, or decoded within bounds, was never
    // looked at for a code: unreadable, not "no code" (§5.5).
    let Ok(file) = std::fs::File::open(path) else {
        return PeersInput::ImageUnreadable;
    };
    // One byte past the bound tells a file too large from one at it.
    let mut bytes = Vec::new();
    let bound = u64::try_from(MAX_IMAGE_BYTES).unwrap_or(u64::MAX) + 1;
    if file.take(bound).read_to_end(&mut bytes).is_err() {
        return PeersInput::ImageUnreadable;
    }
    code::read_image(&bytes).map_or(PeersInput::ImageUnreadable, found)
}

fn found(read: CodeRead) -> PeersInput {
    match read {
        CodeRead::Text(text) => PeersInput::Scanned(text),
        CodeRead::NoCode => PeersInput::ImageNoCode,
    }
}

/// The own card's photo, from the file dialog (contact-cards design 5.2):
/// `None` when the person cancels; the file's bytes otherwise, held to the
/// scan's byte bound. A file that cannot be read, or is past the bound,
/// is handed on empty, which preparing refuses, so the editor says the
/// photo cannot be used rather than that nothing was chosen.
pub(crate) fn photo_file(filter: &str) -> Option<Vec<u8>> {
    let path = rfd::FileDialog::new()
        .add_filter(filter, &["png", "jpg", "jpeg"])
        .pick_file()?;
    let Ok(file) = std::fs::File::open(path) else {
        return Some(Vec::new());
    };
    let mut bytes = Vec::new();
    let bound = u64::try_from(MAX_IMAGE_BYTES).unwrap_or(u64::MAX) + 1;
    if file.take(bound).read_to_end(&mut bytes).is_err() || bytes.len() > MAX_IMAGE_BYTES {
        return Some(Vec::new());
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;

    /// A Paste press reads no clipboard and ends as a cancelled scan,
    /// whatever the window offers (#409 F1): the clipboard crate's
    /// decode is unbounded, so the reader must not reach it.
    #[test]
    fn a_paste_press_reads_no_clipboard_and_ends_cancelled() {
        let (tx, rx) = mpsc::channel();
        read(ScanControl::PasteImage, String::new(), move |input| {
            let _ = tx.send(input);
        });
        let input = rx.recv_timeout(Duration::from_secs(10));
        assert_eq!(input, Ok(PeersInput::ScannerClosed));
    }
}
