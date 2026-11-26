// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
// Drives the playground module. See src/lib.rs for the calling convention.
(async () => {
  const status = document.getElementById("status");
  const out = document.getElementById("out");
  const enc = new TextEncoder();
  const dec = new TextDecoder();

  let wasm;
  try {
    const { instance } = await WebAssembly.instantiateStreaming(fetch("playground.wasm"), {});
    wasm = instance.exports;
    status.textContent = "Ready.";
  } catch (e) {
    status.textContent = "Could not load playground.wasm. Run playground/build.sh first.";
    return;
  }

  // Sends `text` to the module, runs `mode`, and shows the report.
  function run(mode, text) {
    const data = enc.encode(text);
    const inPtr = wasm.alloc_bytes(data.length);
    new Uint8Array(wasm.memory.buffer, inPtr, data.length).set(data);
    const packed = wasm.run(mode, inPtr, data.length);
    const outPtr = Number(packed >> 32n);
    const outLen = Number(packed & 0xffffffffn);
    const report = dec.decode(new Uint8Array(wasm.memory.buffer, outPtr, outLen));
    wasm.free_bytes(outPtr, outLen);
    wasm.free_bytes(inPtr, data.length);
    out.textContent = report;
    status.textContent = report.startsWith("ERROR") ? "Reported errors." : "Done.";
  }

  document.getElementById("check").onclick = () =>
    run(0, document.getElementById("css").value);
  document.getElementById("compute").onclick = () =>
    run(1, document.getElementById("tree").value);
  document.getElementById("all").onclick = () =>
    run(2, document.getElementById("tree").value);
})();
