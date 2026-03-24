//! Single-threaded mutable global for WASM child singletons.
//!
//! WASM is single-threaded (wasm32-wasip2 has no threads).
//! No concurrent access is possible. `WasmCell<T>` replaces `static mut`
//! which is deprecated in edition 2024. The `unsafe impl Sync` is required
//! because `static` items must be `Sync`, but WASM's single-threaded
//! execution model makes this sound.

use std::cell::UnsafeCell;

pub(crate) struct WasmCell<T>(pub(crate) UnsafeCell<T>);

#[cfg(not(target_feature = "atomics"))]
unsafe impl<T> Sync for WasmCell<T> {}

#[cfg(target_feature = "atomics")]
compile_error!("WasmCell assumes single-threaded WASM. Use thread_local! with atomics.");
