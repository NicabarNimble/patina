use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tree_sitter::Node;

/// Compact 16-byte fingerprint for code patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fingerprint {
    pub pattern: u32,    // AST shape hash
    pub imports: u32,    // Dependency hash
    pub complexity: u16, // Cyclomatic complexity
    pub flags: u16,      // Feature flags
}

impl Fingerprint {
    /// Generate fingerprint from tree-sitter AST node
    pub fn from_ast(node: Node, source: &[u8]) -> Self {
        let pattern = hash_ast_shape(node, source);
        let imports = hash_imports(node, source);
        let complexity = calculate_complexity(node) as u16;
        let flags = detect_features(node, source);

        Self {
            pattern,
            imports,
            complexity,
            flags,
        }
    }

    /// Convert to bytes for storage
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&self.pattern.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.imports.to_le_bytes());
        bytes[8..10].copy_from_slice(&self.complexity.to_le_bytes());
        bytes[10..12].copy_from_slice(&self.flags.to_le_bytes());
        // bytes[12..16] reserved for future use
        bytes
    }

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8; 16]) -> Self {
        Self {
            pattern: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            imports: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            complexity: u16::from_le_bytes([bytes[8], bytes[9]]),
            flags: u16::from_le_bytes([bytes[10], bytes[11]]),
        }
    }
}

/// Hash the AST structure (types only, not content)
fn hash_ast_shape(node: Node, _source: &[u8]) -> u32 {
    let mut hasher = DefaultHasher::new();
    hash_node_shape(&mut hasher, node);
    (hasher.finish() & 0xFFFFFFFF) as u32
}

