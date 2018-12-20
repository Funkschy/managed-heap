use super::address::Address;
use super::heap::Heap;
use super::trace::{GcRoot, Traceable};
use super::types::HalfWord;

/// A virtual Heap which can be garbage collected by calling gc().
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

    /// Run the mark & sweep garbage collector.
    /// roots should return an iterator over all objects still in use.
    /// If an object is neither returned by one of the roots, nor from another
    /// object in the root.children(), it gets automatically freed.
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

        #[derive(Debug)]
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
            let mut i = IntegerObject::new(&mut heap, -42);

            assert_eq!(-42, i.get());
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

    mod complex {
        use super::*;
        use std::fmt;
        use std::iter::Iterator;

        struct MockGcRoot {
            used_elems: Vec<LinkedList>,
        }

        impl MockGcRoot {
            pub fn new(used_elems: Vec<LinkedList>) -> Self {
                MockGcRoot { used_elems }
            }

            pub fn clear(&mut self) {
                self.used_elems.clear();
            }
        }

        impl GcRoot<LinkedList> for MockGcRoot {
            fn children<'a>(&'a mut self) -> Box<Iterator<Item = &'a mut LinkedList> + 'a> {
                Box::new(self.used_elems.iter_mut())
            }
        }

        #[derive(Copy, Clone)]
        struct LinkedList(Address);

        impl LinkedList {
            pub fn new(heap: &mut ManagedHeap, value: isize, next: Option<LinkedList>) -> Self {
                // [mark byte, value, next], each 1 byte
                let mut address = heap.alloc(3).unwrap();

                address.write(false as usize);
                address.add(1).write(value as usize);

                let next = next.map(|n| n.0.into()).unwrap_or(0);
                address.add(2).write(next);

                LinkedList(address)
            }

            pub fn next(self) -> Option<LinkedList> {
                let next = *self.0.add(2);

                if next != 0 {
                    let address = Address::from(next);
                    Some(LinkedList(address))
                } else {
                    None
                }
            }

            pub fn value(self) -> isize {
                *self.0.add(1) as isize
            }

            pub fn iter(self) -> Iter {
                Iter {
                    current: Some(self),
                }
            }
        }

        impl From<Address> for LinkedList {
            fn from(address: Address) -> Self {
                LinkedList(address)
            }
        }

        impl Into<Address> for LinkedList {
            fn into(self) -> Address {
                self.0
            }
        }

        impl Traceable for LinkedList {
            fn mark(&mut self) {
                self.0.write(true as usize);
                if let Some(mut next) = self.next() {
                    next.mark();
                }
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

        impl fmt::Debug for LinkedList {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let string_list: Vec<String> = self
                    .iter()
                    .map(|l| l.value())
                    .map(|v| format!("{}", v))
                    .collect();

                write!(f, "[{}]", string_list.join(", "))
            }
        }

        struct Iter {
            current: Option<LinkedList>,
        }

        impl Iterator for Iter {
            type Item = LinkedList;

            fn next(&mut self) -> Option<LinkedList> {
                let curr = self.current;
                self.current = self.current.and_then(|c| c.next());
                curr
            }
        }

        #[test]
        fn test_linked_list_object_constructor() {
            let mut heap = ManagedHeap::new(200);

            let list = LinkedList::new(&mut heap, 3, None);
            assert_eq!(1, heap.num_used_blocks());

            let list = LinkedList::new(&mut heap, 2, Some(list));
            assert_eq!(2, heap.num_used_blocks());

            let list = LinkedList::new(&mut heap, 1, Some(list));
            assert_eq!(3, heap.num_used_blocks());

            let sum: isize = list.iter().map(|list| list.value()).sum();
            assert_eq!(sum, 6);

            assert_eq!(3, heap.num_used_blocks());
            assert_eq!(1, heap.num_free_blocks());
        }

        macro_rules! list {
            ($heap:expr; $($elems:tt)+) => {
                construct_list!($heap; [$($elems)*])
            };
        }

        macro_rules! construct_list {
            ($heap:expr; [] $head:expr, $($elem:expr),*) => {
                {
                    let mut list = LinkedList::new($heap, $head, None);
                    $(list = LinkedList::new($heap, $elem, Some(list));)*
                    list
                }
            };
            ($heap:expr; [$first:tt $($rest:tt)*] $($reversed:tt)*) => {
                construct_list!($heap; [$($rest)*] $first $($reversed)*)
            };
        }

        #[test]
        fn test_single_linked_list_gets_freed_when_not_marked() {
            let mut heap = ManagedHeap::new(100);
            let list = LinkedList::new(&mut heap, 1, None);
            assert_eq!("[1]", format!("{:?}", list));

            let mut gc_root = MockGcRoot::new(vec![list]);
            assert_eq!(1, heap.num_used_blocks());
            assert_eq!(1, heap.num_free_blocks());

            {
                let mut roots: Vec<&mut GcRoot<LinkedList>> = vec![&mut gc_root];
                heap.gc(&mut roots[..]);
                assert_eq!(1, heap.num_used_blocks());
                assert_eq!(1, heap.num_free_blocks());
            }

            {
                let mut roots: Vec<&mut GcRoot<LinkedList>> = vec![&mut gc_root];
                heap.gc(&mut roots[..]);
                assert_eq!(1, heap.num_used_blocks());
                assert_eq!(1, heap.num_free_blocks());
            }

            gc_root.clear();
            let mut roots: Vec<&mut GcRoot<LinkedList>> = vec![&mut gc_root];
            heap.gc(&mut roots[..]);
            assert_eq!(0, heap.num_used_blocks());
            assert_eq!(1, heap.num_free_blocks());
        }

        #[test]
        fn test_double_linked_list_gets_freed_when_not_marked() {
            let mut heap = ManagedHeap::new(100);
            let list = list![&mut heap; 1, 2];
            assert_eq!("[1, 2]", format!("{:?}", list));

            let mut gc_root = MockGcRoot::new(vec![list]);
            assert_eq!(2, heap.num_used_blocks());
            assert_eq!(1, heap.num_free_blocks());

            {
                let mut roots: Vec<&mut GcRoot<LinkedList>> = vec![&mut gc_root];
                heap.gc(&mut roots[..]);
                assert_eq!(2, heap.num_used_blocks());
                assert_eq!(1, heap.num_free_blocks());
                assert_eq!(false, list.is_marked());
            }

            {
                let mut roots: Vec<&mut GcRoot<LinkedList>> = vec![&mut gc_root];
                heap.gc(&mut roots[..]);
                assert_eq!(2, heap.num_used_blocks());
                assert_eq!(1, heap.num_free_blocks());
                assert_eq!(false, list.is_marked());
            }

            gc_root.clear();
            let mut roots: Vec<&mut GcRoot<LinkedList>> = vec![&mut gc_root];
            heap.gc(&mut roots[..]);
            assert_eq!(0, heap.num_used_blocks());
            assert_eq!(1, heap.num_free_blocks());
        }

        #[test]
        fn test_triple_linked_list_gets_freed_when_not_marked() {
            let mut heap = ManagedHeap::new(1000);
            let list = list![&mut heap; 1, 2, 3];

            assert_eq!("[1, 2, 3]", format!("{:?}", list));

            let mut gc_root = MockGcRoot::new(vec![list]);
            assert_eq!(3, heap.num_used_blocks());
            assert_eq!(1, heap.num_free_blocks());

            {
                let mut roots: Vec<&mut GcRoot<LinkedList>> = vec![&mut gc_root];
                heap.gc(&mut roots[..]);
                assert_eq!(3, heap.num_used_blocks());
                assert_eq!(1, heap.num_free_blocks());
            }

            {
                let mut roots: Vec<&mut GcRoot<LinkedList>> = vec![&mut gc_root];
                heap.gc(&mut roots[..]);
                assert_eq!(3, heap.num_used_blocks());
                assert_eq!(1, heap.num_free_blocks());
            }

            gc_root.clear();
            let mut roots: Vec<&mut GcRoot<LinkedList>> = vec![&mut gc_root];
            heap.gc(&mut roots[..]);
            assert_eq!(0, heap.num_used_blocks());
            assert_eq!(1, heap.num_free_blocks());
        }
    }
}
