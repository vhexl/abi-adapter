//! Mechanical slot-based ABI adapter — single-crate core library.
//!
//! A slot is one 32-bit word (wasm `i32`; Rust `u32` / `[u32; N]`). This crate
//! provides the two conversion traits ([`FromSlots`] / [`IntoSlots`]), a finite
//! set of built-in implementations, and the [`abi!`] boundary declaration macro
//! that expands one declaration into one host or guest wrapper function.
//!
//! Contract (`docs/architecture.md` §3, §9): `no_std`, zero dependencies, no
//! allocation, no unsafe blocks. Guest declarations emit an unsafe export
//! attribute; consumers must ensure the symbol and its ABI are valid (see
//! [`abi!`] for the export safety contract).
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

#![no_std]
#![forbid(unsafe_code)]

mod boundary;
mod impls;
mod slots;

pub use slots::{FromSlots, IntoSlots};
