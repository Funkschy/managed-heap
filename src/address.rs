use crate::block::header::BlockHeader;
use crate::block::Block;
use core::ptr::NonNull;
use std::ops::Deref;

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct Address {
    ptr: usize,
}

impl Address {
    pub fn new(ptr: NonNull<BlockHeader>) -> Self {
        unsafe {
            Address {
                ptr: ptr.as_ptr().add(1) as usize,
            }
        }
    }

    fn from_usize_ptr(ptr: *mut usize) -> Self {
        Address { ptr: ptr as usize }
    }
}

impl Address {
    pub fn add(&self, value: usize) -> Self {
        let ptr = self.ptr as *mut usize;
        unsafe { Address::from_usize_ptr(ptr.add(value)) }
    }
}

impl Address {
    pub fn as_mut(&mut self) -> *mut usize {
        self.ptr as *mut usize
    }

    pub fn write(&mut self, value: usize) {
        let ptr = self.as_mut();
        unsafe {
            *ptr = value;
        }
    }
}

impl From<Block> for Address {
    fn from(value: Block) -> Address {
        let ptr: NonNull<BlockHeader> = value.into();
        Address::new(ptr)
    }
}

impl Into<Block> for Address {
    fn into(self) -> Block {
        unsafe {
            let ptr = (self.ptr as *mut usize).offset(-1) as *mut BlockHeader;
            Block::from(ptr)
        }
    }
}

impl Deref for Address {
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
