//! Behavioral contract of the `abi!` boundary declaration macro:
//! decode order, exactly one business call, zero-slot encoding, panic cleanup,
//! context pass-through, and wrapper shapes.

use abi_adapter::{FromSlots, IntoSlots, abi};
use std::cell::RefCell;

thread_local! {
    static EVENTS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

fn record(event: &'static str) {
    EVENTS.with_borrow_mut(|events| events.push(event));
}

fn take_events() -> Vec<&'static str> {
    EVENTS.with_borrow_mut(std::mem::take)
}

pub struct Rgb {
    pub r: u32,
    pub g: u32,
    pub b: u32,
}

impl FromSlots<3> for Rgb {
    fn from_slots([r, g, b]: [u32; 3]) -> Self {
        Self { r, g, b }
    }
}

pub struct Pair(pub u32, pub u32);

impl FromSlots<2> for Pair {
    fn from_slots([x, y]: [u32; 2]) -> Self {
        Self(x, y)
    }
}

/// Decodes but never encodes: only `FromSlots` exists.
pub struct OnlyIn(pub u32);

impl FromSlots<1> for OnlyIn {
    fn from_slots([x]: [u32; 1]) -> Self {
        Self(x)
    }
}

/// Encodes but never decodes: only `IntoSlots` exists.
pub struct OnlyOut(pub u32);

impl IntoSlots<1> for OnlyOut {
    fn into_slots(self) -> [u32; 1] {
        [self.0]
    }
}

pub struct Newtype(pub u32);

impl FromSlots<1> for Newtype {
    fn from_slots([x]: [u32; 1]) -> Self {
        Self(x)
    }
}

impl IntoSlots<1> for Newtype {
    fn into_slots(self) -> [u32; 1] {
        [self.0]
    }
}

pub type Alias = u32;

/// Host context: not a slot, passed through by value.
pub struct Ctx<'a>(pub &'a mut u32);

pub struct EvCtx;

impl Drop for EvCtx {
    fn drop(&mut self) {
        record("drop-ctx");
    }
}

pub struct First;

impl Drop for First {
    fn drop(&mut self) {
        record("drop-first");
    }
}

impl FromSlots<1> for First {
    fn from_slots(_slots: [u32; 1]) -> Self {
        record("decode-first");
        Self
    }
}

pub struct Second;

impl Drop for Second {
    fn drop(&mut self) {
        record("drop-second");
    }
}

impl FromSlots<1> for Second {
    fn from_slots(slots: [u32; 1]) -> Self {
        record("decode-second");
        assert_ne!(slots[0], 99, "intentional decoder panic");
        Self
    }
}

pub struct Zero;

impl Drop for Zero {
    fn drop(&mut self) {
        record("drop-zero");
    }
}

impl IntoSlots<0> for Zero {
    fn into_slots(self) -> [u32; 0] {
        record("encode-zero");
        []
    }
}

pub struct PanickingZero;

impl Drop for PanickingZero {
    fn drop(&mut self) {
        record("drop-panicking-zero");
    }
}

impl IntoSlots<0> for PanickingZero {
    fn into_slots(self) -> [u32; 0] {
        record("encode-zero-panic");
        panic!("intentional zero-slot encoder panic");
    }
}

pub struct PanickingOne;

impl Drop for PanickingOne {
    fn drop(&mut self) {
        record("drop-panicking-one");
    }
}

impl IntoSlots<1> for PanickingOne {
    fn into_slots(self) -> [u32; 1] {
        record("encode-one-panic");
        panic!("intentional one-slot encoder panic");
    }
}

pub struct Energy(pub u32);

impl FromSlots<1> for Energy {
    fn from_slots([x]: [u32; 1]) -> Self {
        Self(x)
    }
}

impl IntoSlots<1> for Energy {
    fn into_slots(self) -> [u32; 1] {
        [self.0]
    }
}

impl Energy {
    pub fn double(energy: Energy) -> Energy {
        Energy(energy.0 * 2)
    }
}

pub fn noop() {}

pub fn produce() -> u32 {
    7
}

pub fn consume(x: u32) {
    let _ = x;
}

pub fn echo(x: u32) -> u32 {
    x + 1
}

pub fn rgb_sum(color: Rgb, scale: u32) -> u32 {
    (color.r + 2 * color.g + 3 * color.b) * scale
}

pub fn pair_sum(pair: Pair) -> u32 {
    pair.0 + pair.1
}

pub fn opaque(x: u32) -> impl IntoSlots<1> {
    x * 2
}

pub fn take_only_in(value: OnlyIn) -> u32 {
    value.0 + 1
}

