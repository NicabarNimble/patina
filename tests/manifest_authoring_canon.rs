use std::fs;
use std::path::{Path, PathBuf};

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[test]
fn child_manifest_authoring_surfaces_use_needs_not_capabilities() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut checked = Vec::new();

    // Authoring surfaces only: real child manifests + scaffold templates.
    let children_dir = repo.join("children");
    if children_dir.exists() {
        for entry in fs::read_dir(&children_dir).expect("read children dir") {
            let path = entry.expect("dir entry").path().join("child.toml");
            if path.exists() {
                checked.push(path);
            }
        }
    }

    let template_dir = repo.join("resources/templates/child");
    let mut template_files = Vec::new();
    collect_files(&template_dir, &mut template_files);
    for path in template_files {
        if path.extension().and_then(|e| e.to_str()) == Some("tmpl")
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with("child.toml.tmpl"))
                .unwrap_or(false)
        {
            checked.push(path);
        }
    }

    let mut offenders = Vec::new();
    for path in checked {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        if content.contains("[capabilities]") {
            offenders.push(path);
        }
    }

    assert!(
        offenders.is_empty(),
        "authoring manifests/templates must use [needs].toys; found legacy [capabilities] in: {}",
        offenders
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
}
