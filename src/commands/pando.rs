pub fn native_command_names() -> Vec<String> {
    let names = vec![
        "init",
        "doctor",
        "child",
        "version",
        "scrape",
        "oxidize",
        "rebuild",
        "scry",
        "context",
        "eval",
        "bench",
        "persona",
        "repo",
        "model",
        "connect",
        "lake",
        "mother",
        "pando",
        "secrets",
        "yolo",
        "serve",
        "interface",
        "report",
        "measure",
        "ai",
        "hook",
        "belief",
        "setup",
        "spec",
        "schema",
        "events",
        "assay",
    ];

    #[cfg(feature = "dev")]
    {
        names.push("upgrade");
        names.push("dev");
    }

    names.into_iter().map(String::from).collect()
}

pub fn init_registry_best_effort() {
    let request = patina_protocol::PandoRegistryInit {
        protocol_version: patina_protocol::PANDO_REGISTRY_PROTOCOL_VERSION,
        native_commands: native_command_names(),
        binary_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let _ = patina::mother::control_plane_client().pando_registry_init(&request);
}
