use std::path::PathBuf;

struct ArtifactSpec {
    kind: &'static str,
    crate_stem: &'static str,
    max_bytes: u64,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn artifact_path(kind: &str, crate_stem: &str) -> Option<PathBuf> {
    let root = workspace_root();
    let prefix = match kind {
        "child" => "patina_ai_child",
        "adapter" => "patina_ai_adapter",
        _ => return None,
    };
    let candidates = [
        root.join(format!(
            "target/wasm32-wasip2/debug/{prefix}_{crate_stem}.wasm"
        )),
        root.join(format!(
            "target/wasm32-wasip2/release/{prefix}_{crate_stem}.wasm"
        )),
    ];
    candidates.into_iter().find(|path| path.exists())
}

fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KiB", bytes as f64 / KB)
    } else {
        format!("{:.2} MiB", bytes as f64 / MB)
    }
}

#[test]
fn pando_artifact_size_baseline_post_fix2() {
    // Post-Fix-2 baseline checkpoint captured in CI/log output as of 2026-04-09.
    let artifacts = [
        ArtifactSpec {
            kind: "child",
            crate_stem: "file_system_monitor",
            max_bytes: 120 * 1024 * 1024,
        },
        ArtifactSpec {
            kind: "child",
            crate_stem: "content_extractor",
            max_bytes: 120 * 1024 * 1024,
        },
        ArtifactSpec {
            kind: "child",
            crate_stem: "schema_enforcer",
            max_bytes: 120 * 1024 * 1024,
        },
        ArtifactSpec {
            kind: "child",
            crate_stem: "dedup_filter",
            max_bytes: 120 * 1024 * 1024,
        },
        ArtifactSpec {
            kind: "child",
            crate_stem: "record_writer",
            max_bytes: 120 * 1024 * 1024,
        },
        ArtifactSpec {
            kind: "child",
            crate_stem: "lakehouse_catalog",
            max_bytes: 120 * 1024 * 1024,
        },
        ArtifactSpec {
            kind: "adapter",
            crate_stem: "fsm_pando",
            max_bytes: 6 * 1024 * 1024,
        },
        ArtifactSpec {
            kind: "adapter",
            crate_stem: "ce_pando",
            max_bytes: 6 * 1024 * 1024,
        },
        ArtifactSpec {
            kind: "adapter",
            crate_stem: "se_pando",
            max_bytes: 6 * 1024 * 1024,
        },
        ArtifactSpec {
            kind: "adapter",
            crate_stem: "df_pando",
            max_bytes: 6 * 1024 * 1024,
        },
        ArtifactSpec {
            kind: "adapter",
            crate_stem: "rw_pando",
            max_bytes: 6 * 1024 * 1024,
        },
        ArtifactSpec {
            kind: "adapter",
            crate_stem: "lc_pando",
            max_bytes: 6 * 1024 * 1024,
        },
    ];

    println!("\nPando WASM artifact size table (post-Fix-2 baseline):");
    println!("kind\tcrate\tbytes\thuman\tpath");

    for artifact in artifacts {
        let path = artifact_path(artifact.kind, artifact.crate_stem).unwrap_or_else(|| {
            panic!(
                "missing wasm artifact for {}:{} (build wasm32-wasip2 first)",
                artifact.kind, artifact.crate_stem
            )
        });
        let metadata = std::fs::metadata(&path)
            .unwrap_or_else(|error| panic!("failed to stat '{}': {}", path.display(), error));
        let bytes = metadata.len();

        assert!(
            bytes > 0,
            "{}:{} has zero size",
            artifact.kind,
            artifact.crate_stem
        );
        assert!(
            bytes < artifact.max_bytes,
            "{}:{} is {} bytes (limit {})",
            artifact.kind,
            artifact.crate_stem,
            bytes,
            artifact.max_bytes
        );

        let rel_path = path
            .strip_prefix(workspace_root())
            .unwrap_or(path.as_path())
            .display()
            .to_string();
        println!(
            "{}\t{}\t{}\t{}\t{}",
            artifact.kind,
            artifact.crate_stem,
            bytes,
            human_size(bytes),
            rel_path
        );
    }
}
