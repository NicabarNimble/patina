//! Example from Issue #1
//!
//! ```compile_fail
//! struct Thing {
//!     rcu: Arcu<std::cell::RefCell<u8>, GlobalEpochCounterPool>,
//! }
//! impl Thing {
//!     fn send(&self) {
//!         std::thread::scope(|scope| {
//!             scope.spawn(|| {
//!                 let mut ref_mut = self.rcu.read();
//!                 let mut ref_mut = ref_mut.borrow_mut();
//!                 *ref_mut = 1;
//!                 println!("{}", ref_mut);
//!             });
//!
//!             let mut ref_mut = self.rcu.read();
//!             let mut ref_mut = ref_mut.borrow_mut();
//!             *ref_mut = 2;
//!             println!("{}", ref_mut);
//!         });
//!     }
//! }
//!
//! let thing = Thing {
//!     rcu: Arcu::new(std::cell::RefCell::new(0), GlobalEpochCounterPool)
//! };
//!
//! thing.send();
//! ```
