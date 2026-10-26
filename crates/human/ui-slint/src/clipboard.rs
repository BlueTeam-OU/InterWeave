// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The desktop's Paste an image on Linux (`design/2026-10-22-add-a-peer.md`
//! §5.5): the clipboard's image as its encoded bytes, read within
//! [`MAX_IMAGE_BYTES`] and [`WAIT`], for [`crate::code::read_image`] to
//! decode under the same bounds a file has.
//!
//! The clipboard is another program's: it decides what it offers, how
//! much it sends and how fast. So nothing here decodes, and every read
//! stops at one byte past the bound or at the deadline, whichever comes
//! first. The toolkit's clipboard crate is not used for it: it decodes an
//! image whole before handing it over, under no bound of this client's
//! (#409 F1).
//!
//! Wayland is asked first, through the data-control protocol; a
//! compositor without it, or a session with no Wayland, is asked through
//! X11, which a Wayland desktop's X server mirrors.

use std::io::{ErrorKind, Read};
use std::os::fd::AsFd;
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec};

use crate::code::MAX_IMAGE_BYTES;

/// How long a paste may take, from the press to the last byte: past it
/// the program holding the clipboard is not answering.
pub(crate) const WAIT: Duration = Duration::from_secs(5);

/// The image types asked for, in order: the two a code is read from.
const IMAGE_TYPES: [&str; 2] = ["image/png", "image/jpeg"];

/// What a paste came to.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Pasted {
    /// An image's encoded bytes, at most [`MAX_IMAGE_BYTES`].
    Image(Vec<u8>),
    /// The clipboard is empty or holds no PNG or JPEG.
    NoImage,
    /// The clipboard could not be read, so what it holds is not known.
    Unreadable,
    /// An image was offered and could not be had: past the bound, refused
    /// when asked for, or not sent in time.
    ImageUnread,
}

/// The clipboard's image, read within the bounds above.
pub(crate) fn read() -> Pasted {
    let deadline = Instant::now() + WAIT;
    if std::env::var_os("WAYLAND_DISPLAY").is_some()
        && let Some(pasted) = wayland::read(deadline)
    {
        return pasted;
    }
    if std::env::var_os("DISPLAY").is_some() {
        return x11::read(deadline);
    }
    Pasted::Unreadable
}

/// The image type to ask for among those `offered`: PNG before JPEG.
fn chosen<'a>(offered: impl IntoIterator<Item = &'a str>) -> Option<&'static str> {
    let offered: Vec<&str> = offered.into_iter().collect();
    IMAGE_TYPES.into_iter().find(|t| offered.contains(t))
}

/// Read `source` to its end, stopping one byte past [`MAX_IMAGE_BYTES`] or
/// at `deadline`: a source still open then is a writer that is not
/// answering, never waited on further. Without blocking, so the deadline
/// holds however the writer paces itself.
fn read_bounded(mut source: impl Read + AsFd, deadline: Instant) -> Pasted {
    if rustix::io::ioctl_fionbio(&source, true).is_err() {
        return Pasted::ImageUnread;
    }
    let mut bytes = Vec::new();
    let mut chunk = vec![0; 64 * 1024];
    loop {
        match source.read(&mut chunk) {
            Ok(0) => return Pasted::Image(bytes),
            Ok(n) => {
                if bytes.len() + n > MAX_IMAGE_BYTES {
                    return Pasted::ImageUnread;
                }
                bytes.extend_from_slice(&chunk[..n]);
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                if !wait_readable(&source, deadline) {
                    return Pasted::ImageUnread;
                }
            }
            Err(_) => return Pasted::ImageUnread,
        }
    }
}

/// Wait until `fd` can be read or `deadline` passes; `false` at the
/// deadline or on a failed wait.
fn wait_readable(fd: &impl AsFd, deadline: Instant) -> bool {
    loop {
        let Some(left) = deadline.checked_duration_since(Instant::now()) else {
            return false;
        };
        if left.is_zero() {
            return false;
        }
        let Ok(timeout) = Timespec::try_from(left) else {
            return false;
        };
        let mut fds = [PollFd::new(fd, PollFlags::IN)];
        match rustix::event::poll(&mut fds, Some(&timeout)) {
            Ok(0) => return false,
            Ok(_) => return true,
            Err(rustix::io::Errno::INTR) => {}
            Err(_) => return false,
        }
    }
}

