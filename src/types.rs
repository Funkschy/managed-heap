use std::mem;

#[cfg(target_pointer_width = "64")]
mod inner {
    pub use std::u32;

    pub const HALF_WORD_MAX: u32 = u32::MAX;

    pub type HalfWord = u32;

    pub type Word = u64;
}

#[cfg(target_pointer_width = "32")]
mod inner {
    pub use std::u16;

    pub const HALF_WORD_MAX: u16 = u16::MAX;

    pub type HalfWord = u16;

    pub type Word = u32;
}

pub use self::inner::*;

pub const WORD_SIZE: usize = mem::size_of::<usize>();
