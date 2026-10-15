#![allow(unused_macros)]

// An unqualified compile_error! in an external macro would resolve here and
// turn both the return type and the function body into unit without calling
// the business function.
macro_rules! compile_error {
    ($message:expr) => {
        ()
    };
}

pub fn business() -> u32 {
    7
}

abi_adapter::abi! { host pub fn boundary() -> 2 => crate::business; }
