#!/usr/bin/env node
// Verifies the wasm artifact built from examples/guest.rs.
// Usage: node examples/verify-wasm.cjs <path-to-guest.wasm>
//
// Dependency-free: uses only the Node.js built-in WebAssembly API.
const fs = require('node:fs');
const assert = require('node:assert/strict');

const WIRE_SIGNATURES = [
  { name: 'score.v1', params: 3, results: 1 },
  { name: 'score.v2', params: 3, results: 1 },
  { name: 'score.manual', params: 3, results: 1 },
  { name: 'sink.v1', params: 2, results: 0 },
  { name: 'signed.v1', params: 1, results: 1 },
  { name: 'noop.v1', params: 0, results: 0 },
  { name: 'produce.v1', params: 0, results: 1 },
  { name: 'panic.decode.v1', params: 1, results: 0 },
  { name: 'panic.business.v1', params: 0, results: 0 },
  { name: 'panic.encode.v1', params: 0, results: 1 },
];

function uleb(value) {
  const bytes = [];
  do {
    const next = value & 0x7f;
    value >>>= 7;
    bytes.push(next | (value ? 0x80 : 0));
  } while (value);
  return bytes;
}

function string(value) {
  const bytes = [...Buffer.from(value)];
  return [...uleb(bytes.length), ...bytes];
}

function section(id, bytes) {
  return [id, ...uleb(bytes.length), ...bytes];
}

// Import actual wasm functions into a module whose signatures explicitly use
// i32. Instantiation checks parameter AND result types/counts: JS .length and
// numeric outputs alone cannot distinguish an i32 ABI from an f64 ABI.
function assertWireTypes(api, signatures = WIRE_SIGNATURES) {
  const types = [...uleb(signatures.length)];
  const imports = [...uleb(signatures.length)];
  for (const [index, { name, params, results }] of signatures.entries()) {
    assert.equal(typeof api[name], 'function', `missing function export: ${name}`);
    types.push(0x60, ...uleb(params), ...Array(params).fill(0x7f),
      ...uleb(results), ...Array(results).fill(0x7f));
    imports.push(...string('abi'), ...string(name), 0x00, ...uleb(index));
  }
  const harness = new WebAssembly.Module(Uint8Array.from([
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
    ...section(1, types), ...section(2, imports),
  ]));
  new WebAssembly.Instance(harness, { abi: api });
}

function verifyWasm(bytes) {
  const wasmModule = new WebAssembly.Module(bytes);
  const names = WebAssembly.Module.exports(wasmModule).map((entry) => entry.name);
  const api = new WebAssembly.Instance(wasmModule, {}).exports;
  assertWireTypes(api);
  assert.ok(!names.includes('cfg.absent.v1'), 'cfg-disabled wrapper must not be exported');
  assert.equal(api['score.v1'](2, 3, 4), 128);
  assert.equal(api['score.v2'](2, 3, 4), 128);
  assert.equal(api['sink.v1'](1, 2), undefined);
  assert.equal(
    api['score.v1'],
    api['score.manual'],
    'macro-generated and hand-written exports must be one wasm function',
  );
  assert.equal(api['signed.v1'](-1), -1);
  assert.equal(api['signed.v1'](0x8000_0000), -0x8000_0000);
  assert.equal(api['signed.v1'](0xffff_ffff), -1);
  assert.equal(api['noop.v1'](), undefined);
  assert.equal(api['produce.v1'](), -0x8000_0000);
  for (const [name, args] of [
    ['panic.decode.v1', [1]],
    ['panic.business.v1', []],
    ['panic.encode.v1', []],
  ]) {
    assert.throws(() => api[name](...args), WebAssembly.RuntimeError,
      `${name} must trap at the guest boundary`);
  }
  return { names, byteLength: bytes.byteLength };
}

module.exports = { verifyWasm, assertWireTypes };

if (require.main === module) {
  const path = process.argv[2];
  if (!path) {
    console.error('Usage: node examples/verify-wasm.cjs <path-to-guest.wasm>');
    process.exitCode = 1;
  } else {
    const { names, byteLength } = verifyWasm(fs.readFileSync(path));
    console.log(`verified ${names.length} exports in ${byteLength} bytes`);
  }
}
