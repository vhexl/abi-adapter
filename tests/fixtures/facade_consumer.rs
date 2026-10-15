#![no_std]
#![forbid(unsafe_code)]

pub struct Input(u32);

impl facade::FromSlots<1> for Input {
    fn from_slots([value]: [u32; 1]) -> Self {
        Self(value)
    }
}

pub struct Output(u32);

impl facade::IntoSlots<1> for Output {
    fn into_slots(self) -> [u32; 1] {
        [self.0]
    }
}

pub fn business(input: Input) -> Output {
    Output(input.0 + 1)
}

facade::abi! { host pub fn host_boundary([value]) -> 1 => crate::business; }
facade::abi! { guest("facade.v1") pub fn guest_boundary([value]) -> 1 => crate::business; }

// Forward raw tokens so abi!'s literal 0/1 rules remain matchable.
macro_rules! declare {
    ($width:tt) => {
        facade::abi! { host pub fn forwarded_boundary([value]) -> $width => crate::business; }
    };
}

declare!(1);
