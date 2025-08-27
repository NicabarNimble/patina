pub mod discovery;
pub mod duckdb;
pub mod git;
pub mod parsers;
pub mod schema;
pub mod sql;

pub use discovery::{discover_files, detect_language};
pub use duckdb::load_into_duckdb;
pub use git::{analyze_git, GitMetrics};
pub use parsers::parse_file;
pub use schema::AstData;
pub use sql::generate_sql;