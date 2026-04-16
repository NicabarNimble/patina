wit_bindgen::generate!({
    path: "wit",
    world: "lc-pando",
    generate_all,
});

struct LcPando;

impl exports::patina::pando::catalog::Guest for LcPando {
    fn run() -> Result<Vec<patina::records::types::CatalogEntry>, String> {
        let files = patina::pando::write::run()?;
        patina::records::catalog::register(&files)
    }
}

export!(LcPando);
