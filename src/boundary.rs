//! Slot-based host wrappers and guest exports.

/// Declare one host Rust wrapper or guest `extern "C"` export.
///
/// # Example
///
/// ```
/// use abi_adapter::abi;
///
/// fn double(value: u32) -> u32 { value * 2 }
///
/// abi! { host pub fn double_abi([value]) -> 1 => crate::double; }
///
/// fn main() {
///     assert_eq!(double_abi(21), 42);
/// }
/// ```
///
/// # Common uses
///
/// ## Custom argument group
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
/// ## Guest export
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
/// ## Host context
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
/// # Declaration rules
///
/// ```text
/// abi! { host #[attrs] VIS fn NAME([a, b], [c])         -> R => PATH; }
/// abi! { host #[attrs] VIS fn NAME(ctx CX: CTY; [a, b]) -> R => PATH; }
/// abi! { host #[attrs] VIS fn NAME(ctx CX: CTY;)        -> R => PATH; }
/// abi! { guest("SYMBOL") #[attrs] VIS fn NAME([a, b])   -> R => PATH; }
/// ```
///
/// `R` must be the literal `0` or `1`. Attributes and visibility apply only
/// to the generated wrapper. No-argument functions use `fn name()`; context-only
/// host functions use `fn name(ctx cx: Ctx;)`.
///
/// ## Slots and type inference
///
/// - Each nonempty bracketed group maps to one business argument.
///   Names become `::core::primitive::u32` parameters in written order.
/// - Each group calls [`FromSlots::from_slots`](crate::FromSlots::from_slots).
///   The business signature determines `Self`; the group size determines `N`.
/// - Const-generic `[u32; N]` arguments and `impl IntoSlots<1>` results are supported.
///
/// A successful call decodes arguments from left to right, calls the business
/// function once, then encodes its result. Each conversion runs once.
/// A panic stops later steps.
///
/// ## Return values
///
/// | Declaration | Required business result trait | Wrapper result |
/// | --- | --- | --- |
/// | `-> 0` | [`IntoSlots<0>`](crate::IntoSlots) | `()` |
/// | `-> 1` | [`IntoSlots<1>`](crate::IntoSlots) | `u32` |
///
/// `-> 0` still invokes result conversion before discarding the empty array.
/// Host and guest wrappers use the same slot rules. Larger results need a
/// separate memory or resource protocol; see `docs/buffers.md` in the repository.
/// The crate does not access result memory or manage resources.
///
/// ## ctx
///
/// `ctx` is host-only, precedes all slot groups, and ends with a semicolon.
/// It passes unchanged as the first business argument. Its type is used
/// verbatim, including lifetimes such as `Ctx<'_>`.
///
/// # Export safety and panics
///
/// `guest("SYMBOL")` generates a safe `extern "C"` function carrying
/// `#[unsafe(export_name = ...)]`. The export name must be a string literal.
/// Users must ensure it does not conflict with another linked definition,
/// especially a runtime function with a different signature. Callers must
/// use the generated parameter and result types. These `export_name`
/// obligations apply even though calling the Rust wrapper requires no `unsafe`.
///
/// The current Rust compiler's `unsafe_code` lint does not report these
/// attributes from another crate's macro. Guest declarations therefore work
/// under `#![forbid(unsafe_code)]`. Adding `#[allow(unsafe_code)]` conflicts
/// with `forbid` (E0453).
///
/// Panics from decoding, the business call, and encoding are not caught or
/// converted to error codes. Host wrappers follow the configured Rust panic
/// strategy: unwind to the caller or terminate with `panic = "abort"`.
///
/// A guest `extern "C"` wrapper cannot unwind outwards: a panic terminates the
/// program (a Wasm trap). Cleanup of previously decoded values is not guaranteed.
/// Handle expected errors in the business function with an agreed result.
///
/// # Limitations
///
/// ## Paths and names
///
/// Use a qualified target such as `crate::business::score`. A short name can be
/// hidden by a same-named slot parameter (E0618). `<Type as Trait>::method` is
/// not accepted by the macro's path syntax; use an unambiguous `Type::method`
/// or a concrete forwarding function.
///
/// Slot names must be unique (E0415) and must not match visible constants,
/// unit structs such as `struct X;`, or fieldless enum variants. Such names
/// may be parsed as constant patterns (E0005); rename the slot or move the
/// declaration to a different scope. Caller-provided names retain their scope.
///
/// Slot counts are checked against business types. Swapped slots of the same
/// type require protocol tests to detect.
///
/// ## Generic functions
///
/// Specify unresolved generics, for example `crate::generic::<u32>`, or use a
/// concrete forwarding function. Argument-position `impl Trait` can be
/// similarly ambiguous (E0283).
///
/// ## Forwarding return widths
///
/// An outer macro must capture the return width as `$width:tt`. Capturing it
/// as `literal` or `expr` prevents matching it against the inner `0` / `1` rules.
/// See `docs/guide.md` for common problems and fixes.
///
/// # Examples that do not compile
///
/// ## Argument width mismatch
///
/// `u32` requires one input slot:
///
/// ```compile_fail
/// use abi_adapter::abi;
///
/// fn echo(value: u32) -> u32 { value }
///
/// abi! { host fn wrong([x, y]) -> 1 => crate::echo; }
///
/// fn main() {}
/// ```
///
/// ## Unresolved generics
///
/// Use `crate::echo::<u32>` or a concrete forwarding function:
///
/// ```compile_fail
/// use abi_adapter::{abi, IntoSlots};
///
/// fn echo<T: IntoSlots<1>>(value: T) -> T { value }
///
/// abi! { host fn wrong([x]) -> 1 => crate::echo; }
///
/// fn main() {}
/// ```
///
/// Argument-position `impl Trait` also requires a concrete type:
///
/// ```compile_fail
/// use abi_adapter::{abi, IntoSlots};
///
/// fn echo(value: impl IntoSlots<1>) -> u32 { value.into_slots()[0] }
///
/// abi! { host fn wrong([x]) -> 1 => crate::echo; }
///
/// fn main() {}
/// ```
///
/// ## Unsupported return width
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
/// ## Guest context
///
/// Use `host` for `ctx`:
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
/// ## Paths and names
///
/// Qualify the target as `crate::echo`:
///
/// ```compile_fail
/// use abi_adapter::abi;
///
/// fn echo(value: u32) -> u32 { value }
///
/// abi! { host fn wrong([echo]) -> 1 => echo; }
///
/// fn main() {}
/// ```
///
/// Use distinct slot names:
///
/// ```compile_fail
/// use abi_adapter::abi;
///
/// fn sum(a: u32, b: u32) -> u32 { a + b }
///
/// abi! { host fn wrong([x], [x]) -> 1 => crate::sum; }
///
/// fn main() {}
/// ```
///
/// Avoid visible constants as slot names:
///
/// ```compile_fail
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
/// ## Missing conversions
///
/// `bool` has no built-in slot conversions:
///
/// ```compile_fail
/// use abi_adapter::abi;
/// fn business(value: bool) -> u32 { u32::from(value) }
/// abi! { host fn wrong([x]) -> 1 => crate::business; }
/// fn main() {}
/// ```
///
/// ```compile_fail
/// use abi_adapter::abi;
/// fn business() -> bool { true }
/// abi! { host fn wrong() -> 1 => crate::business; }
/// fn main() {}
/// ```
///
/// ## Empty argument group
///
/// A function with no arguments uses `fn name()`:
///
/// ```compile_fail
/// use abi_adapter::abi;
/// fn business(_: ()) {}
/// abi! { host fn wrong([]) -> 0 => crate::business; }
/// fn main() {}
/// ```
///
/// ## Missing return width or unsupported modifiers
///
/// The return width is required; wrappers accept neither `async` nor `unsafe fn`:
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
        $(ctx $ctx:ident : $ctx_ty:ty;)?
        $( [$($slot:ident),+] ),* $(,)?
    ) -> $ret:tt => $target:path;) => {
        $crate::abi!(@emit [$(#[$attr])*] [] $vis fn $name
            [$($ctx : $ctx_ty)?] [$( [$($slot),+] ),*] -> $ret => $target);
    };
    (guest($symbol:literal) $(#[$attr:meta])* $vis:vis fn $name:ident(
        $( [$($slot:ident),+] ),* $(,)?
    ) -> $ret:tt => $target:path;) => {
        $crate::abi!(@emit [#[unsafe(export_name = $symbol)] $(#[$attr])*]
            [extern "C"] $vis fn $name
            [] [$( [$($slot),+] ),*] -> $ret => $target);
    };
    // These fallback rules only match after the supported forms above fail.
    // Keep errors inside an attributed function so cfg/cfg_attr can remove it.
    (host $(#[$attr:meta])* $vis:vis fn $name:ident(
        $(ctx $ctx:ident : $ctx_ty:ty;)?
        $( [$($slot:ident),*] ),* $(,)?
    ) -> $ret:tt => $target:path;) => {
        $crate::abi!(@error [$(#[$attr])*] [] $vis fn $name
            "ABI slot groups cannot be empty; list at least one slot in each group, or omit all groups for a function with no arguments");
    };
    (guest($symbol:literal) $(#[$attr:meta])* $vis:vis fn $name:ident(
        $( [$($slot:ident),*] ),* $(,)?
    ) -> $ret:tt => $target:path;) => {
        $crate::abi!(@error [#[unsafe(export_name = $symbol)] $(#[$attr])*]
            [extern "C"] $vis fn $name
            "ABI slot groups cannot be empty; list at least one slot in each group, or omit all groups for a function with no arguments");
    };
    (guest($symbol:literal) $(#[$attr:meta])* $vis:vis fn $name:ident(
        ctx $($args:tt)*
    ) -> $ret:tt => $target:path;) => {
        $crate::abi!(@error [#[unsafe(export_name = $symbol)] $(#[$attr])*]
            [extern "C"] $vis fn $name
            "ABI guest functions do not accept ctx; use host for a context parameter, or remove ctx from the guest declaration");
    };
    (host $(#[$attr:meta])* $vis:vis fn $name:ident(
        $($args:tt)*
    ) => $target:path;) => {
        $crate::abi!(@error [$(#[$attr])*] [] $vis fn $name
            "ABI return width is missing; add -> 0 or -> 1 before =>");
    };
    (guest($symbol:literal) $(#[$attr:meta])* $vis:vis fn $name:ident(
        $($args:tt)*
    ) => $target:path;) => {
        $crate::abi!(@error [#[unsafe(export_name = $symbol)] $(#[$attr])*]
            [extern "C"] $vis fn $name
            "ABI return width is missing; add -> 0 or -> 1 before =>");
    };
    (@error [$($attrs:tt)*] [$($abi:tt)*] $vis:vis fn $name:ident $message:literal) => {
        $($attrs)*
        $vis $($abi)* fn $name() {
            ::core::compile_error!($message);
        }
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
    // Report an unsupported width once, from @encode in the function body.
    (@return $other:tt) => { () };
    (@encode 0; $value:expr) => {{
        let _: [::core::primitive::u32; 0] = $crate::IntoSlots::<0>::into_slots($value);
    }};
    (@encode 1; $value:expr) => {
        $crate::IntoSlots::<1>::into_slots($value)[0]
    };
    (@encode $other:tt; $value:expr) => {
        ::core::compile_error!("ABI returns must be 0 or 1 slots; use -> 0 or -> 1")
    };
}
