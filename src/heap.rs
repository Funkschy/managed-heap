use crate::block::BlockHeader;
use crate::block_vec::BlockVec;

use core::marker::PhantomData;
use core::ptr::NonNull;
use std::alloc::{alloc, dealloc, Layout};
use std::mem;
use std::ops::Deref;
use std::u16;

#[derive(Copy, Clone)]
pub struct Address<T> {
    ptr: usize,
    phantom: PhantomData<T>,
}

impl<T> Address<T> {
    fn new(ptr: usize) -> Self {
        Address {
            ptr,
            phantom: PhantomData,
        }
    }
}

impl<T> Deref for Address<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*(self.ptr as *mut T) }
    }
}

impl<T> From<NonNull<T>> for Address<T> {
    fn from(value: NonNull<T>) -> Self {
        let us = value.as_ptr() as usize;
        Self::new(us)
    }
}

struct Heap {
    size: usize,
    data: *mut usize,
    layout: Layout,
    free_blocks: BlockVec,
    used_blocks: BlockVec,
}

impl Heap {
    pub unsafe fn new(layout: Layout) -> Self {
        let size = layout.size();

        if size > u16::MAX as usize {
            panic!("Size too big (MAX: {})", u16::MAX);
        }

        let data = NonNull::new(alloc(layout))
            .unwrap()
            .cast::<usize>()
            .as_ptr();

        Heap {
            size,
            data,
            layout,
            free_blocks: BlockVec::from_raw(data, size as u16),
            used_blocks: BlockVec::default(),
        }
    }

    unsafe fn address<T>(&mut self, address: &Address<T>) -> *mut T {
        self.data.add(address.ptr) as *mut T
    }
}

impl Heap {
    fn round_up(n: u16, m: u16) -> u16 {
        ((n + m - 1) / m) * m
    }

    fn alloc(&mut self, size: u16) -> Option<Address<BlockHeader>> {
        let align = mem::align_of::<usize>() as u16;
        let h_size = mem::size_of::<usize>() as u16;

        let total_size = Heap::round_up(size + h_size, align);
        let mut block = self.free_blocks.get_block(total_size)?;

        if block.size() > (total_size + h_size * 2) {
            unsafe {
                let (first, second) = block.split_after(total_size);
                block = first;
                self.free_blocks.add_block(second);
            }
        }

        let block: NonNull<BlockHeader> = block.into();
        Some(Address::from(block))
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.data as *mut u8, self.layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn test_alloc_returns_correct_type() {
        unsafe {
            let layout = Layout::from_size_align(4096, mem::align_of::<usize>());
            let mut heap = Heap::new(layout.unwrap());

            let address = heap.alloc(10).unwrap();
            let expected;

            #[cfg(target_pointer_width = "64")]
            {
                // (header size (8) + 10) rounded to next multiple of 8
                expected = 24;
            }

            #[cfg(target_pointer_width = "32")]
            {
                // (header size (4) + 10) rounded to next multiple of 4
                expected = 16;
            }

            assert_eq!(expected, (*address).block_size());
        }
    }
}
