#!/usr/bin/env node
// Run the same checks locally and on GitHub without npm dependencies.
const path = require('node:path');
const fs = require('node:fs');
const { spawnSync } = require('node:child_process');
const { parseArgs } = require('node:util');

let options;
try {
  ({ values: options } = parseArgs({ options: {
    toolchain: { type: 'string' },
    compatibility: { type: 'boolean', default: false },
    help: { type: 'boolean', short: 'h' },
  } }));
  if (options.toolchain !== undefined && !/^[a-zA-Z0-9][a-zA-Z0-9._-]*$/.test(options.toolchain)) {
    throw new Error('--toolchain requires a Rust version such as stable or 1.85.0');
  }
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
if (options.help) {
  console.log(`Usage: node scripts/check.cjs [--toolchain stable|1.85.0] [--compatibility]

Requires Node.js 22+, Rust, and the wasm32-unknown-unknown target.
Full checks also require rustfmt and Clippy. All Cargo commands run offline.
--toolchain      Use an already installed Rust version for every check.
--compatibility  Skip formatting and Clippy; run all other checks.
Without --toolchain, use the current Rust installation.`);
  process.exit(0);
}

const root = path.resolve(__dirname, '..');
const env = { ...process.env };
env.CARGO_TARGET_DIR = path.resolve(root, env.CARGO_TARGET_DIR || 'target');
env.RUSTDOCFLAGS = `${env.RUSTDOCFLAGS || ''} -D warnings`.trim();
if (options.toolchain) {
  env.RUSTUP_TOOLCHAIN = options.toolchain;
  // The Rust and Node tests invoke rustc directly; keep them on this version too.
  for (const tool of ['rustc', 'rustdoc']) {
    const result = spawnSync('rustup', ['which', '--toolchain', options.toolchain, tool], {
      cwd: root, env, encoding: 'utf8',
    });
    if (result.error || result.status !== 0) {
      console.error(result.error?.message || result.stderr);
      process.exit(1);
    }
    env[tool.toUpperCase()] = result.stdout.trim();
  }
}

function run(label, command, args) {
  console.log(`\n${label}: ${command} ${args.join(' ')}`);
  const result = spawnSync(command, args, { cwd: root, env, stdio: 'inherit' });
  if (result.error || result.status !== 0) {
    console.error(`${label} failed${result.error ? `: ${result.error.message}` : ''}.`);
    process.exit(result.status || 1);
  }
}
function cargo(label, args) {
  if (options.toolchain) {
    run(label, 'rustup', ['run', options.toolchain, 'cargo', ...args]);
  } else {
    run(label, 'cargo', args);
  }
}

if (!options.compatibility) {
  cargo('Formatting', ['fmt', '--all', '--', '--check']);
  cargo('Clippy', ['clippy', '--offline', '--all-targets', '--', '-D', 'warnings']);
}
cargo('Rust tests', ['test', '--offline', '--all-targets']);
cargo('Documentation examples', ['test', '--offline', '--doc']);
cargo('Documentation', ['doc', '--offline', '--no-deps']);
const nodeTests = fs.readdirSync(path.join(root, 'tests'))
  .filter((name) => name.endsWith('.cjs')).sort().map((name) => path.join('tests', name));
run('Node tests', process.execPath, ['--test', ...nodeTests]);
cargo('Wasm build', ['build', '--offline', '--release', '--target', 'wasm32-unknown-unknown', '--example', 'guest']);
run('Wasm verification', process.execPath, ['examples/verify-wasm.cjs',
  path.join(env.CARGO_TARGET_DIR, 'wasm32-unknown-unknown/release/examples/guest.wasm')]);
cargo('Package build', ['package', '--allow-dirty', '--offline']);
console.log('\nAll checks passed.');
