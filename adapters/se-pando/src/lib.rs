wit_bindgen::generate!({
    path: "wit",
    world: "se-pando",
    generate_all,
});

struct SePando;

impl exports::patina::pando::transform::Guest for SePando {
    fn run() -> Result<patina::records::types::TransformResult, String> {
        let records = patina::pando::extract::run()?;
        patina::records::transform::transform(&records)
    }
}

export!(SePando);
