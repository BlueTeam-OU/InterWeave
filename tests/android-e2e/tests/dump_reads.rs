// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! Every `dumpsys` command in this crate is built by
//! `platform::dump_command`.
//!
//! The cases read negatives off a dump ("no notice", "not focused", "not
//! running"), so a dump cut short reads as one. `platform::dump_ended`
//! refuses such a dump by the end marker `dump_command` appends, so the
//! crate's two readers, `platform::dump` and `Screens::dump` in
//! `screen.rs`, pass `dump_command`'s output through it. A case that
//! runs `dumpsys` itself (as `app_lifecycle.rs` once did) has no marker
//! and bypasses it silently, so this fails on any `"dumpsys"` command
//! word outside `dump_command`.
//!
//! What it does not catch: `dumpsys` inside a longer string (a whole
//! shell line as one argument) or built at run time; `dump_command`'s
//! output parsed without `dump_ended`; and, in `platform.rs`, anything
//! after the first `#[cfg(test)]` line, which is meant to be its test
//! module. None of these shapes is used here.

#![allow(clippy::expect_used, clippy::panic)]
#![cfg(target_os = "linux")]

use std::path::{Path, PathBuf};

/// The command word as a string literal, split so this file does not
/// carry it.
const WORD: &str = concat!("\"dump", "sys\"");

fn sources(dir: &Path, into: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a source directory") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            sources(&path, into);
        } else if path.extension().is_some_and(|e| e == "rs") {
            into.push(path);
        }
    }
}

/// Where `text` carries [`WORD`] on a code line outside `platform.rs`'s
/// `dump_command`, as `file: line`. A line whose first word is a comment
/// is prose.
fn raw_reads(file: &str, text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut in_dump_command = false;
    for (n, line) in text.lines().enumerate() {
        // platform.rs's own tests spell the words dump_command builds.
        if file == "platform.rs" && line.starts_with("#[cfg(test)]") {
            break;
        }
        if file == "platform.rs" && line.starts_with("pub fn dump_command") {
            in_dump_command = true;
        } else if in_dump_command && line == "}" {
            in_dump_command = false;
        }
        if !in_dump_command && !line.trim_start().starts_with("//") && line.contains(WORD) {
            found.push(format!("{file}: {}", n + 1));
        }
    }
    found
}

#[test]
fn every_dumpsys_command_is_built_by_dump_command() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    sources(&root.join("tests"), &mut files);
    let mut raw = Vec::new();
    let mut seen_platform = false;
    for path in files {
        let file = path
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned();
        seen_platform |= file == "platform.rs";
        let text = std::fs::read_to_string(&path).expect("a readable source");
        raw.extend(raw_reads(&file, &text));
    }
    assert!(seen_platform, "control: the walk reached src/platform.rs");
    assert!(
        raw.is_empty(),
        "dumpsys built outside platform::dump_command, so a cut dump reads as a negative: {raw:?}"
    );
}

#[test]
fn a_raw_read_is_found_and_dump_command_is_not() {
    let case = "fn f() {\n    shell(&[\"dumpsys\", \"window\"]);\n}\n";
    assert_eq!(raw_reads("app_lifecycle.rs", case), ["app_lifecycle.rs: 2"]);
    let platform = "pub fn dump_command<'a>(args: &[&'a str]) -> Vec<&'a str> {\n    let mut words = vec![\"dumpsys\"];\n    words\n}\n\nfn other() {\n    shell(&[\"dumpsys\", \"window\"]);\n}\n";
    assert_eq!(raw_reads("platform.rs", platform), ["platform.rs: 7"]);
    let commented = "// shell(&[\"dumpsys\", \"window\"]);\n";
    assert!(
        raw_reads("x.rs", commented).is_empty(),
        "prose is not a read"
    );
}
