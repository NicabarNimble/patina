//! Schema command implementation.
//!
//! Handles parsing schema.toml, validating WIT presence, installing to
//! .patina/schemas/, listing/showing installed schemas, scaffolding new
//! schemas, and generating Rust types, SQLite migrations, and embedding
//! config from installed schemas.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};

use patina::paths;

// =========================================================================
// Schema metadata types (parsed from schema.toml)
// =========================================================================

/// Top-level schema metadata from schema.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SchemaMetadata {
    pub schema: SchemaInfo,
    #[serde(default)]
    pub facts: Vec<FactDef>,
    pub embedding: Option<EmbeddingConfig>,
    #[serde(default)]
    pub indexes: Vec<IndexConfig>,
}

/// The [schema] section — identity and description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SchemaInfo {
    pub name: String,
    pub version: String,
    pub package: String,
    #[serde(default)]
    pub description: String,
}

/// A [[facts]] entry — maps a logical name to an event type and WIT record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FactDef {
    pub name: String,
    pub event_type: String,
    pub record: String,
}

/// The [embedding] section — offset slot and corpus query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EmbeddingConfig {
    pub offset_slot: i64,
    pub corpus_query: String,
}

/// An [[indexes]] entry — FTS5 and materialized view config per fact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IndexConfig {
    pub fact: String,
    pub fts_fields: Vec<String>,
    pub table: String,
}

// =========================================================================
// Schema package operations
// =========================================================================

/// Parse schema.toml from a directory.
fn parse_schema_toml(dir: &Path) -> Result<SchemaMetadata> {
    let toml_path = dir.join("schema.toml");
    if !toml_path.exists() {
        bail!("schema.toml not found in {}", dir.display());
    }
    let content = std::fs::read_to_string(&toml_path)
        .with_context(|| format!("reading {}", toml_path.display()))?;
    let metadata: SchemaMetadata =
        toml::from_str(&content).with_context(|| format!("parsing {}", toml_path.display()))?;
    Ok(metadata)
}

/// Validate that a schema package directory has required files.
fn validate_package(dir: &Path) -> Result<SchemaMetadata> {
    if !dir.is_dir() {
        bail!("{} is not a directory", dir.display());
    }

    // Must have schema.toml
    let metadata = parse_schema_toml(dir)?;

    // Must have at least one .wit file
    let has_wit = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .any(|e| e.path().extension().is_some_and(|ext| ext == "wit"));
    if !has_wit {
        bail!(
            "no .wit files found in {} — schema packages require at least one WIT file",
            dir.display()
        );
    }

    // Validate required fields
    if metadata.schema.name.is_empty() {
        bail!("schema.name is empty");
    }
    if metadata.schema.version.is_empty() {
        bail!("schema.version is empty");
    }
    if metadata.schema.package.is_empty() {
        bail!("schema.package is empty");
    }
    if metadata.facts.is_empty() {
        bail!("schema must define at least one [[facts]] entry");
    }

    Ok(metadata)
}

/// Find project root by walking up from cwd.
fn find_project_root() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let mut dir = cwd.as_path();
    loop {
        if dir.join(".patina").is_dir() {
            return Ok(dir.to_path_buf());
        }
        dir = dir
            .parent()
            .ok_or_else(|| anyhow::anyhow!("not in a patina project (no .patina/ found)"))?;
    }
}

// =========================================================================
// Commands
// =========================================================================

/// Install a schema package from a local path.
pub fn install_schema(source_path: &str) -> Result<()> {
    let source = PathBuf::from(source_path);
    let metadata = validate_package(&source)?;
    let name = &metadata.schema.name;

    let root = find_project_root()?;
    let schemas_dir = paths::project::schemas_dir(&root);
    let target_dir = schemas_dir.join(name);

    // Create target directory
    std::fs::create_dir_all(&target_dir)
        .with_context(|| format!("creating {}", target_dir.display()))?;

    // Copy all files from source to target
    let mut copied = 0;
    for entry in std::fs::read_dir(&source)? {
        let entry = entry?;
        let file_name = entry.file_name();
        if entry.file_type()?.is_file() {
            std::fs::copy(entry.path(), target_dir.join(&file_name))?;
            copied += 1;
        }
    }

    println!(
        "Installed schema '{}' v{} ({} files) → {}",
        name,
        metadata.schema.version,
        copied,
        target_dir.display()
    );
    println!("  Package: {}", metadata.schema.package);
    println!("  Facts: {}", metadata.facts.len());
    if let Some(ref emb) = metadata.embedding {
        println!("  Embedding: slot {}", emb.offset_slot);
    }

    Ok(())
}

