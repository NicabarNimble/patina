//! This module contains [`EpochCounter`], [`EpochCounterPool`] and related functionality.

use alloc::sync::{Arc, Weak};
use core::sync::atomic::{AtomicU8, Ordering};

// the epoch counters of all threads that have ever accessed an Rcu
// threads that have finished will have a dangling Weak reference and can be cleaned up
// having this be shared between all Rcu's is a tradeoff:
// - writes will be slower as more epoch counters need to be waited for
// - reads should be faster as a thread only needs to register itself once on the first read
#[cfg(feature = "global_counters")]
static GLOBAL_EPOCH_COUNTERS: std::sync::RwLock<Vec<alloc::sync::Weak<EpochCounter>>> =
    std::sync::RwLock::new(Vec::new());

#[cfg(feature = "global_counters")]
pub fn register_epoch_counter(epoch_counter: alloc::sync::Weak<EpochCounter>) {
    GLOBAL_EPOCH_COUNTERS.write().unwrap().push(epoch_counter)
}

#[cfg(feature = "global_counters")]
pub fn global_counters() -> Vec<::alloc::sync::Weak<EpochCounter>> {
    GLOBAL_EPOCH_COUNTERS.read().unwrap().clone()
}

#[cfg(feature = "thread_local_counter")]
thread_local! {
    // odd value means the current thread is about to access the active_epoch of an Rcu
    // - threads observing this while leaving the write critical section will need to wait for this to change to a different (odd or even) value
    // a thread has a single epoch counter for all Rcu it accesses, as a thread can only access one Rcu at a time
    static THREAD_EPOCH_COUNTER: std::cell::OnceCell<std::sync::Arc<EpochCounter>> = const { std::cell::OnceCell::new() };
}

#[cfg(feature = "global_counters")]
pub struct GlobalEpochCounterPool;

#[cfg(feature = "global_counters")]
unsafe impl EpochCounterPool for GlobalEpochCounterPool {
    fn wait_for_epochs(&self) {
        global_counters.wait_for_epochs()
    }
}

/// Calls the provided function with the thread local epoch counter
///
/// Per Thread: On first use registers the epoch counter
#[cfg(feature = "thread_local_counter")]
pub(crate) fn with_thread_local_epoch_counter<T>(fun: impl FnOnce(&EpochCounter) -> T) -> T {
    THREAD_EPOCH_COUNTER.with(|epoch_counter| {
        let epoch_counter = epoch_counter.get_or_init(|| {
            let epoch_counter = Arc::new(EpochCounter::new());

            // register the current threads epoch counter on init
            register_epoch_counter(Arc::downgrade(&epoch_counter));

            epoch_counter
        });

        fun(&epoch_counter)
    })
}

/// An epoch counter for Arcu
///
/// This is used to prevent deallocating
/// the old content or an Arcu while a reader is reading
///
/// An even counter values means the EpochCounter is inactive i.e outside the critical section.
/// An odd counter value means the EpochCounter is active i.e. in the critical section.
#[repr(transparent)]
pub struct EpochCounter(core::sync::atomic::AtomicU8);

impl EpochCounter {
    /// Create a new EpochCounter
    #[inline]
    pub const fn new() -> Self {
        Self(AtomicU8::new(0))
    }

    /// Increment the epoch counter to enter the read-critical-section
    ///
    /// # Panics
    /// - when the Epoch counter odd i.e. is already active/in the read critical section
    #[inline]
    pub(crate) fn enter_rcs(&self) {
        let old = self.0.fetch_add(1, Ordering::Acquire);
        assert!(old % 2 == 0, "Old Epoch counter value should be even!");
    }

    /// Increment the epoch counter to leave the read-critical-section
    ///
    /// # Panics
    /// - when the Epoch counter even i.e. is inactive/outside the read critical section
    #[inline]
    pub(crate) fn leave_rcs(&self) {
        let old = self.0.fetch_add(1, Ordering::Release);
        assert!(old % 2 != 0, "Old Epoch counter value should be odd!");
    }

    /// Get the current epoch counter value
    pub(crate) fn get_epoch(&self) -> u8 {
        self.0.load(Ordering::Acquire)
    }
}

impl Default for EpochCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// ## Safety
/// `wait_for_epochs` must not return normally until all epoch counters have been witnessed to be even or to have changed
///
/// The first one is necessary to not get stuck on inactive EpochCounters
/// The second one is necessary to not get stuck when we race to only witness the EpochCounter in different visits to the read-critical-section.
/// It is sufficient to witness a change rather than inactivity as the only way for the epoch counter to change is
/// - to go from inactive to active or
/// - to go from active to inactive
pub unsafe trait EpochCounterPool {
    /// Wait for each epoch counter of the pool to be inactive at least once
    ///
    /// We know that an epoch counter has been inactive at least once when have witnessed it to
    /// - be inactive
    /// - have changed
    fn wait_for_epochs(&self);
}

// Safety:
// `wait_for_epochs` does not return normally until all epoch counters have been witnessed to be even or to have changed
unsafe impl<F: Fn() -> Vec<Weak<EpochCounter>>> EpochCounterPool for F {
    fn wait_for_epochs(&self) {
        // Get the current state of the epoch counters,
        // we can only drop the old value once we have observed all to be even or to have changed
        let epochs = self();

        let mut epochs = epochs
            .into_iter()
            .flat_map(|elem| {
                let arc = elem.upgrade()?;
                let init_val = arc.get_epoch();
                if init_val % 2 == 0 {
                    // already even can be ignored
                    return None;
                }
                // odd initial value thread is in the read critical section
                // we need to wait for the value to change before we can drop the arc
                Some((init_val, elem))
            })
            .collect::<Vec<_>>();

        while !epochs.is_empty() {
            epochs.retain(|elem| {
                let Some(arc) = elem.1.upgrade() else {
                    // as the thread is dead it can't have a pointer to the old arc
                    return false;
                };
                // the epoch counter has not changed so the thread is still in the same instance of the critical section
                // any different value is ok as
                // - even values indicate the thread is outside of the critical section
                // - a different odd value indicates the thread has left the critical section and can subsequently only read the new active_value
                arc.get_epoch() == elem.0
            })
        }
    }
}

// Safety:
// `wait_for_epochs` does not return normally until all epoch counters have been witnessed to be even or to have changed
unsafe impl<const N: usize> EpochCounterPool for [Arc<EpochCounter>; N] {
    fn wait_for_epochs(&self) {
        (|| self.iter().map(Arc::downgrade).collect::<Vec<_>>()).wait_for_epochs()
    }
}
