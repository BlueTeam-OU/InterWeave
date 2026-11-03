// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Andrea Benetton
//! The views compile without a single diagnostic.
//!
//! The build compiles `ui/app.slint` too, but a warning there is printed
//! and the build goes on, and Slint 1.18 warns of a binding loop that it
//! "may cause panic at runtime". One such loop -- the panes' widths
//! following `narrow`, which reads the window's width, which Slint took
//! from the panes -- held a desktop window wider than 600 px whatever it
//! was resized to. So the views are compiled again here, as the build
//! does, and any warning fails.

#![allow(clippy::expect_used)]

use std::path::Path;

use i_slint_compiler::CompilerConfiguration;
use i_slint_compiler::diagnostics::BuildDiagnostics;
use i_slint_compiler::generator::OutputFormat;

#[test]
fn the_views_compile_without_a_warning() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/app.slint");
    let mut diag = BuildDiagnostics::default();
    let syntax = i_slint_compiler::parser::parse_file(&path, &mut diag);
    assert!(!diag.has_errors(), "{}", diag.to_string_vec().join("\n"));
    let syntax = syntax.expect("parsed");
    let config = CompilerConfiguration::new(OutputFormat::Llr);
    let (_, diag, _) =
        spin_on::spin_on(i_slint_compiler::compile_syntax_node(syntax, diag, config));
    let all = diag.to_string_vec();
    assert!(
        all.is_empty(),
        "the views compile with diagnostics:\n{}",
        all.join("\n")
    );
}

/// Every wrapped `Text` in the views carries a height of its own. Slint
/// 1.18 measures a word- or char-wrapped Text under a conditional or
/// repeated layout as if it did not wrap, a line short per wrapped line,
/// so a container sized from layout info ends before what it draws: the
/// card page's scroll stopped above Make a new key on a phone (j54's device
/// run on the Galaxy A40, 2026-11-03). `min-height: self.preferred-height`
/// is the cure, since a Text's own preferred height comes from the width
/// it is drawn at. Held for every one rather than for those under a
/// condition today, since a Text moves under one in a later change without
/// anyone looking at its height. Mutation: the line taken off any one.
#[test]
fn every_wrapped_text_has_a_height_of_its_own() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/app.slint");
    let source = std::fs::read_to_string(&path).expect("app.slint");
    let lines: Vec<&str> = source.lines().collect();
    let mut missing = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let wraps = line.trim_start().starts_with("wrap:")
            && (line.contains("word-wrap") || line.contains("char-wrap"));
        if !wraps {
            continue;
        }
        let Some(open) = element_start(&lines, at) else {
            missing.push(format!("{}: no element opens above it", at + 1));
            continue;
        };
        if element_name(lines[open]) != Some("Text") {
            continue;
        }
        let own = direct_bindings(&lines, open);
        if !own.iter().any(|b| *b == "min-height" || *b == "height") {
            missing.push(format!("{}: {}", open + 1, lines[open].trim()));
        }
    }
    assert!(
        missing.is_empty(),
        "wrapped Text without min-height: self.preferred-height:\n{}",
        missing.join("\n")
    );
}

/// The line that opens the element holding line `at`: walking up, the
/// first `{` not closed in between.
fn element_start(lines: &[&str], at: usize) -> Option<usize> {
    let mut depth = 0_i32;
    for index in (0..at).rev() {
        let line = lines[index];
        depth += braces(line, '}') - braces(line, '{');
        if depth < 0 {
            return Some(index);
        }
    }
    None
}

/// The element type an opening line names: `Text {`, `name := Text {`,
/// `if cond: Text {`, `for x in xs: Text {`.
fn element_name(line: &str) -> Option<&str> {
    let head = line.trim().strip_suffix('{')?.trim_end();
    let head = head.rsplit(":=").next()?.trim();
    let head = head.rsplit(": ").next()?.trim();
    head.split_whitespace().last()
}

/// The names of the bindings written directly in the element opening at
/// `open`, not in its children.
fn direct_bindings<'a>(lines: &[&'a str], open: usize) -> Vec<&'a str> {
    let mut depth = 0_i32;
    let mut names = Vec::new();
    for line in &lines[open + 1..] {
        if depth == 0
            && let Some((name, _)) = line.trim().split_once(':')
        {
            names.push(name.trim());
        }
        depth += braces(line, '{') - braces(line, '}');
        if depth < 0 {
            break;
        }
    }
    names
}

/// How many of `brace` the line's code holds, its comment left out.
fn braces(line: &str, brace: char) -> i32 {
    let code = line.split("//").next().unwrap_or_default();
    i32::try_from(code.matches(brace).count()).unwrap_or(i32::MAX)
}