pub fn make_only_out(x: u32) -> OnlyOut {
    OnlyOut(x + 1)
}

pub fn float(x: f32) -> f32 {
    x * 2.0
}

pub fn signed(x: i32) -> i32 {
    -x
}

pub fn array_param(values: [u32; 2]) -> u32 {
    values[0] + values[1]
}

pub fn aliased(x: Alias) -> u32 {
    x + 1
}

pub fn newtyped(value: Newtype) -> Newtype {
    Newtype(value.0 + 1)
}

pub fn r#type(value: u32) -> u32 {
    value
}

pub const fn constant(x: u32) -> u32 {
    x + 3
}

pub extern "C" fn extc(x: u32) -> u32 {
    x * 3
}

pub fn patterns(_: u32, mut y: u32) -> u32 {
    y += 1;
    y
}

pub fn destructured([a, b]: [u32; 2], c: u32) -> u32 {
    10 * a + b + c
}

pub fn with_ctx(cx: Ctx<'_>, pair: Pair) -> u32 {
    *cx.0 += pair.0 + pair.1;
    *cx.0
}

pub fn only_ctx(cx: Ctx<'_>) {
    *cx.0 += 1;
}

pub fn process(_cx: EvCtx, _first: First, _second: Second) -> Zero {
    record("call");
    Zero
}

pub fn zero_only() -> Zero {
    record("call-zero");
    Zero
}

pub fn panicking_business(_cx: EvCtx, _first: First, _second: Second) -> Zero {
    record("call-panic");
    panic!("intentional business panic");
}

pub fn make_panicking_zero(_cx: EvCtx, _first: First) -> PanickingZero {
    record("call-zero-panic");
    PanickingZero
}

pub fn make_panicking_one(_cx: EvCtx, _first: First) -> PanickingOne {
    record("call-one-panic");
    PanickingOne
}

pub fn generic<T: IntoSlots<1>>(value: T) -> T {
    value
}

pub fn const_generic<const N: usize>(values: [u32; N]) -> u32 {
    values.iter().copied().sum()
}

pub fn gated_business(x: u32) -> u32 {
    x + 5
}

pub mod wire {
    use super::Pair;

    pub fn score(point: Pair, scale: u32) -> u32 {
        (point.0 + 10 * point.1) * scale
    }

    pub fn sink(_point: Pair) {}
}

// --- boundary declarations ---

