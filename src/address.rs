use crate::block::{Block, BlockHeader};
use core::ptr::NonNull;
use std::collections::BTreeMap;
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

#[derive(Default)]
pub struct LookupTable<'a>(BTreeMap<Address<'a>, Block>);

impl<'a> LookupTable<'a> {
    pub fn add_block(&mut self, block: Block) -> Address<'a> {
        let address = Address::from(block);
        self.0.insert(address, block);
        address
    }

    pub fn remove_block(&mut self, address: Address<'a>) -> Option<Block> {
        self.0.remove(&address)
    }
}

// only used in unit tests
#[cfg(test)]
impl<'a> LookupTable<'a> {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}
