// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! What a profile's secret looks like in a stream it should never reach
//! (S11; threat-model.md's "Private-key theft", "Recovery-phrase theft"
//! and "Log leakage" rows).
//!
//! The recovery phrase IS the key: its 32 bytes of entropy are the
//! Ed25519 seed (`ProfileIdentity::from_phrase`), so a stream carrying
//! the seed in any encoding carries the phrase, and the scan looks for
//! both. Every form is searched as a PART of the secret rather than the
//! whole, a window of it at every start, so a line cut short or a value
//! printed from an offset is still found: any [`RAW`] bytes of the seed
//! raw or in hex, any [`LISTED`] as a derived `Debug`, and any
//! [`BASE64`] + 2 in base64 or base64url wherever the encoded value
//! begins. Each form has a positive control below, cut at both ends,
//! since a scan that finds nothing because it cannot find anything
//! passes every stream.

use interweave_profile_identity::ProfileIdentity;

/// How many consecutive phrase words count as the phrase. One word is
/// ordinary English (`able`, `about`, `act` are BIP-39 words), and three
/// in the phrase's own order is not something a log line says by chance.
const RUN: usize = 3;

/// The window, in bytes, of the raw and hex forms: 12 bytes of an
/// Ed25519 seed are 96 bits no stream carries by chance.
const RAW: usize = 12;

/// The window of a derived `Debug` listing.
const LISTED: usize = 8;

/// The window of the base64 forms, a whole number of 3-byte groups: an
/// encoded slice of `BASE64 + 2` bytes holds a group boundary within its
/// first three, and from there `BASE64` bytes encode to characters that
/// depend on nothing else.
const BASE64: usize = 18;

/// One identity's secret, in every form the scan looks for.
pub struct Secret {
    words: Vec<String>,
    needles: Vec<(String, Vec<u8>)>,
}

impl Secret {
    /// The secret `identity` holds.
    pub fn of(identity: &ProfileIdentity) -> Self {
        let phrase = identity.recovery_phrase().expect("a phrase");
        let seed = phrase.expose_entropy().expect("the entropy");
        let words = phrase
            .expose_words()
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        Self {
            words,
            needles: needles(&seed),
        }
    }

    /// The phrase as `transportctl` reads it from stdin.
    pub fn phrase(&self) -> String {
        self.words.join(" ")
    }

    /// The phrase's words.
    pub fn words(&self) -> &[String] {
        &self.words
    }

    /// The form each part of the secret `stream` carries is in, named; an
    /// empty list is a clean stream.
    pub fn found_in(&self, stream: &[u8]) -> Vec<String> {
        let mut found: Vec<String> = self
            .needles
            .iter()
            .filter(|(_, needle)| contains(stream, needle))
            .map(|(form, _)| form.clone())
            .collect();
        if let Some(at) = self.phrase_run(stream) {
            found.push(format!("{RUN} phrase words in order from word {at}"));
        }
        found
    }

    /// Where a run of [`RUN`] consecutive phrase words starts, if `stream`
    /// holds one: words are its alphabetic runs, case folded, so a JSON
    /// array, a quoted list and capitals all match.
    fn phrase_run(&self, stream: &[u8]) -> Option<usize> {
        let text = String::from_utf8_lossy(stream).to_lowercase();
        let tokens: Vec<&str> = text
            .split(|c: char| !c.is_ascii_alphabetic())
            .filter(|t| !t.is_empty())
            .collect();
        let phrase: Vec<&str> = self.words.iter().map(String::as_str).collect();
        tokens.windows(RUN).find_map(|window| {
            phrase
                .windows(RUN)
                .position(|run| run == window)
                .map(|at| at + 1)
        })
    }
}

/// Every needle of `seed`: each a window of it in one encoding.
fn needles(seed: &[u8; 32]) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for at in 0..=seed.len() - RAW {
        let window = &seed[at..at + RAW];
        out.push((format!("raw bytes from byte {at}"), window.to_vec()));
        let lower = hex(window);
        out.push((
            format!("lower hex from byte {at}"),
            lower.clone().into_bytes(),
        ));
        out.push((
            format!("upper hex from byte {at}"),
            lower.to_uppercase().into_bytes(),
        ));
    }
    for at in 0..=seed.len() - BASE64 {
        let window = &seed[at..at + BASE64];
        out.push((
            format!("base64 from byte {at}"),
            base64(window, STANDARD).into_bytes(),
        ));
        out.push((
            format!("base64url from byte {at}"),
            base64(window, URL_SAFE).into_bytes(),
        ));
    }
    // A derived `Debug` of a byte array or a `Vec<u8>`: decimals, or hex
    // under `{:x?}` and `{:02x?}`, each byte apart.
    for at in 0..=seed.len() - LISTED {
        let window = &seed[at..at + LISTED];
        let listed =
            |byte: fn(&u8) -> String| window.iter().map(byte).collect::<Vec<_>>().join(", ");
        out.push((
            format!("decimal from byte {at}"),
            listed(u8::to_string).into_bytes(),
        ));
        out.push((
            format!("listed hex from byte {at}"),
            listed(|b| format!("{b:x}")).into_bytes(),
        ));
        out.push((
            format!("listed padded hex from byte {at}"),
            listed(|b| format!("{b:02x}")).into_bytes(),
        ));
    }
    out
}

/// Lower-case hex of `bytes`.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

fn contains(stream: &[u8], needle: &[u8]) -> bool {
    stream.windows(needle.len()).any(|w| w == needle)
}

