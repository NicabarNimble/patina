//! Thi module contains the atomic and Arc based Rcu

extern crate alloc;

#[cfg(feature = "thread_local_counter")]
use core::ops::Deref;
use core::sync::atomic::{AtomicPtr, Ordering};
use std::marker::PhantomData;

use alloc::sync::Arc;

#[cfg(feature = "thread_local_counter")]
use crate::epoch_counters::GlobalEpochCounterPool;
use crate::epoch_counters::{EpochCounter, EpochCounterPool};

use super::Rcu;

/// A Rcu based on an atomic pointer to an [`Arc`] and a [`EpochCounterPool`]
///
pub struct Arcu<T, P> {
    // Safety invariant
    // - the pointer has been created with Arc::into_raw
    // - Arcu "owns" one strong reference count
    active_value: AtomicPtr<T>,
    epoch_counter_pool: P,
    phantom: PhantomData<Arc<T>>,
}

#[cfg(feature = "thread_local_counter")]
impl<T: core::fmt::Display> core::fmt::Display for Arcu<T, GlobalEpochCounterPool> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let data = self.read();
        core::fmt::Display::fmt(&data.deref(), f)
    }
}

impl<T: core::fmt::Debug, P> core::fmt::Debug for Arcu<T, P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Rcu")
            .field("active_value", &"Opaque")
            .field("epoch_counter_pool", &"Opaque")
            .finish()
    }
}

/// ## Safety
/// - When mixing safe and unsafe functions care needs to be taken that write operations see all Epochs used by concurrent read operations
/// - The safe read operations assume that the writer will observe `epoch_counters::THREAD_EPOCH_COUNTER`, see `epoch_counters::with_thread_local_epoch_counter`.
/// - The safe writers assume that the readers will use one of the epoch counters in `epoch_counters::GLOBAL_EPOCH_COUNTERS`, see `epoch_counters::register_epoch_counter`.
impl<T, P: EpochCounterPool> Rcu for Arcu<T, P> {
    type Item = T;
    type Pool = P;

    #[inline]
    fn new(initial: impl Into<Arc<T>>, epoch_counter_pool: P) -> Self {
        Arcu {
            active_value: AtomicPtr::new(Arc::into_raw(initial.into()).cast_mut()),
            epoch_counter_pool,
            phantom: PhantomData,
        }
    }

    /// ## Safety
    /// - The epoch counter must not be used concurrently
    /// - The epoch counter must be made available to write operations
    #[inline]
    unsafe fn raw_read(&self, epoch_counter: &EpochCounter) -> Arc<T> {
        epoch_counter.enter_rcs();

        let arc_ptr = self.active_value.load(Ordering::SeqCst);

        // Safety: See comments inside the block
        let arc = unsafe {
            // Safety:
            // - the ptr was created in Rcu::new or Rcu::replace with Arc::into_raw
            // - the Rcu is responsible for of the arc's strong references
            // - the Rcu is alive as this function takes a reference to the Rcu
            // - replace will wait with decrementing the old values strong count until our epoch counter is even again
            Arc::increment_strong_count(arc_ptr);
            // Safety:
            // - the ptr was created in Rcu::new or Rcu::replace with Arc::into_raw
            // - we have just ensured an additional strong count by incrementing the count
            Arc::from_raw(arc_ptr)
        };

        epoch_counter.leave_rcs();

        arc
    }

    /// ## Safety
    /// - `get_epoch_counters` must return a vector containing all epoch counters used with this Rcu that are odd at the time it is called
    /// - the vector may contain more epoch counters than required, i.e. epoch counters that are even and epoch counters in use with this Rcu
    #[inline]
    fn replace(&self, new_value: impl Into<Arc<T>>) -> Arc<T> {
        let arc_ptr = self.active_value.swap(
            Arc::into_raw(new_value.into()).cast_mut(),
            Ordering::Acquire,
        );
        self.epoch_counter_pool.wait_for_epochs();

        // Safety:
        // - the ptr was created in Arcu::new or Arcu::replace with Arc::into_raw
        // - we took the strong count of the Rcu
        // - we witnessed all threads either with an even epoch count or with a new odd count,
        //   as such they must have left the critical section at some point
        unsafe { Arc::from_raw(arc_ptr) }
    }

    /// Update the Rcu using the provided update function
    /// Retries when the Rcu has been updated/replaced between reading the old value and writing the new value
    /// Aborts when the update function returns None
    ///
    /// ## Safety
    /// - `epoch_counter` must be valid for `raw_read`
    /// - `get_epoch_counters` must be valid for `raw_replace`
    unsafe fn raw_try_update<'a>(
        &self,
        mut update: impl FnMut(&T) -> Option<Arc<T>>,
        epoch_counter: &EpochCounter,
    ) -> Option<Arc<T>> {
        loop {
            let old = self.raw_read(epoch_counter);

            let new = Arc::into_raw(update(&old)?);

            // we now exchange the ownership of rcu(old) for rcu(new)
            // if rcu(?) is rcu(old)
            let result = self.active_value.compare_exchange_weak(
                Arc::as_ptr(&old).cast_mut(),
                new.cast_mut(),
                Ordering::AcqRel,
                Ordering::Relaxed,
            );

            match result {
                Ok(old) => {
                    // Compare Exchange Succeeded, ensure the old Arc gets dropped after waiting for all readers to leave the read critical section

                    // we exchanged the old/new arc pointer
                    // we are now responsible for one strong count of old,
                    // in exchange for giving the rcu the responsibility of one strong count of new

                    self.epoch_counter_pool.wait_for_epochs();

                    // Safety:
                    // - the ptr was created in Arcu::new, Arcu::raw_replace, Arcu::raw_try_update with Arc::into_raw
                    // - we took the strong count of the Arcu
                    // - we witnessed all threads either with an even epoch count or with a new odd count,
                    //   as such they must have left the critical section at some point
                    return Some(unsafe { Arc::from_raw(old) });
                }
                Err(_new_old) => {
                    // Compare Exchange failed, reclaim the new arc we leaked with Arc::into_raw above

                    // Safety:
                    // - the ptr was just created using Arc::into_raw
                    // - there still one strong count left

                    // we haven't exchanged the references so we are still responsible to clean up one strong count of new
                    let _ = unsafe { Arc::from_raw(new) };

                    continue;
                }
            }
        }
    }
}

impl<T, P> Drop for Arcu<T, P> {
    fn drop(&mut self) {
        // Safety:
        // - The Pointer was created by Arc::into_raw
        // - The Arcu is responsible for one strong count, so the string count is at least 1
        unsafe { Arc::from_raw(self.active_value.load(Ordering::Acquire)) };
    }
}
