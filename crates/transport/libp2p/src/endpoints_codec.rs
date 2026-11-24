// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The `/interweave/endpoints/2.0.0` request-response codec (ADR-0031).
//!
//! The bytes are decided in `architecture/transport/libp2p/ENDPOINTS.md`
//! and pinned by `fixtures/endpoints/endpoint-directory-v1-frame.json`;
//! this module moves them across a substream with a bound on how many
//! it will read, exactly as `direct_codec` does for direct v2.
//!
//! # Both reads are bounded before they allocate
//!
//! A request is ONE byte and a response is at most 1710, the largest
//! frame that decodes (32 derived entries), and each read `.take()`s its
//! ceiling plus one — the extra byte being how "exactly at the limit" is
//! told from "over it". A peer that streams forever is
//! cut off at a length this profile chose.
//!
//! # Grammar is refused here, not later
//!
//! Each entry is decoded by [`EndpointId::parse_derived`]: on the wire an
//! entry is a derived id (ADR-0067),
//! 52 bytes (`ENDPOINT_FIELD_BYTES`), and a label of the retired grammar is
//! malformed metadata. So a response carrying a label, or an entry whose
//! length byte is not 52, fails to DECODE, and request-response reports that to the requester as an
//! `Io(InvalidData)` the runtime reads as `ProtocolViolation`
//! (`an_entry_that_is_not_a_derived_id_does_not_decode`). The count and
//! duplicate rules are the runtime's (`transport-runtime::directory`) because they are
//! about the list, not the bytes — and the codec refuses a count above
//! 32 only because such a frame cannot fit the response ceiling.

use std::io;

use futures::{AsyncReadExt as _, AsyncWriteExt as _};
use libp2p::StreamProtocol;
use libp2p::request_response::Codec;

use interweave_transport_api::{
    DirectoryRefusal, ENDPOINT_FIELD_BYTES, EndpointDirectoryV1, EndpointId, MAX_DIRECTORY_ENTRIES,
};

/// The protocol this codec speaks (ADR-0031). Versioned in the string,
/// so an incompatible framing is a NEW id and `UnsupportedProtocols`
/// stays a usable major-version signal.
pub const ENDPOINTS_PROTOCOL: StreamProtocol = StreamProtocol::new("/interweave/endpoints/2.0.0");

/// The whole request: one tag byte. Not empty, because an empty request
/// is indistinguishable from a closed stream.
pub const MAX_REQUEST_BYTES: usize = 1;

/// The read bound of a response: tag, `generated_at_ms`, `ttl_ms`, count,
/// and 32 entries each a length byte and a derived id. 1710, which is also
/// the largest frame that decodes: the fixture's `largest-32-entries`
/// (`the_largest_legal_frame_is_the_read_ceiling`).
pub const MAX_RESPONSE_BYTES: usize =
    1 + 8 + 4 + 1 + MAX_DIRECTORY_ENTRIES * (1 + ENDPOINT_FIELD_BYTES);

const TAG_REQUEST: u8 = 0x01;
const TAG_DIRECTORY: u8 = 0x01;
const TAG_REFUSED: u8 = 0x02;

/// The request. Carries nothing; the peer's authenticated identity is
/// the whole of the question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ListEndpointsV1;

/// The answer: a directory, or a coarse refusal carrying no list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryResponse {
    /// The advertised endpoints, unvalidated on the way in.
    Directory(EndpointDirectoryV1),
    /// Refused; nothing about any endpoint is disclosed.
    Refused(DirectoryRefusal),
}

/// Wire codes, hand-assigned so a reordering of the enum cannot renumber
/// the wire.
const fn reason_code(reason: DirectoryRefusal) -> u8 {
    match reason {
        DirectoryRefusal::Overloaded => 1,
        DirectoryRefusal::Unauthorized => 2,
        DirectoryRefusal::Unavailable => 3,
    }
}

/// The inverse, strict: only the three assigned codes decode. An
/// unassigned value (0, 4, 200…) is malformed protocol metadata, not a
/// fourth refusal, so it is a decode error rather than being coerced to
/// `Unavailable` — the same stance the direct-v2 decoder takes on an
/// unassigned reason code, and the reason a masked interoperability fault
/// does not read to the caller as an ordinary refusal.
const fn reason_from_code(code: u8) -> Result<DirectoryRefusal, &'static str> {
    match code {
        1 => Ok(DirectoryRefusal::Overloaded),
        2 => Ok(DirectoryRefusal::Unauthorized),
        3 => Ok(DirectoryRefusal::Unavailable),
        _ => Err("unassigned directory refusal code"),
    }
}

