// #![cfg_attr(not(feature = "std"), no_std)]

#![deny(clippy::undocumented_unsafe_blocks)]
#![warn(missing_docs)]

//! Arc based Rcu implementation originally implementated in [mthom/scryer-prolog#1980](https://github.com/mthom/scryer-prolog/pull/1980)
//!
//! ```text
//! A r c
//!   R c u
//! A r c u
//! ```
//!
//! The atomics based version performs lock-free[^1] reads.
//! By using Arc we keep the **r**ead-**c**ritical-**s**ection short and free of user defined code and
//! automatically perform cleanup when no reference remains.
//!
//! To coordinate reads and writes [EpochCounter]s from an [EpochCounterPool] are used.
//! Each read used an `EpochCounter` from the `EpochCounterPool` of the `Arcu` incrementing it once before entering the RCS and once more on leaving the RCS.
//! Each write checks against all `EpochCounter`s in the pool, blocking until it is safe to decrement the strong count of the `Arc` that was replaced by the write.
//!
//! [^1]: when using thread local epoch counter with the global epoch counter pool, the initial read may block while adding the threads epoch counter to the pool
//!
extern crate alloc;

pub mod epoch_counters;

use alloc::sync::Arc;
use epoch_counters::EpochCounterPool;

use crate::epoch_counters::EpochCounter;

pub mod atomic;
pub mod rwlock;

pub mod rcu_ref;

mod doc_tests;

/// An abstract Rcu to abstract over the atomic based [`atomic::Arcu`] and the RwLock based [`rwlock::Arcu`]
pub trait Rcu {
    /// The type contained in this Rcu
    type Item;

    /// The type for the pool of epoch counters used by this Rcu
    type Pool: EpochCounterPool;

    /// Create a new Rcu with the given initial value and epoch counter pool
    fn new(initial: impl Into<Arc<Self::Item>>, epoch_counter_pool: Self::Pool) -> Self;

    /// Read the value of the Rcu for the current epoch
    ///
    /// ## Blocking
    /// The initial read on each thread may block while registering the epoch counter.
    /// Further read on the same thread won't block even for different Rcu.
    ///
    /// ## Procedure
    ///
    /// 1. Register the Epoch Counter (only done once per thread, may block)
    /// 2. atomically increment the epoch counter (by one from even to odd)
    /// 3. atomically load the arc pointer
    /// 4. atomically increment the arc strong count
    /// 5. atomically increment the epoch counter (by one from odd back to even)
    #[cfg(feature = "thread_local_counter")]
    fn read(&self) -> rcu_ref::RcuRef<Self::Item, Self::Item>
    where
        Self: Rcu<Pool = epoch_counters::GlobalEpochCounterPool>,
    {
        let arc = crate::epoch_counters::with_thread_local_epoch_counter(|epoch_counter| {
            // Safety:
            // - we just registered the epoch counter
            // - this is a thread local epoch counter that is only used here, so there can't be a concurrent use
            unsafe { self.raw_read(epoch_counter) }
        });

        rcu_ref::RcuRef::<Self::Item, Self::Item>::new(arc)
    }

    /// Replace the Rcu's content with a new value
    ///
    /// This does not synchronize writes and the last to update the active_value pointer wins.
    ///
    /// all writes that do not win will be lost, though not leaked.
    /// This will block until the old value can be reclaimed,
    /// i.e. all threads witnessed to be in the read critical sections
    /// have been witnessed to have left the critical section at least once
    fn replace(&self, new_value: impl Into<Arc<Self::Item>>) -> Arc<Self::Item>;

    /// Update the Rcu using the provided update function
    /// Retries when the Rcu has been updated/replaced between reading the old value and writing the new value
    /// Aborts when the update function returns None
    #[cfg(feature = "thread_local_counter")]
    fn try_update<F, R>(&self, mut update: F) -> Option<Arc<Self::Item>>
    where
        Self: Rcu<Pool = epoch_counters::GlobalEpochCounterPool>,
        F: FnMut(&Self::Item) -> Option<R>,
        R: Into<Arc<Self::Item>>,
    {
        // Safety:
        // epoch_counter is thread local and as such can't be in use concurrently
        // get_epoch_counters returns the list of all registered epoch counters
        crate::epoch_counters::with_thread_local_epoch_counter(|epoch_counter| unsafe {
            self.raw_try_update(move |old| update(old).map(Into::into), epoch_counter)
        })
    }

    /// ## Safety
    /// - The epoch counter must not be used concurrently
    /// - The epoch counter must belong to the EpochCounterPool of this Rcu
    unsafe fn raw_read(&self, epoch_counter: &EpochCounter) -> Arc<Self::Item>;

    /// Update the Rcu using the provided update function
    /// Retries when the Rcu has been updated/replaced between reading the old value and writing the new value
    /// Aborts when the update function returns None
    ///
    /// ## Safety
    /// - The epoch counter must not be used concurrently
    /// - The epoch counter must belong to the EpochCounterPool of this Rcu
    unsafe fn raw_try_update(
        &self,
        update: impl FnMut(&Self::Item) -> Option<Arc<Self::Item>>,
        epoch_counter: &EpochCounter,
    ) -> Option<Arc<Self::Item>>;
}
