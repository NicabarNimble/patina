use patina::semantic::languages::{create_parser, Language};

fn main() -> anyhow::Result<()> {
    println!("Testing parser creation...");
    
    // Test each language
    let languages = vec![
        Language::Rust,
        Language::Go,
        Language::Solidity,
        Language::Python,
        Language::JavaScript,
        Language::TypeScript,
    ];
    
    for lang in languages {
        print!("Testing {:?}... ", lang);
        match create_parser(lang) {
            Ok(mut parser) => {
                println!("✅ Parser created");
                // Try parsing a simple snippet
                let code = match lang {
                    Language::Rust => "fn main() {}",
                    Language::Go => "func main() {}",
                    Language::Solidity => "contract Test {}",
                    Language::Python => "def main(): pass",
                    Language::JavaScript => "function main() {}",
                    Language::TypeScript => "function main(): void {}",
                    _ => "",
                };
                
                match parser.parse(code, None) {
                    Some(tree) => println!("  ✅ Parsed sample code (root: {})", tree.root_node().kind()),
                    None => println!("  ❌ Failed to parse sample code"),
                }
            }
            Err(e) => {
                println!("❌ Failed: {}", e);
            }
        }
    }
    
    Ok(())
}