/// Encode the request.
#[must_use]
pub const fn encode_request() -> [u8; 1] {
    [TAG_REQUEST]
}

/// Decode a request.
///
/// # Errors
/// Anything but the single tag byte.
pub fn decode_request(bytes: &[u8]) -> Result<ListEndpointsV1, &'static str> {
    match bytes {
        [TAG_REQUEST] => Ok(ListEndpointsV1),
        [] => Err("an empty request is a closed stream, not a query"),
        _ => Err("a request is exactly one tag byte"),
    }
}

/// Encode a response, per the frozen layout.
#[must_use]
pub fn encode_response(response: &DirectoryResponse) -> Vec<u8> {
    match response {
        DirectoryResponse::Directory(directory) => {
            let mut out = Vec::with_capacity(MAX_RESPONSE_BYTES);
            out.push(TAG_DIRECTORY);
            out.extend_from_slice(&directory.generated_at_ms.to_be_bytes());
            out.extend_from_slice(&directory.ttl_ms.to_be_bytes());
            // A count above 32 cannot be encoded in a legal frame; the
            // responder never builds one (`advertised_for` caps at the
            // wire bound), so this truncation is unreachable and exists
            // so the encoder cannot produce bytes the decoder refuses.
            let entries = directory
                .endpoints
                .iter()
                .take(MAX_DIRECTORY_ENTRIES)
                .collect::<Vec<_>>();
            out.push(u8::try_from(entries.len()).unwrap_or(u8::MAX));
            for endpoint in entries {
                let label = endpoint.as_str().as_bytes();
                out.push(u8::try_from(label.len()).unwrap_or(u8::MAX));
                out.extend_from_slice(label);
            }
            out
        }
        DirectoryResponse::Refused(reason) => vec![TAG_REFUSED, reason_code(*reason)],
    }
}

/// Decode a response.
///
/// # Errors
/// A static description of the first thing wrong: unknown tag, short
/// frame, an entry whose length byte is not 52 or that is not a derived
/// `EndpointId` (a label of the retired grammar among them), more than 32 entries, or trailing bytes.
pub fn decode_response(bytes: &[u8]) -> Result<DirectoryResponse, &'static str> {
    let (&tag, rest) = bytes.split_first().ok_or("empty response")?;
    match tag {
        TAG_REFUSED => match rest {
            [code] => reason_from_code(*code).map(DirectoryResponse::Refused),
            _ => Err("a refusal is a tag and one reason byte"),
        },
        TAG_DIRECTORY => {
            let (generated, rest) = rest.split_at_checked(8).ok_or("short generated_at_ms")?;
            let (ttl, rest) = rest.split_at_checked(4).ok_or("short ttl_ms")?;
            let (&count, mut rest) = rest.split_first().ok_or("short count")?;
            let count = usize::from(count);
            if count > MAX_DIRECTORY_ENTRIES {
                return Err("more entries than the frame may carry");
            }
            let mut endpoints = Vec::with_capacity(count);
            for _ in 0..count {
                let (&len, tail) = rest.split_first().ok_or("short entry length")?;
                if usize::from(len) != ENDPOINT_FIELD_BYTES {
                    return Err("entry length is not that of a derived id");
                }
                let (label, tail) = tail
                    .split_at_checked(usize::from(len))
                    .ok_or("short entry")?;
                let label = std::str::from_utf8(label).map_err(|_| "entry is not ASCII")?;
                endpoints.push(
                    EndpointId::parse_derived(label).map_err(|_| "entry outside the grammar")?,
                );
                rest = tail;
            }
            if !rest.is_empty() {
                return Err("trailing bytes after the last entry");
            }
            let generated_at_ms = u64::from_be_bytes(generated.try_into().map_err(|_| "width")?);
            let ttl_ms = u32::from_be_bytes(ttl.try_into().map_err(|_| "width")?);
            Ok(DirectoryResponse::Directory(EndpointDirectoryV1 {
                generated_at_ms,
                ttl_ms,
                endpoints,
            }))
        }
        _ => Err("unknown response tag"),
    }
}

/// Read at most `limit` bytes, refusing a stream that carries more.
async fn read_bounded<T>(io: &mut T, limit: usize) -> io::Result<Vec<u8>>
where
    T: futures::AsyncRead + Unpin + Send,
{
    let mut buffer = Vec::new();
    io.take(limit as u64 + 1).read_to_end(&mut buffer).await?;
    if buffer.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame exceeds the protocol ceiling",
        ));
    }
    Ok(buffer)
}

/// The codec. Stateless.
#[derive(Debug, Clone, Copy, Default)]
pub struct EndpointsCodec;

