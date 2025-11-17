//! This module contains the RwLock and Arc based Rcu
//!
//! This is primarily intended to sanity check the atomic based one in [`super::atomic`]

extern crate alloc;

use std::{marker::PhantomData, sync::RwLock};

use alloc::sync::Arc;

use crate::epoch_counters::{EpochCounter, EpochCounterPool};

use super::Rcu;

/// An Rcu based on an RwLock containing an Arc.
///
/// You probably want the Atomics basec one [`super::atomic::Arcu`].
///
/// This Rcu uses a RwLocks for synchronization instead of the EpochCounterPool.
/// The EpochCounterPool is kept to keep the API compatible with the atomics based one.
pub struct Arcu<T, P> {
    active_value: RwLock<Arc<T>>,
    epoch_counter_pool: PhantomData<P>,
}

impl<T: core::fmt::Display, P> core::fmt::Display for Arcu<T, P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        T::fmt(&self.active_value.read().unwrap(), f)
    }
}

impl<T: core::fmt::Debug, P> core::fmt::Debug for Arcu<T, P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Rcu")
            .field("active_value", &self.active_value.read().unwrap())
            .field("epoch_counter_pool", &"Opaque")
            .finish()
    }
}

impl<T, P: EpochCounterPool> Rcu for Arcu<T, P> {
    type Item = T;
    type Pool = P;

    #[inline]
    fn new(initial: impl Into<Arc<T>>, _epoch_counter_pool: P) -> Self {
        Arcu {
            // active_value: AtomicPtr::new(Arc::into_raw(initial.into()).cast_mut()),
            active_value: RwLock::new(initial.into()),
            epoch_counter_pool: PhantomData,
        }
    }

    /// ## Safety
    /// - this impl is actually safe
    #[inline]
    unsafe fn raw_read(&self, _epoch_counter: &EpochCounter) -> Arc<T> {
        self.active_value.read().unwrap().clone()
    }

    #[inline]
    fn replace(&self, new_value: impl Into<Arc<T>>) -> Arc<T> {
        std::mem::replace(&mut self.active_value.write().unwrap(), new_value.into())
    }

    /// Update the Rcu using the provided update function
    /// Retries when the Rcu has been updated/replaced between reading the old value and writing the new value
    /// Aborts when the update function returns None
    ///
    /// ## Safety
    /// - this impl is actually safe
    #[inline]
    unsafe fn raw_try_update<'a>(
        &self,
        mut update: impl FnMut(&T) -> Option<Arc<T>>,
        _epoch_counter: &EpochCounter,
    ) -> Option<Arc<T>> {
        loop {
            let old = self.active_value.read().unwrap().clone();
            let new = update(&old)?;
            let mut cur = self.active_value.write().unwrap();
            if Arc::ptr_eq(&cur, &old) {
                return Some(std::mem::replace(&mut cur, new));
            } else {
                println!("Ptr neq, retry!")
            }
        }
    }
}
