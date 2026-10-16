# abi-adapter

English | [简体中文](README.zh-CN.md)

Generate wasm interfaces for ordinary Rust functions. `abi!` converts arguments
according to a declaration, calls the target, and converts its result.
No third-party dependencies; supports `no_std`. Requires Rust 1.85+.

## Quick start

Add the crate as a Git dependency:

```toml
[dependencies]
abi-adapter = { git = "https://github.com/vhexl/abi-adapter" }
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
- [Development guide](docs/development.md): checks, API documentation, and maintenance.

## License

[MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