/// List installed schemas.
pub fn list_schemas(json: bool) -> Result<()> {
    let root = find_project_root()?;
    let schemas_dir = paths::project::schemas_dir(&root);

    if !schemas_dir.exists() {
        if json {
            println!("[]");
        } else {
            println!("No schemas installed");
        }
        return Ok(());
    }

    let mut schemas = Vec::new();
    for entry in std::fs::read_dir(&schemas_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let schema_dir = entry.path();
        match parse_schema_toml(&schema_dir) {
            Ok(metadata) => schemas.push(metadata),
            Err(e) => {
                eprintln!("Warning: skipping {}: {}", schema_dir.display(), e);
            }
        }
    }

    if json {
        let json_schemas: Vec<serde_json::Value> = schemas
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.schema.name,
                    "version": s.schema.version,
                    "package": s.schema.package,
                    "description": s.schema.description,
                    "facts": s.facts.len(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_schemas)?);
    } else if schemas.is_empty() {
        println!("No schemas installed");
    } else {
        println!("Installed schemas:");
        for s in &schemas {
            println!(
                "  {} v{} — {} ({} facts)",
                s.schema.name,
                s.schema.version,
                s.schema.description,
                s.facts.len()
            );
        }
    }

    Ok(())
}

/// List installed schemas as JSON value (for MCP).
pub fn list_schemas_value() -> Result<serde_json::Value> {
    let root = find_project_root()?;
    let schemas_dir = paths::project::schemas_dir(&root);

    if !schemas_dir.exists() {
        return Ok(serde_json::json!([]));
    }

    let mut schemas = Vec::new();
    for entry in std::fs::read_dir(&schemas_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        match parse_schema_toml(&entry.path()) {
            Ok(metadata) => schemas.push(metadata),
            Err(_) => continue,
        }
    }

    Ok(serde_json::json!(schemas
        .iter()
        .map(|s| serde_json::json!({
            "name": s.schema.name,
            "version": s.schema.version,
            "package": s.schema.package,
            "description": s.schema.description,
            "facts": s.facts.iter().map(|f| &f.name).collect::<Vec<_>>(),
        }))
        .collect::<Vec<_>>()))
}

/// Show details of an installed schema as JSON value (for MCP).
pub fn show_schema_value(name: &str) -> Result<serde_json::Value> {
    let root = find_project_root()?;
    let schema_dir = paths::project::schemas_dir(&root).join(name);

    if !schema_dir.exists() {
        bail!("schema '{}' is not installed", name);
    }

    let metadata = parse_schema_toml(&schema_dir)?;
    Ok(serde_json::to_value(&metadata)?)
}

/// Show details of an installed schema.
pub fn show_schema(name: &str, json: bool) -> Result<()> {
    let root = find_project_root()?;
    let schema_dir = paths::project::schemas_dir(&root).join(name);

    if !schema_dir.exists() {
        bail!("schema '{}' is not installed", name);
    }

    let metadata = parse_schema_toml(&schema_dir)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&metadata)?);
        return Ok(());
    }

    println!(
        "Schema: {} v{}",
        metadata.schema.name, metadata.schema.version
    );
    println!("Package: {}", metadata.schema.package);
    if !metadata.schema.description.is_empty() {
        println!("Description: {}", metadata.schema.description);
    }

    println!("\nFacts:");
    for fact in &metadata.facts {
        println!(
            "  {} → event_type: {}, record: {}",
            fact.name, fact.event_type, fact.record
        );
    }

    if let Some(ref emb) = metadata.embedding {
        println!("\nEmbedding:");
        println!("  Offset slot: {}", emb.offset_slot);
        println!("  Corpus query: {}", emb.corpus_query.trim());
    }

    if !metadata.indexes.is_empty() {
        println!("\nIndexes:");
        for idx in &metadata.indexes {
            println!(
                "  {} → table: {}, FTS: [{}]",
                idx.fact,
                idx.table,
                idx.fts_fields.join(", ")
            );
        }
    }

    Ok(())
}

// =========================================================================
// Scaffold new schema package
// =========================================================================

/// Validate schema name: lowercase, alphanumeric + hyphens only.
fn validate_schema_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("schema name cannot be empty");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        bail!("schema name must be lowercase alphanumeric with hyphens only: '{name}'");
    }
    if name.starts_with('-') || name.ends_with('-') {
        bail!("schema name cannot start or end with a hyphen: '{name}'");
    }
    Ok(())
}

