wit_bindgen::generate!({
    path: "wit",
    world: "rw-pando",
    generate_all,
});

struct RwPando;

impl exports::patina::pando::write::Guest for RwPando {
    fn run() -> Result<Vec<patina::records::types::FileWritten>, String> {
        let records = patina::pando::transform::run()?;
        patina::records::write::write(&records.accepted)
    }
}

export!(RwPando);
