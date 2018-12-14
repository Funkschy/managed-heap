use super::address::Address;
use super::heap::Heap;
use super::trace::{GcRoot, Traceable};
use super::types::HalfWord;

pub struct ManagedHeap {
    heap: Heap,
}

impl ManagedHeap {
    /// Expects the heap size in bytes.
    pub fn new(size: usize) -> Self {
        let heap = unsafe { Heap::new(size) };

        ManagedHeap { heap }
    }
}

impl ManagedHeap {
    pub fn num_used_blocks(&self) -> usize {
        self.heap.num_used_blocks()
    }

    pub fn num_free_blocks(&self) -> usize {
        self.heap.num_free_blocks()
    }
}

impl ManagedHeap {
    /// Takes the blocksize as a number of usize values.
    /// The size in bytes of the block is therefore size * mem::size_of::<usize>()
    /// (technically + one more usize to store information about the block)
    pub fn alloc(&mut self, size: HalfWord) -> Option<Address> {
        self.heap.alloc(size)
    }

    pub fn gc<T>(&mut self, roots: &mut [&mut GcRoot<T>])
    where
        T: Traceable + From<Address> + Into<Address>,
    {
        for traceable in roots.iter_mut().flat_map(|r| r.children()) {
            traceable.mark();
        }

        let freeable: Vec<Address> = self
            .heap
            .used()
            .map(|b| T::from(Address::from(*b)))
            .filter(|t| !t.is_marked())
            .map(|t| t.into())
            .collect();

        for a in freeable {
            self.heap.free(a);
        }

        self.heap
            .used()
            .map(|b| Address::from(*b))
            .map(T::from)
            .for_each(|mut t| t.unmark());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod simple {
        use super::*;

        struct MockGcRoot {
            used_elems: Vec<IntegerObject>,
        }

        impl MockGcRoot {
            pub fn new(used_elems: Vec<IntegerObject>) -> Self {
                MockGcRoot { used_elems }
            }

            pub fn clear(&mut self) {
                self.used_elems.clear();
            }
        }

        impl GcRoot<IntegerObject> for MockGcRoot {
            fn children<'a>(&'a mut self) -> Box<Iterator<Item = &'a mut IntegerObject> + 'a> {
                Box::new(self.used_elems.iter_mut())
            }
        }

        struct IntegerObject(Address);

        impl IntegerObject {
            pub fn new(heap: &mut ManagedHeap, value: isize) -> Self {
                // reserve one usize for mark byte
                let mut address = heap.alloc(2).unwrap();

                address.write(false as usize);
                address.add(1).write(value as usize);

                IntegerObject(address)
            }

            pub fn get(&self) -> isize {
                *self.0.add(1) as isize
            }
        }

        impl From<Address> for IntegerObject {
            fn from(address: Address) -> Self {
                IntegerObject(address)
            }
        }

        impl Into<Address> for IntegerObject {
            fn into(self) -> Address {
                self.0
            }
        }

        impl Traceable for IntegerObject {
            fn mark(&mut self) {
                self.0.write(true as usize);
            }

            fn unmark(&mut self) {
                self.0.write(false as usize);
            }

            fn trace(&mut self) -> Box<Iterator<Item = &mut Address>> {
                unimplemented!()
            }

            fn is_marked(&self) -> bool {
                (*self.0) != 0
            }
        }

        #[test]
        fn test_integer_object_constructor() {
            let mut heap = ManagedHeap::new(100);
            let mut i = IntegerObject::new(&mut heap, 42);

            assert_eq!(42, i.get());
            assert_eq!(false, i.is_marked());

            i.mark();
            assert_eq!(true, i.is_marked());
        }

        #[test]
        fn test_integer_gets_freed_when_not_marked() {
            let mut heap = ManagedHeap::new(100);
            let i = IntegerObject::new(&mut heap, 42);

            let mut gc_root = MockGcRoot::new(vec![i]);
            assert_eq!(1, heap.num_used_blocks());
            assert_eq!(1, heap.num_free_blocks());

            {
                let mut roots: Vec<&mut GcRoot<IntegerObject>> = vec![&mut gc_root];
                heap.gc(&mut roots[..]);
                assert_eq!(1, heap.num_used_blocks());
                assert_eq!(1, heap.num_free_blocks());
            }

            gc_root.clear();
            let mut roots: Vec<&mut GcRoot<IntegerObject>> = vec![&mut gc_root];
            heap.gc(&mut roots[..]);
            assert_eq!(0, heap.num_used_blocks());
            assert_eq!(1, heap.num_free_blocks());
        }
    }
}
