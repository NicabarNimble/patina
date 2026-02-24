// Scaffolded for spec-create — will be used when that spec is implemented.
#![allow(dead_code)]

use patina::release::BumpType;

/// A registered spec type with all its conventions.
pub struct SpecType {
    /// Type name used in frontmatter and CLI: "feat", "fix", etc.
    pub name: &'static str,
    /// Version bump behavior. None = no release (explore).
    pub bump: Option<BumpType>,
    /// Directory under layer/surface/build/
    pub directory: &'static str,
    /// Markdown body template (section headings only).
    pub body_template: &'static str,
}

/// All registered spec types.
pub const SPEC_TYPES: &[SpecType] = &[
    SpecType {
        name: "feat",
        bump: Some(BumpType::Minor),
        directory: "feat",
        body_template: FEAT_TEMPLATE,
    },
    SpecType {
        name: "fix",
        bump: Some(BumpType::Patch),
        directory: "fix",
        body_template: FIX_TEMPLATE,
    },
    SpecType {
        name: "refactor",
        bump: Some(BumpType::Patch),
        directory: "refactor",
        body_template: REFACTOR_TEMPLATE,
    },
    SpecType {
        name: "explore",
        bump: None,
        directory: "explore",
        body_template: EXPLORE_TEMPLATE,
    },
];

/// Look up a spec type by name.
pub fn lookup(name: &str) -> Option<&'static SpecType> {
    SPEC_TYPES.iter().find(|t| t.name == name)
}

// Body templates — section headings only, LLM/user fills in content.
// Derived from survey of 117 archived specs.

const FEAT_TEMPLATE: &str = "\
## Problem

## Solution

## Design Decisions

## Implementation

## Exit Criteria

- [ ]

## Key Files

```
```

## Non-Goals

## Provenance
";

const FIX_TEMPLATE: &str = "\
## Problem

## Solution

## Exit Criteria

- [ ]

## Key Files

```
```

## Provenance
";

const REFACTOR_TEMPLATE: &str = "\
## Problem

## Solution

## Migration

## Exit Criteria

- [ ]

## Key Files

```
```

## Provenance
";

const EXPLORE_TEMPLATE: &str = "\
## Exit Criteria

- [ ]
";
