// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! The WASM entry points for the playground (DESIGN.md decision 40).
//!
//! The module has no bindings library. The page writes bytes into the module's
//! memory with `alloc_bytes`, calls `run`, reads the result, and frees it with
//! `free_bytes`. `run` returns the result's address and length packed into one
//! `u64`: the address in the high 32 bits, the length in the low 32 bits.
//!
//! The result starts with `OK` or `ERROR` on its own line, then the report.
//!
//! Modes: `0` checks the stylesheet in `a`. `1` computes styles for the fixture
//! in `a`, listing changed properties. `2` lists every property.
//!
//! The engine's own code is `forbid(unsafe_code)`. These raw-pointer entry
//! points live only in this crate, which is outside `src/`.

#![allow(clippy::missing_safety_doc)]

#[path = "../../tools/fixture.rs"]
#[allow(dead_code)]
mod fixture;
#[path = "../../tools/json.rs"]
#[allow(dead_code)]
mod json;

/// Reserves `len` bytes of module memory and returns their address.
#[no_mangle]
pub extern "C" fn alloc_bytes(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len.max(1));
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

/// Frees memory from `alloc_bytes` or a result of `run`.
///
/// # Safety
/// `ptr` and `len` must come from `alloc_bytes(len)` or `run`, and be freed once.
#[no_mangle]
pub unsafe extern "C" fn free_bytes(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
}

/// Runs a mode over the input in `a` (and `b`, if non-empty, is unused by the
/// current modes). Returns the packed address and length of the result.
///
/// # Safety
/// `a_ptr` must point to `a_len` readable bytes of valid UTF-8, from `alloc_bytes`.
#[no_mangle]
pub unsafe extern "C" fn run(mode: u32, a_ptr: *const u8, a_len: usize) -> u64 {
    let input = match std::str::from_utf8(std::slice::from_raw_parts(a_ptr, a_len)) {
        Ok(s) => s,
        Err(_) => return pack("ERROR\ninput is not UTF-8"),
    };
    let text = match mode {
        0 => {
            let (lines, errors) = fixture::check_report(input);
            let head = if errors { "ERROR" } else { "OK" };
            format!("{head}\n{}", lines.join("\n"))
        }
        1 | 2 => match fixture::compute_report(input, mode == 2, &[]) {
            Ok(lines) => format!("OK\n{}", lines.join("\n")),
            Err(e) => format!("ERROR\n{e}"),
        },
        _ => "ERROR\nunknown mode".to_string(),
    };
    pack(&text)
}

fn pack(text: &str) -> u64 {
    let bytes = text.as_bytes();
    let mut owned = bytes.to_vec().into_boxed_slice();
    let ptr = owned.as_mut_ptr() as u64;
    let len = owned.len() as u64;
    std::mem::forget(owned);
    (ptr << 32) | len
}
