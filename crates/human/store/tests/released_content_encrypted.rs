// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! `RETENTION.md` §8 on the ENCRYPTED store (ADR-0060 decision 5): released
//! content is absent from every page of the database and its write-ahead
//! log as the engine decrypts them, after a clean close and after the next
//! open following an unclean one.
//!
//! A byte search of an encrypted file finds no marker whatever it holds,
//! so it proves nothing. The search here runs over the pages DECRYPTED, by
//! this file's own reading of `SQLCipher` 4's page format -- AES-256-CBC
//! over each page but its reserved tail, the IV in that tail, the first
//! page's opening 16 bytes the salt -- under the raw key the store's key
//! file holds. The kept marker is the control: a decryption that went wrong
//! would find it nowhere, and the case fails.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use std::path::Path;

use aes::Aes256;
use aes::cipher::{BlockDecrypt as _, KeyInit as _, generic_array::GenericArray};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use interweave_human_store::key_custody::FilesystemCustody;
use interweave_human_store::store_key;
use interweave_human_store::{
    AppMessageId, HumanStore, InboundOrigin, NewInbound, STORE_FILE, StoreOptions,
};
use interweave_profile_config::TrustBoundary;
use interweave_profile_config::paths::ProfilePaths;
use interweave_transport_api::TransportIdentity;

const PEER: &str = "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN";
/// TEST-ONLY markers: long and unique, so no other byte run matches.
const RELEASED: &[u8] = b"RETENTION8-ENC-RELEASED-CONTENT-7d1b3f5a9c2e4b6d8f0a";
const KEPT: &[u8] = b"RETENTION8-ENC-KEPT-CONTROL-CONTENT-2e4c6a8f0b1d3f5b7d";

/// `SQLCipher` 4's defaults, which the store does not change.
const PAGE: usize = 4096;
const RESERVE: usize = 16 + 64; // the IV, then the HMAC-SHA512
const SALT: usize = 16;
const WAL_HEADER: usize = 32;
const FRAME_HEADER: usize = 24;

fn inbound(id: u32, payload: &[u8]) -> NewInbound {
    NewInbound {
        app_message_id: AppMessageId::parse(format!("{id:032x}")).expect("canonical id"),
        origin: InboundOrigin {
            peer: TransportIdentity::parse(PEER).expect("canonical peer"),
            endpoint: None,
            channel: None,
        },
        media_type: None,
        payload: payload.to_vec(),
        received_at: 2_000,
    }
}

fn profile() -> (tempfile::TempDir, ProfilePaths) {
    let dir = common::private_tempdir();
    let boundary = TrustBoundary::new(dir.path()).expect("a boundary");
    let paths = ProfilePaths::resolve_embedded("default", boundary).expect("paths");
    std::os::unix::fs::DirBuilderExt::mode(std::fs::DirBuilder::new().recursive(true), 0o700)
        .create(paths.human_dir())
        .expect("the human directory");
    (dir, paths)
}

fn open(paths: &ProfilePaths) -> HumanStore {
    HumanStore::open_profile(paths, &FilesystemCustody, StoreOptions::default()).expect("opens")
}

/// The raw key in the desktop's key file: its body, after the six header
/// bytes, is the key itself at the filesystem custody.
fn raw_key(paths: &ProfilePaths) -> [u8; 32] {
    let line = std::fs::read_to_string(store_key::key_path(paths)).expect("the key file");
    let record = STANDARD.decode(line.trim_end()).expect("base64");
    record[6..].try_into().expect("32 bytes")
}

/// One page decrypted: the bytes the engine reads, its reserve dropped.
fn decrypt_page(cipher: &Aes256, page: &[u8], first: bool) -> Vec<u8> {
    assert_eq!(page.len(), PAGE);
    let start = if first { SALT } else { 0 };
    let end = PAGE - RESERVE;
    let iv = &page[end..end + 16];
    let mut out = Vec::with_capacity(end - start);
    let mut previous: [u8; 16] = iv.try_into().unwrap();
    for block in page[start..end].as_chunks::<16>().0 {
        let mut plain = GenericArray::clone_from_slice(block);
        cipher.decrypt_block(&mut plain);
        for (p, c) in plain.iter_mut().zip(previous) {
            *p ^= c;
        }
        out.extend_from_slice(&plain);
        previous = *block;
    }
    out
}

