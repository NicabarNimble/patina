wit_bindgen::generate!({
    path: "wit",
    world: "sdk-types",
    generate_all,
});

pub use patina::records::types::{
    CatalogEntry, FileFound, FileWritten, RecordEnvelope, RejectedRecord, TransformResult,
};
