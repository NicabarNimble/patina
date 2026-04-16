#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillContentMode {
    Markdown,
    Toml,
    Executable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillFile {
    pub projection_file: &'static str,
    pub bytes: &'static str,
    pub mode: SkillContentMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillContent {
    pub files: &'static [SkillFile],
}

const CLAUDE_SESSION_START: &str = include_str!("../../../resources/claude/session-start.md");
const CLAUDE_SESSION_UPDATE: &str = include_str!("../../../resources/claude/session-update.md");
const CLAUDE_SESSION_NOTE: &str = include_str!("../../../resources/claude/session-note.md");
const CLAUDE_SESSION_END: &str = include_str!("../../../resources/claude/session-end.md");
const CLAUDE_PATINA_REVIEW: &str = include_str!("../../../resources/claude/patina-review.md");
const CLAUDE_SPEC: &str = include_str!("../../../resources/claude/spec.md");
const CLAUDE_SKILL_EPISTEMIC_BELIEFS: &str =
    include_str!("../../../resources/claude/skills/epistemic-beliefs/SKILL.md");
const CLAUDE_SKILL_CREATE_BELIEF: &str =
    include_str!("../../../resources/claude/skills/epistemic-beliefs/scripts/create-belief.sh");
const CLAUDE_SKILL_EXAMPLE: &str =
    include_str!("../../../resources/claude/skills/epistemic-beliefs/references/belief-example.md");
const CLAUDE_SKILL_VERIFICATION_SCHEMA: &str = include_str!(
    "../../../resources/claude/skills/epistemic-beliefs/references/verification-schema.md"
);

const GEMINI_SESSION_START: &str = include_str!("../../../resources/gemini/session-start.toml");
const GEMINI_SESSION_UPDATE: &str = include_str!("../../../resources/gemini/session-update.toml");
const GEMINI_SESSION_NOTE: &str = include_str!("../../../resources/gemini/session-note.toml");
const GEMINI_SESSION_END: &str = include_str!("../../../resources/gemini/session-end.toml");
const GEMINI_PATINA_REVIEW: &str = include_str!("../../../resources/gemini/patina-review.toml");
const GEMINI_SPEC: &str = include_str!("../../../resources/gemini/spec.toml");
const GEMINI_EPISTEMIC_BELIEFS: &str =
    include_str!("../../../resources/gemini/epistemic-beliefs.toml");

const OPENCODE_SESSION_START: &str = include_str!("../../../resources/opencode/session-start.md");
const OPENCODE_SESSION_UPDATE: &str = include_str!("../../../resources/opencode/session-update.md");
const OPENCODE_SESSION_NOTE: &str = include_str!("../../../resources/opencode/session-note.md");
const OPENCODE_SESSION_END: &str = include_str!("../../../resources/opencode/session-end.md");
const OPENCODE_PATINA_REVIEW: &str = include_str!("../../../resources/opencode/patina-review.md");
const OPENCODE_SPEC: &str = include_str!("../../../resources/opencode/spec.md");
const OPENCODE_EPISTEMIC_BELIEFS: &str =
    include_str!("../../../resources/opencode/epistemic-beliefs.md");

const SKILL_WRAPPER_SET: &[&str] = &[
    "session-start",
    "session-update",
    "session-note",
    "session-end",
    "spec",
    "epistemic-beliefs",
];

pub fn known_skills(interface: &str) -> Vec<&'static str> {
    match interface {
        "claude" | "gemini" | "opencode" | "pi" => SKILL_WRAPPER_SET.to_vec(),
        _ => vec![],
    }
}

