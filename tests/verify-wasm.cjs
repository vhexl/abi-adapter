// Run with: node --test tests/verify-wasm.cjs
const test = require('node:test');
const assert = require('node:assert/strict');
const { verifyWasm, assertWireTypes } = require('../examples/verify-wasm.cjs');

function uleb(value) {
  const bytes = [];
  do {
    const next = value & 0x7f;
    value >>>= 7;
    bytes.push(next | (value ? 0x80 : 0));
  } while (value);
  return bytes;
}

function vector(values) {
  return [...uleb(values.length), ...values.flat()];
}

function string(value) {
  return vector([...Buffer.from(value)]);
}

function section(id, bytes) {
  return [id, ...uleb(bytes.length), ...bytes];
}

// Small independent wasm fixtures: no compiler, npm package, or existing
// build artifact is needed to exercise the verifier's rejection behavior.
function fixture(functions) {
  const exports = functions.flatMap(({ names }, index) =>
    names.map((name) => [...string(name), 0, ...uleb(index)]));
  const bodies = functions.map(({ body }) => vector([0, ...body, 0x0b]));
  return Uint8Array.from([
    0, 97, 115, 109, 1, 0, 0, 0,
    ...section(1, vector(functions.map(({ params, results }) =>
      [0x60, ...vector(params), ...vector(results)]))),
    ...section(3, vector(functions.map((_, index) => uleb(index)))),
    ...section(7, vector(exports)),
    ...section(10, vector(bodies)),
  ]);
}

const I32 = 0x7f;
const F64 = 0x7c;
const f64Constant128 = [0x44, ...Buffer.from([0, 0, 0, 0, 0, 0, 0x60, 0x40])];

function correctFunctions() {
  return [
    {
      names: ['score.v1', 'score.v2', 'score.manual'],
      params: [I32, I32, I32], results: [I32],
      // (point.x + 10 * point.y) * scale
      body: [0x20, 0, 0x20, 1, 0x41, 10, 0x6c, 0x6a, 0x20, 2, 0x6c],
    },
    { names: ['sink.v1'], params: [I32, I32], results: [], body: [] },
    { names: ['signed.v1'], params: [I32], results: [I32], body: [0x20, 0] },
    { names: ['noop.v1'], params: [], results: [], body: [] },
    { names: ['produce.v1'], params: [], results: [I32],
      body: [0x41, 0x80, 0x80, 0x80, 0x80, 0x78] },
    { names: ['panic.decode.v1'], params: [I32], results: [], body: [0] },
    { names: ['panic.business.v1'], params: [], results: [], body: [0] },
    { names: ['panic.encode.v1'], params: [], results: [I32], body: [0] },
  ];
}

test('accepts the complete i32 fixture, including high bits and traps', () => {
  const bytes = fixture(correctFunctions());
  const result = verifyWasm(bytes);
  assert.equal(result.names.length, 10);
  assert.equal(result.byteLength, bytes.byteLength);
});

for (const [name, params, results, body] of [
  ['f64 parameters', [F64, F64, F64], [I32], [0x41, 0x80, 0x01]],
  ['f64 result', [I32, I32, I32], [F64], f64Constant128],
  ['f64 parameters and result', [F64, F64, F64], [F64], f64Constant128],
  ['missing one-slot result', [I32, I32, I32], [], []],
]) {
  test(`rejects ${name} through typed wasm linking`, () => {
    const functions = correctFunctions();
    Object.assign(functions[0], { params, results, body });
    assert.throws(() => verifyWasm(fixture(functions)), WebAssembly.LinkError);
  });
}

test('rejects a result on a zero-slot export', () => {
  const functions = correctFunctions();
  Object.assign(functions[1], { results: [I32], body: [0x41, 0] });
  assert.throws(() => verifyWasm(fixture(functions)), WebAssembly.LinkError);
});

test('zero and one result widths are incompatible at wasm linking', () => {
  for (const actualWidth of [0, 1]) {
    const bytes = fixture([{
      names: ['probe'], params: [], results: Array(actualWidth).fill(I32),
      body: actualWidth ? [0x41, 0] : [],
    }]);
    const api = new WebAssembly.Instance(new WebAssembly.Module(bytes), {}).exports;
    assertWireTypes(api, [{ name: 'probe', params: 0, results: actualWidth }]);
    assert.throws(() => assertWireTypes(api,
      [{ name: 'probe', params: 0, results: 1 - actualWidth }]), WebAssembly.LinkError);
  }
});

test('rejects a missing required export', () => {
  const functions = correctFunctions();
  functions[0].names = ['score.v1', 'score.manual'];
  assert.throws(() => verifyWasm(fixture(functions)), /missing function export: score\.v2/);
});

test('rejects an exported cfg-disabled wrapper', () => {
  const functions = correctFunctions();
  functions[3].names.push('cfg.absent.v1');
  assert.throws(() => verifyWasm(fixture(functions)), /cfg-disabled wrapper must not be exported/);
});

test('rejects a panic export that returns normally', () => {
  const functions = correctFunctions();
  functions[7].body = [0x41, 0];
  assert.throws(() => verifyWasm(fixture(functions)), /panic\.encode\.v1 must trap/);
});
