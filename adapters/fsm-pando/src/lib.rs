wit_bindgen::generate!({
    path: "wit",
    world: "fsm-pando",
    generate_all,
});

struct FsmPando;

impl exports::patina::pando::source::Guest for FsmPando {
    fn run() -> Result<Vec<patina::records::types::FileFound>, String> {
        let folder = patina::config::config::get("folder_path")?;
        patina::records::source::scan(&folder)
    }
}

export!(FsmPando);
