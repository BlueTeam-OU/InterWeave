// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The host's stdin, one line at a time, under a ceiling checked as the
//! bytes arrive: before UTF-8 and JSON, so no line the host writes grows
//! the bridge past [`MAX_HOST_LINE_BYTES`].
//!
//! **Cancel-safe.** The serve loop re-enters [`HostLines::next_line`] in
//! a `select!` every turn and drops it whenever another branch wins, so
//! the partial line and the drain state live here, never in the future:
//! the only await is `fill_buf`, which reads nothing when cancelled, and
//! every consume happens after it with no await between.

use tokio::io::{AsyncBufRead, AsyncBufReadExt as _};

/// The most bytes one host line may hold before its newline.
///
/// Sized from the payload ceiling: content escaped at JSON's worst
/// (`\u00XX`, six bytes per byte) with room for the envelope, so every
/// line that carries an admissible payload, and most that carry one too
/// large, reach the parse and are answered under their id; a line past
/// this is refused unread, and answered with nothing.
pub const MAX_HOST_LINE_BYTES: usize = 1 << 20;

/// The envelope beside the content: the JSON-RPC frame, the tool's name
/// and its other arguments.
const ENVELOPE_ROOM_BYTES: usize = 64 << 10;

// A payload ceiling raised past this would have the bridge refuse, with
// no answer, a `tools/call` the payload check would accept.
const _: () = assert!(
    6 * interweave_transport_api::MAX_PAYLOAD_BYTES + ENVELOPE_ROOM_BYTES <= MAX_HOST_LINE_BYTES
);

/// One line of the host's, as the serve loop takes it.
#[derive(Debug, PartialEq, Eq)]
pub enum HostLine {
    /// A whole line within the ceiling, without its newline (or a
    /// trailing `\r`).
    Text(String),
    /// A line within the ceiling that is not UTF-8: malformed, as a line
    /// that is not JSON is.
    NotUtf8,
    /// A line past the ceiling. Its bytes are not kept: the rest of it is
    /// dropped as it arrives, up to its newline, by the calls that follow.
    TooLong,
}

/// The host's lines under a ceiling.
pub struct HostLines<R> {
    input: R,
    ceiling: usize,
    line: Vec<u8>,
    /// Past the ceiling, until the newline that ends the refused line.
    draining: bool,
}

impl<R: AsyncBufRead + Unpin> HostLines<R> {
    /// The host's lines under [`MAX_HOST_LINE_BYTES`].
    pub fn new(input: R) -> Self {
        Self::with_ceiling(input, MAX_HOST_LINE_BYTES)
    }

    /// Under another ceiling: the tests', at a size a test can cross.
    pub fn with_ceiling(input: R, ceiling: usize) -> Self {
        Self {
            input,
            ceiling,
            line: Vec::new(),
            draining: false,
        }
    }