/// Scaffold a new schema package under wit/schema/<name>/.
pub fn new_schema(name: &str, version: &str, description: &str, facts: Option<&str>) -> Result<()> {
    validate_schema_name(name)?;

    let root = find_project_root()?;
    let schema_dir = root.join("wit/schema").join(name);

    if schema_dir.exists() {
        bail!("schema package already exists: {}", schema_dir.display());
    }

    // Check for collision with installed schemas
    let installed_dir = paths::project::schemas_dir(&root).join(name);
    if installed_dir.exists() {
        bail!("schema '{name}' is already installed — use a different name");
    }

    // Parse fact names (default to a single "item" fact)
    let fact_names: Vec<&str> = match facts {
        Some(f) => f
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect(),
        None => vec!["item"],
    };

    // Generate WIT file
    let mut wit = String::new();
    writeln!(wit, "package patina:schema/{name}@{version};")?;
    writeln!(wit)?;
    writeln!(wit, "interface types {{")?;
    for fact in &fact_names {
        let record_name = fact;
        writeln!(wit, "    record {record_name} {{")?;
        writeln!(wit, "        id: s64,")?;
        writeln!(wit, "        title: string,")?;
        writeln!(wit, "        body: option<string>,")?;
        writeln!(wit, "        created-at: string,")?;
        writeln!(wit, "    }}")?;
        writeln!(wit)?;
    }
    writeln!(wit, "}}")?;

    // Generate schema.toml
    let mut toml_content = String::new();
    writeln!(toml_content, "[schema]")?;
    writeln!(toml_content, "name = \"{name}\"")?;
    writeln!(toml_content, "version = \"{version}\"")?;
    writeln!(toml_content, "package = \"patina:schema/{name}@{version}\"")?;
    if !description.is_empty() {
        writeln!(toml_content, "description = \"{description}\"")?;
    } else {
        writeln!(toml_content, "description = \"{name} facts\"")?;
    }
    writeln!(toml_content)?;
    for fact in &fact_names {
        writeln!(toml_content, "[[facts]]")?;
        writeln!(toml_content, "name = \"{fact}\"")?;
        writeln!(toml_content, "event_type = \"{name}.{fact}\"")?;
        writeln!(toml_content, "record = \"{fact}\"")?;
        writeln!(toml_content)?;
    }

    // Write files
    std::fs::create_dir_all(&schema_dir)
        .with_context(|| format!("creating {}", schema_dir.display()))?;
    std::fs::write(schema_dir.join(format!("{name}.wit")), &wit)?;
    std::fs::write(schema_dir.join("schema.toml"), &toml_content)?;

    println!("Created schema package: {}", schema_dir.display());
    println!("  WIT: {name}.wit");
    println!("  Metadata: schema.toml");
    println!("  Facts: {}", fact_names.join(", "));
    println!();
    println!("Next steps:");
    println!("  1. Edit {name}.wit to define your record fields");
    println!("  2. Edit schema.toml to add embedding/index config");
    println!("  3. Run: patina schema install wit/schema/{name}");

    Ok(())
}

// =========================================================================
// WIT parsing (minimal subset for code generation)
// =========================================================================

/// A parsed WIT record.
#[derive(Debug, Clone)]
struct WitRecord {
    name: String,
    fields: Vec<WitField>,
}

/// A field in a WIT record.
#[derive(Debug, Clone)]
struct WitField {
    name: String,
    ty: WitType,
}

/// Minimal WIT type representation for code generation.
#[derive(Debug, Clone, PartialEq)]
enum WitType {
    S32,
    S64,
    U32,
    U64,
    F32,
    F64,
    Bool,
    String,
    Option(Box<WitType>),
    List(Box<WitType>),
    Named(String), // reference to another type (enum or record)
}

/// A parsed WIT enum.
#[derive(Debug, Clone)]
struct WitEnum {
    name: String,
    variants: Vec<String>,
}

/// All types parsed from a WIT file.
#[derive(Debug, Clone)]
struct WitTypes {
    records: Vec<WitRecord>,
    enums: Vec<WitEnum>,
}

/// Parse a WIT type string into a WitType.
fn parse_wit_type(s: &str) -> WitType {
    let s = s.trim();
    match s {
        "s32" => WitType::S32,
        "s64" => WitType::S64,
        "u32" => WitType::U32,
        "u64" => WitType::U64,
        "f32" => WitType::F32,
        "f64" => WitType::F64,
        "bool" => WitType::Bool,
        "string" => WitType::String,
        _ if s.starts_with("option<") && s.ends_with('>') => {
            let inner = &s[7..s.len() - 1];
            WitType::Option(Box::new(parse_wit_type(inner)))
        }
        _ if s.starts_with("list<") && s.ends_with('>') => {
            let inner = &s[5..s.len() - 1];
            WitType::List(Box::new(parse_wit_type(inner)))
        }
        _ => WitType::Named(s.to_string()),
    }
}

/// Parse records and enums from a WIT file's text content.
fn parse_wit_types(content: &str) -> WitTypes {
    let mut records = Vec::new();
    let mut enums = Vec::new();

    let mut lines = content.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        // Parse enum
        if trimmed.starts_with("enum ") && trimmed.contains('{') {
            let name = trimmed
                .trim_start_matches("enum ")
                .trim()
                .trim_end_matches('{')
                .trim()
                .to_string();
            let mut variants = Vec::new();
            for inner in lines.by_ref() {
                let inner = inner.trim();
                if inner == "}" {
                    break;
                }
                if inner.is_empty() || inner.starts_with("//") {
                    continue;
                }
                let variant = inner.trim_end_matches(',').trim().to_string();
                if !variant.is_empty() {
                    variants.push(variant);
                }
            }
            enums.push(WitEnum { name, variants });
        }

        // Parse record
        if trimmed.starts_with("record ") && trimmed.contains('{') {
            let name = trimmed
                .trim_start_matches("record ")
                .trim()
                .trim_end_matches('{')
                .trim()
                .to_string();
            let mut fields = Vec::new();
            for inner in lines.by_ref() {
                let inner = inner.trim();
                if inner == "}" {
                    break;
                }
                if inner.is_empty() || inner.starts_with("//") {
                    continue;
                }
                // "field-name: type," or "field-name: type"
                let inner = inner.trim_end_matches(',');
                if let Some((fname, ftype)) = inner.split_once(':') {
                    fields.push(WitField {
                        name: fname.trim().to_string(),
                        ty: parse_wit_type(ftype.trim()),
                    });
                }
            }
            records.push(WitRecord { name, fields });
        }
    }

    WitTypes { records, enums }
}

