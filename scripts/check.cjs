#!/usr/bin/env node
// Shared local and CI checks, with no npm dependencies.
const path = require('node:path');
const fs = require('node:fs');
const { spawnSync } = require('node:child_process');
const { parseArgs } = require('node:util');

let options;
try {
  ({ values: options } = parseArgs({ options: {
    toolchain: { type: 'string' },
    compatibility: { type: 'boolean', default: false },
    only: { type: 'string', multiple: true },
    skip: { type: 'string', multiple: true },
    list: { type: 'boolean', default: false },
    continue: { type: 'boolean', default: false },
    help: { type: 'boolean', short: 'h' },
  } }));
  if (options.toolchain !== undefined && !/^[a-zA-Z0-9][a-zA-Z0-9._-]*$/.test(options.toolchain)) {
    throw new Error('--toolchain requires a Rust version such as stable or 1.85.0');
  }
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
const root = path.resolve(__dirname, '..');
const env = { ...process.env };
env.CARGO_TARGET_DIR = path.resolve(root, env.CARGO_TARGET_DIR || 'target');
env.RUSTDOCFLAGS = `${env.RUSTDOCFLAGS || ''} -D warnings`.trim();

function cargo(subcommand, ...args) {
  const cargoArgs = [subcommand, ...(subcommand === 'fmt' ? [] : ['--locked']), ...args];
  if (options.toolchain) run('rustup', ['run', options.toolchain, 'cargo', ...cargoArgs]);
  else run('cargo', cargoArgs);
}
function node(...args) {
  run(process.execPath, args);
}

const steps = [
  { id: 'fmt', label: 'Formatting', run: () => cargo('fmt', '--all', '--', '--check') },
  { id: 'clippy', label: 'Clippy', run: () => cargo('clippy', '--all-targets', '--', '-D', 'warnings') },
  { id: 'test', label: 'Rust tests', run: () => cargo('test', '--all-targets') },
  { id: 'doctest', label: 'Documentation examples', run: () => cargo('test', '--doc') },
  { id: 'doc', label: 'Documentation', run: () => cargo('doc', '--no-deps') },
  { id: 'node', label: 'Node tests', run: () => {
    const tests = fs.readdirSync(path.join(root, 'tests'))
      .filter((name) => name.endsWith('.cjs')).sort().map((name) => path.join('tests', name));
    node('--test', ...tests);
  } },
  { id: 'wasm', label: 'Wasm build and verification', run: () => {
    // A build failure throws before the verifier can read an older artifact.
    cargo('build', '--release', '--target', 'wasm32-unknown-unknown', '--example', 'guest');
    node('examples/verify-wasm.cjs',
      path.join(env.CARGO_TARGET_DIR, 'wasm32-unknown-unknown/release/examples/guest.wasm'));
  } },
  { id: 'package', label: 'Package build', run: () => cargo('package', '--allow-dirty') },
];

if (options.help) {
  console.log(`Usage: node scripts/check.cjs [options]

Requires Node.js 22+, Rust, rustfmt, Clippy, and wasm32-unknown-unknown
for the default checks. Cargo dependency resolution uses --locked.

--only <steps>   Run these steps; repeat or separate names with commas.
--skip <steps>   Exclude steps; repeat or separate names with commas.
--list          List selected steps in execution order without running tools.
--continue      Continue independent steps after failures; exit nonzero if any fail.
--toolchain     Use an already installed Rust version, such as stable or 1.85.0.
--compatibility Skip formatting and Clippy.
--help, -h      Show this help.

Available steps: ${steps.map((step) => step.id).join(', ')}
--skip and --compatibility filter the selected steps.
Without --toolchain, use the current Rust installation.`);
  process.exit(0);
}

function stepNames(option) {
  const names = (options[option] || []).flatMap((value) => value.split(',').map((name) => name.trim()));
  for (const name of names) {
    if (!steps.some((step) => step.id === name)) {
      throw new Error(`Unknown step '${name}' in --${option}. Available steps: ${steps.map((step) => step.id).join(', ')}`);
    }
  }
  return new Set(names);
}

let selected;
try {
  const only = stepNames('only');
  const skip = stepNames('skip');
  selected = steps.filter((step) =>
    (!only.size || only.has(step.id)) && !skip.has(step.id)
    && !(options.compatibility && ['fmt', 'clippy'].includes(step.id)));
  if (!selected.length) throw new Error('No checks selected. See --help for available steps.');
} catch (error) {
  console.error(error.message);
  process.exit(1);
}

if (options.list) {
  console.log('Selected checks (execution order):');
  for (const step of selected) console.log(`  ${step.id}\t${step.label}`);
  process.exit(0);
}

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

function run(command, args) {
  console.log(`> ${command} ${args.join(' ')}`);
  const result = spawnSync(command, args, { cwd: root, env, stdio: 'inherit' });
  if (result.error || result.status !== 0) {
    const detail = result.error?.message || (result.signal ? `signal ${result.signal}` : `exit code ${result.status}`);
    throw new Error(`${command} failed: ${detail}`);
  }
}

const failures = [];
for (const step of selected) {
  console.log(`\n${step.label}`);
  try {
    step.run();
  } catch (error) {
    console.error(`${step.label} failed: ${error.message}`);
    failures.push(step.id);
    if (!options.continue) break;
  }
}
if (failures.length) {
  console.error(`\n${failures.length} check(s) failed: ${failures.join(', ')}.`);
  process.exitCode = 1;
} else {
  console.log('\nAll checks passed.');
}
