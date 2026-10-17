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
