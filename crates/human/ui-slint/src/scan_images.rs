// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The desktop's scan: an image file the person opens, or the image on
//! the clipboard (`design/2026-10-22-add-a-peer.md` §5.5; no webcam, the
//! owner's word of 2026-10-22). Off the window's thread, since the dialog
//! blocks until the person answers and a large image takes a moment to
//! read; the result goes back as the [`PeersInput`] the model takes.
//!
//! A file is held to [`crate::code`]'s bounds before it is decoded: its
//! bytes, its sides and the decoder's allocation. A clipboard image is
//! read whole and decoded by the clipboard crate, under the image
//! crate's default allocation bound, before its sides are refused past
//! the same bound -- a known limit ([`crate::code::read_pasted`]).

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
        ScanControl::PasteImage => from_clipboard(),
        ScanControl::Camera => PeersInput::ScannerClosed,
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

fn from_clipboard() -> PeersInput {
    let image = arboard::Clipboard::new().and_then(|mut c| c.get_image());
    match image {
        Ok(image) => code::read_pasted(image.width, image.height, &image.bytes)
            .map_or(PeersInput::ImageUnreadable, found),
        // No image there: text only, or nothing.
        Err(arboard::Error::ContentNotAvailable) => PeersInput::ClipboardNoImage,
        // An image that could not be converted, or a clipboard that could
        // not be read at all -- busy, unsupported, failed: never "no
        // image", which was not seen (#409 F6). The design names no line
        // of its own for an unreadable clipboard; the image's is the
        // nearest true one.
        Err(_) => PeersInput::ImageUnreadable,
    }
}

fn found(read: CodeRead) -> PeersInput {
    match read {
        CodeRead::Text(text) => PeersInput::Scanned(text),
        CodeRead::NoCode => PeersInput::ImageNoCode,
    }
}
