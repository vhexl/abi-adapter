#![no_std]
#![forbid(unsafe_code)]

pub fn business(value: u32) -> u32 {
    value + 1
}

adapter::abi! { host pub fn host_boundary([value]) -> 1 => crate::business; }
adapter::abi! { guest("renamed.v1") pub fn guest_boundary([value]) -> 1 => crate::business; }
