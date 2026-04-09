pub mod types;

pub use types::{
    CatalogEntry, FileFound, FileWritten, RecordEnvelope, RejectedRecord, TransformResult,
};

pub mod prelude {
    pub use crate::types::{
        CatalogEntry, FileFound, FileWritten, RecordEnvelope, RejectedRecord, TransformResult,
    };
}
