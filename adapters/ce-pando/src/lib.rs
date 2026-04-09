wit_bindgen::generate!({
    path: "wit",
    world: "ce-pando",
    generate_all,
});

struct CePando;

impl exports::patina::pando::extract::Guest for CePando {
    fn run() -> Result<Vec<patina::records::types::RecordEnvelope>, String> {
        let files = patina::pando::source::run()?;
        patina::records::extract::extract(&files)
    }
}

export!(CePando);
