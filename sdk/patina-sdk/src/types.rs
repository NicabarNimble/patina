wit_bindgen::generate!({
    path: "wit",
    world: "sdk-types",
    generate_all,
});

#[allow(unused_imports)]
pub use patina::records::types::*;