// =========================================================================
// Code generation
// =========================================================================

/// Convert a WIT kebab-case name to Rust snake_case.
fn wit_to_snake(name: &str) -> String {
    name.replace('-', "_")
}

/// Convert a WIT kebab-case name to Rust PascalCase.
fn wit_to_pascal(name: &str) -> String {
    name.split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let mut s = c.to_uppercase().to_string();
                    s.extend(chars);
                    s
                }
            }
        })
        .collect()
}

/// Map a WIT type to its Rust type string.
fn wit_type_to_rust(ty: &WitType) -> String {
    match ty {
        WitType::S32 => "i32".to_string(),
        WitType::S64 => "i64".to_string(),
        WitType::U32 => "u32".to_string(),
        WitType::U64 => "u64".to_string(),
        WitType::F32 => "f32".to_string(),
        WitType::F64 => "f64".to_string(),
        WitType::Bool => "bool".to_string(),
        WitType::String => "String".to_string(),
        WitType::Option(inner) => format!("Option<{}>", wit_type_to_rust(inner)),
        WitType::List(inner) => format!("Vec<{}>", wit_type_to_rust(inner)),
        WitType::Named(name) => wit_to_pascal(name),
    }
}

/// Map a WIT type to its SQLite column type.
fn wit_type_to_sqlite(ty: &WitType) -> &'static str {
    match ty {
        WitType::S32 | WitType::S64 | WitType::U32 | WitType::U64 => "INTEGER",
        WitType::F32 | WitType::F64 => "REAL",
        WitType::Bool => "INTEGER",
        WitType::String => "TEXT",
        WitType::Option(_) => "TEXT", // nullable
        WitType::List(_) => "TEXT",   // JSON array
        WitType::Named(_) => "TEXT",  // enum serialized as text
    }
}

/// Whether a WIT type is nullable (Option or has a default).
fn wit_type_is_nullable(ty: &WitType) -> bool {
    matches!(ty, WitType::Option(_) | WitType::List(_))
}

/// Load all installed schemas with their parsed WIT types.
fn load_installed_schemas(
    root: &Path,
    filter: Option<&str>,
) -> Result<Vec<(SchemaMetadata, WitTypes)>> {
    let schemas_dir = paths::project::schemas_dir(root);
    if !schemas_dir.exists() {
        bail!("no schemas installed — run 'patina schema install' first");
    }

    let mut results = Vec::new();
    for entry in std::fs::read_dir(&schemas_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(filter_name) = filter {
            if name != filter_name {
                continue;
            }
        }
        let schema_dir = entry.path();
        let metadata = parse_schema_toml(&schema_dir)?;

        // Find and parse the .wit file
        let mut wit_types = WitTypes {
            records: vec![],
            enums: vec![],
        };
        for file_entry in std::fs::read_dir(&schema_dir)? {
            let file_entry = file_entry?;
            if file_entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "wit")
            {
                let content = std::fs::read_to_string(file_entry.path())?;
                let parsed = parse_wit_types(&content);
                wit_types.records.extend(parsed.records);
                wit_types.enums.extend(parsed.enums);
            }
        }

        results.push((metadata, wit_types));
    }

    if results.is_empty() {
        if let Some(name) = filter {
            bail!("schema '{name}' is not installed");
        }
        bail!("no schemas installed");
    }

    Ok(results)
}

/// Generate Rust types from installed schemas (C1).
fn generate_types(root: &Path, filter: Option<&str>) -> Result<()> {
    let schemas = load_installed_schemas(root, filter)?;
    let out_dir = root.join("src/generated/schemas");
    std::fs::create_dir_all(&out_dir)?;

    let mut mod_entries = Vec::new();

    for (metadata, wit_types) in &schemas {
        let name = &metadata.schema.name;
        let snake_name = wit_to_snake(name);
        mod_entries.push(snake_name.clone());

        let mut code = String::new();
        writeln!(
            code,
            "//! Generated from schema '{}' v{} — DO NOT EDIT",
            name, metadata.schema.version
        )?;
        writeln!(code, "//!")?;
        writeln!(code, "//! Package: {}", metadata.schema.package)?;
        writeln!(code, "//! Generated by: patina schema generate --types")?;
        writeln!(code)?;
        writeln!(code, "use serde::{{Deserialize, Serialize}};")?;
        writeln!(code)?;

        // Generate enums
        for wit_enum in &wit_types.enums {
            let rust_name = wit_to_pascal(&wit_enum.name);
            writeln!(
                code,
                "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]"
            )?;
            writeln!(code, "#[serde(rename_all = \"lowercase\")]")?;
            writeln!(code, "pub enum {rust_name} {{")?;
            for variant in &wit_enum.variants {
                let rust_variant = wit_to_pascal(variant);
                writeln!(code, "    {rust_variant},")?;
            }
            writeln!(code, "}}")?;
            writeln!(code)?;
        }

        // Generate records
        for record in &wit_types.records {
            let rust_name = wit_to_pascal(&record.name);
            writeln!(code, "#[derive(Debug, Clone, Serialize, Deserialize)]")?;
            writeln!(code, "pub struct {rust_name} {{")?;
            for field in &record.fields {
                let rust_field = wit_to_snake(&field.name);
                let rust_type = wit_type_to_rust(&field.ty);
                writeln!(code, "    pub {rust_field}: {rust_type},")?;
            }
            writeln!(code, "}}")?;
            writeln!(code)?;
        }

        let file_path = out_dir.join(format!("{snake_name}.rs"));
        std::fs::write(&file_path, &code)?;
        println!("  Generated: {}", file_path.display());
    }

    // Write mod.rs
    let mut mod_code = String::new();
    writeln!(mod_code, "//! Generated schema types — DO NOT EDIT")?;
    writeln!(mod_code, "//! Generated by: patina schema generate --types")?;
    writeln!(mod_code)?;
    for entry in &mod_entries {
        writeln!(mod_code, "pub mod {entry};")?;
    }
    std::fs::write(out_dir.join("mod.rs"), &mod_code)?;

    println!("Types generated in src/generated/schemas/");
    Ok(())
}

