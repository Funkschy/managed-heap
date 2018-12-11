use crate::block::{Block, BlockHeader};
use core::ptr::NonNull;
use std::marker::PhantomData;
use std::ops::Deref;

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Address<'a> {
    ptr: usize,
    phantom: PhantomData<&'a usize>,
}

impl<'a> Address<'a> {
    pub fn new(ptr: NonNull<BlockHeader>) -> Self {
        unsafe {
            Address {
                ptr: ptr.as_ptr().add(1) as usize,
                phantom: PhantomData,
            }
        }
    }
}

impl<'a> From<Block> for Address<'a> {
    fn from(value: Block) -> Address<'a> {
        let ptr: NonNull<BlockHeader> = value.into();
        Address::new(ptr)
    }
}

impl<'a> Into<Block> for Address<'a> {
    fn into(self) -> Block {
        unsafe {
            let ptr = (self.ptr as *mut usize).offset(-1) as *mut BlockHeader;
            Block::from(ptr)
        }
    }
}

impl<'a> Deref for Address<'a> {
    type Target = usize;

    fn deref(&self) -> &usize {
        unsafe { (self.ptr as *mut usize).as_ref().unwrap() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn test_address_has_same_size_as_usize() {
        assert_eq!(mem::size_of::<usize>(), mem::size_of::<Address>());
    }
}
