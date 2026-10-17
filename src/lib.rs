//! Slot-based wrappers for Wasm ABI boundaries.
//!
//! Each slot is a Rust `u32` and a Wasm `i32`; `[u32; N]` holds `N` slots.
//!
//! # API
//!
//! - [`abi!`]: declare a host wrapper or guest export.
//! - [`FromSlots`]: decode business arguments.
//! - [`IntoSlots`]: encode business results.
//!
//! Repository guides: `docs/guide.md` for usage and common problems and
//! `docs/architecture.md` for design.
//!
//! # Example
//!
//! ```
//! use abi_adapter::abi;
//!
//! fn double(x: u32) -> u32 {
//!     x * 2
//! }
//!
//! abi! { host pub fn double_abi([x]) -> 1 => crate::double; }
//!
//! fn main() {
//!     assert_eq!(double_abi(21), 42);
//! }
//! ```
//!
//! # Built-in conversions
//!
//! These types implement both [`FromSlots`] and [`IntoSlots`].
//!
//! | Type | Slots | Representation |
//! | --- | --- | --- |
//! | `()` | 0 | `[]` |
//! | `u32` | 1 | Unchanged |
//! | `i32` | 1 | Original bits; `-1` becomes `0xffff_ffff` |
//! | `f32` | 1 | `to_bits` / `from_bits`, without a numeric cast |
//! | `[u32; N]` | N | Unchanged, including `N = 0` |
//!
//! Implement the required direction for custom types. [`abi!`] directly
//! returns only zero or one slot, regardless of a type's other conversions.
//!
//! # Constraints
//!
//! The crate is `no_std`, has no dependencies, allocates no heap memory, and
//! generates no `unsafe` blocks. User conversions and business functions
//! retain their own allocation and error behavior. Guest declarations emit
//! `#[unsafe(export_name = ...)]`; users must uphold the export-name and
//! signature requirements documented by [`abi!`].

#![no_std]
#![forbid(unsafe_code)]

mod boundary;
mod impls;
mod slots;

pub use slots::{FromSlots, IntoSlots};