fn hash_node_shape(hasher: &mut impl Hasher, node: Node) {
    // Hash node type (structure, not content)
    node.kind().hash(hasher);

    // Hash child structure recursively
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            hash_node_shape(hasher, cursor.node());
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// Hash imports/dependencies
fn hash_imports(node: Node, source: &[u8]) -> u32 {
    let mut hasher = DefaultHasher::new();
    let mut cursor = node.walk();

    find_imports(&mut cursor, source, &mut hasher);
    (hasher.finish() & 0xFFFFFFFF) as u32
}

fn find_imports(cursor: &mut tree_sitter::TreeCursor, source: &[u8], hasher: &mut impl Hasher) {
    let node = cursor.node();

    if node.kind() == "use_declaration" {
        if let Ok(text) = node.utf8_text(source) {
            text.hash(hasher);
        }
    }

    if cursor.goto_first_child() {
        loop {
            find_imports(cursor, source, hasher);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Calculate cyclomatic complexity
fn calculate_complexity(node: Node) -> usize {
    let mut complexity = 1; // Base complexity
    let mut cursor = node.walk();

    count_branches(&mut cursor, &mut complexity);
    complexity
}

fn count_branches(cursor: &mut tree_sitter::TreeCursor, complexity: &mut usize) {
    let node = cursor.node();

    match node.kind() {
        "if_expression" | "match_expression" | "while_expression" | "for_expression" => {
            *complexity += 1;
        }
        "match_arm" => {
            // Each arm adds a branch
            *complexity += 1;
        }
        _ => {}
    }

    if cursor.goto_first_child() {
        loop {
            count_branches(cursor, complexity);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Detect feature flags (async, unsafe, etc.)
fn detect_features(node: Node, source: &[u8]) -> u16 {
    let mut flags = 0u16;
    let mut cursor = node.walk();

    detect_features_recursive(&mut cursor, source, &mut flags);
    flags
}

fn detect_features_recursive(cursor: &mut tree_sitter::TreeCursor, source: &[u8], flags: &mut u16) {
    let node = cursor.node();

    // Check for various features
    match node.kind() {
        "async" => *flags |= 0x0001,                   // Bit 0: async
        "unsafe_block" | "unsafe" => *flags |= 0x0002, // Bit 1: unsafe
        "macro_invocation" => {
            if let Ok(text) = node.utf8_text(source) {
                if text.starts_with("panic!") || text.starts_with("unreachable!") {
                    *flags |= 0x0004; // Bit 2: has panic
                }
                if text.starts_with("todo!") || text.starts_with("unimplemented!") {
                    *flags |= 0x0008; // Bit 3: has todo
                }
            }
        }
        "question_mark" => *flags |= 0x0010, // Bit 4: uses ?
        "generic_type" | "generic_function" => *flags |= 0x0020, // Bit 5: generic
        "trait_bounds" => *flags |= 0x0040,  // Bit 6: has trait bounds
        "lifetime" => *flags |= 0x0080,      // Bit 7: has lifetimes
        _ => {}
    }

    if cursor.goto_first_child() {
        loop {
            detect_features_recursive(cursor, source, flags);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Generate DuckDB schema for fingerprint storage
pub fn generate_schema() -> &'static str {
    r#"
-- Compact fingerprint storage (columnar for SIMD)
CREATE TABLE IF NOT EXISTS code_fingerprints (
    path VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    kind VARCHAR NOT NULL,  -- function, struct, trait, impl
    pattern UINTEGER,       -- AST shape hash
    imports UINTEGER,       -- Dependency hash  
    complexity USMALLINT,   -- Cyclomatic complexity
    flags USMALLINT,        -- Feature bitmask
    PRIMARY KEY (path, name, kind)
);

-- Full-text search for actual code search
CREATE TABLE IF NOT EXISTS code_search (
    path VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    signature VARCHAR,      -- Function/struct signature
    context VARCHAR,        -- Surrounding code snippet
    PRIMARY KEY (path, name)
);

-- Type vocabulary: The domain language (compiler-verified truth)
CREATE TABLE IF NOT EXISTS type_vocabulary (
    file VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    definition TEXT,        -- 'type NodeId = u32' or 'struct User { ... }'
    kind VARCHAR,          -- 'type_alias', 'struct', 'enum', 'const'
    visibility VARCHAR,     -- 'pub', 'pub(crate)', 'private'
    usage_count INTEGER DEFAULT 0,
    PRIMARY KEY (file, name)
);

-- Function facts: Behavioral signals without interpretation
CREATE TABLE IF NOT EXISTS function_facts (
    file VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    takes_mut_self BOOLEAN,     -- Thread safety signal
    takes_mut_params BOOLEAN,   -- Mutation indicator
    returns_result BOOLEAN,     -- Error handling
    returns_option BOOLEAN,     -- Nullability
    is_async BOOLEAN,          -- Concurrency
    is_unsafe BOOLEAN,         -- Safety requirements
    is_public BOOLEAN,         -- API surface
    parameter_count INTEGER,
    generic_count INTEGER,      -- Complexity indicator
    PRIMARY KEY (file, name)
);

-- Import facts: Navigation and dependencies
CREATE TABLE IF NOT EXISTS import_facts (
    importer_file VARCHAR NOT NULL,
    imported_item VARCHAR NOT NULL,
    imported_from VARCHAR,      -- Source module/crate
    is_external BOOLEAN,       -- External crate?
    import_kind VARCHAR,        -- 'use', 'mod', 'extern'
    PRIMARY KEY (importer_file, imported_item)
);

-- Behavioral hints: Code smell detection (facts only)
CREATE TABLE IF NOT EXISTS behavioral_hints (
    file VARCHAR NOT NULL,
    function VARCHAR NOT NULL,
    calls_unwrap INTEGER DEFAULT 0,     -- Count of .unwrap()
    calls_expect INTEGER DEFAULT 0,     -- Count of .expect()
    has_panic_macro BOOLEAN,           -- Contains panic!()
    has_todo_macro BOOLEAN,            -- Contains todo!()
    has_unsafe_block BOOLEAN,          -- Contains unsafe {}
    has_mutex BOOLEAN,                 -- Thread synchronization
    has_arc BOOLEAN,                   -- Shared ownership
    PRIMARY KEY (file, function)
);

-- Index metadata for incremental updates
CREATE TABLE IF NOT EXISTS index_state (
    path VARCHAR PRIMARY KEY,
    mtime BIGINT NOT NULL,
    hash VARCHAR,           -- File content hash
    indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for fast lookups
CREATE INDEX IF NOT EXISTS idx_fingerprint_pattern ON code_fingerprints(pattern);
CREATE INDEX IF NOT EXISTS idx_fingerprint_complexity ON code_fingerprints(complexity);
CREATE INDEX IF NOT EXISTS idx_fingerprint_flags ON code_fingerprints(flags);
CREATE INDEX IF NOT EXISTS idx_type_vocabulary_kind ON type_vocabulary(kind);
CREATE INDEX IF NOT EXISTS idx_function_facts_public ON function_facts(is_public);
CREATE INDEX IF NOT EXISTS idx_import_facts_external ON import_facts(is_external);
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_serialization() {
        let fp = Fingerprint {
            pattern: 0x12345678,
            imports: 0x9ABCDEF0,
            complexity: 42,
            flags: 0b1010_1010_1010_1010,
        };

        let bytes = fp.to_bytes();
        let restored = Fingerprint::from_bytes(&bytes);

        assert_eq!(fp, restored);
    }

    #[test]
    fn test_feature_flags() {
        // Bit meanings:
        // 0: async, 1: unsafe, 2: panic, 3: todo
        // 4: ?, 5: generic, 6: trait bounds, 7: lifetimes

        let flags = 0b0001_0001; // async + uses ?
        assert_eq!(flags & 0x0001, 0x0001); // has async
        assert_eq!(flags & 0x0010, 0x0010); // has ?
        assert_eq!(flags & 0x0002, 0x0000); // no unsafe
    }
}
