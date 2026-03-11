pub struct WasmCell<T>(pub std::cell::UnsafeCell<T>);

unsafe impl<T> Sync for WasmCell<T> {}