mod wayland {
    use std::time::Instant;

    use wl_clipboard_rs::paste::{self, ClipboardType, Error, MimeType, Seat};

    use super::{Pasted, chosen, read_bounded};

    /// The clipboard through the data-control protocol; `None` when the
    /// compositor cannot be asked so, for X11 to be asked instead.
    pub(super) fn read(deadline: Instant) -> Option<Pasted> {
        let offered = match paste::get_mime_types_ordered(ClipboardType::Regular, Seat::Unspecified)
        {
            Ok(offered) => offered,
            Err(e) => return listed_error(&e),
        };
        let Some(wanted) = chosen(offered.iter().map(String::as_str)) else {
            return Some(Pasted::NoImage);
        };
        Some(
            match paste::get_contents(
                ClipboardType::Regular,
                Seat::Unspecified,
                MimeType::Specific(wanted),
            ) {
                Ok((pipe, _)) => read_bounded(pipe, deadline),
                Err(_) => Pasted::ImageUnread,
            },
        )
    }

    /// What a failure to list the clipboard's types says: empty is no
    /// image; a compositor that cannot be reached or lacks the protocol
    /// is a question for X11; anything else is a clipboard unread.
    pub(super) fn listed_error(e: &Error) -> Option<Pasted> {
        match e {
            Error::ClipboardEmpty | Error::NoMimeType => Some(Pasted::NoImage),
            Error::MissingProtocol { .. }
            | Error::SocketOpenError(_)
            | Error::WaylandConnection(_) => None,
            _ => Some(Pasted::Unreadable),
        }
    }
}

mod x11 {
    use std::time::Instant;

    use x11rb::connection::Connection as _;
    use x11rb::protocol::Event;
    use x11rb::protocol::xproto::{
        Atom, AtomEnum, ConnectionExt as _, CreateWindowAux, EventMask, Property, Window,
        WindowClass,
    };
    use x11rb::rust_connection::RustConnection;

    use super::{MAX_IMAGE_BYTES, Pasted, wait_readable};

    /// The most targets read from an owner's list.
    const MAX_TARGETS: u32 = 1024;

    struct Atoms {
        clipboard: Atom,
        targets: Atom,
        incr: Atom,
        into: Atom,
        png: Atom,
        jpeg: Atom,
    }

    /// The clipboard through the X server: the owner's targets, then the
    /// image's bytes, plain or incremental.
    pub(super) fn read(deadline: Instant) -> Pasted {
        let Ok((conn, screen)) = RustConnection::connect(None) else {
            return Pasted::Unreadable;
        };
        let Some((atoms, window)) = prepare(&conn, screen) else {
            return Pasted::Unreadable;
        };
        let pasted = pasted(&conn, &atoms, window, deadline);
        let _ = conn.destroy_window(window);
        let _ = conn.flush();
        pasted
    }