// NO `#[async_trait]` SINCE `libp2p-request-response` 0.30 (the 0.57
// bump): the trait declares each method as returning
// `impl Future<Output = ..> + Send`, which a plain `async fn` satisfies
// natively. Keeping the macro is what produced E0195 -- it rewrites the
// signature to a boxed future with its own lifetimes, which no longer
// match the trait's.
impl Codec for EndpointsCodec {
    type Protocol = StreamProtocol;
    type Request = ListEndpointsV1;
    type Response = DirectoryResponse;

    async fn read_request<T>(&mut self, _: &StreamProtocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: futures::AsyncRead + Unpin + Send,
    {
        let bytes = read_bounded(io, MAX_REQUEST_BYTES).await?;
        decode_request(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _: &StreamProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: futures::AsyncRead + Unpin + Send,
    {
        let bytes = read_bounded(io, MAX_RESPONSE_BYTES).await?;
        crate::direct_codec::refuse_an_unanswered_exchange(&bytes)?;
        decode_response(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(
        &mut self,
        _: &StreamProtocol,
        io: &mut T,
        _: Self::Request,
    ) -> io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        io.write_all(&encode_request()).await?;
        io.close().await
    }

    async fn write_response<T>(
        &mut self,
        _: &StreamProtocol,
        io: &mut T,
        response: Self::Response,
    ) -> io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        io.write_all(&encode_response(&response)).await?;
        io.close().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use interweave_test_support::endpoint_keys::endpoint_id;

    fn unhex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
            .collect()
    }

    /// The frozen frames, byte for byte. Read from the fixture file so
    /// this test cannot agree with the encoder for free.
    fn fixture() -> serde_json::Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/endpoints/endpoint-directory-v1-frame.json");
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture present"))
            .expect("fixture parses")
    }

    #[test]
    fn every_frozen_frame_round_trips() {
        let doc = fixture();
        let mut seen = 0;
        for v in doc["vectors"].as_array().expect("vectors") {
            let frame = unhex(v["frame_hex"].as_str().expect("hex"));
            match v["kind"].as_str().expect("kind") {
                "request" => {
                    assert_eq!(encode_request().to_vec(), frame);
                    assert_eq!(decode_request(&frame), Ok(ListEndpointsV1));
                }
                "refused" => {
                    let decoded = decode_response(&frame).expect("decodes");
                    assert_eq!(encode_response(&decoded), frame);
                }
                "directory" => {
                    let decoded = decode_response(&frame).expect("decodes");
                    let DirectoryResponse::Directory(d) = &decoded else {
                        panic!("a directory");
                    };
                    let names: Vec<&str> = d.endpoints.iter().map(EndpointId::as_str).collect();
                    let expected: Vec<&str> = v["endpoints"]
                        .as_array()
                        .expect("list")
                        .iter()
                        .map(|e| e.as_str().expect("str"))
                        .collect();
                    assert_eq!(names, expected);
                    // Each entry is the TEST-ONLY id its named key derives,
                    // so the fixture, the Python verifier and this crate's
                    // key rule agree on one id per name.
                    let keys: Vec<EndpointId> = v["endpoints_test_keys"]
                        .as_array()
                        .map(|keys| {
                            keys.iter()
                                .map(|k| endpoint_id(k.as_str().expect("a key name")))
                                .collect()
                        })
                        .unwrap_or_default();
                    assert_eq!(d.endpoints, keys, "{}", v["name"]);
                    assert_eq!(encode_response(&decoded), frame);
                    assert!(frame.len() <= MAX_RESPONSE_BYTES);
                }
                other => panic!("unknown kind {other}"),
            }
            seen += 1;
        }
        assert_eq!(seen, 5, "every vector in the file was exercised");
    }

    /// The largest legal frame IS the read ceiling: 32 derived entries,
    /// 1710 bytes. A frame one byte longer is refused by the bound itself,
    /// through the codec's own `read_response`.
    #[test]
    fn the_largest_legal_frame_is_the_read_ceiling() {
        let doc = fixture();
        let largest = doc["vectors"]
            .as_array()
            .expect("vectors")
            .iter()
            .find(|v| v["name"] == "largest-32-entries")
            .expect("the largest vector");
        let frame = unhex(largest["frame_hex"].as_str().expect("hex"));
        assert_eq!(largest["frame_len"].as_u64(), Some(1710));
        assert_eq!(frame.len(), MAX_RESPONSE_BYTES);
        assert_eq!(MAX_RESPONSE_BYTES, 1710);
        assert!(decode_response(&frame).is_ok());

        let read = |bytes: Vec<u8>| {
            futures::executor::block_on(async move {
                EndpointsCodec
                    .read_response(&ENDPOINTS_PROTOCOL, &mut futures::io::Cursor::new(bytes))
                    .await
            })
        };
        assert!(read(frame.clone()).is_ok(), "the ceiling passes the bound");
        let mut over = frame;
        over.push(0);
        let err = read(over).expect_err("one byte past the ceiling");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("ceiling"), "{err}");
    }

    /// An entry whose length byte is not 52 is malformed whatever it
    /// holds: 51 and 53 bytes, and the retired 64-byte label.
    #[test]
    fn an_entry_of_any_length_but_52_does_not_decode() {
        let entry = |len: usize| {
            let mut frame = vec![TAG_DIRECTORY];
            frame.extend_from_slice(&0u64.to_be_bytes());
            frame.extend_from_slice(&0u32.to_be_bytes());
            frame.push(1);
            frame.push(u8::try_from(len).expect("short"));
            frame.extend_from_slice(&vec![b'a'; len]);
            frame
        };
        for len in [0, 1, 51, 53, 64] {
            assert_eq!(
                decode_response(&entry(len)),
                Err("entry length is not that of a derived id"),
                "{len}"
            );
        }
        // The control: 52 bytes reach the grammar check, and fail there.
        assert_eq!(
            decode_response(&entry(52)),
            Err("entry outside the grammar")
        );
    }

    /// A derived id one character off, and a label of the retired grammar,
    /// both fail to decode.
    #[test]
    fn an_entry_that_is_not_a_derived_id_does_not_decode() {
        let id = endpoint_id("claude");
        let frame = encode_response(&DirectoryResponse::Directory(EndpointDirectoryV1 {
            generated_at_ms: 0,
            ttl_ms: 0,
            endpoints: vec![id.clone()],
        }));
        assert_eq!(
            decode_response(&frame),
            Ok(DirectoryResponse::Directory(EndpointDirectoryV1 {
                generated_at_ms: 0,
                ttl_ms: 0,
                endpoints: vec![id],
            })),
            "the control decodes"
        );
        // `12D3KooW` read as `12D3KooX`: 52 characters still, and the
        // prefix bytes no longer an Ed25519 key's. (A change in the key's
        // own characters would name another key, as legal as the first.)
        let mut altered = frame;
        let w = 1 + 8 + 4 + 1 + 1 + 7;
        assert_eq!(altered[w], b'W');
        altered[w] = b'X';
        assert_eq!(decode_response(&altered), Err("entry outside the grammar"));

        // A label cannot be built as an `EndpointId`: the bytes are the
        // peer's.
        let label = "human";
        let mut labelled = vec![TAG_DIRECTORY];
        labelled.extend_from_slice(&0u64.to_be_bytes());
        labelled.extend_from_slice(&0u32.to_be_bytes());
        labelled.push(1);
        labelled.push(u8::try_from(label.len()).expect("short"));
        labelled.extend_from_slice(label.as_bytes());
        // 5 bytes: refused by its length before the grammar is asked.
        assert_eq!(
            decode_response(&labelled),
            Err("entry length is not that of a derived id")
        );
    }

    #[test]
    fn thirty_three_entries_do_not_decode() {
        let mut frame = vec![TAG_DIRECTORY];
        frame.extend_from_slice(&0u64.to_be_bytes());
        frame.extend_from_slice(&0u32.to_be_bytes());
        frame.push(33);
        for i in 0..33u8 {
            frame.push(3);
            frame.extend_from_slice(format!("e{i:02}").as_bytes());
        }
        assert_eq!(
            decode_response(&frame),
            Err("more entries than the frame may carry")
        );
    }

    #[test]
    fn trailing_bytes_and_bad_tags_are_refused() {
        let mut ok = encode_response(&DirectoryResponse::Refused(DirectoryRefusal::Overloaded));
        ok.push(0);
        assert!(decode_response(&ok).is_err());
        assert_eq!(decode_response(&[0x09]), Err("unknown response tag"));
        assert_eq!(
            decode_request(&[]),
            Err("an empty request is a closed stream, not a query")
        );
        assert!(decode_request(&[TAG_REQUEST, 0]).is_err());
    }

    #[test]
    fn an_unassigned_refusal_code_does_not_decode() {
        // 3 is Unavailable; 0, 4, 200 are unassigned and rejected rather
        // than coerced to a valid reason.
        assert_eq!(
            decode_response(&[TAG_REFUSED, 3]),
            Ok(DirectoryResponse::Refused(DirectoryRefusal::Unavailable))
        );
        for bad in [0u8, 4, 200] {
            assert_eq!(
                decode_response(&[TAG_REFUSED, bad]),
                Err("unassigned directory refusal code")
            );
        }
    }
}
