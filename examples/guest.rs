//! Build a wasm module with `abi!` exports and check the compiled result.
//!
//! Run from the project root. Rust needs the `wasm32-unknown-unknown` target;
//! Node.js 22 or newer runs the checks without npm packages.
//!
//! ```text
//! cargo build --offline --release --target wasm32-unknown-unknown --example guest
//! node examples/verify-wasm.cjs target/wasm32-unknown-unknown/release/examples/guest.wasm
//! ```
//!
//! Start with `Point`, `business::score`, and the `score.v1` declaration:
//! two `u32` values become one `Point`, then a third value supplies the scale.
//! `score.v2` calls the same function under a second export name;
//! `score.manual` shows the equivalent wrapper written by hand.
//!
//! The other exports exercise empty argument lists, zero or one return value,
//! signed values, conditional compilation, and panics. `verify-wasm.cjs`
//! checks the actual wasm parameter and result types, preserves all 32 bits,
//! and confirms that panics stop the call with a wasm trap. It also checks
//! that this release build shares one function for `score.v1` and
//! `score.manual`.
//!
//! Export names must be unique, and callers must use the matching signature.
//! The hand-written export attribute on `manual_score` is checked by Rust's
//! `unsafe_code` lint. Attributes produced by another crate's macro currently
//! bypass that lint; the export obligations still apply. See
//! `tests/forbid-unsafe.rs` and `docs/architecture.md` for details.

use abi_adapter::{FromSlots, IntoSlots};

struct Point {
    x: u32,
    y: u32,
}

impl FromSlots<2> for Point {
    fn from_slots([x, y]: [u32; 2]) -> Self {
        Self { x, y }
    }
}

struct PanicInput;

impl FromSlots<1> for PanicInput {
    fn from_slots(_slots: [u32; 1]) -> Self {
        panic!("intentional decoder panic");
    }
}

struct PanicOutput;

impl IntoSlots<1> for PanicOutput {
    fn into_slots(self) -> [u32; 1] {
        panic!("intentional encoder panic");
    }
}

mod business {
    use super::{PanicInput, PanicOutput, Point};

    pub fn score(point: Point, scale: u32) -> u32 {
        (point.x + 10 * point.y) * scale
    }

    pub fn sink(_point: Point) {}

    pub fn signed(value: i32) -> i32 {
        value
    }

    pub fn noop() {}

    pub fn produce() -> u32 {
        0x8000_0000
    }

    pub fn panic_decode(_value: PanicInput) {}

    pub fn panic_business() {
        panic!("intentional business panic");
    }

    pub fn panic_encode() -> PanicOutput {
        PanicOutput
    }
}

abi_adapter::abi! { guest("score.v1") pub fn score_v1([x, y], [scale]) -> 1 => crate::business::score; }
abi_adapter::abi! { guest("score.v2") pub fn score_v2([x, y], [scale]) -> 1 => crate::business::score; }
abi_adapter::abi! { guest("sink.v1") pub fn sink_v1([x, y]) -> 0 => crate::business::sink; }
abi_adapter::abi! { guest("signed.v1") pub fn signed_v1([value]) -> 1 => crate::business::signed; }
abi_adapter::abi! { guest("noop.v1") pub fn noop_v1() -> 0 => crate::business::noop; }
abi_adapter::abi! { guest("produce.v1") pub fn produce_v1() -> 1 => crate::business::produce; }
abi_adapter::abi! { guest("panic.decode.v1") pub fn panic_decode_v1([value]) -> 0 => crate::business::panic_decode; }
abi_adapter::abi! { guest("panic.business.v1") pub fn panic_business_v1() -> 0 => crate::business::panic_business; }
abi_adapter::abi! { guest("panic.encode.v1") pub fn panic_encode_v1() -> 1 => crate::business::panic_encode; }

// If the macro drops cfg, this missing business path makes compilation fail.
abi_adapter::abi! {
    guest("cfg.absent.v1") #[cfg(any())] pub fn absent_v1([value])
        -> 1 => crate::missing_business;
}

/// Hand-written equivalent of `score.v1`.
#[unsafe(export_name = "score.manual")]
pub extern "C" fn manual_score(x: u32, y: u32, scale: u32) -> u32 {
    IntoSlots::<1>::into_slots(business::score(
        Point::from_slots([x, y]),
        u32::from_slots([scale]),
    ))[0]
}

fn main() {
    assert_eq!(score_v1(2, 3, 4), 128);
    assert_eq!(score_v2(2, 3, 4), 128);
    assert_eq!(manual_score(2, 3, 4), 128);
    sink_v1(1, 2);
}
