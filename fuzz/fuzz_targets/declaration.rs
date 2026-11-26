// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_css::{parse_declaration_list, ParseOptions};

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = parse_declaration_list(text, ParseOptions::default());
    }
});