abi! { host pub fn noop_abi() -> 0 => crate::noop; }
abi! { host pub fn produce_abi() -> 1 => crate::produce; }
abi! { host pub fn consume_abi([x]) -> 0 => crate::consume; }
abi! { host pub fn echo_abi([x]) -> 1 => crate::echo; }
abi! { host pub fn rgb_abi([r, g, b], [scale]) -> 1 => crate::rgb_sum; }
abi! { host pub fn pair_abi([x, y]) -> 1 => crate::pair_sum; }
abi! { host pub fn opaque_abi([x]) -> 1 => crate::opaque; }
abi! { host pub fn only_in_abi([x]) -> 1 => crate::take_only_in; }
abi! { host pub fn only_out_abi([x]) -> 1 => crate::make_only_out; }
abi! { host pub fn float_abi([x]) -> 1 => crate::float; }
abi! { host pub fn signed_abi([x]) -> 1 => crate::signed; }
abi! { host pub fn array_abi([x, y]) -> 1 => crate::array_param; }
abi! { host pub fn alias_abi([x]) -> 1 => crate::aliased; }
abi! { host pub fn newtype_abi([x]) -> 1 => crate::newtyped; }
abi! { guest("raw-keyword.v1") pub fn type_abi([value]) -> 1 => crate::r#type; }
abi! { guest("const.v1") pub fn constant_abi([x]) -> 1 => crate::constant; }
abi! { guest("extc.v1") pub fn extc_abi([x]) -> 1 => crate::extc; }
abi! { host pub fn patterns_abi([a], [b]) -> 1 => crate::patterns; }
abi! { host pub fn destructured_abi([x, y], [z]) -> 1 => crate::destructured; }
abi! { host pub fn ctx_abi(ctx cx: Ctx<'_>; [x, y]) -> 1 => crate::with_ctx; }
abi! { host pub fn only_ctx_abi(ctx cx: Ctx<'_>;) -> 0 => crate::only_ctx; }
abi! { host pub fn process_abi(ctx cx: EvCtx; [first], [second]) -> 0 => crate::process; }
abi! { host pub fn zero_abi() -> 0 => crate::zero_only; }
abi! { host pub fn business_panic_abi(ctx cx: EvCtx; [first], [second]) -> 0 => crate::panicking_business; }
abi! { host pub fn zero_encoder_panic_abi(ctx cx: EvCtx; [first]) -> 0 => crate::make_panicking_zero; }
abi! { host pub fn one_encoder_panic_abi(ctx cx: EvCtx; [first]) -> 1 => crate::make_panicking_one; }
abi! { host pub fn generic_abi([x]) -> 1 => crate::generic::<u32>; }
abi! { host pub fn const_generic_abi([x, y, z]) -> 1 => crate::const_generic; }
abi! { host pub fn energy_abi([x]) -> 1 => crate::Energy::double; }
abi! { host #[inline] pub fn inlined_abi([x]) -> 1 => crate::echo; }

// Attributes apply to the wrapper only; the declaration disappears, so no
// test may reference `absent_abi`.
// Missing targets prove that cfg removes the entire wrapper before its body
// is resolved, in all three forms and with both kinds of cfg attribute.
abi! { host #[cfg(any())] pub fn absent_abi([x]) -> 1 => crate::missing::business; }
abi! { host #[cfg(any())] pub fn absent_ctx_abi(ctx cx: Ctx<'_>; [x]) -> 1 => crate::missing::business; }
abi! { guest("absent.v1") #[cfg(any())] pub fn absent_guest_abi([x]) -> 1 => crate::missing::business; }
abi! { host #[cfg_attr(all(), cfg(any()))] pub fn absent_cfg_attr_abi([x]) -> 0 => crate::missing::business; }
abi! { host #[cfg_attr(all(), cfg(any()))] pub fn absent_ctx_cfg_attr_abi(ctx cx: Ctx<'_>; [x]) -> 0 => crate::missing::business; }
abi! { guest("absent.cfg-attr.v1") #[cfg_attr(all(), cfg(any()))] pub fn absent_guest_cfg_attr_abi([x]) -> 0 => crate::missing::business; }
// Same business function behind two versioned guest symbols.
abi! { guest("score.v1") pub fn score_v1([x, y], [scale]) -> 1 => crate::wire::score; }
abi! { guest("score.v2") pub fn score_v2([x, y], [scale]) -> 1 => crate::wire::score; }
abi! { guest("sink.v1") pub fn sink_abi([x, y]) -> 0 => crate::wire::sink; }

/// Hand-written equivalent of `score.v1` for output comparison.
#[unsafe(export_name = "score.manual")]
pub extern "C" fn manual_score(x: u32, y: u32, scale: u32) -> u32 {
    IntoSlots::<1>::into_slots(wire::score(
        Pair::from_slots([x, y]),
        u32::from_slots([scale]),
    ))[0]
}

// Gating business and wrapper together is a module-level decision: the whole
// module (business functions and declarations) disappears with one `cfg`.
#[cfg(test)]
pub mod gated {
    pub fn business(x: u32) -> u32 {
        crate::gated_business(x)
    }

    abi_adapter::abi! { host pub fn gated_abi([x]) -> 1 => crate::gated::business; }
}

/// A private wrapper stays callable inside its own module.
mod private_entry {
    pub fn business(x: u32) -> u32 {
        x + 1
    }

    abi_adapter::abi! { host fn private_abi([x]) -> 1 => crate::private_entry::business; }

    #[test]
    fn private_wrapper_is_callable_inside_its_module() {
        assert_eq!(private_abi(1), 2);
    }
}

// The `unsafe_code` lint contract for guest declarations (crate-level
// `#![forbid(unsafe_code)]`) is covered by `tests/forbid-unsafe.rs`.

#[test]
fn no_args_zero_and_one_slot_returns() {
    noop_abi();
    assert_eq!(produce_abi(), 7);
}

#[test]
fn one_param_zero_and_one_slot_returns() {
    consume_abi(5);
    assert_eq!(echo_abi(41), 42);
}

#[test]
fn multi_slot_groups_decode_in_source_order() {
    // rgb_sum = (r + 2g + 3b) * scale with wire order r, g, b, scale.
    assert_eq!(rgb_abi(1, 2, 3, 4), 56);
    assert_eq!(pair_abi(4, 5), 9);
}

#[test]
fn opaque_and_direction_specific_types() {
    assert_eq!(opaque_abi(21), 42);
    assert_eq!(only_in_abi(3), 4);
    assert_eq!(only_out_abi(3), 4);
}

#[test]
fn bit_level_semantics_for_float_and_signed() {
    assert_eq!(f32::from_bits(float_abi(1.5_f32.to_bits())), 3.0);
    assert_eq!(signed_abi((-5_i32) as u32) as i32, 5);
}

#[test]
fn array_alias_newtype_and_raw_identifier() {
    assert_eq!(array_abi(3, 4), 7);
    assert_eq!(alias_abi(1), 2);
    assert_eq!(newtype_abi(9), 10);
    assert_eq!(type_abi(17), 17);
}

#[test]
fn const_and_extern_c_business_functions() {
    assert_eq!(constant_abi(1), 4);
    assert_eq!(extc_abi(2), 6);
}

#[test]
fn parameter_patterns_live_in_the_business_signature() {
    assert_eq!(patterns_abi(1, 2), 3);
    assert_eq!(destructured_abi(7, 3, 100), 173);
}

#[test]
fn ctx_is_first_and_not_a_slot() {
    let mut state = 0;
    assert_eq!(ctx_abi(Ctx(&mut state), 3, 4), 7);
    assert_eq!(state, 7);
    only_ctx_abi(Ctx(&mut state));
    assert_eq!(state, 8);
}

#[test]
fn zero_slot_return_still_encodes_once() {
    take_events();
    zero_abi();
    assert_eq!(take_events(), ["call-zero", "encode-zero", "drop-zero"]);
}

#[test]
fn decode_call_encode_run_once_in_order() {
    take_events();
    process_abi(EvCtx, 1, 2);
    assert_eq!(
        take_events(),
        [
            "decode-first",
            "decode-second",
            "call",
            "drop-second",
            "drop-first",
            "drop-ctx",
            "encode-zero",
            "drop-zero",
        ]
    );
}

#[test]
fn decoder_panic_drops_decoded_values_and_context() {
    take_events();
    let result = std::panic::catch_unwind(|| process_abi(EvCtx, 1, 99));
    assert!(result.is_err());
    assert_eq!(
        take_events(),
        ["decode-first", "decode-second", "drop-first", "drop-ctx"]
    );
}

#[test]
fn business_panic_propagates_and_drops_arguments_and_context_without_encoding() {
    take_events();
    let panic = std::panic::catch_unwind(|| business_panic_abi(EvCtx, 1, 2))
        .expect_err("business panic must propagate");
    assert_eq!(
        panic.downcast_ref::<&str>(),
        Some(&"intentional business panic")
    );
    assert_eq!(
        take_events(),
        [
            "decode-first",
            "decode-second",
            "call-panic",
            "drop-second",
            "drop-first",
            "drop-ctx",
        ]
    );
}

#[test]
fn zero_slot_encoder_panic_propagates_and_drops_the_return_value() {
    take_events();
    let panic = std::panic::catch_unwind(|| zero_encoder_panic_abi(EvCtx, 1))
        .expect_err("zero-slot encoder panic must propagate");
    assert_eq!(
        panic.downcast_ref::<&str>(),
        Some(&"intentional zero-slot encoder panic")
    );
    assert_eq!(
        take_events(),
        [
            "decode-first",
            "call-zero-panic",
            "drop-first",
            "drop-ctx",
            "encode-zero-panic",
            "drop-panicking-zero",
        ]
    );
}

#[test]
fn one_slot_encoder_panic_propagates_and_drops_the_return_value() {
    take_events();
    let panic = std::panic::catch_unwind(|| one_encoder_panic_abi(EvCtx, 1))
        .expect_err("one-slot encoder panic must propagate");
    assert_eq!(
        panic.downcast_ref::<&str>(),
        Some(&"intentional one-slot encoder panic")
    );
    assert_eq!(
        take_events(),
        [
            "decode-first",
            "call-one-panic",
            "drop-first",
            "drop-ctx",
            "encode-one-panic",
            "drop-panicking-one",
        ]
    );
}

#[test]
fn generic_specialization_and_const_generic_inference() {
    assert_eq!(generic_abi(19), 19);
    assert_eq!(const_generic_abi(3, 4, 5), 12);
}

#[test]
fn associated_function_target() {
    assert_eq!(energy_abi(4), 8);
}

#[test]
fn versioned_symbols_and_one_business_behind_several_wrappers() {
    assert_eq!(score_v1(2, 3, 4), 128);
    assert_eq!(score_v2(2, 3, 4), 128);
    assert_eq!(manual_score(2, 3, 4), 128);
    sink_abi(1, 2);
}

#[test]
fn wrapper_attributes_and_module_level_cfg() {
    assert_eq!(inlined_abi(1), 2);
    assert_eq!(gated::gated_abi(1), 6);
}
