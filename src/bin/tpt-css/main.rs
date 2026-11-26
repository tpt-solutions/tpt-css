// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! The `tpt-css` command line (DESIGN.md decision 40).
//!
//! ```text
//! tpt-css check <file.css | ->            diagnostics for a stylesheet
//! tpt-css compute <tree.json> [--all]     computed styles for a fixture tree
//! ```
//!
//! `check` exits with 1 if any error was reported, 2 on a usage or read error.
//! `compute` prints each element's computed properties that an author style
//! changed, or every property with `--all`. The fixture format is in
//! `docs/CLI.md`. The reports are in `tools/fixture.rs`, shared with the
//! playground.

#[path = "../../../tools/fixture.rs"]
mod fixture;
#[path = "../../../tools/json.rs"]
mod json;

use std::io::Read;
use std::process::ExitCode;

const USAGE: &str = "usage:
  tpt-css check <file.css | ->
  tpt-css compute <tree.json> [--all] [--css <file.css>]...";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check") if args.len() == 2 => check(&args[1]),
        Some("compute") if args.len() >= 2 => match parse_compute_flags(&args[2..]) {
            Ok((all, sheets)) => compute(&args[1], all, &sheets),
            Err(e) => {
                eprintln!("{e}\n{USAGE}");
                ExitCode::from(2)
            }
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn read_input(path: &str) -> Result<String, String> {
    if path == "-" {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|e| format!("cannot read stdin: {e}"))?;
        Ok(text)
    } else {
        std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))
    }
}

fn check(path: &str) -> ExitCode {
    let source = match read_input(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let (lines, errors) = fixture::check_report(&source);
    for line in lines {
        println!("{line}");
    }
    if errors {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// The flags after `compute <tree.json>`: `--all`, and `--css <file>` (repeatable).
fn parse_compute_flags(flags: &[String]) -> Result<(bool, Vec<String>), String> {
    let mut all = false;
    let mut sheets = Vec::new();
    let mut i = 0;
    while i < flags.len() {
        match flags[i].as_str() {
            "--all" => all = true,
            "--css" => {
                i += 1;
                let file = flags.get(i).ok_or("--css needs a file")?;
                sheets.push(read_input(file)?);
            }
            other => return Err(format!("unknown option {other}")),
        }
        i += 1;
    }
    Ok((all, sheets))
}

fn compute(path: &str, all: bool, sheets: &[String]) -> ExitCode {
    let result = read_input(path).and_then(|text| fixture::compute_report(&text, all, sheets));
    match result {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
