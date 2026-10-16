//! Fixed-width slot conversion traits. They do not select an output memory
//! destination or provide an error return.

/// Decode `Self` from exactly `N` slots.
///
/// [`abi!`](crate::abi) infers `Self` from the business parameter type and
/// `N` from its bracketed slot group.
///
/// # Example
///
/// ```
/// use abi_adapter::FromSlots;
///
/// #[derive(Debug, PartialEq)]
/// struct Point { x: u32, y: u32 }
///
/// impl FromSlots<2> for Point {
///     fn from_slots([x, y]: [u32; 2]) -> Self {
///         Self { x, y }
///     }
/// }
///
/// assert_eq!(Point::from_slots([3, 5]), Point { x: 3, y: 5 });
/// ```
///
/// # Contract
///
/// Implementations must remain memory-safe for every slot bit pattern.
/// There is no error return; implementations may panic. The adapter does not
/// catch panics, and a panic in a guest `extern "C"` wrapper terminates the
/// program. See [`abi!`](crate::abi) for host and guest panic behavior.
///
/// Fallible validation can stay in the business function, with an agreed
/// status value as its result.
pub trait FromSlots<const N: usize>: Sized {
    /// Decode `slots` in the agreed order.
    fn from_slots(slots: [u32; N]) -> Self;
}

/// Encode `Self` as exactly `N` slots.
///
/// # Example
///
/// ```
/// use abi_adapter::IntoSlots;
///
/// struct Status(u32);
///
/// impl IntoSlots<1> for Status {
///     fn into_slots(self) -> [u32; 1] {
///         [self.0]
///     }
/// }
///
/// assert_eq!(Status(7).into_slots(), [7]);
/// ```
///
/// # Return width
///
/// This trait supports arbitrary `N`, but [`abi!`](crate::abi) directly
/// returns only zero or one slot. A `-> 0` declaration still invokes
/// `IntoSlots<0>`, discards the empty array, and returns `()`.
/// Larger results require a separate memory or resource protocol.
///
/// Panic behavior follows the contract of [`FromSlots`].
pub trait IntoSlots<const N: usize>: Sized {
    /// Encode `self` in the agreed slot order.
    fn into_slots(self) -> [u32; N];
}
