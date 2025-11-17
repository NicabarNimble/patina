//! This module contains the [`RcuRef`] type which is a smart pointer to the content of an [`super::Rcu`]

use alloc::sync::Arc;
use core::{fmt::Debug, ops::Deref, ptr::NonNull};

/// A smard pointer for a reference to the content of an [`super::Rcu`]
pub struct RcuRef<T, M>
where
    T: ?Sized,
    M: ?Sized,
{
    // we keep the arc to ensure its still alive, but we only access its data through data
    #[allow(dead_code)]
    arc: Arc<T>,
    data: NonNull<M>,
}

impl<T: ?Sized, M: ?Sized + Debug> Debug for RcuRef<T, M> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RcuRef")
            .field("data", &self.deref())
            .finish()
    }
}

impl<T: ?Sized> RcuRef<T, T> {
    /// Create a new `RcuRef` from an `Arc`
    pub fn new(arc: Arc<T>) -> Self {
        Self {
            data: arc.as_ref().into(),
            arc,
        }
    }
}

// use associated functions rather than methods so that we don't overlap
// with functions of the Deref Target type
impl<T: ?Sized, M: ?Sized> RcuRef<T, M> {
    /// apply the mapping function to the reference in this RcuRef
    pub fn map<N: ?Sized, F: for<'a> FnOnce(&'a M) -> &'a N>(
        reference: Self,
        f: F,
    ) -> RcuRef<T, N> {
        RcuRef {
            arc: reference.arc,
            // Safety: See deref
            data: f(unsafe { reference.data.as_ref() }).into(),
        }
    }

    /// try to apply the faillable mapping function to the reference in this RcuRef
    pub fn try_map<N: ?Sized, F: for<'a> FnOnce(&'a M) -> Option<&'a N>>(
        reference: Self,
        f: F,
    ) -> Option<RcuRef<T, N>> {
        // Safety: See deref
        let val = f(unsafe { reference.data.as_ref() })?;
        Some(RcuRef {
            arc: Arc::clone(&reference.arc),
            data: val.into(),
        })
    }

    /// Check whether the two RcuRefs reference values in the same epoch
    pub fn same_epoch<M2>(this: &Self, other: &RcuRef<T, M2>) -> bool {
        Arc::ptr_eq(&this.arc, &other.arc)
    }

    /// Compares the RcuRefs references via [`core::ptr::eq`]
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        core::ptr::eq(this.data.as_ptr(), other.data.as_ptr())
    }

    /// Compares the RcuRefs references via [`core::ptr::addr_eq`]
    pub fn ptr_addr_eq(this: &Self, other: &Self) -> bool {
        std::ptr::addr_eq(this.data.as_ptr(), other.data.as_ptr())
    }

    /// Clones the RcuRef
    ///
    /// Not implementing clone to not shadow the inner types clone impl
    #[allow(clippy::should_implement_trait)]
    pub fn clone(this: &Self) -> Self {
        Self {
            arc: Arc::clone(&this.arc),
            data: this.data,
        }
    }

    /// Get a reference to root of the RcuRef
    ///
    /// i.e. the value that was stored in the Rcu
    /// before applying any mappings
    pub fn get_root(this: &Self) -> &T {
        &this.arc
    }
}

impl<T: ?Sized, M: ?Sized> Deref for RcuRef<T, M> {
    type Target = M;

    fn deref(&self) -> &Self::Target {
        // Safety: The pointer points into the arc we are holding
        // while we are alive so is the target
        // as the content is in an Rcu no mutable access is given out
        unsafe { self.data.as_ref() }
    }
}
