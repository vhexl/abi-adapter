//! Built-in slot conversions: `()`, `u32`, `i32`, `f32`, and `[u32; N]`.
//!
//! Use custom types for additional formats. Generic implementations may
//! overlap these implementations (E0119). Representations are listed on the
//! crate's main page.

use crate::{FromSlots, IntoSlots};

impl FromSlots<0> for () {
    fn from_slots(_slots: [u32; 0]) -> Self {}
}

impl IntoSlots<0> for () {
    fn into_slots(self) -> [u32; 0] {
        []
    }
}

impl FromSlots<1> for u32 {
    fn from_slots(slots: [u32; 1]) -> Self {
        slots[0]
    }
}

impl IntoSlots<1> for u32 {
    fn into_slots(self) -> [u32; 1] {
        [self]
    }
}

impl FromSlots<1> for i32 {
    // Preserve the signed integer's original 32 bits.
    fn from_slots(slots: [u32; 1]) -> Self {
        slots[0] as i32
    }
}

impl IntoSlots<1> for i32 {
    fn into_slots(self) -> [u32; 1] {
        [self as u32]
    }
}

impl FromSlots<1> for f32 {
    // Preserve floating-point bits without a numeric cast.
    fn from_slots(slots: [u32; 1]) -> Self {
        f32::from_bits(slots[0])
    }
}

impl IntoSlots<1> for f32 {
    fn into_slots(self) -> [u32; 1] {
        [self.to_bits()]
    }
}

impl<const N: usize> FromSlots<N> for [u32; N] {
    fn from_slots(slots: [u32; N]) -> Self {
        slots
    }
}

impl<const N: usize> IntoSlots<N> for [u32; N] {
    fn into_slots(self) -> [u32; N] {
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::{FromSlots, IntoSlots};

    #[test]
    fn unit_converts_to_and_from_zero_slots() {
        let () = <() as FromSlots<0>>::from_slots([]);
        assert_eq!(().into_slots(), []);
    }

    #[test]
    fn zero_length_array_converts_without_changing_slots() {
        let slots: [u32; 0] = [];
        assert_eq!(<[u32; 0] as FromSlots<0>>::from_slots(slots), slots);
        assert_eq!(IntoSlots::<0>::into_slots(slots), slots);
    }

    #[test]
    fn signed_slots_preserve_twos_complement_bits() {
        for (value, bits) in [
            (i32::MIN, 0x8000_0000),
            (-1, 0xffff_ffff),
            (0, 0),
            (1, 1),
            (i32::MAX, 0x7fff_ffff),
        ] {
            assert_eq!(value.into_slots(), [bits]);
            assert_eq!(i32::from_slots([bits]), value);
        }
    }

    #[test]
    fn float_slots_preserve_zero_infinity_and_nan_bits() {
        for bits in [
            0x0000_0000, // positive zero
            0x8000_0000, // negative zero
            0x0000_0001, // smallest positive subnormal
            0x7f7f_ffff, // largest finite value
            0x7f80_0000, // positive infinity
            0xff80_0000, // negative infinity
            0x7fc0_1234, // quiet NaN with a payload
            0xffc0_5678, // negative quiet NaN with a payload
            0x7f80_0001, // signaling NaN
        ] {
            assert_eq!(f32::from_slots([bits]).to_bits(), bits);
            assert_eq!(f32::from_bits(bits).into_slots(), [bits]);
        }
    }
}