    /// The next line, or `None` at the end of the input. A line cut by
    /// the end is a line, as `Lines` reads it; a refused line cut by the
    /// end was answered already.
    ///
    /// # Errors
    ///
    /// A read error of the input.
    pub async fn next_line(&mut self) -> std::io::Result<Option<HostLine>> {
        loop {
            let chunk = self.input.fill_buf().await?;
            if chunk.is_empty() {
                self.draining = false;
                if self.line.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(self.finish()));
            }
            let newline = chunk.iter().position(|&b| b == b'\n');
            let segment = &chunk[..newline.unwrap_or(chunk.len())];
            let used = newline.map_or(chunk.len(), |at| at + 1);
            if self.draining {
                self.input.consume(used);
                if newline.is_some() {
                    self.draining = false;
                }
                continue;
            }
            if segment.len() > self.ceiling - self.line.len() {
                self.line = Vec::new();
                self.draining = newline.is_none();
                self.input.consume(used);
                return Ok(Some(HostLine::TooLong));
            }
            self.line.extend_from_slice(segment);
            self.input.consume(used);
            if newline.is_some() {
                return Ok(Some(self.finish()));
            }
        }
    }

    fn finish(&mut self) -> HostLine {
        let mut line = std::mem::take(&mut self.line);
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        match String::from_utf8(line) {
            Ok(text) => HostLine::Text(text),
            Err(_) => HostLine::NotUtf8,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use tokio::io::{AsyncRead, AsyncWriteExt as _, ReadBuf};

    use super::*;

    /// An input that hands out its bytes in the chunks given, one chunk
    /// per read, as a pipe does.
    struct Chunks(std::collections::VecDeque<Vec<u8>>);

    impl AsyncRead for Chunks {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            if let Some(mut chunk) = self.0.pop_front() {
                let n = chunk.len().min(buf.remaining());
                buf.put_slice(&chunk[..n]);
                if n < chunk.len() {
                    self.0.push_front(chunk.split_off(n));
                }
            }
            Poll::Ready(Ok(()))
        }
    }

    fn lines(ceiling: usize, chunks: &[&[u8]]) -> HostLines<tokio::io::BufReader<Chunks>> {
        let chunks = chunks.iter().map(|c| c.to_vec()).collect();
        HostLines::with_ceiling(tokio::io::BufReader::new(Chunks(chunks)), ceiling)
    }

    async fn all(mut input: HostLines<impl AsyncBufRead + Unpin>) -> Vec<HostLine> {
        let mut out = Vec::new();
        while let Some(line) = input.next_line().await.expect("reads") {
            out.push(line);
        }
        out
    }

    fn text(s: &str) -> HostLine {
        HostLine::Text(s.to_owned())
    }

    #[tokio::test]
    async fn a_line_at_the_ceiling_is_read_and_one_byte_more_is_refused() {
        let got = all(lines(4, &[b"abcd\nabcde\nok\n"])).await;
        assert_eq!(got, [text("abcd"), HostLine::TooLong, text("ok")]);
    }

    #[tokio::test]
    async fn the_ceiling_holds_across_chunks() {
        let got = all(lines(
            4,
            &[b"ab", b"cd", b"\n", b"ab", b"cd", b"e", b"\nok\n"],
        ))
        .await;
        assert_eq!(got, [text("abcd"), HostLine::TooLong, text("ok")]);
    }

    #[tokio::test]
    async fn a_refused_line_is_drained_to_its_newline_in_a_later_chunk() {
        let got = all(lines(4, &[b"abcdef", b"ghij", b"klm\nok\n"])).await;
        assert_eq!(got, [HostLine::TooLong, text("ok")]);
    }

    #[tokio::test]
    async fn a_refused_line_keeps_none_of_its_bytes() {
        let mut input = lines(4, &[b"ab", b"cdefgh", b"ijklmnop"]);
        assert_eq!(
            input.next_line().await.expect("reads"),
            Some(HostLine::TooLong)
        );
        assert_eq!(
            input.line.capacity(),
            0,
            "nothing of the refused line is held"
        );
        assert_eq!(input.next_line().await.expect("reads"), None);
    }

    #[tokio::test]
    async fn the_end_cuts_a_line_and_ends_a_drain() {
        assert_eq!(all(lines(4, &[b"ok\nab"])).await, [text("ok"), text("ab")]);
        assert_eq!(all(lines(4, &[b"abcdef"])).await, [HostLine::TooLong]);
    }

    #[tokio::test]
    async fn crlf_and_empty_lines_read_as_lines_does() {
        let got = all(lines(4, &[b"ab\r\n\n"])).await;
        assert_eq!(got, [text("ab"), text("")]);
    }

    #[tokio::test]
    async fn a_line_that_is_not_utf8_is_malformed_and_the_next_is_read() {
        let got = all(lines(8, &[b"a\xffb\n", b"ok\n"])).await;
        assert_eq!(got, [HostLine::NotUtf8, text("ok")]);
    }

    #[tokio::test]
    async fn a_read_cancelled_between_chunks_loses_nothing() {
        let (mut host, bridge) = tokio::io::duplex(64);
        let mut input = HostLines::with_ceiling(tokio::io::BufReader::new(bridge), 8);
        host.write_all(b"ab").await.expect("writes");
        // The read is dropped mid-line, as the serve loop's select! drops it.
        let cancelled =
            tokio::time::timeout(std::time::Duration::from_millis(20), input.next_line()).await;
        assert!(cancelled.is_err(), "no line yet");
        host.write_all(b"cd\n").await.expect("writes");
        assert_eq!(input.next_line().await.expect("reads"), Some(text("abcd")));

        host.write_all(b"0123456789").await.expect("writes");
        assert_eq!(
            input.next_line().await.expect("reads"),
            Some(HostLine::TooLong)
        );
        let cancelled =
            tokio::time::timeout(std::time::Duration::from_millis(20), input.next_line()).await;
        assert!(cancelled.is_err(), "still draining");
        host.write_all(b"more\nok\n").await.expect("writes");
        assert_eq!(input.next_line().await.expect("reads"), Some(text("ok")));
    }
}
