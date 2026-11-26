// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_css::{parse_stylesheet, ParseOptions};

// The input is a selector prelude, parsed as the prelude of a style rule.
fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = parse_stylesheet(&format!("{text} {{ }}"), ParseOptions::default());
    }
});