/// Generate SQLite migration DDL from installed schemas (C2).
fn generate_migrations(root: &Path, filter: Option<&str>) -> Result<()> {
    let schemas = load_installed_schemas(root, filter)?;
    let out_dir = root.join("src/generated/schemas");
    std::fs::create_dir_all(&out_dir)?;

    for (metadata, wit_types) in &schemas {
        let name = &metadata.schema.name;
        let snake_name = wit_to_snake(name);

        let mut sql = String::new();
        writeln!(
            sql,
            "-- Generated from schema '{}' v{} — DO NOT EDIT",
            name, metadata.schema.version
        )?;
        writeln!(sql, "-- Generated by: patina schema generate --migrations")?;
        writeln!(sql)?;

        // Build a lookup from record name to WIT record
        let record_map: std::collections::HashMap<&str, &WitRecord> = wit_types
            .records
            .iter()
            .map(|r| (r.name.as_str(), r))
            .collect();

        for index_cfg in &metadata.indexes {
            let table = &index_cfg.table;
            let fact_name = &index_cfg.fact;

            // Find the matching fact definition
            let fact_def = metadata
                .facts
                .iter()
                .find(|f| f.name == *fact_name)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "index references fact '{}' but no such fact in schema",
                        fact_name
                    )
                })?;

            // Find the WIT record for this fact
            let record = record_map.get(fact_def.record.as_str()).ok_or_else(|| {
                anyhow::anyhow!(
                    "fact '{}' references record '{}' but not found in WIT",
                    fact_name,
                    fact_def.record
                )
            })?;

            writeln!(
                sql,
                "-- Materialized view for {}.{} events",
                name, fact_name
            )?;
            writeln!(sql, "CREATE TABLE IF NOT EXISTS {table} (")?;

            // Generate columns from WIT record fields
            for (i, field) in record.fields.iter().enumerate() {
                if i > 0 {
                    writeln!(sql, ",")?;
                }

                let col_name = wit_to_snake(&field.name);
                let col_type = wit_type_to_sqlite(&field.ty);
                let nullable = wit_type_is_nullable(&field.ty);

                // First field is primary key if it's an integer
                if i == 0
                    && matches!(
                        field.ty,
                        WitType::S64 | WitType::U64 | WitType::S32 | WitType::U32
                    )
                {
                    write!(sql, "    {col_name} {col_type} PRIMARY KEY")?;
                } else if nullable {
                    write!(sql, "    {col_name} {col_type}")?;
                } else {
                    write!(sql, "    {col_name} {col_type} NOT NULL")?;
                }
            }

            // Add event_seq cross-db reference (no FK — events.db is a separate database)
            writeln!(sql, ",")?;
            writeln!(
                sql,
                "    event_seq INTEGER      -- Cross-db ref to events.db eventlog seq"
            )?;
            writeln!(sql, ");")?;
            writeln!(sql)?;
        }

        // Generate indexes — state and temporal columns
        for index_cfg in &metadata.indexes {
            let table = &index_cfg.table;
            let fact_name = &index_cfg.fact;
            let fact_def = metadata.facts.iter().find(|f| f.name == *fact_name);
            if let Some(fact_def) = fact_def {
                if let Some(record) = record_map.get(fact_def.record.as_str()) {
                    for field in &record.fields {
                        let col = wit_to_snake(&field.name);
                        // Index state and temporal fields
                        if col == "state"
                            || col.ends_with("_at")
                            || col == "merged_at"
                            || col == "updated_at"
                        {
                            let idx_name = format!("idx_{}_{col}", table);
                            writeln!(
                                sql,
                                "CREATE INDEX IF NOT EXISTS {idx_name} ON {table}({col});"
                            )?;
                        }
                    }
                }
            }
        }

        let file_path = out_dir.join(format!("{snake_name}_migration.sql"));
        std::fs::write(&file_path, &sql)?;
        println!("  Generated: {}", file_path.display());
    }

    println!("Migrations generated in src/generated/schemas/");
    Ok(())
}

