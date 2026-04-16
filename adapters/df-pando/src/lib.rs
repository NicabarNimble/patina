wit_bindgen::generate!({
    path: "wit",
    world: "df-pando",
    generate_all,
});

struct DfPando;

impl exports::patina::pando::transform::Guest for DfPando {
    fn run() -> Result<patina::records::types::TransformResult, String> {
        let upstream = patina::pando::transform::run()?;
        patina::records::transform::transform(&upstream.accepted)
    }
}

export!(DfPando);
