#![allow(unused_macros)]

// An unqualified compile_error! in an external macro would resolve here and
// hide the invalid return width without calling the business function.
macro_rules! compile_error {
    ($message:expr) => {
        ()
    };
}

pub fn business() -> u32 {
    7
}

abi_adapter::abi! { host pub fn boundary() -> 2 => crate::business; }