const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_SAFE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Unpadded base64 of `bytes`, whose length is a multiple of three. The
/// crate carries no base64 dependency, and this is the whole of it.
fn base64(bytes: &[u8], alphabet: &[u8; 64]) -> String {
    assert!(bytes.len().is_multiple_of(3), "a whole number of groups");
    let mut out = String::new();
    for group in bytes.chunks(3) {
        let n = u32::from(group[0]) << 16 | u32::from(group[1]) << 8 | u32::from(group[2]);
        for shift in [18, 12, 6, 0] {
            out.push(char::from(alphabet[(n >> shift & 63) as usize]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> (ProfileIdentity, Secret, [u8; 32]) {
        let identity = ProfileIdentity::generate();
        let secret = Secret::of(&identity);
        let seed = identity
            .recovery_phrase()
            .expect("a phrase")
            .expose_entropy()
            .expect("entropy");
        (identity, secret, seed)
    }

    /// The encoder against RFC 4648's own vector, so the base64 needles
    /// are what a real encoder writes.
    #[test]
    fn the_encoder_writes_rfc_4648() {
        assert_eq!(base64(b"foobar", STANDARD), "Zm9vYmFy");
        assert_eq!(base64(&[0xfb, 0xff, 0xbf], STANDARD), "+/+/");
        assert_eq!(base64(&[0xfb, 0xff, 0xbf], URL_SAFE), "-_-_");
    }

    /// THE POSITIVE CONTROLS: each form a stream could carry the secret
    /// in is found, embedded in other text. A scan that cannot find a
    /// form passes every stream that carries it.
    #[test]
    fn every_form_of_the_secret_is_found() {
        let (identity, secret, seed) = identity();
        let words = secret.words().to_vec();
        let key_file = {
            // 0700 whatever the umask: `save` refuses a wider directory.
            let dir = tempfile::Builder::new()
                .permissions(
                    <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o700),
                )
                .tempdir()
                .expect("tempdir");
            let path = dir.path().join("key");
            identity.save(&path).expect("saved");
            std::fs::read(&path).expect("read")
        };
        let seed_hex = hex(&seed);
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("the phrase as typed", secret.phrase().into_bytes()),
            (
                "the phrase as a JSON array",
                serde_json::to_vec(&serde_json::json!({ "words": words })).expect("json"),
            ),
            (
                "three words in capitals",
                format!("got {} then", words[5..8].join(" ").to_uppercase()).into_bytes(),
            ),
            ("the seed raw", [b"x".as_slice(), &seed, b"y"].concat()),
            ("12 raw bytes from byte 13", seed[13..25].to_vec()),
            ("the seed's tail raw", seed[20..].to_vec()),
            (
                "12 bytes in hex, cut at both ends",
                hex(&seed[7..19]).into_bytes(),
            ),
            ("hex of bytes 4..28", hex(&seed[4..28]).into_bytes()),
            ("the key file", key_file.clone()),
            ("the seed in hex", format!("seed={seed_hex}").into_bytes()),
            (
                "half the seed in upper hex",
                seed_hex[..32].to_uppercase().into_bytes(),
            ),
            (
                "the key file in base64",
                base64_padded_to_group(&key_file, STANDARD).into_bytes(),
            ),
            (
                "the key file in base64url",
                base64_padded_to_group(&key_file, URL_SAFE).into_bytes(),
            ),
            (
                "base64 of the seed from byte 3",
                base64_padded_to_group(&seed[3..], STANDARD).into_bytes(),
            ),
            (
                "base64url of 20 bytes, cut at both ends",
                base64_padded_to_group(&seed[5..25], URL_SAFE).into_bytes(),
            ),
            (
                "base64 of the seed cut short at byte 26",
                base64_padded_to_group(&seed[..26], STANDARD).into_bytes(),
            ),
            (
                "the seed as a derived Debug",
                format!("Secret({seed:?})").into_bytes(),
            ),
            (
                "the seed from an offset",
                format!("{:?}", &seed[9..30]).into_bytes(),
            ),
            (
                "8 bytes listed, cut at both ends",
                format!("{:02x?}", &seed[11..19]).into_bytes(),
            ),
            (
                "the key file as {:x?}",
                format!("{key_file:x?}").into_bytes(),
            ),
            (
                "the key file as {:02x?}",
                format!("{key_file:02x?}").into_bytes(),
            ),
        ];
        for (what, stream) in cases {
            assert!(!secret.found_in(&stream).is_empty(), "{what} is not found");
        }
    }

    /// And the negative beside it: what a clean stream carries -- the
    /// `PeerId`, two phrase words, another identity's secret -- is not
    /// taken for it.
    #[test]
    fn a_clean_stream_is_clean() {
        let (identity, secret, _) = identity();
        let words = secret.words();
        let peer = identity.transport_identity().expect("a peer id");
        let (_, other, _) = super::tests::identity();
        let stream = format!(
            "peer {} said {} {} and {}",
            peer.as_str(),
            words[0],
            words[1],
            other.phrase()
        );
        assert_eq!(secret.found_in(stream.as_bytes()), Vec::<String>::new());
    }

    /// A whole value in base64 whatever its length, zero-padded to a
    /// group: only its last group's characters differ from an encoder's.
    fn base64_padded_to_group(bytes: &[u8], alphabet: &[u8; 64]) -> String {
        let mut padded = bytes.to_vec();
        while !padded.len().is_multiple_of(3) {
            padded.push(0);
        }
        base64(&padded, alphabet)
    }
}
