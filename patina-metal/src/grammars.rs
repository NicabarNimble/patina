use tree_sitter::Language;

extern "C" {
    fn tree_sitter_rust() -> Language;
    fn tree_sitter_go() -> Language;
    fn tree_sitter_solidity() -> Language;
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