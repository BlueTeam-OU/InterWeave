// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Every IPC byte between the daemon and its clients, recorded (S11's
//! scan of "IPC"): the daemon's socket is moved aside and a recorder
//! listens in its place, so `transportctl` and the IPC client connect
//! where they always do and are recorded without being told.
//!
//! A bound Unix socket keeps serving its listener when renamed, since
//! the listener holds the inode and not the name, so the daemon is
//! reached through the name it was moved to. Each direction of each
//! connection is its own stream: one buffer for both would interleave
//! the two, and a secret written across a chunk boundary of one could be
//! cut by the other's bytes.

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{UnixListener, UnixStream};
use tokio::task::JoinHandle;

type Streams = Arc<Mutex<Vec<Arc<Mutex<Vec<u8>>>>>>;

/// A recorder in a serving socket's place.
pub struct Tap {
    socket: PathBuf,
    real: PathBuf,
    streams: Streams,
    accept: JoinHandle<()>,
}

impl Tap {
    /// Move the socket at `socket`, which a daemon serves, aside and
    /// listen in its place, owner-only as the daemon made it.
    pub fn interpose(socket: &Path) -> Self {
        let name = socket.file_name().expect("a socket name").to_string_lossy();
        let real = socket.with_file_name(format!("{name}.tapped"));
        std::fs::rename(socket, &real).expect("the socket moved aside");
        let listener = UnixListener::bind(socket).expect("the tap listens");
        std::fs::set_permissions(socket, std::fs::Permissions::from_mode(0o600))
            .expect("the tap is owner-only");
        let streams = Streams::default();
        let accept = tokio::spawn({
            let (real, streams) = (real.clone(), streams.clone());
            async move {
                while let Ok((client, _)) = listener.accept().await {
                    let Ok(daemon) = UnixStream::connect(&real).await else {
                        continue;
                    };
                    let (up, down) = (stream(&streams), stream(&streams));
                    let (client_read, client_write) = client.into_split();
                    let (daemon_read, daemon_write) = daemon.into_split();
                    tokio::spawn(copy(client_read, daemon_write, up));
                    tokio::spawn(copy(daemon_read, client_write, down));
                }
            }
        });
        Self {
            socket: socket.to_path_buf(),
            real,
            streams,
            accept,
        }
    }

    /// Every direction of every connection so far, each as one stream.
    pub fn streams(&self) -> Vec<Vec<u8>> {
        self.streams
            .lock()
            .expect("the streams")
            .iter()
            .map(|s| s.lock().expect("a stream").clone())
            .collect()
    }

    /// Stop recording and put the daemon's socket back under its name, so
    /// the daemon's own unlink at shutdown removes it.
    pub fn restore(self) {
        self.accept.abort();
        std::fs::remove_file(&self.socket).expect("the tap removed");
        std::fs::rename(&self.real, &self.socket).expect("the socket put back");
    }
}

fn stream(streams: &Streams) -> Arc<Mutex<Vec<u8>>> {
    let one = Arc::new(Mutex::new(Vec::new()));
    streams.lock().expect("the streams").push(one.clone());
    one
}

/// `from` to `to` until `from` ends, every byte kept; the end is passed
/// on as a write shutdown, so a close reaches the other side as a close.
async fn copy(
    mut from: tokio::net::unix::OwnedReadHalf,
    mut to: tokio::net::unix::OwnedWriteHalf,
    kept: Arc<Mutex<Vec<u8>>>,
) {
    let mut chunk = [0_u8; 8192];
    loop {
        match from.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                kept.lock()
                    .expect("a stream")
                    .extend_from_slice(&chunk[..n]);
                if to.write_all(&chunk[..n]).await.is_err() {
                    break;
                }
            }
        }
    }
    let _ = to.shutdown().await;
}
