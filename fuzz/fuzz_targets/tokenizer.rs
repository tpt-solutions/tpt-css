// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_css::syntax::{tokenize, SourceId};
use tpt_css::ParseLimits;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = tokenize(SourceId(0), text, &ParseLimits::default());
    }
});
