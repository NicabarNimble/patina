use patina_metal::{Analyzer, Metal};

#[test]
fn debug_analyzer_creation() {
    println!("Creating analyzer...");
    let analyzer = Analyzer::new().expect("Failed to create analyzer");

    println!("Available metals: {:?}", analyzer.available_metals());

    for metal in analyzer.available_metals() {
        println!(
            "Metal {:?} has parser: {}",
            metal,
            analyzer.has_parser(metal)
        );
    }
}