pub fn skill_content(interface: &str, skill: &str) -> Option<SkillContent> {
    match (interface, skill) {
        ("claude", "session-start") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/session-start.md",
                bytes: CLAUDE_SESSION_START,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("claude", "session-update") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/session-update.md",
                bytes: CLAUDE_SESSION_UPDATE,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("claude", "session-note") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/session-note.md",
                bytes: CLAUDE_SESSION_NOTE,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("claude", "session-end") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/session-end.md",
                bytes: CLAUDE_SESSION_END,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("claude", "patina-review") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/patina-review.md",
                bytes: CLAUDE_PATINA_REVIEW,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("claude", "spec") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/spec.md",
                bytes: CLAUDE_SPEC,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("claude", "epistemic-beliefs") => Some(SkillContent {
            files: &[
                SkillFile {
                    projection_file: "skills/epistemic-beliefs/SKILL.md",
                    bytes: CLAUDE_SKILL_EPISTEMIC_BELIEFS,
                    mode: SkillContentMode::Markdown,
                },
                SkillFile {
                    projection_file: "skills/epistemic-beliefs/scripts/create-belief.sh",
                    bytes: CLAUDE_SKILL_CREATE_BELIEF,
                    mode: SkillContentMode::Executable,
                },
                SkillFile {
                    projection_file: "skills/epistemic-beliefs/references/belief-example.md",
                    bytes: CLAUDE_SKILL_EXAMPLE,
                    mode: SkillContentMode::Markdown,
                },
                SkillFile {
                    projection_file: "skills/epistemic-beliefs/references/verification-schema.md",
                    bytes: CLAUDE_SKILL_VERIFICATION_SCHEMA,
                    mode: SkillContentMode::Markdown,
                },
            ],
        }),
        ("gemini", "session-start") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/session-start.toml",
                bytes: GEMINI_SESSION_START,
                mode: SkillContentMode::Toml,
            }],
        }),
        ("gemini", "session-update") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/session-update.toml",
                bytes: GEMINI_SESSION_UPDATE,
                mode: SkillContentMode::Toml,
            }],
        }),
        ("gemini", "session-note") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/session-note.toml",
                bytes: GEMINI_SESSION_NOTE,
                mode: SkillContentMode::Toml,
            }],
        }),
        ("gemini", "session-end") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/session-end.toml",
                bytes: GEMINI_SESSION_END,
                mode: SkillContentMode::Toml,
            }],
        }),
        ("gemini", "patina-review") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/patina-review.toml",
                bytes: GEMINI_PATINA_REVIEW,
                mode: SkillContentMode::Toml,
            }],
        }),
        ("gemini", "spec") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/spec.toml",
                bytes: GEMINI_SPEC,
                mode: SkillContentMode::Toml,
            }],
        }),
        ("gemini", "epistemic-beliefs") => Some(SkillContent {
            files: &[
                SkillFile {
                    projection_file: "commands/epistemic-beliefs.toml",
                    bytes: GEMINI_EPISTEMIC_BELIEFS,
                    mode: SkillContentMode::Toml,
                },
                SkillFile {
                    projection_file: "bin/create-belief.sh",
                    bytes: CLAUDE_SKILL_CREATE_BELIEF,
                    mode: SkillContentMode::Executable,
                },
            ],
        }),
        ("opencode", "session-start") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/session-start.md",
                bytes: OPENCODE_SESSION_START,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("opencode", "session-update") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/session-update.md",
                bytes: OPENCODE_SESSION_UPDATE,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("opencode", "session-note") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/session-note.md",
                bytes: OPENCODE_SESSION_NOTE,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("opencode", "session-end") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/session-end.md",
                bytes: OPENCODE_SESSION_END,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("opencode", "patina-review") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/patina-review.md",
                bytes: OPENCODE_PATINA_REVIEW,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("opencode", "spec") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/spec.md",
                bytes: OPENCODE_SPEC,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("opencode", "epistemic-beliefs") => Some(SkillContent {
            files: &[
                SkillFile {
                    projection_file: "commands/epistemic-beliefs.md",
                    bytes: OPENCODE_EPISTEMIC_BELIEFS,
                    mode: SkillContentMode::Markdown,
                },
                SkillFile {
                    projection_file: "bin/create-belief.sh",
                    bytes: CLAUDE_SKILL_CREATE_BELIEF,
                    mode: SkillContentMode::Executable,
                },
            ],
        }),
        ("pi", "session-start") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/session-start.md",
                bytes: OPENCODE_SESSION_START,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("pi", "session-update") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/session-update.md",
                bytes: OPENCODE_SESSION_UPDATE,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("pi", "session-note") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/session-note.md",
                bytes: OPENCODE_SESSION_NOTE,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("pi", "session-end") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/session-end.md",
                bytes: OPENCODE_SESSION_END,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("pi", "patina-review") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/patina-review.md",
                bytes: OPENCODE_PATINA_REVIEW,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("pi", "spec") => Some(SkillContent {
            files: &[SkillFile {
                projection_file: "commands/spec.md",
                bytes: OPENCODE_SPEC,
                mode: SkillContentMode::Markdown,
            }],
        }),
        ("pi", "epistemic-beliefs") => Some(SkillContent {
            files: &[
                SkillFile {
                    projection_file: "commands/epistemic-beliefs.md",
                    bytes: OPENCODE_EPISTEMIC_BELIEFS,
                    mode: SkillContentMode::Markdown,
                },
                SkillFile {
                    projection_file: "bin/create-belief.sh",
                    bytes: CLAUDE_SKILL_CREATE_BELIEF,
                    mode: SkillContentMode::Executable,
                },
            ],
        }),
        _ => None,
    }
}