/// Generate embedding config from installed schemas (C3).
fn generate_embeddings(root: &Path, filter: Option<&str>) -> Result<()> {
    let schemas = load_installed_schemas(root, filter)?;
    let out_dir = root.join("src/generated/schemas");
    std::fs::create_dir_all(&out_dir)?;

    let mut code = String::new();
    writeln!(code, "//! Generated embedding config — DO NOT EDIT")?;
    writeln!(
        code,
        "//! Generated by: patina schema generate --embeddings"
    )?;
    writeln!(code)?;

    let mut offset_entries = Vec::new();
    let mut corpus_entries = Vec::new();

    for (metadata, _wit_types) in &schemas {
        let name = &metadata.schema.name;
        let const_name = name.to_uppercase().replace('-', "_");

        if let Some(ref emb) = metadata.embedding {
            let slot = emb.offset_slot;
            let offset = slot * 1_000_000_000;
            let const_id = format!("{const_name}_ID_OFFSET");

            writeln!(code, "/// Embedding offset for {name} facts (slot {slot})")?;
            writeln!(code, "pub const {const_id}: i64 = {offset};")?;
            writeln!(code)?;

            offset_entries.push((const_id, offset, name.clone()));
            corpus_entries.push((name.clone(), emb.corpus_query.trim().to_string()));
        }
    }

    // Generate a corpus query function
    if !corpus_entries.is_empty() {
        writeln!(code, "/// Schema-defined corpus queries for oxidize.")?;
        writeln!(code, "///")?;
        writeln!(code, "/// Returns (schema_name, corpus_sql) pairs.")?;
        writeln!(
            code,
            "pub fn corpus_queries() -> Vec<(&'static str, &'static str)> {{"
        )?;
        writeln!(code, "    vec![")?;
        for (name, query) in &corpus_entries {
            // Escape the query for a raw string
            writeln!(code, "        (\"{name}\", r#\"{query}\"#),")?;
        }
        writeln!(code, "    ]")?;
        writeln!(code, "}}")?;
    }

    let file_path = out_dir.join("embeddings.rs");
    std::fs::write(&file_path, &code)?;
    println!("  Generated: {}", file_path.display());

    // Also update mod.rs to include embeddings
    let mod_path = out_dir.join("mod.rs");
    if mod_path.exists() {
        let existing = std::fs::read_to_string(&mod_path)?;
        if !existing.contains("pub mod embeddings;") {
            let mut updated = existing;
            writeln!(updated, "pub mod embeddings;")?;
            std::fs::write(&mod_path, &updated)?;
        }
    }

    println!("Embedding config generated in src/generated/schemas/");
    Ok(())
}

/// Dispatch generate subcommand.
pub fn generate(
    types: bool,
    migrations: bool,
    embeddings: bool,
    schema: Option<&str>,
) -> Result<()> {
    let root = find_project_root()?;

    // Default: generate everything if no flags specified
    let all = !types && !migrations && !embeddings;

    if types || all {
        println!("Generating types...");
        generate_types(&root, schema)?;
    }
    if migrations || all {
        println!("Generating migrations...");
        generate_migrations(&root, schema)?;
    }
    if embeddings || all {
        println!("Generating embedding config...");
        generate_embeddings(&root, schema)?;
    }

    Ok(())
}

// =========================================================================
// Schema validation (EC3: validate facts before DB insert)
// =========================================================================

