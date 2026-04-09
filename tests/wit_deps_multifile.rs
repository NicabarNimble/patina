mod bindings {
    wasmtime::component::bindgen!({
        path: "tests/fixtures/wit-deps-multifile/child",
        world: "probe",
    });
}

#[test]
fn wasmtime_bindgen_resolves_multifile_deps_layout() {
    let _ = std::any::TypeId::of::<bindings::wasi::io::streams::InputStream>();
}
