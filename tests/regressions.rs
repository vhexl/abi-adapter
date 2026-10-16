//! Check that names in the calling code do not change `abi!` output.

use abi_adapter::{FromSlots, IntoSlots};

/// The business function keeps its own name; the wrapper is a separate item.
pub fn slot_0(x: u32) -> u32 {
    x + 1
}

abi_adapter::abi! { host pub fn slot_0_abi([x]) -> 1 => crate::slot_0; }

// A slot parameter may carry the business function's name: the qualified
// target path still selects the function, rather than the parameter.
abi_adapter::abi! { host pub fn slot_named_abi([slot_0]) -> 1 => crate::slot_0; }

#[test]
fn business_function_named_like_a_slot_still_resolves_through_its_path() {
    assert_eq!(slot_0_abi(7), 8);
    assert_eq!(slot_named_abi(7), 8);
}

// Constants in scope leave the wrapper alone as long as no slot name matches:
// a matching name would parse as a constant pattern (documented rejection,
// E0005). The macro keeps the meaning of names supplied by its caller.
#[allow(non_upper_case_globals)]
mod constants_in_scope {
    pub const RATE: u32 = 11;
    pub const BASE: u32 = 7;

    pub fn value(x: u32) -> u32 {
        x * RATE + BASE
    }

    abi_adapter::abi! { host pub fn value_abi([x]) -> 1 => crate::constants_in_scope::value; }

    #[test]
    fn constants_in_scope_do_not_disturb_the_wrapper() {
        assert_eq!(value_abi(2), 2 * RATE + BASE);
    }
}

// User code can define its own `u32`; generated parameters must still use
// Rust's built-in 32-bit type.
#[allow(non_camel_case_types)]
mod primitive_shadow {
    #[allow(dead_code)]
    type u32 = u64;

    pub fn value(x: i32) -> i32 {
        x
    }

    abi_adapter::abi! { host pub fn value_abi([x]) -> 1 => crate::primitive_shadow::value; }
    abi_adapter::abi! { guest("primitive.slot.v1") pub fn module_value_abi([x]) -> 1 => crate::primitive_shadow::value; }

    #[test]
    fn generated_slots_use_the_primitive_u32_type() {
        assert_eq!(
            value_abi(::core::primitive::u32::MAX),
            ::core::primitive::u32::MAX
        );
        assert_eq!(module_value_abi(0x8000_0000), 0x8000_0000);
    }
}

// `IntoSlots` must remain usable with an explicit trait path.
#[test]
fn explicit_trait_calls_round_trip() {
    assert_eq!(<u32 as IntoSlots<1>>::into_slots(7), [7]);
    assert_eq!(<u32 as FromSlots<1>>::from_slots([7]), 7);
}