    fn prepare(conn: &RustConnection, screen: usize) -> Option<(Atoms, Window)> {
        let atom = |name: &[u8]| -> Option<Atom> {
            Some(conn.intern_atom(false, name).ok()?.reply().ok()?.atom)
        };
        let atoms = Atoms {
            clipboard: atom(b"CLIPBOARD")?,
            targets: atom(b"TARGETS")?,
            incr: atom(b"INCR")?,
            into: atom(b"INTERWEAVE_PASTE")?,
            png: atom(b"image/png")?,
            jpeg: atom(b"image/jpeg")?,
        };
        let root = conn.setup().roots.get(screen)?.root;
        let window = conn.generate_id().ok()?;
        conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            window,
            root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_ONLY,
            x11rb::COPY_FROM_PARENT,
            &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )
        .ok()?
        .check()
        .ok()?;
        Some((atoms, window))
    }

    fn pasted(conn: &RustConnection, atoms: &Atoms, window: Window, deadline: Instant) -> Pasted {
        let Ok(owner) = conn.get_selection_owner(atoms.clipboard) else {
            return Pasted::Unreadable;
        };
        match owner.reply() {
            Ok(r) if r.owner == x11rb::NONE => return Pasted::NoImage,
            Ok(_) => {}
            Err(_) => return Pasted::Unreadable,
        }
        // The owner's list of targets: one it will not give is a clipboard
        // that cannot be read.
        if !converted(conn, atoms, window, atoms.targets, deadline) {
            return Pasted::Unreadable;
        }
        let Ok(Ok(list)) = conn
            .get_property(true, window, atoms.into, AtomEnum::ATOM, 0, MAX_TARGETS)
            .map(x11rb::cookie::Cookie::reply)
        else {
            return Pasted::Unreadable;
        };
        let offered: Vec<Atom> = list.value32().map(Iterator::collect).unwrap_or_default();
        let Some(wanted) = [atoms.png, atoms.jpeg]
            .into_iter()
            .find(|t| offered.contains(t))
        else {
            return Pasted::NoImage;
        };
        // From here an image is offered: whatever stops it is an image
        // that could not be had.
        if !converted(conn, atoms, window, wanted, deadline) {
            return Pasted::ImageUnread;
        }
        image(conn, atoms, window, deadline)
    }

    /// Ask the owner for `target` into this window's property, and wait
    /// for its answer: `false` when it refuses, or does not answer by the
    /// deadline.
    fn converted(
        conn: &RustConnection,
        atoms: &Atoms,
        window: Window,
        target: Atom,
        deadline: Instant,
    ) -> bool {
        if conn
            .convert_selection(
                window,
                atoms.clipboard,
                target,
                atoms.into,
                x11rb::CURRENT_TIME,
            )
            .is_err()
            || conn.flush().is_err()
        {
            return false;
        }
        loop {
            match next_event(conn, deadline) {
                Some(Event::SelectionNotify(e)) if e.requestor == window => {
                    return e.target == target && e.property == atoms.into;
                }
                Some(_) => {}
                None => return false,
            }
        }
    }

    /// The image the owner put in this window's property: whole, or in
    /// the incremental chunks of a large one, held to the byte bound.
    fn image(conn: &RustConnection, atoms: &Atoms, window: Window, deadline: Instant) -> Pasted {
        // One 32-bit unit past the bound tells an image past it apart.
        let units = u32::try_from(MAX_IMAGE_BYTES / 4 + 1).unwrap_or(u32::MAX);
        let Ok(Ok(first)) = conn
            .get_property(true, window, atoms.into, AtomEnum::ANY, 0, units)
            .map(x11rb::cookie::Cookie::reply)
        else {
            return Pasted::ImageUnread;
        };
        if first.type_ != atoms.incr {
            if first.bytes_after > 0 || first.value.len() > MAX_IMAGE_BYTES {
                return Pasted::ImageUnread;
            }
            return Pasted::Image(first.value);
        }
        // Incremental: the property's deletion above asked for the first
        // chunk; each chunk comes as a new value, and an empty one ends it.
        // The owner's announced size is a lower bound, so past the bound
        // already is past it.
        let announced = first.value32().and_then(|mut v| v.next()).unwrap_or(0);
        if usize::try_from(announced).map_or(true, |n| n > MAX_IMAGE_BYTES) {
            return Pasted::ImageUnread;
        }
        if conn.flush().is_err() {
            return Pasted::ImageUnread;
        }
        let mut bytes = Vec::new();
        loop {
            match next_event(conn, deadline) {
                Some(Event::PropertyNotify(e))
                    if e.window == window
                        && e.atom == atoms.into
                        && e.state == Property::NEW_VALUE =>
                {
                    let left = MAX_IMAGE_BYTES - bytes.len();
                    let units = u32::try_from(left / 4 + 1).unwrap_or(u32::MAX);
                    let Ok(Ok(chunk)) = conn
                        .get_property(true, window, atoms.into, AtomEnum::ANY, 0, units)
                        .map(x11rb::cookie::Cookie::reply)
                    else {
                        return Pasted::ImageUnread;
                    };
                    if chunk.value.is_empty() {
                        return Pasted::Image(bytes);
                    }
                    if chunk.bytes_after > 0 || chunk.value.len() > left {
                        return Pasted::ImageUnread;
                    }
                    bytes.extend_from_slice(&chunk.value);
                    if conn.flush().is_err() {
                        return Pasted::ImageUnread;
                    }
                }
                Some(_) => {}
                None => return Pasted::ImageUnread,
            }
        }
    }

    /// The next event, or `None` at the deadline or a broken connection.
    fn next_event(conn: &RustConnection, deadline: Instant) -> Option<Event> {
        loop {
            if let Some(event) = conn.poll_for_event().ok()? {
                return Some(event);
            }
            if !wait_readable(conn.stream(), deadline) {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::*;

    fn pipe() -> (std::io::PipeReader, std::io::PipeWriter) {
        std::io::pipe().expect("pipe")
    }

    #[test]
    fn png_is_chosen_before_jpeg_and_nothing_else_is() {
        assert_eq!(
            chosen(["text/plain", "image/jpeg", "image/png"]),
            Some("image/png")
        );
        assert_eq!(chosen(["image/jpeg", "image/gif"]), Some("image/jpeg"));
        assert_eq!(chosen(["text/plain", "image/gif", "image/bmp"]), None);
        assert_eq!(chosen(std::iter::empty()), None);
    }

    /// What the writer sends, to its end, is the image.
    #[test]
    fn a_read_to_the_writers_end_is_the_image() {
        let (reader, mut writer) = pipe();
        let sent = std::thread::spawn(move || {
            writer.write_all(&vec![7; 200_000]).expect("write");
        });
        let pasted = read_bounded(reader, Instant::now() + Duration::from_secs(10));
        sent.join().expect("writer");
        assert_eq!(pasted, Pasted::Image(vec![7; 200_000]));
    }

    /// One byte past the bound stops the read, whatever more is coming:
    /// the bytes are never held past it.
    #[test]
    fn a_read_stops_one_byte_past_the_bound() {
        let (reader, mut writer) = pipe();
        let sent = std::thread::spawn(move || {
            let chunk = vec![1; 1024 * 1024];
            let mut left = MAX_IMAGE_BYTES + 1;
            while left > 0 {
                let n = left.min(chunk.len());
                if writer.write_all(&chunk[..n]).is_err() {
                    return;
                }
                left -= n;
            }
            // Keep the pipe open: the bound, not the end, must stop it.
            std::thread::sleep(Duration::from_secs(3));
        });
        let start = Instant::now();
        let pasted = read_bounded(reader, Instant::now() + Duration::from_secs(30));
        assert_eq!(pasted, Pasted::ImageUnread);
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "stopped at the bound"
        );
        sent.join().expect("writer");
    }

    /// A writer that holds the pipe open and sends nothing is given up at
    /// the deadline.
    #[test]
    fn a_writer_that_never_ends_is_given_up_at_the_deadline() {
        let (reader, writer) = pipe();
        let start = Instant::now();
        let pasted = read_bounded(reader, Instant::now() + Duration::from_millis(300));
        assert_eq!(pasted, Pasted::ImageUnread);
        let took = start.elapsed();
        assert!(took >= Duration::from_millis(300), "{took:?}");
        assert!(took < Duration::from_secs(3), "{took:?}");
        drop(writer);
    }

    /// Wayland: an empty clipboard or one with no type is no image; a
    /// compositor without data-control, or none reachable, is asked
    /// through X11; any other failure is a clipboard unread, never "no
    /// image" (§5.5).
    #[test]
    fn a_wayland_listing_failure_says_what_it_knows() {
        use wl_clipboard_rs::paste::Error;
        let io = || std::io::Error::other("x");
        assert_eq!(
            wayland::listed_error(&Error::ClipboardEmpty),
            Some(Pasted::NoImage)
        );
        assert_eq!(
            wayland::listed_error(&Error::NoMimeType),
            Some(Pasted::NoImage)
        );
        assert_eq!(
            wayland::listed_error(&Error::MissingProtocol {
                name: "ext_data_control_manager_v1",
                version: 1
            }),
            None
        );
        assert_eq!(wayland::listed_error(&Error::SocketOpenError(io())), None);
        for e in [
            Error::NoSeats,
            Error::SeatNotFound,
            Error::PrimarySelectionUnsupported,
            Error::PipeCreation(io()),
        ] {
            assert_eq!(wayland::listed_error(&e), Some(Pasted::Unreadable), "{e}");
        }
    }

    /// The X11 read against a real X server, the clipboard's owner xclip
    /// (or, for one that never answers, a client of this test's own):
    /// an image whole and in incremental chunks, past the bound, text
    /// only, no owner, and an owner silent past the deadline. One test,
    /// since every case owns the same clipboard.
    ///
    /// `xvfb-run -a cargo test -p interweave-human-ui-slint --features
    /// desktop --lib -- --ignored x11_reads`
    #[test]
    #[ignore = "needs an X server and xclip: run under xvfb-run, see the doc"]
    fn x11_reads_the_clipboard_within_bounds() {
        use std::process::{Child, Command, Stdio};

        use x11rb::connection::Connection as _;
        use x11rb::protocol::xproto::{ConnectionExt as _, CreateWindowAux, WindowClass};

        let dir = std::env::temp_dir().join(format!("iw-clip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let own = |kind: &str, bytes: &[u8]| -> Child {
            let file = dir.join("data");
            std::fs::write(&file, bytes).expect("write");
            let child = Command::new("xclip")
                .args(["-quiet", "-selection", "clipboard", "-t", kind, "-i"])
                .arg(&file)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("xclip");
            // xclip owns the clipboard once it has read its file.
            std::thread::sleep(Duration::from_millis(500));
            child
        };
        let soon = || Instant::now() + WAIT;

        // No owner yet: an empty clipboard.
        assert_eq!(x11::read(soon()), Pasted::NoImage);

        let lines = vec!["/ip4/192.168.1.20/tcp/4001/p2p/peer".to_owned()];
        let png = crate::code::png_for(&lines);
        for (kind, bytes, want) in [
            ("image/png", png.clone(), Pasted::Image(png.clone())),
            // Past xclip's single-request size: sent in chunks.
            (
                "image/png",
                vec![9; 3 * 1024 * 1024],
                Pasted::Image(vec![9; 3 * 1024 * 1024]),
            ),
            ("image/jpeg", vec![5; 1000], Pasted::Image(vec![5; 1000])),
            (
                "image/png",
                vec![1; MAX_IMAGE_BYTES + 1],
                Pasted::ImageUnread,
            ),
            ("text/plain", b"/ip4/192.168.1.20".to_vec(), Pasted::NoImage),
        ] {
            let mut owner = own(kind, &bytes);
            let start = Instant::now();
            let got = x11::read(soon());
            let _ = owner.kill();
            let _ = owner.wait();
            assert!(start.elapsed() < WAIT, "{kind} {}: in time", bytes.len());
            assert!(got == want, "{kind} {}: {:?}", bytes.len(), {
                match &got {
                    Pasted::Image(b) => format!("Image({})", b.len()),
                    other => format!("{other:?}"),
                }
            });
        }

        // An owner that never answers: unread at the deadline.
        let (conn, screen) = x11rb::connect(None).expect("X");
        let root = conn.setup().roots[screen].root;
        let window = conn.generate_id().expect("id");
        conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            window,
            root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_ONLY,
            x11rb::COPY_FROM_PARENT,
            &CreateWindowAux::new(),
        )
        .expect("window");
        let clipboard = conn
            .intern_atom(false, b"CLIPBOARD")
            .expect("atom")
            .reply()
            .expect("atom")
            .atom;
        conn.set_selection_owner(window, clipboard, x11rb::CURRENT_TIME)
            .expect("own");
        // A round trip: the ownership is the server's before the read.
        conn.get_input_focus().expect("sync").reply().expect("sync");
        let start = Instant::now();
        assert_eq!(
            x11::read(Instant::now() + Duration::from_millis(500)),
            Pasted::Unreadable
        );
        assert!(start.elapsed() < Duration::from_secs(3));
        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
