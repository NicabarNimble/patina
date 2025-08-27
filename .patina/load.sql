-- Generated SQL for bulk loading into DuckDB
-- Generated at: 2025-08-27T19:11:45.405579+00:00

-- Create tables if they don't exist
CREATE TABLE IF NOT EXISTS functions (
    file TEXT NOT NULL,
    name TEXT NOT NULL,
    visibility TEXT,
    is_async BOOLEAN,
    is_unsafe BOOLEAN,
    params_count INTEGER,
    returns TEXT,
    line_start INTEGER,
    line_end INTEGER,
    doc_comment TEXT
);

CREATE TABLE IF NOT EXISTS types (
    file TEXT NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    visibility TEXT,
    fields_count INTEGER,
    methods_count INTEGER,
    line_start INTEGER,
    line_end INTEGER,
    doc_comment TEXT
);

CREATE TABLE IF NOT EXISTS imports (
    file TEXT NOT NULL,
    path TEXT NOT NULL,
    items_count INTEGER,
    alias TEXT,
    line INTEGER
);

CREATE TABLE IF NOT EXISTS calls (
    file TEXT NOT NULL,
    target TEXT NOT NULL,
    caller TEXT NOT NULL,
    line INTEGER,
    is_method BOOLEAN,
    is_async BOOLEAN
);

-- Clear existing data
DELETE FROM calls;
DELETE FROM imports;
DELETE FROM functions;
DELETE FROM types;

-- Insert data from 168 files