/// Every page of the database and of its log, decrypted.
fn decrypted(paths: &ProfilePaths) -> (Vec<u8>, Vec<u8>) {
    let cipher = Aes256::new(GenericArray::from_slice(&raw_key(paths)));
    let db = std::fs::read(paths.human_dir().join(STORE_FILE)).expect("the database");
    let mut pages = Vec::new();
    for (i, page) in db.as_chunks::<PAGE>().0.iter().enumerate() {
        pages.extend(decrypt_page(&cipher, page, i == 0));
    }
    let mut log = Vec::new();
    let wal =
        std::fs::read(paths.human_dir().join(format!("{STORE_FILE}-wal"))).unwrap_or_default();
    if wal.len() > WAL_HEADER {
        for frame in wal[WAL_HEADER..].as_chunks::<{ FRAME_HEADER + PAGE }>().0 {
            let number = u32::from_be_bytes(frame[..4].try_into().unwrap());
            log.extend(decrypt_page(&cipher, &frame[FRAME_HEADER..], number == 1));
        }
    }
    (pages, log)
}

fn holds(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|w| w == needle)
}

fn raw_holds(dir: &Path, needle: &[u8]) -> bool {
    std::fs::read_dir(dir)
        .expect("dir")
        .flatten()
        .any(|e| std::fs::read(e.path()).is_ok_and(|bytes| holds(&bytes, needle)))
}

/// A store holding the kept marker kept and the released marker unread.
fn with_both(paths: &ProfilePaths) -> (HumanStore, interweave_human_store::RowId) {
    let mut store = open(paths);
    let kept = store
        .commit_unread_inbound(&inbound(2, KEPT))
        .expect("kept");
    let read = store.mark_read(kept, 3_000).expect("read");
    store.keep(&read, 3_001).expect("kept");
    let released = store
        .commit_unread_inbound(&inbound(1, RELEASED))
        .expect("released commits");
    (store, released)
}

#[test]
fn released_content_is_absent_from_every_decrypted_page_after_a_clean_close() {
    let (_dir, paths) = profile();
    let (mut store, row) = with_both(&paths);
    drop(store);
    let (pages, _) = decrypted(&paths);
    assert!(
        holds(&pages, RELEASED),
        "the setup: the unread message reached the database's pages"
    );
    store = open(&paths);
    store.mark_read(row, 4_000).expect("released");
    drop(store);
    let (pages, log) = decrypted(&paths);
    assert!(
        !holds(&pages, RELEASED),
        "released content left in the pages"
    );
    assert!(!holds(&log, RELEASED), "released content left in the log");
    assert!(
        holds(&pages, KEPT),
        "the control: the decryption sees kept content"
    );
    assert!(
        !raw_holds(&paths.human_dir(), KEPT),
        "the raw files hold no content in clear"
    );
}

#[test]
fn released_content_is_absent_from_every_decrypted_page_after_the_next_open_following_an_unclean_one()
 {
    let (_dir, paths) = profile();
    let (mut store, row) = with_both(&paths);
    store.mark_read(row, 4_000).expect("released");
    // Killed: no close, no checkpoint.
    std::mem::forget(store);
    let store = open(&paths);
    let (pages, log) = decrypted(&paths);
    assert!(
        !holds(&pages, RELEASED),
        "released content left in the pages"
    );
    assert!(!holds(&log, RELEASED), "released content left in the log");
    assert!(
        holds(&pages, KEPT) || holds(&log, KEPT),
        "the control: the decryption sees kept content"
    );
    drop(store);
}

/// The decryption's own control: a store that never released the marker
/// shows it in its decrypted pages, so a search that finds nothing above
/// is a finding and not a broken reader.
#[test]
fn the_decrypted_pages_show_content_never_released() {
    let (_dir, paths) = profile();
    let (store, _) = with_both(&paths);
    drop(store);
    let (pages, _) = decrypted(&paths);
    assert!(holds(&pages, RELEASED) && holds(&pages, KEPT));
    assert!(!raw_holds(&paths.human_dir(), RELEASED));
}
