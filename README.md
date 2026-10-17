# abi-adapter

[![CI](https://github.com/vhexl/abi-adapter/actions/workflows/ci.yml/badge.svg)](https://github.com/vhexl/abi-adapter/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/vhexl/abi-adapter/branch/main/graph/badge.svg)](https://codecov.io/gh/vhexl/abi-adapter)
[![crates.io](https://img.shields.io/crates/v/abi-adapter.svg)](https://crates.io/crates/abi-adapter)
[![docs.rs](https://img.shields.io/docsrs/abi-adapter)](https://docs.rs/abi-adapter)
[![MSRV](https://img.shields.io/crates/msrv/abi-adapter)](https://github.com/vhexl/abi-adapter/blob/main/Cargo.toml)
[![License](https://img.shields.io/crates/l/abi-adapter.svg)](#license)

English | [简体中文](README.zh-CN.md)

Generate wasm interfaces for ordinary Rust functions. `abi!` converts arguments
according to a declaration, calls the target, and converts its result.
No third-party dependencies; supports `no_std`. Requires Rust 1.85+.

## Quick start

Add the crate to your project:

```text
cargo add abi-adapter
```

To use unreleased changes from `main`, depend on the Git repository instead:

```text
cargo add abi-adapter --git https://github.com/vhexl/abi-adapter
```

```rust
use abi_adapter::abi;

fn double(value: u32) -> u32 {
    value * 2
}

abi! { host pub fn double_abi([value]) -> 1 => crate::double; }

fn main() {
    assert_eq!(double_abi(21), 42);
}
```

`host` generates a regular Rust function; `guest("double.v1")` generates a wasm export.
Each bracketed group maps to one target argument. Slots are Rust `u32` values
and wasm `i32` values; wrappers directly return zero or one slot.
Custom types use `FromSlots<N>` and `IntoSlots<N>`.
For a complete wasm example, see [examples/guest.rs](examples/guest.rs).

Users define runtime registration, memory protocols, and error codes.
Panics from conversions and target functions are not caught; guest exports trap.

## Documentation

The following guides are in Chinese:

- [Usage guide](docs/guide.md): custom conversions, `ctx`, wasm exports, and common problems.
- [Buffer example](docs/buffers.md): multi-slot results and memory conventions.
- [Architecture](docs/architecture.md): modules, execution flow, and design decisions.

## Development

`node scripts/check.cjs` runs `fmt`, `clippy`, `test`, `doctest`, `doc`, `node`,
`wasm`, and `package` in order. `doctest` tests documentation examples; `doc`
generates documentation. `wasm` builds and verifies the example.
Requires Rust 1.85+ (with `rustfmt`, Clippy, and the
`wasm32-unknown-unknown` target) and Node.js 22+.

```text
node scripts/check.cjs --list
node scripts/check.cjs --only wasm
node scripts/check.cjs --skip doc --continue
node scripts/check.cjs --toolchain 1.85.0 --compatibility
```

`--only`/`--skip` accept comma-separated names and repeated flags; `--list` only lists steps.
Checks stop on the first failure by default. `--continue` runs remaining independent
checks, summarizes failures, and returns a nonzero exit code if any failed.
Cargo checks use `--locked`, permitting downloads without lockfile changes.
`--toolchain` selects an installed Rust version; use 1.85.0 above to check MSRV.
`--compatibility` skips `fmt` and `clippy`.

CI's test job reuses this script. Dependency auditing (`cargo-deny`) and coverage
remain separate CI jobs. A local run checks one platform and toolchain; CI also
checks its Linux/Windows and MSRV matrix and uploads coverage reports.

## Releasing

This project uses [release-plz](https://release-plz.dev/) to automate releases:

1. Every push to `main` triggers release-plz to open/update a **Release PR**
   with the correct version bumps and a changelog derived from
   [Conventional Commits](https://www.conventionalcommits.org/).
2. Merge the Release PR when ready — release-plz will then:
   - Create the git tag (`v<version>`)
   - Publish to [crates.io](https://crates.io/crates/abi-adapter)
   - Create a GitHub release with release notes

Publishing uses crates.io [Trusted Publishing](https://crates.io/docs/trusted-publishing),
so no registry token is stored; setup notes are in
[release-plz.yml](.github/workflows/release-plz.yml).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
