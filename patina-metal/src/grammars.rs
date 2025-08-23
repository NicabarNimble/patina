use tree_sitter::Language;

extern "C" {
    fn tree_sitter_rust() -> Language;
    fn tree_sitter_go() -> Language;
    fn tree_sitter_solidity() -> Language;
    fn tree_sitter_python() -> Language;
    fn tree_sitter_javascript() -> Language;
    fn tree_sitter_typescript() -> Language;
    fn tree_sitter_tsx() -> Language;
}

pub fn language_rust() -> Language {
    unsafe { tree_sitter_rust() }
}

pub fn language_go() -> Language {
    unsafe { tree_sitter_go() }
}

pub fn language_solidity() -> Language {
    unsafe { tree_sitter_solidity() }
}

pub fn language_python() -> Language {
    unsafe { tree_sitter_python() }
}

pub fn language_javascript() -> Language {
    unsafe { tree_sitter_javascript() }
}

pub fn language_typescript() -> Language {
    unsafe { tree_sitter_typescript() }
}

pub fn language_tsx() -> Language {
    unsafe { tree_sitter_tsx() }
}
