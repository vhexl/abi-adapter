//! The slot conversion traits (`docs/architecture.md` §3).
//!
//! The traits answer only one question: how does a value map to `N` 32-bit
//! slots (and back). They do not imply a destination — a conversion is not a
//! direct return, a buffer write, or an error channel.

/// Decode a value from exactly `N` 32-bit slots.
///
/// Implementations must handle any slot combination. There is no failure
/// channel: implementations may panic, and the adapter does not catch or
/// translate panics. A panic raised inside a `guest` wrapper cannot unwind
/// across the `extern "C"` boundary and aborts the program instead; see the
/// [`abi!`](crate::abi) documentation.
pub trait FromSlots<const N: usize>: Sized {
    /// Decode `Self` from `slots`.
    fn from_slots(slots: [u32; N]) -> Self;
}

/// Encode a value into exactly `N` 32-bit slots.
///
/// Same contract as [`FromSlots`]: no failure channel, no destination implied.
pub trait IntoSlots<const N: usize>: Sized {
    /// Encode `self` into `N` slots.
    fn into_slots(self) -> [u32; N];
}