INSERT INTO functions VALUES ('./src/bin/patina-index.rs', 'main', 'public', false, false, 0, NULL, 31, 131, NULL);
INSERT INTO functions VALUES ('./src/bin/patina-index.rs', 'get_cache_path', 'public', false, false, 0, NULL, 134, 140, NULL);
INSERT INTO functions VALUES ('./src/bin/patina-index.rs', 'should_skip_parse', 'public', false, false, 0, NULL, 142, 151, NULL);
INSERT INTO functions VALUES ('./src/bin/patina-index.rs', 'load_into_duckdb', 'public', false, false, 0, NULL, 154, 158, NULL);
INSERT INTO types VALUES ('./src/bin/patina-index.rs', 'Args', 'struct', 'public', 0, 0, 9, 29, NULL);
INSERT INTO imports VALUES ('./src/bin/patina-index.rs', 'anyhow::{Context, Result}', 0, NULL, 1);
INSERT INTO imports VALUES ('./src/bin/patina-index.rs', 'clap::Parser', 0, NULL, 2);
INSERT INTO imports VALUES ('./src/bin/patina-index.rs', 'patina::pipeline::{analyze_git, detect_language, discover_files, generate_sql, parse_file}', 0, NULL, 3);
INSERT INTO imports VALUES ('./src/bin/patina-index.rs', 'std::path::{Path, PathBuf}', 0, NULL, 4);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'Args::parse', 'main', 32, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'with_context', 'main', 35, true, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'std::fs::create_dir_all', 'main', 35, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'format!', 'main', 36, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'join', 'main', 39, true, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'std::fs::create_dir_all', 'main', 40, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'println!', 'main', 42, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'println!', 'main', 43, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'println!', 'main', 47, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'discover_files', 'main', 50, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'analyze_git', 'main', 51, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'println!', 'main', 53, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'join', 'main', 56, true, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'std::fs::write', 'main', 57, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'serde_json::to_string_pretty', 'main', 59, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'println!', 'main', 64, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'get_cache_path', 'main', 71, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'should_skip_parse', 'main', 74, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'println!', 'main', 76, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'println!', 'main', 83, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'parse_file', 'main', 87, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'parent', 'main', 90, true, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'std::fs::create_dir_all', 'main', 91, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'std::fs::write', 'main', 95, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'serde_json::to_string_pretty', 'main', 97, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'println!', 'main', 103, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'println!', 'main', 106, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'Ok', 'main', 107, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'println!', 'main', 112, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'join', 'main', 115, true, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'generate_sql', 'main', 116, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'println!', 'main', 118, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'println!', 'main', 122, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'join', 'main', 125, true, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'load_into_duckdb', 'main', 126, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'println!', 'main', 128, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'Ok', 'main', 130, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'unwrap_or', 'get_cache_path', 135, true, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'strip_prefix', 'get_cache_path', 135, true, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'join', 'get_cache_path', 136, true, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'format!', 'get_cache_path', 137, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'set_extension', 'get_cache_path', 138, true, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'exists', 'should_skip_parse', 143, true, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'Ok', 'should_skip_parse', 144, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'modified', 'should_skip_parse', 147, true, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'std::fs::metadata', 'should_skip_parse', 147, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'modified', 'should_skip_parse', 148, true, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'std::fs::metadata', 'should_skip_parse', 148, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'Ok', 'should_skip_parse', 150, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'println!', 'load_into_duckdb', 156, false, false);
INSERT INTO calls VALUES ('./src/bin/patina-index.rs', 'Ok', 'load_into_duckdb', 157, false, false);
INSERT INTO imports VALUES ('./src/pipeline/mod.rs', 'discovery::{discover_files, detect_language}', 0, NULL, 7);
INSERT INTO imports VALUES ('./src/pipeline/mod.rs', 'git::{analyze_git, GitMetrics}', 0, NULL, 8);
INSERT INTO imports VALUES ('./src/pipeline/mod.rs', 'parsers::parse_file', 0, NULL, 9);
INSERT INTO imports VALUES ('./src/pipeline/mod.rs', 'schema::AstData', 0, NULL, 10);
INSERT INTO imports VALUES ('./src/pipeline/mod.rs', 'sql::generate_sql', 0, NULL, 11);
INSERT INTO functions VALUES ('./src/pipeline/parsers/mod.rs', 'parse_file', 'public', false, false, 0, NULL, 9, 19, NULL);
INSERT INTO imports VALUES ('./src/pipeline/parsers/mod.rs', 'anyhow::Result', 0, NULL, 3);
INSERT INTO imports VALUES ('./src/pipeline/parsers/mod.rs', 'std::path::Path', 0, NULL, 4);
INSERT INTO imports VALUES ('./src/pipeline/parsers/mod.rs', 'super::schema::AstData', 0, NULL, 6);
INSERT INTO calls VALUES ('./src/pipeline/parsers/mod.rs', 'super::discovery::detect_language', 'parse_file', 10, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/mod.rs', 'rust::parse_rust_file', 'parse_file', 13, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/mod.rs', 'Ok', 'parse_file', 16, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/mod.rs', 'AstData::from_path', 'parse_file', 16, false, false);
INSERT INTO functions VALUES ('./src/pipeline/parsers/rust.rs', 'parse_rust_file', 'public', false, false, 0, NULL, 8, 59, NULL);
INSERT INTO functions VALUES ('./src/pipeline/parsers/rust.rs', 'extract_imports', 'public', false, false, 0, NULL, 62, 67, NULL);
INSERT INTO functions VALUES ('./src/pipeline/parsers/rust.rs', 'extract_imports_recursive', 'public', false, false, 0, NULL, 69, 96, NULL);
INSERT INTO functions VALUES ('./src/pipeline/parsers/rust.rs', 'extract_calls', 'public', false, false, 0, NULL, 98, 103, NULL);
INSERT INTO functions VALUES ('./src/pipeline/parsers/rust.rs', 'extract_calls_recursive', 'public', false, false, 0, NULL, 105, 169, NULL);
INSERT INTO imports VALUES ('./src/pipeline/parsers/rust.rs', 'anyhow::{Context, Result}', 0, NULL, 1);
INSERT INTO imports VALUES ('./src/pipeline/parsers/rust.rs', 'patina_metal::{Analyzer, Metal}', 0, NULL, 2);
INSERT INTO imports VALUES ('./src/pipeline/parsers/rust.rs', 'std::path::Path', 0, NULL, 3);
INSERT INTO imports VALUES ('./src/pipeline/parsers/rust.rs', 'crate::pipeline::schema::{AstData, Call, Function, Import, TypeDef}', 0, NULL, 5);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'with_context', 'parse_rust_file', 9, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'std::fs::read_to_string', 'parse_rust_file', 9, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'format!', 'parse_rust_file', 10, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'Analyzer::new', 'parse_rust_file', 12, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'parse', 'parse_rust_file', 13, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'AstData::from_path', 'parse_rust_file', 15, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'extract_symbols', 'parse_rust_file', 18, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'push', 'parse_rust_file', 24, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'to_string', 'parse_rust_file', 26, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'Vec::new', 'parse_rust_file', 29, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'push', 'parse_rust_file', 38, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'to_lowercase', 'parse_rust_file', 40, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'format!', 'parse_rust_file', 40, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'to_string', 'parse_rust_file', 41, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'Vec::new', 'parse_rust_file', 42, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'Vec::new', 'parse_rust_file', 43, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'extract_imports', 'parse_rust_file', 55, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'extract_calls', 'parse_rust_file', 56, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'Ok', 'parse_rust_file', 58, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'walk', 'extract_imports', 64, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'extract_imports_recursive', 'extract_imports', 65, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'Ok', 'extract_imports', 66, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'node', 'extract_imports_recursive', 70, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'kind', 'extract_imports_recursive', 72, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'child_by_field_name', 'extract_imports_recursive', 73, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'to_string', 'extract_imports_recursive', 74, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'byte_range', 'extract_imports_recursive', 74, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'start_position', 'extract_imports_recursive', 75, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'push', 'extract_imports_recursive', 77, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'Vec::new', 'extract_imports_recursive', 79, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'goto_first_child', 'extract_imports_recursive', 87, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'extract_imports_recursive', 'extract_imports_recursive', 89, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'goto_next_sibling', 'extract_imports_recursive', 90, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'goto_parent', 'extract_imports_recursive', 94, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'walk', 'extract_calls', 100, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'extract_calls_recursive', 'extract_calls', 101, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'Ok', 'extract_calls', 102, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'node', 'extract_calls_recursive', 106, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'kind', 'extract_calls_recursive', 109, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'unwrap_or_else', 'extract_calls_recursive', 110, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'map', 'extract_calls_recursive', 110, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'child_by_field_name', 'extract_calls_recursive', 110, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'to_string', 'extract_calls_recursive', 111, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'byte_range', 'extract_calls_recursive', 111, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'to_string', 'extract_calls_recursive', 112, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'to_string', 'extract_calls_recursive', 114, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'kind', 'extract_calls_recursive', 118, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'child_by_field_name', 'extract_calls_recursive', 120, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'kind', 'extract_calls_recursive', 122, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'unwrap_or_else', 'extract_calls_recursive', 123, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'map', 'extract_calls_recursive', 123, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'child_by_field_name', 'extract_calls_recursive', 123, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'to_string', 'extract_calls_recursive', 124, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'byte_range', 'extract_calls_recursive', 124, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'to_string', 'extract_calls_recursive', 125, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'to_string', 'extract_calls_recursive', 127, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'byte_range', 'extract_calls_recursive', 127, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'to_string', 'extract_calls_recursive', 130, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'push', 'extract_calls_recursive', 133, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'clone', 'extract_calls_recursive', 135, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'start_position', 'extract_calls_recursive', 136, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'unwrap_or', 'extract_calls_recursive', 137, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'map', 'extract_calls_recursive', 137, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'child_by_field_name', 'extract_calls_recursive', 137, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'kind', 'extract_calls_recursive', 138, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'child_by_field_name', 'extract_calls_recursive', 144, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'format!', 'extract_calls_recursive', 145, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'push', 'extract_calls_recursive', 147, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'clone', 'extract_calls_recursive', 149, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'start_position', 'extract_calls_recursive', 150, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'goto_first_child', 'extract_calls_recursive', 160, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'extract_calls_recursive', 'extract_calls_recursive', 162, false, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'goto_next_sibling', 'extract_calls_recursive', 163, true, false);
INSERT INTO calls VALUES ('./src/pipeline/parsers/rust.rs', 'goto_parent', 'extract_calls_recursive', 167, true, false);
INSERT INTO functions VALUES ('./src/pipeline/schema.rs', 'new', 'public', false, false, 0, NULL, 82, 91, NULL);
INSERT INTO functions VALUES ('./src/pipeline/schema.rs', 'from_path', 'public', false, false, 0, NULL, 94, 100, NULL);
INSERT INTO types VALUES ('./src/pipeline/schema.rs', 'AstData', 'struct', 'public', 0, 0, 5, 12, NULL);
INSERT INTO types VALUES ('./src/pipeline/schema.rs', 'Function', 'struct', 'public', 0, 0, 16, 28, NULL);
INSERT INTO types VALUES ('./src/pipeline/schema.rs', 'Parameter', 'struct', 'public', 0, 0, 32, 37, NULL);
INSERT INTO types VALUES ('./src/pipeline/schema.rs', 'TypeDef', 'struct', 'public', 0, 0, 41, 50, NULL);
INSERT INTO types VALUES ('./src/pipeline/schema.rs', 'Field', 'struct', 'public', 0, 0, 54, 59, NULL);
INSERT INTO types VALUES ('./src/pipeline/schema.rs', 'Import', 'struct', 'public', 0, 0, 63, 68, NULL);
INSERT INTO types VALUES ('./src/pipeline/schema.rs', 'Call', 'struct', 'public', 0, 0, 72, 78, NULL);
INSERT INTO imports VALUES ('./src/pipeline/schema.rs', 'serde::{Deserialize, Serialize}', 0, NULL, 1);
INSERT INTO calls VALUES ('./src/pipeline/schema.rs', 'Vec::new', 'new', 86, false, false);
INSERT INTO calls VALUES ('./src/pipeline/schema.rs', 'Vec::new', 'new', 87, false, false);
INSERT INTO calls VALUES ('./src/pipeline/schema.rs', 'Vec::new', 'new', 88, false, false);
INSERT INTO calls VALUES ('./src/pipeline/schema.rs', 'Vec::new', 'new', 89, false, false);
INSERT INTO calls VALUES ('./src/pipeline/schema.rs', 'to_string', 'from_path', 95, true, false);
INSERT INTO calls VALUES ('./src/pipeline/schema.rs', 'display', 'from_path', 95, true, false);
INSERT INTO calls VALUES ('./src/pipeline/schema.rs', 'to_string', 'from_path', 96, true, false);
INSERT INTO calls VALUES ('./src/pipeline/schema.rs', 'unwrap_or', 'from_path', 96, true, false);
INSERT INTO calls VALUES ('./src/pipeline/schema.rs', 'super::discovery::detect_language', 'from_path', 96, false, false);
INSERT INTO calls VALUES ('./src/pipeline/schema.rs', 'Self::new', 'from_path', 99, false, false);
INSERT INTO functions VALUES ('./src/pipeline/sql.rs', 'generate_sql', 'public', false, false, 0, NULL, 9, 43, NULL);
INSERT INTO functions VALUES ('./src/pipeline/sql.rs', 'write_table_definitions', 'public', false, false, 0, NULL, 45, 95, NULL);
INSERT INTO functions VALUES ('./src/pipeline/sql.rs', 'write_indexes', 'public', false, false, 0, NULL, 97, 108, NULL);
INSERT INTO functions VALUES ('./src/pipeline/sql.rs', 'find_json_files', 'public', false, false, 0, NULL, 110, 132, NULL);
INSERT INTO functions VALUES ('./src/pipeline/sql.rs', 'visit_dir', 'public', false, false, 0, NULL, 113, 126, NULL);
INSERT INTO functions VALUES ('./src/pipeline/sql.rs', 'process_json_file', 'public', false, false, 0, NULL, 134, 204, NULL);
INSERT INTO functions VALUES ('./src/pipeline/sql.rs', 'escape_string', 'public', false, false, 0, NULL, 206, 208, NULL);
INSERT INTO imports VALUES ('./src/pipeline/sql.rs', 'anyhow::{Context, Result}', 0, NULL, 1);
INSERT INTO imports VALUES ('./src/pipeline/sql.rs', 'std::fs', 0, NULL, 2);
INSERT INTO imports VALUES ('./src/pipeline/sql.rs', 'std::io::Write', 0, NULL, 3);
INSERT INTO imports VALUES ('./src/pipeline/sql.rs', 'std::path::Path', 0, NULL, 4);
INSERT INTO imports VALUES ('./src/pipeline/sql.rs', 'super::schema::AstData', 0, NULL, 6);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'with_context', 'generate_sql', 10, true, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'fs::File::create', 'generate_sql', 10, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'format!', 'generate_sql', 11, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'generate_sql', 14, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'generate_sql', 15, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'generate_sql', 16, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'write_table_definitions', 'generate_sql', 19, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'generate_sql', 22, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'generate_sql', 23, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'generate_sql', 24, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'generate_sql', 25, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'generate_sql', 26, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'generate_sql', 27, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'find_json_files', 'generate_sql', 30, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'generate_sql', 32, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'generate_sql', 33, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'process_json_file', 'generate_sql', 36, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'generate_sql', 39, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'write_indexes', 'generate_sql', 40, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'Ok', 'generate_sql', 42, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 46, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 48, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 49, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 50, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 51, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 52, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 53, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 54, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 55, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 56, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 57, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 58, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 59, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 60, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 62, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 63, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 64, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 65, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 66, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 67, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 68, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 69, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 70, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 71, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 72, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 73, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 75, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 76, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 77, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 78, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 79, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 80, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 81, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 82, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 84, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 85, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 86, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 87, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 88, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 89, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 90, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 91, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_table_definitions', 92, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'Ok', 'write_table_definitions', 94, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_indexes', 98, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_indexes', 99, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_indexes', 100, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_indexes', 101, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_indexes', 102, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_indexes', 103, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_indexes', 104, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'write_indexes', 105, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'Ok', 'write_indexes', 107, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'Vec::new', 'find_json_files', 111, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'is_dir', 'visit_dir', 114, true, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'fs::read_dir', 'visit_dir', 115, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'path', 'visit_dir', 117, true, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'is_dir', 'visit_dir', 118, true, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'visit_dir', 'visit_dir', 119, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'and_then', 'visit_dir', 120, true, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'extension', 'visit_dir', 120, true, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'to_str', 'visit_dir', 120, true, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'Some', 'visit_dir', 120, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'push', 'visit_dir', 121, true, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'Ok', 'visit_dir', 125, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'visit_dir', 'find_json_files', 128, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'sort', 'find_json_files', 129, true, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'Ok', 'find_json_files', 131, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'with_context', 'process_json_file', 135, true, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'fs::read_to_string', 'process_json_file', 135, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'format!', 'process_json_file', 136, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'with_context', 'process_json_file', 138, true, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'serde_json::from_str', 'process_json_file', 138, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'format!', 'process_json_file', 139, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'process_json_file', 143, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'process_json_file', 161, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'process_json_file', 178, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'writeln!', 'process_json_file', 191, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'Ok', 'process_json_file', 203, false, false);
INSERT INTO calls VALUES ('./src/pipeline/sql.rs', 'format!', 'escape_string', 207, false, false);
-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_functions_name ON functions(name);
CREATE INDEX IF NOT EXISTS idx_functions_file ON functions(file);
CREATE INDEX IF NOT EXISTS idx_types_name ON types(name);
CREATE INDEX IF NOT EXISTS idx_types_file ON types(file);
CREATE INDEX IF NOT EXISTS idx_calls_target ON calls(target);
CREATE INDEX IF NOT EXISTS idx_calls_caller ON calls(caller);
CREATE INDEX IF NOT EXISTS idx_imports_path ON imports(path);

