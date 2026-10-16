// Run with: node --test tests/doc-buffer.cjs
// Requires Rust 1.85+ and its installed wasm32-unknown-unknown target.
// Compile the complete RET_BUF example from docs/buffers.md and check
// that invalid addresses leave both the reserved and other buffers unchanged.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { execFileSync } = require('node:child_process');

test('documented guest writes only its reserved buffer', () => {
  const root = path.resolve(__dirname, '..');
  const document = fs.readFileSync(path.join(root, 'docs/buffers.md'), 'utf8');
  const snippets = [...document.matchAll(/^```rust[^\r\n]*\r?\n([\s\S]*?)^```[ \t]*\r?$/gm)]
    .map((match) => match[1])
    .filter((source) => /\bstatic mut RET_BUF\b/.test(source));
  assert.equal(snippets.length, 1, 'find exactly one reserved-buffer Rust example');

  const temporaryRoot = path.resolve(os.tmpdir());
  const directory = fs.mkdtempSync(path.join(temporaryRoot, 'abi-adapter-doc-buffer-'));
  try {
    const library = path.join(directory, 'libabi_adapter.rlib');
    const source = path.join(directory, 'doc_buffer.rs');
    const wasm = path.join(directory, 'doc_buffer.wasm');
    // A separately allocated, writable buffer proves that an otherwise valid
    // address is still rejected when it does not identify the reserved buffer.
    fs.writeFileSync(source, snippets[0] + `
static mut FOREIGN_BUF: [u32; 3] = [101, 102, 103];

#[unsafe(export_name = "__test_foreign_buf")]
pub extern "C" fn test_foreign_buf() -> u32 {
    (&raw mut FOREIGN_BUF).expose_provenance() as u32
}
`);

    const rustc = process.env.RUSTC || 'rustc';
    const common = ['--edition=2024', '--color=never', '--target', 'wasm32-unknown-unknown'];
    function compile(args) {
      execFileSync(rustc, [...common, ...args], { cwd: root, stdio: 'pipe' });
    }
    compile(['--crate-name', 'abi_adapter', '--crate-type', 'rlib',
      path.join(root, 'src/lib.rs'), '-o', library]);
    compile(['--crate-name', 'doc_buffer', '--crate-type', 'cdylib', '-O', source,
      '--extern', `abi_adapter=${library}`, '-o', wasm]);

    const api = new WebAssembly.Instance(new WebAssembly.Module(fs.readFileSync(wasm)), {}).exports;
    const address = api.__abi_ret_buf() >>> 0;
    const foreign = api.__test_foreign_buf() >>> 0;
    const setColor = api['set_color.v1'];
    const bytesAt = (offset) => Array.from(new Uint8Array(api.memory.buffer, offset, 12));
    assert.notEqual(address, 0);
    assert.ok(foreign + 12 <= address || address + 12 <= foreign, 'buffers are disjoint');

    const highBits = [0, 0, 0, 128, 152, 186, 220, 254, 255, 255, 255, 255];
    const foreignBefore = bytesAt(foreign);
    assert.equal(setColor(address, 0x80000000, 0xfedcba98, 0xffffffff), 0);
    assert.deepEqual(bytesAt(address), highBits, 'write all bits in little-endian order');
    assert.deepEqual(bytesAt(foreign), foreignBefore);

    for (const [name, invalid] of [
      ['zero', 0],
      ['unaligned', address + 1],
      ['interior', address + 4],
      ['end', address + 12],
      ['out of bounds', api.memory.buffer.byteLength],
      ['maximum u32 address', 0xffffffff],
      ['foreign allocation', foreign],
    ]) {
      assert.notEqual(setColor(invalid, 7, 8, 9), 0, `${name}: return failure without trapping`);
      assert.deepEqual(bytesAt(address), highBits, `${name}: reserved buffer unchanged`);
      assert.deepEqual(bytesAt(foreign), foreignBefore, `${name}: foreign buffer unchanged`);
    }

    assert.equal(setColor(address, 7, 8, 9), 0, 'valid calls still work after rejected addresses');
    assert.deepEqual(bytesAt(address), [7, 0, 0, 0, 8, 0, 0, 0, 9, 0, 0, 0]);
    assert.deepEqual(bytesAt(foreign), foreignBefore);
  } finally {
    assert.equal(path.dirname(path.resolve(directory)), temporaryRoot);
    assert.ok(path.basename(directory).startsWith('abi-adapter-doc-buffer-'));
    fs.rmSync(directory, { recursive: true, force: true });
  }
});
