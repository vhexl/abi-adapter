//! The `abi!` boundary declaration macro (`docs/architecture.md` §4–§6).
//!
//! One declaration expands into exactly one wrapper function: decode each slot
//! group through [`FromSlots`](crate::FromSlots), call the business function
//! exactly once, and encode the result through [`IntoSlots`](crate::IntoSlots)
//! at width 0 or 1. Decoded types come from the business signature and group
//! widths from the number of listed slots, so the declaration repeats no
//! business type; the expansion keeps no intermediate representation and emits
//! no `unsafe` blocks. Guest exports still carry an unsafe `export_name`
//! attribute whose symbol contract is the consumer's responsibility.

/// Declares a host-side or guest-side ABI boundary function.
///
/// # Forms
///
/// ```text
/// abi! { host  #[attrs] VIS fn NAME([a, b], [c])         -> 0|1 => PATH; }
/// abi! { host  #[attrs] VIS fn NAME(ctx CX: CTY; [a, b]) -> 0|1 => PATH; }
/// abi! { host  #[attrs] VIS fn NAME(ctx CX: CTY;)        -> 0|1 => PATH; }
/// abi! { guest("SYMBOL") #[attrs] VIS fn NAME([a, b])    -> 0|1 => PATH; }
/// ```
///
/// - `host` expands to a plain Rust function for the host runtime to register
///   as the implementation of a module import.
/// - `guest` expands to a safe `extern "C"` function exported under the literal
///   `SYMBOL` (`#[unsafe(export_name = ...)]`, edition 2024 syntax). The consumer
///   must ensure the symbol does not conflict with another linked definition
///   or a symbol expected by the runtime under a different signature. This is
///   the safety obligation of `export_name`, even though calling the generated
///   Rust function is safe. The
///   `unsafe_code` lint does not inspect other crates' macro output, so this
///   compiles under `#![forbid(unsafe_code)]`; adding `#[allow(unsafe_code)]`
///   there is E0453.
/// - Each bracketed list is one business parameter; every listed name becomes
///   one `::core::primitive::u32` wrapper parameter. Group order is wire order
///   and slot order.
/// - Attributes and visibility apply to the wrapper function, not to the
///   business function.
///
/// # Inference
///
/// The declaration never names a business type. For an argument of type `T` the
/// expansion calls `FromSlots::from_slots([...])`, where `Self = T` is inferred
/// from the business signature and the const parameter `N` from the length of
/// the array literal. Return-position `impl IntoSlots<1>` and const-generic
/// array parameters (`[u32; N]`) both infer successfully.
///
/// # Returns
///
/// The width must be the literal `0` or `1`: `0` runs `IntoSlots<0>` and
/// discards the result; `1` returns the single encoded slot. Both directions
/// have the same wire shape on both sides, so they pair directly. Larger
/// results are a consumer protocol — declare an address or handle as an
/// ordinary slot parameter and return a status word
/// (`docs/architecture.md` §7).
///
/// # Context
///
/// `ctx` is host-side syntax only, must be the first parameter, and is passed
/// through before the decoded arguments without slot conversion. Its type is
/// copied verbatim, including elided lifetimes (`Ctx<'_>`).
///
/// # Panics
///
/// Decoding, the business call, and encoding all run inside the wrapper, and
/// nothing is caught or translated. In a `host` wrapper a panic unwinds to the
/// caller like any ordinary Rust panic. A `guest` wrapper is an `extern "C"`
/// function, so a panic cannot unwind out of it: the program aborts (a trap on
/// wasm), and whether values decoded so far are dropped first is unspecified.
/// Handle failures inside the business function itself.
///
/// # Limitations
///
/// - `PATH` must be a qualified path such as `crate::business::score`; a bare
///   name would be captured by a same-named slot parameter (E0618).
///   The `<Type as Trait>::method` form does not match the macro's `path`
///   fragment; use an unambiguous `Type::method` or a concrete thin wrapper.
/// - An outer macro forwarding the return width must capture it as `tt`, not
///   `literal` or `expr`: those fragments become opaque and cannot match the
///   `0` / `1` rules after forwarding.
/// - Slot names must be unique within one declaration (E0415), and a slot name
///   must not collide with a constant (or unit struct or enum variant) in
///   scope: the parameter would parse as a constant pattern (E0005), because
///   `macro_rules!` cannot give caller-provided identifiers hygiene. Rename the
///   slot or move the declaration to a scope without that item.
/// - A generic business function needs an unambiguous target: write a turbofish
///   specialization (`crate::generic::<u32>`) or a concrete thin wrapper.
///   Parameter-position `impl Trait` is an anonymous generic and is equally
///   ambiguous (E0283).
/// - Group widths are checked against the business signature by the compiler,
///   but a semantic swap between two same-width slots is not detectable
///   (`docs/architecture.md` §10).
///
/// # Examples
///
/// A host boundary decoding a two-slot group and returning one slot:
///
/// ```
/// use abi_adapter::{abi, FromSlots};
///
/// struct Point { x: u32, y: u32 }
///
/// impl FromSlots<2> for Point {
///     fn from_slots([x, y]: [u32; 2]) -> Self {
///         Self { x, y }
///     }
/// }
///
/// fn score(point: Point, scale: u32) -> u32 {
///     (point.x + 10 * point.y) * scale
/// }
///
/// abi! { host pub fn score_abi([x, y], [scale]) -> 1 => crate::score; }
///
/// fn main() {
///     assert_eq!(score_abi(2, 3, 4), 128);
/// }
/// ```
///
/// A guest boundary with a versioned symbol and a zero-slot return:
///
/// ```
/// use abi_adapter::abi;
///
/// fn sink(_point: [u32; 2]) {}
///
/// abi! { guest("sink.v1") pub fn sink_abi([x, y]) -> 0 => crate::sink; }
///
/// fn main() {
///     sink_abi(1, 2);
/// }
/// ```
///
/// A host boundary with the runtime context passed through unconverted:
///
/// ```
/// use abi_adapter::abi;
///
/// struct Ctx<'a>(&'a mut u32);
///
/// fn tick(cx: Ctx<'_>, delta: u32) -> u32 {
///     *cx.0 += delta;
///     *cx.0
/// }
///
/// abi! { host pub fn tick_abi(ctx cx: Ctx<'_>; [delta]) -> 1 => crate::tick; }
///
/// fn main() {
///     let mut state = 0;
///     assert_eq!(tick_abi(Ctx(&mut state), 5), 5);
///     assert_eq!(state, 5);
/// }
/// ```
///
/// # Rejections
///
/// The group width and the business signature must agree:
///
/// ```compile_fail,E0277
/// use abi_adapter::abi;
///
/// fn echo(value: u32) -> u32 { value }
///
/// abi! { host fn wrong([x, y]) -> 1 => crate::echo; }
///
/// fn main() {}
/// ```
///
/// A generic business function needs an unambiguous target:
///
/// ```compile_fail,E0283
/// use abi_adapter::{abi, IntoSlots};
///
/// fn echo<T: IntoSlots<1>>(value: T) -> T { value }
///
/// abi! { host fn wrong([x]) -> 1 => crate::echo; }
///
/// fn main() {}
/// ```
///
/// Parameter-position `impl Trait` is an anonymous generic:
///
/// ```compile_fail,E0283
/// use abi_adapter::{abi, IntoSlots};
///
/// fn echo(value: impl IntoSlots<1>) -> u32 { value.into_slots()[0] }
///
/// abi! { host fn wrong([x]) -> 1 => crate::echo; }
///
/// fn main() {}
/// ```
///
/// Direct returns are limited to zero or one slot:
///
/// ```compile_fail
/// use abi_adapter::abi;
///
/// fn pair() -> [u32; 2] { [1, 2] }
///
/// abi! { host fn wrong() -> 2 => crate::pair; }
///
/// fn main() {}
/// ```
///
/// `ctx` is host-side syntax; guest declarations do not accept it:
///
/// ```compile_fail
/// use abi_adapter::abi;
///
/// fn tick(cx: &mut u32) -> u32 { *cx += 1; *cx }
///
/// abi! { guest("tick") fn wrong(ctx cx: &mut u32;) -> 1 => crate::tick; }
///
/// fn main() {}
/// ```
///
/// A bare target path is captured by a same-named slot parameter:
///
/// ```compile_fail,E0618
/// use abi_adapter::abi;
///
/// fn echo(value: u32) -> u32 { value }
///
/// abi! { host fn wrong([echo]) -> 1 => echo; }
///
/// fn main() {}
/// ```
///
/// Slot names must be unique within one wrapper:
///
/// ```compile_fail,E0415
/// use abi_adapter::abi;
///
/// fn sum(a: u32, b: u32) -> u32 { a + b }
///
/// abi! { host fn wrong([x], [x]) -> 1 => crate::sum; }
///
/// fn main() {}
/// ```
///
/// A slot name matching a constant in scope parses as a constant pattern:
///
/// ```compile_fail,E0005
/// use abi_adapter::abi;
///
/// const x: u32 = 1;
///
/// fn echo(value: u32) -> u32 { value }
///
/// abi! { host fn wrong([x]) -> 1 => crate::echo; }
///
/// fn main() {}
/// ```
///
/// Input and output conversions must exist for the declared width:
///
/// ```compile_fail,E0277
/// use abi_adapter::abi;
/// fn business(value: bool) -> u32 { u32::from(value) }
/// abi! { host fn wrong([x]) -> 1 => crate::business; }
/// fn main() {}
/// ```
///
/// ```compile_fail,E0277
/// use abi_adapter::abi;
/// fn business() -> bool { true }
/// abi! { host fn wrong() -> 1 => crate::business; }
/// fn main() {}
/// ```
///
/// Empty slot groups are rejected; a zero-argument business function needs no
/// group at all:
///
/// ```compile_fail
/// use abi_adapter::abi;
/// fn business(_: ()) {}
/// abi! { host fn wrong([]) -> 0 => crate::business; }
/// fn main() {}
/// ```
///
/// Boundary declarations require an explicit return width and are synchronous
/// safe functions:
///
/// ```compile_fail
/// use abi_adapter::abi;
/// fn business() {}
/// abi! { host fn wrong() => crate::business; }
/// fn main() {}
/// ```
///
/// ```compile_fail
/// use abi_adapter::abi;
/// fn business() {}
/// abi! { host async fn wrong() -> 0 => crate::business; }
/// fn main() {}
/// ```
///
/// ```compile_fail
/// use abi_adapter::abi;
/// fn business() {}
/// abi! { host unsafe fn wrong() -> 0 => crate::business; }
/// fn main() {}
/// ```
#[macro_export]
macro_rules! abi {
    (host $(#[$attr:meta])* $vis:vis fn $name:ident(
        $( [$($slot:ident),+] ),* $(,)?
    ) -> $ret:tt => $target:path;) => {
        $crate::abi!(@emit [$(#[$attr])*] [] $vis fn $name
            [] [$( [$($slot),+] ),*] -> $ret => $target);
    };
    (host $(#[$attr:meta])* $vis:vis fn $name:ident(
        ctx $ctx:ident : $ctx_ty:ty;
        $( [$($slot:ident),+] ),* $(,)?
    ) -> $ret:tt => $target:path;) => {
        $crate::abi!(@emit [$(#[$attr])*] [] $vis fn $name
            [$ctx : $ctx_ty] [$( [$($slot),+] ),*] -> $ret => $target);
    };
    (guest($symbol:literal) $(#[$attr:meta])* $vis:vis fn $name:ident(
        $( [$($slot:ident),+] ),* $(,)?
    ) -> $ret:tt => $target:path;) => {
        $crate::abi!(@emit [#[unsafe(export_name = $symbol)] $(#[$attr])*]
            [extern "C"] $vis fn $name
            [] [$( [$($slot),+] ),*] -> $ret => $target);
    };
    (@emit [$($attrs:tt)*] [$($abi:tt)*] $vis:vis fn $name:ident
        [$($ctx:ident : $ctx_ty:ty)?]
        [$( [$($slot:ident),+] ),*]
        -> $ret:tt => $target:path
    ) => {
        $($attrs)*
        $vis $($abi)* fn $name(
            $($ctx: $ctx_ty,)?
            $($($slot: ::core::primitive::u32,)+)*
        ) -> $crate::abi!(@return $ret) {
            $crate::abi!(@encode $ret; $target(
                $($ctx,)?
                $($crate::FromSlots::from_slots([$($slot),+]),)*
            ))
        }
    };
    (@return 0) => { () };
    (@return 1) => { ::core::primitive::u32 };
    (@return $other:tt) => { ::core::compile_error!("ABI returns must be 0 or 1 slots") };
    (@encode 0; $value:expr) => {{
        let _: [::core::primitive::u32; 0] = $crate::IntoSlots::<0>::into_slots($value);
    }};
    (@encode 1; $value:expr) => {
        $crate::IntoSlots::<1>::into_slots($value)[0]
    };
    (@encode $other:tt; $value:expr) => { ::core::compile_error!("ABI returns must be 0 or 1 slots") };
}