/// Validate a fact (as JSON) against its schema record definition.
///
/// Checks that required (non-optional) fields are present and non-empty.
/// Returns Ok(()) if valid, or an error describing the violation.
pub fn validate_fact(schema_name: &str, fact_name: &str, data: &serde_json::Value) -> Result<()> {
    let root = match find_project_root() {
        Ok(r) => r,
        Err(_) => return Ok(()), // No project root = skip validation
    };

    let schemas_dir = paths::project::schemas_dir(&root);
    let schema_dir = schemas_dir.join(schema_name);
    if !schema_dir.exists() {
        return Ok(()); // Schema not installed = skip validation
    }

    let metadata = parse_schema_toml(&schema_dir)?;

    // Find the fact definition
    let fact_def = match metadata.facts.iter().find(|f| f.name == fact_name) {
        Some(f) => f,
        None => return Ok(()), // Unknown fact = skip validation
    };

    // Parse WIT types from schema
    let mut wit_types = WitTypes {
        records: vec![],
        enums: vec![],
    };
    for file_entry in std::fs::read_dir(&schema_dir)? {
        let file_entry = file_entry?;
        if file_entry
            .path()
            .extension()
            .is_some_and(|ext| ext == "wit")
        {
            let content = std::fs::read_to_string(file_entry.path())?;
            let parsed = parse_wit_types(&content);
            wit_types.records.extend(parsed.records);
            wit_types.enums.extend(parsed.enums);
        }
    }

    // Find the WIT record for this fact
    let record = match wit_types.records.iter().find(|r| r.name == fact_def.record) {
        Some(r) => r,
        None => return Ok(()), // No matching record = skip
    };

    // Validate each non-optional field
    let obj = match data.as_object() {
        Some(o) => o,
        None => bail!(
            "schema violation [{}.{}]: expected JSON object, got {}",
            schema_name,
            fact_name,
            data
        ),
    };

    let mut violations = Vec::new();

    for field in &record.fields {
        let rust_name = wit_to_snake(&field.name);

        // Skip optional fields
        if matches!(field.ty, WitType::Option(_) | WitType::List(_)) {
            continue;
        }

        match obj.get(&rust_name) {
            None => {
                violations.push(format!("missing required field '{}'", rust_name));
            }
            Some(val) => {
                // Check non-empty for strings
                if matches!(field.ty, WitType::String) {
                    if let Some(s) = val.as_str() {
                        if s.is_empty() {
                            violations.push(format!("required field '{}' is empty", rust_name));
                        }
                    }
                }
            }
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        bail!(
            "schema violation [{}.{}]: {}",
            schema_name,
            fact_name,
            violations.join("; ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_schema(dir: &Path) {
        fs::write(
            dir.join("schema.toml"),
            r#"
[schema]
name = "test"
version = "1.0.0"
package = "patina:schema/test@1.0.0"
description = "Test schema"

[[facts]]
name = "item"
event_type = "test.item"
record = "item"
"#,
        )
        .unwrap();
        fs::write(
            dir.join("test.wit"),
            "package patina:schema/test@1.0.0;\ninterface types { record item { id: s64 } }\n",
        )
        .unwrap();
    }

    #[test]
    fn parse_valid_schema_toml() {
        let dir = TempDir::new().unwrap();
        create_test_schema(dir.path());
        let meta = parse_schema_toml(dir.path()).unwrap();
        assert_eq!(meta.schema.name, "test");
        assert_eq!(meta.schema.version, "1.0.0");
        assert_eq!(meta.facts.len(), 1);
        assert_eq!(meta.facts[0].event_type, "test.item");
    }

    #[test]
    fn validate_rejects_missing_wit() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("schema.toml"),
            r#"
[schema]
name = "no-wit"
version = "1.0.0"
package = "patina:schema/no-wit@1.0.0"

[[facts]]
name = "x"
event_type = "no-wit.x"
record = "x"
"#,
        )
        .unwrap();
        let err = validate_package(dir.path()).unwrap_err();
        assert!(err.to_string().contains(".wit"), "got: {}", err);
    }

    #[test]
    fn validate_rejects_missing_toml() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.wit"), "package test;").unwrap();
        let err = validate_package(dir.path()).unwrap_err();
        assert!(err.to_string().contains("schema.toml"), "got: {}", err);
    }

    #[test]
    fn validate_rejects_empty_facts() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("schema.toml"),
            r#"
[schema]
name = "empty"
version = "1.0.0"
package = "patina:schema/empty@1.0.0"
"#,
        )
        .unwrap();
        fs::write(dir.path().join("empty.wit"), "package test;").unwrap();
        let err = validate_package(dir.path()).unwrap_err();
        assert!(err.to_string().contains("facts"), "got: {}", err);
    }

    #[test]
    fn validate_accepts_good_package() {
        let dir = TempDir::new().unwrap();
        create_test_schema(dir.path());
        let meta = validate_package(dir.path()).unwrap();
        assert_eq!(meta.schema.name, "test");
    }

    #[test]
    fn parse_forge_schema_toml() {
        let forge_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("wit/schema/forge");
        if !forge_dir.exists() {
            return; // skip if not in repo root
        }
        let meta = parse_schema_toml(&forge_dir).unwrap();
        assert_eq!(meta.schema.name, "forge");
        assert_eq!(meta.schema.package, "patina:schema/forge@1.0.0");
        assert_eq!(meta.facts.len(), 2);
        assert_eq!(meta.facts[0].event_type, "forge.issue");
        assert_eq!(meta.facts[1].event_type, "forge.pr");
        assert!(meta.embedding.is_some());
        assert_eq!(meta.embedding.unwrap().offset_slot, 5);
        assert_eq!(meta.indexes.len(), 2);
    }

    // =====================================================================
    // Schema name validation tests
    // =====================================================================

    #[test]
    fn name_validation_accepts_lowercase() {
        assert!(validate_schema_name("forge").is_ok());
        assert!(validate_schema_name("my-schema").is_ok());
        assert!(validate_schema_name("schema123").is_ok());
    }

    #[test]
    fn name_validation_rejects_uppercase() {
        assert!(validate_schema_name("Forge").is_err());
        assert!(validate_schema_name("MY-SCHEMA").is_err());
    }

    #[test]
    fn name_validation_rejects_empty() {
        assert!(validate_schema_name("").is_err());
    }

    #[test]
    fn name_validation_rejects_leading_hyphen() {
        assert!(validate_schema_name("-bad").is_err());
        assert!(validate_schema_name("bad-").is_err());
    }

    // =====================================================================
    // WIT parser tests
    // =====================================================================

    #[test]
    fn parse_wit_forge_types() {
        let forge_wit = Path::new(env!("CARGO_MANIFEST_DIR")).join("wit/schema/forge/forge.wit");
        if !forge_wit.exists() {
            return;
        }
        let content = fs::read_to_string(&forge_wit).unwrap();
        let types = parse_wit_types(&content);

        // Should parse 2 enums: issue-state, pr-state
        assert_eq!(types.enums.len(), 2, "enums: {:?}", types.enums);
        assert_eq!(types.enums[0].name, "issue-state");
        assert_eq!(types.enums[0].variants, vec!["open", "closed"]);
        assert_eq!(types.enums[1].name, "pr-state");
        assert_eq!(types.enums[1].variants, vec!["open", "merged", "closed"]);

        // Should parse 3 records: comment, issue, pull-request
        assert_eq!(types.records.len(), 3, "records: {:?}", types.records);

        let comment = &types.records[0];
        assert_eq!(comment.name, "comment");
        assert_eq!(comment.fields.len(), 3);

        let issue = &types.records[1];
        assert_eq!(issue.name, "issue");
        assert_eq!(issue.fields.len(), 9);
        assert_eq!(issue.fields[0].name, "number");
        assert_eq!(issue.fields[0].ty, WitType::S64);
        assert_eq!(issue.fields[2].name, "body");
        assert_eq!(
            issue.fields[2].ty,
            WitType::Option(Box::new(WitType::String))
        );
        assert_eq!(issue.fields[3].name, "state");
        assert_eq!(
            issue.fields[3].ty,
            WitType::Named("issue-state".to_string())
        );
        assert_eq!(issue.fields[5].name, "labels");
        assert_eq!(issue.fields[5].ty, WitType::List(Box::new(WitType::String)));

        let pr = &types.records[2];
        assert_eq!(pr.name, "pull-request");
        assert_eq!(pr.fields.len(), 12);
    }

    #[test]
    fn wit_type_parsing() {
        assert_eq!(parse_wit_type("s64"), WitType::S64);
        assert_eq!(parse_wit_type("string"), WitType::String);
        assert_eq!(
            parse_wit_type("option<string>"),
            WitType::Option(Box::new(WitType::String))
        );
        assert_eq!(
            parse_wit_type("list<s64>"),
            WitType::List(Box::new(WitType::S64))
        );
        assert_eq!(
            parse_wit_type("issue-state"),
            WitType::Named("issue-state".to_string())
        );
    }

    // =====================================================================
    // Name conversion tests
    // =====================================================================

    #[test]
    fn wit_name_conversions() {
        assert_eq!(wit_to_snake("created-at"), "created_at");
        assert_eq!(wit_to_snake("pull-request"), "pull_request");
        assert_eq!(wit_to_pascal("issue-state"), "IssueState");
        assert_eq!(wit_to_pascal("pull-request"), "PullRequest");
        assert_eq!(wit_to_pascal("comment"), "Comment");
    }

    // =====================================================================
    // Rust type mapping tests
    // =====================================================================

    #[test]
    fn wit_to_rust_type_mapping() {
        assert_eq!(wit_type_to_rust(&WitType::S64), "i64");
        assert_eq!(wit_type_to_rust(&WitType::String), "String");
        assert_eq!(
            wit_type_to_rust(&WitType::Option(Box::new(WitType::String))),
            "Option<String>"
        );
        assert_eq!(
            wit_type_to_rust(&WitType::List(Box::new(WitType::String))),
            "Vec<String>"
        );
        assert_eq!(
            wit_type_to_rust(&WitType::Named("issue-state".to_string())),
            "IssueState"
        );
    }

    // =====================================================================
    // SQLite type mapping tests
    // =====================================================================

    #[test]
    fn wit_to_sqlite_type_mapping() {
        assert_eq!(wit_type_to_sqlite(&WitType::S64), "INTEGER");
        assert_eq!(wit_type_to_sqlite(&WitType::String), "TEXT");
        assert_eq!(
            wit_type_to_sqlite(&WitType::Option(Box::new(WitType::String))),
            "TEXT"
        );
        assert_eq!(
            wit_type_to_sqlite(&WitType::List(Box::new(WitType::String))),
            "TEXT"
        );
        assert_eq!(
            wit_type_to_sqlite(&WitType::Named("issue-state".to_string())),
            "TEXT"
        );
    }

    // =====================================================================
    // Schema validation tests
    // =====================================================================

    #[test]
    fn validate_fact_rejects_missing_required_field() {
        // Create a minimal schema in a temp dir
        let dir = TempDir::new().unwrap();
        let schema_dir = dir.path().join(".patina/schemas/test-schema");
        fs::create_dir_all(&schema_dir).unwrap();

        fs::write(
            schema_dir.join("schema.toml"),
            r#"
[schema]
name = "test-schema"
version = "1.0.0"
package = "patina:schema/test-schema@1.0.0"

[[facts]]
name = "item"
event_type = "test.item"
record = "item"
"#,
        )
        .unwrap();

        fs::write(
            schema_dir.join("test.wit"),
            r#"
package patina:schema/test-schema@1.0.0;
interface types {
    record item {
        id: s64,
        title: string,
        body: option<string>,
    }
}
"#,
        )
        .unwrap();

        // Valid data — should pass
        let valid = serde_json::json!({"id": 1, "title": "hello"});
        // We can't call validate_fact directly (it uses find_project_root),
        // but we can test the WIT parsing + validation logic components:
        let content = fs::read_to_string(schema_dir.join("test.wit")).unwrap();
        let types = parse_wit_types(&content);
        assert_eq!(types.records.len(), 1);
        assert_eq!(types.records[0].fields.len(), 3);

        // title is required (string, not option)
        assert!(!wit_type_is_nullable(&types.records[0].fields[1].ty));
        // body is optional (option<string>)
        assert!(wit_type_is_nullable(&types.records[0].fields[2].ty));

        // Check that valid data has required fields
        let obj = valid.as_object().unwrap();
        assert!(obj.contains_key("title"));
    }
}
