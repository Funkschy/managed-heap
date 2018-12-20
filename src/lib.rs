//! An implementation of virtual heap, inspired by VMs like the JVM.
//! It can be used while creating your own virtual machine in Rust.
//!
//! To using it, you have to implement the traits in the trace module.
//!
//! # Example
//! ```
//! extern crate managed_heap;
//!
//! use managed_heap::managed::*;
//! use managed_heap::trace::*;
//! use managed_heap::address::*;
//!
//! struct MockGcRoot {
//!     used_elems: Vec<IntegerObject>,
//! }
//!
//! impl MockGcRoot {
//!     pub fn new(used_elems: Vec<IntegerObject>) -> Self {
//!         MockGcRoot { used_elems }
//!     }
//!
//!     pub fn clear(&mut self) {
//!         self.used_elems.clear();
//!     }
//! }
//!
//! impl GcRoot<IntegerObject> for MockGcRoot {
//!     fn children<'a>(&'a mut self) -> Box<Iterator<Item = &'a mut IntegerObject> + 'a> {
//!         Box::new(self.used_elems.iter_mut())
//!     }
//! }
//!
//! #[derive(Debug)]
//! struct IntegerObject(Address);
//!
//! impl IntegerObject {
//!     pub fn new(heap: &mut ManagedHeap, value: isize) -> Self {
//!         // reserve one usize for mark byte
//!         let mut address = heap.alloc(2).unwrap();
//!
//!         address.write(false as usize);
//!         (address + 1).write(value as usize);
//!
//!         IntegerObject(address)
//!     }
//!
//!     pub fn get(&self) -> isize {
//!         *(self.0 + 1) as isize
//!     }
//! }
//!
//! impl From<Address> for IntegerObject {
//!     fn from(address: Address) -> Self {
//!         IntegerObject(address)
//!     }
//! }
//!
//! impl Into<Address> for IntegerObject {
//!     fn into(self) -> Address {
//!         self.0
//!     }
//! }
//!
//! impl Traceable for IntegerObject {
//!     fn mark(&mut self) {
//!         self.0.write(true as usize);
//!     }
//!
//!     fn unmark(&mut self) {
//!         self.0.write(false as usize);
//!     }
//!
//!     fn trace(&mut self) -> Box<Iterator<Item = &mut Address>> {
//!         unimplemented!()
//!     }
//!
//!     fn is_marked(&self) -> bool {
//!         (*self.0) != 0
//!     }
//! }
//!
//! let mut heap = ManagedHeap::new(100);
//! let mut i = IntegerObject::new(&mut heap, -42);
//!
//! assert_eq!(-42, i.get());
//! assert_eq!(false, i.is_marked());
//!
//! i.mark();
//! assert_eq!(true, i.is_marked());
//! ```

pub mod address;
mod block;
mod heap;
pub mod managed;
pub mod trace;
mod types;
