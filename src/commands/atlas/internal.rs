use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use patina::spec::parse_spec_file;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use super::AtlasOptions;

#[derive(Debug, Clone, Serialize)]
pub struct AtlasSnapshot {
    pub generated_at: String,
    pub project_root: String,
    pub summary: AtlasSummary,
    pub specs: Vec<SpecNode>,
    pub spec_edges: Vec<SpecEdge>,
    pub children: Vec<ChildNode>,
    pub toys: Vec<ToyNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AtlasSummary {
    pub spec_count: usize,
    pub status_counts: BTreeMap<String, usize>,
    pub lane_counts: BTreeMap<String, usize>,
    pub edge_count: usize,
    pub child_count: usize,
    pub toy_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpecNode {
    pub id: String,
    pub title: String,
    pub path: String,
    pub spec_type: String,
    pub status: String,
    pub target: Option<String>,
    pub lane: String,
    pub checked: usize,
    pub total: usize,
    pub passed: bool,
    pub blocked_by: Vec<String>,
    pub related: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpecEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChildNode {
    pub folder: String,
    pub manifest_name: String,
    pub kind: Option<String>,
    pub role: Option<String>,
    pub toys: Vec<String>,
    pub lane_hint: String,
    pub has_wit_world: bool,
    pub handle_lane: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToyNode {
    pub id: String,
    pub file: Option<String>,
    pub version: Option<String>,
    pub source: Option<String>,
}

pub fn generate(options: AtlasOptions) -> Result<()> {
    if options.html && options.json {
        anyhow::bail!("atlas flags conflict: choose one of --json or --html");
    }

    let root = std::env::current_dir().context("resolve current directory")?;
    let snapshot = build_snapshot(&root)?;

    if options.html {
        let html = render_html(&snapshot)?;
        let output = options
            .output
            .unwrap_or_else(|| "layer/surface/reports/atlas/spec-atlas.html".to_string());
        write_output(Path::new(&output), &html)?;
        println!("Atlas HTML written to {}", output);
        return Ok(());
    }

    let json = serde_json::to_string_pretty(&snapshot)?;
    if let Some(path) = options.output {
        write_output(Path::new(&path), &json)?;
        println!("Atlas JSON written to {}", path);
    } else {
        println!("{}", json);
    }

    Ok(())
}

pub(crate) fn build_snapshot(root: &Path) -> Result<AtlasSnapshot> {
    let specs = scan_specs(root)?;
    let spec_edges = build_spec_edges(&specs);
    let children = scan_children(root)?;
    let toys = scan_toys(root)?;

    let mut status_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut lane_counts: BTreeMap<String, usize> = BTreeMap::new();

    for spec in &specs {
        *status_counts.entry(spec.status.clone()).or_insert(0) += 1;
        *lane_counts.entry(spec.lane.clone()).or_insert(0) += 1;
    }

    let summary = AtlasSummary {
        spec_count: specs.len(),
        status_counts,
        lane_counts,
        edge_count: spec_edges.len(),
        child_count: children.len(),
        toy_count: toys.len(),
    };

    Ok(AtlasSnapshot {
        generated_at: Utc::now().to_rfc3339(),
        project_root: root.to_string_lossy().to_string(),
        summary,
        specs,
        spec_edges,
        children,
        toys,
    })
}

fn scan_specs(root: &Path) -> Result<Vec<SpecNode>> {
    let build_dir = root.join("layer/surface/build");
    if !build_dir.exists() {
        return Ok(vec![]);
    }

    let mut paths = Vec::new();
    collect_named_files(&build_dir, "SPEC.md", &mut paths)?;
    paths.sort();

    let mut specs = Vec::new();
    for path in paths {
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("read spec file {}", path.display()))?;
        let (frontmatter, body) = parse_spec_file(&raw)
            .map_err(|e| anyhow!("parse spec frontmatter {}: {}", path.display(), e))?;

        if frontmatter.id.trim().is_empty() {
            anyhow::bail!("spec {} missing frontmatter id", path.display());
        }

        let title = first_heading(&body).unwrap_or_else(|| frontmatter.id.clone());
        let total = frontmatter.exit_criteria.len();
        let checked = frontmatter
            .exit_criteria
            .iter()
            .filter(|c| c.checked)
            .count();
        let status = frontmatter
            .status
            .map(|s| s.to_string())
            .unwrap_or_else(|| "draft".to_string());

        let relative_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        let lane = infer_spec_lane(&frontmatter.id, &relative_path);

        specs.push(SpecNode {
            id: frontmatter.id,
            title,
            path: relative_path,
            spec_type: frontmatter.r#type,
            status,
            target: frontmatter.target,
            lane,
            checked,
            total,
            passed: total == checked,
            blocked_by: frontmatter.blocked_by,
            related: frontmatter.related,
        });
    }

    specs.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(specs)
}

fn build_spec_edges(specs: &[SpecNode]) -> Vec<SpecEdge> {
    let ids: HashSet<&str> = specs.iter().map(|s| s.id.as_str()).collect();
    let mut seen: HashSet<(String, String, String)> = HashSet::new();
    let mut edges = Vec::new();

    for spec in specs {
        for blocker in &spec.blocked_by {
            if blocker.trim().is_empty() {
                continue;
            }
            let key = (blocker.clone(), spec.id.clone(), "blocks".to_string());
            if seen.insert(key.clone()) {
                edges.push(SpecEdge {
                    from: key.0,
                    to: key.1,
                    kind: key.2,
                });
            }
        }

        for rel in &spec.related {
            let Some(target) = infer_related_spec_id(rel) else {
                continue;
            };
            if target == spec.id {
                continue;
            }
            if !ids.contains(target.as_str()) {
                continue;
            }
            let (from, to) = if spec.id <= target {
                (spec.id.clone(), target)
            } else {
                (target, spec.id.clone())
            };
            let key = (from, to, "related".to_string());
            if seen.insert(key.clone()) {
                edges.push(SpecEdge {
                    from: key.0,
                    to: key.1,
                    kind: key.2,
                });
            }
        }
    }

    edges.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then(a.from.cmp(&b.from))
            .then(a.to.cmp(&b.to))
    });
    edges
}

fn infer_related_spec_id(value: &str) -> Option<String> {
    if value.ends_with("/SPEC.md") {
        let path = Path::new(value);
        return path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());
    }

    if value.ends_with(".md") || value.contains('/') {
        return None;
    }

    if value.trim().is_empty() {
        None
    } else {
        Some(value.trim().to_string())
    }
}

fn infer_spec_lane(id: &str, path: &str) -> String {
    let lower_id = id.to_ascii_lowercase();
    let lower_path = path.to_ascii_lowercase();

    if lower_id.contains("sdk") || lower_path.contains("/sdk") {
        return "sdk".to_string();
    }
    if lower_id.contains("ba-") || lower_id.starts_with("ba") {
        return "ba".to_string();
    }
    if lower_id.contains("interface") || lower_path.contains("interface") {
        return "interface".to_string();
    }
    if lower_id.contains("child")
        || lower_id.contains("mother")
        || lower_id.contains("toy")
        || lower_id.contains("mct")
    {
        return "mct".to_string();
    }

    "general".to_string()
}

fn scan_children(root: &Path) -> Result<Vec<ChildNode>> {
    let children_root = root.join("children");
    if !children_root.exists() {
        return Ok(vec![]);
    }

    let mut items = Vec::new();
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&children_root)
        .with_context(|| format!("read children dir {}", children_root.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();

    for dir in dirs {
        let manifest_path = dir.join("child.toml");
        if !manifest_path.exists() {
            continue;
        }

        let raw = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("read child manifest {}", manifest_path.display()))?;
        let doc: toml::Value = raw
            .parse::<toml::Value>()
            .map_err(|e| anyhow!("parse child manifest {}: {}", manifest_path.display(), e))?;

        let child = doc
            .get("child")
            .and_then(|v| v.as_table())
            .ok_or_else(|| anyhow!("manifest {} missing [child]", manifest_path.display()))?;

        let manifest_name = child
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let kind = child
            .get("kind")
            .and_then(|v| v.as_str())
            .map(ToString::to_string);
        let role = child
            .get("role")
            .and_then(|v| v.as_str())
            .map(ToString::to_string);

        let toys = doc
            .get("needs")
            .and_then(|v| v.as_table())
            .and_then(|needs| needs.get("toys"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let lane_hint = infer_child_lane(&toys);

        let has_wit_world = dir.join("wit/world.wit").exists();
        let handle_lane = std::fs::read_to_string(dir.join("src/lib.rs"))
            .map(|src| src.contains("register_child!("))
            .unwrap_or(false);

        let folder = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        items.push(ChildNode {
            folder,
            manifest_name,
            kind,
            role,
            toys,
            lane_hint,
            has_wit_world,
            handle_lane,
        });
    }

    items.sort_by(|a, b| a.folder.cmp(&b.folder));
    Ok(items)
}

fn infer_child_lane(toys: &[String]) -> String {
    let legacy_aliases = ["log", "state", "store", "fs"];
    if toys
        .iter()
        .any(|toy| legacy_aliases.contains(&toy.as_str()))
    {
        return "legacy-alias-lane".to_string();
    }

    "typed-manifest-lane".to_string()
}

fn scan_toys(root: &Path) -> Result<Vec<ToyNode>> {
    let registry_path = root.join("wit/toys/deps/toys-registry.toml");
    if !registry_path.exists() {
        return Ok(vec![]);
    }

    let raw = std::fs::read_to_string(&registry_path)
        .with_context(|| format!("read toy registry {}", registry_path.display()))?;
    let doc: toml::Value = raw
        .parse::<toml::Value>()
        .map_err(|e| anyhow!("parse toy registry {}: {}", registry_path.display(), e))?;

    let table = doc
        .as_table()
        .ok_or_else(|| anyhow!("toy registry {} is not a table", registry_path.display()))?;

    let mut toys = Vec::new();
    for (id, value) in table {
        let Some(item) = value.as_table() else {
            continue;
        };
        toys.push(ToyNode {
            id: id.clone(),
            file: item
                .get("file")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            version: item
                .get("version")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            source: item
                .get("source")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        });
    }

    toys.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(toys)
}

fn first_heading(body: &str) -> Option<String> {
    body.lines()
        .map(str::trim)
        .find(|line| line.starts_with('#'))
        .map(|line| line.trim_start_matches('#').trim().to_string())
        .filter(|line| !line.is_empty())
}

fn collect_named_files(root: &Path, file_name: &str, out: &mut Vec<PathBuf>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }

    let entries =
        std::fs::read_dir(root).with_context(|| format!("read directory {}", root.display()))?;

    for entry in entries {
        let entry = entry.with_context(|| format!("read entry under {}", root.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_named_files(&path, file_name, out)?;
        } else if path.file_name().and_then(|n| n.to_str()) == Some(file_name) {
            out.push(path);
        }
    }

    Ok(())
}

fn write_output(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create output dir {}", parent.display()))?;
    }
    std::fs::write(path, content).with_context(|| format!("write output {}", path.display()))
}

pub(crate) fn render_html(snapshot: &AtlasSnapshot) -> Result<String> {
    let data = serde_json::to_string(snapshot)?;
    Ok(format!(
        r#"<!doctype html>
<html lang=\"en\">
<head>
  <meta charset=\"utf-8\" />
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />
  <title>Patina Atlas</title>
  <style>
    :root {{
      color-scheme: dark;
      --bg: #0b1020;
      --panel: #111933;
      --muted: #8ea0c5;
      --text: #e5ecff;
      --accent: #7aa2ff;
      --ok: #3ddc97;
      --warn: #ffca63;
      --bad: #ff7b7b;
      --border: #24325a;
    }}
    body {{
      margin: 0;
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, sans-serif;
      background: var(--bg);
      color: var(--text);
    }}
    .wrap {{
      max-width: 1200px;
      margin: 0 auto;
      padding: 24px;
    }}
    h1, h2 {{ margin: 0 0 12px; }}
    p {{ color: var(--muted); }}
    .grid {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 12px;
      margin: 16px 0 24px;
    }}
    .card {{
      background: var(--panel);
      border: 1px solid var(--border);
      border-radius: 10px;
      padding: 12px;
    }}
    .k {{ color: var(--muted); font-size: 12px; text-transform: uppercase; letter-spacing: .06em; }}
    .v {{ font-size: 20px; font-weight: 700; margin-top: 6px; }}
    .controls {{
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
      margin: 12px 0;
    }}
    select {{
      background: #0f1730;
      color: var(--text);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 8px;
    }}
    table {{
      width: 100%;
      border-collapse: collapse;
      margin-top: 8px;
      font-size: 13px;
      background: var(--panel);
      border: 1px solid var(--border);
      border-radius: 10px;
      overflow: hidden;
      display: block;
      overflow-x: auto;
    }}
    th, td {{
      padding: 8px 10px;
      border-bottom: 1px solid var(--border);
      text-align: left;
      white-space: nowrap;
    }}
    th {{ color: var(--muted); font-weight: 600; }}
    .ok {{ color: var(--ok); }}
    .warn {{ color: var(--warn); }}
    .bad {{ color: var(--bad); }}
    code {{
      background: #0c142c;
      border: 1px solid var(--border);
      border-radius: 6px;
      padding: 2px 6px;
      color: #c7d5ff;
    }}
  </style>
</head>
<body>
  <div class=\"wrap\">
    <h1>Patina Atlas</h1>
    <p id=\"meta\"></p>

    <div class=\"grid\" id=\"summary\"></div>

    <h2>Specs</h2>
    <div class=\"controls\">
      <label>Status <select id=\"statusFilter\"><option value=\"all\">all</option></select></label>
      <label>Lane <select id=\"laneFilter\"><option value=\"all\">all</option></select></label>
    </div>
    <div id=\"specTable\"></div>

    <h2>Spec edges</h2>
    <div id=\"edgeTable\"></div>

    <h2>Children</h2>
    <div id=\"childTable\"></div>

    <h2>Toys</h2>
    <div id=\"toyTable\"></div>
  </div>

  <script>
    const DATA = {data};

    const el = (id) => document.getElementById(id);

    function renderSummary() {{
      const s = DATA.summary;
      const cards = [
        ["Specs", s.spec_count],
        ["Edges", s.edge_count],
        ["Children", s.child_count],
        ["Toys", s.toy_count],
      ];
      el("summary").innerHTML = cards
        .map(([k, v]) => `<div class=\"card\"><div class=\"k\">${{k}}</div><div class=\"v\">${{v}}</div></div>`)
        .join("");
      el("meta").textContent = `Generated ${{DATA.generated_at}} • root: ${{DATA.project_root}}`;
    }}

    function renderSpecFilters() {{
      const statuses = [...new Set(DATA.specs.map(s => s.status))].sort();
      const lanes = [...new Set(DATA.specs.map(s => s.lane))].sort();
      const status = el("statusFilter");
      const lane = el("laneFilter");
      statuses.forEach(v => status.insertAdjacentHTML("beforeend", `<option value=\"${{v}}\">${{v}}</option>`));
      lanes.forEach(v => lane.insertAdjacentHTML("beforeend", `<option value=\"${{v}}\">${{v}}</option>`));
      status.addEventListener("change", renderSpecTable);
      lane.addEventListener("change", renderSpecTable);
    }}

    function renderSpecTable() {{
      const status = el("statusFilter").value;
      const lane = el("laneFilter").value;
      const rows = DATA.specs
        .filter(s => status === "all" || s.status === status)
        .filter(s => lane === "all" || s.lane === lane)
        .map(s => {{
          const progress = `${{s.checked}}/${{s.total}}`;
          const cls = s.passed ? "ok" : (s.total === 0 ? "warn" : "bad");
          return `<tr>
            <td><code>${{s.id}}</code></td>
            <td>${{s.status}}</td>
            <td>${{s.lane}}</td>
            <td class=\"${{cls}}\">${{progress}}</td>
            <td>${{s.title}}</td>
            <td><code>${{s.path}}</code></td>
          </tr>`;
        }}).join("");

      el("specTable").innerHTML = `<table>
        <thead><tr><th>ID</th><th>Status</th><th>Lane</th><th>Criteria</th><th>Title</th><th>Path</th></tr></thead>
        <tbody>${{rows}}</tbody>
      </table>`;
    }}

    function renderEdgeTable() {{
      const rows = DATA.spec_edges
        .map(e => `<tr><td>${{e.kind}}</td><td><code>${{e.from}}</code></td><td><code>${{e.to}}</code></td></tr>`)
        .join("");
      el("edgeTable").innerHTML = `<table>
        <thead><tr><th>Kind</th><th>From</th><th>To</th></tr></thead>
        <tbody>${{rows}}</tbody>
      </table>`;
    }}

    function renderChildTable() {{
      const rows = DATA.children
        .map(c => `<tr>
          <td><code>${{c.folder}}</code></td>
          <td>${{c.manifest_name}}</td>
          <td>${{c.kind || "-"}}</td>
          <td>${{c.role || "-"}}</td>
          <td>${{c.lane_hint}}</td>
          <td>${{c.toys.join(", ")}}</td>
          <td>${{c.has_wit_world ? "yes" : "no"}}</td>
          <td>${{c.handle_lane ? "yes" : "no"}}</td>
        </tr>`)
        .join("");
      el("childTable").innerHTML = `<table>
        <thead><tr><th>Folder</th><th>Manifest name</th><th>Kind</th><th>Role</th><th>Lane</th><th>Toys</th><th>wit/world.wit</th><th>handle lane</th></tr></thead>
        <tbody>${{rows}}</tbody>
      </table>`;
    }}

    function renderToyTable() {{
      const rows = DATA.toys
        .map(t => `<tr><td><code>${{t.id}}</code></td><td>${{t.file || "-"}}</td><td>${{t.version || "-"}}</td><td>${{t.source || "-"}}</td></tr>`)
        .join("");
      el("toyTable").innerHTML = `<table>
        <thead><tr><th>Toy</th><th>File</th><th>Version</th><th>Source</th></tr></thead>
        <tbody>${{rows}}</tbody>
      </table>`;
    }}

    renderSummary();
    renderSpecFilters();
    renderSpecTable();
    renderEdgeTable();
    renderChildTable();
    renderToyTable();
  </script>
</body>
</html>
"#
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn build_snapshot_collects_specs_edges_children_and_toys() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        write(
            &root.join("layer/surface/build/feat/a/SPEC.md"),
            r#"---
type: feat
id: a
status: active
blocked_by: [b]
related:
- layer/surface/build/feat/b/SPEC.md
exit_criteria:
- id: a1
  text: one
  checked: true
- id: a2
  text: two
  checked: false
---
# A spec
"#,
        );

        write(
            &root.join("layer/surface/build/feat/b/SPEC.md"),
            r#"---
type: feat
id: b
status: ready
exit_criteria: []
---
# B spec
"#,
        );

        write(
            &root.join("children/sample/child.toml"),
            r#"[child]
name = "sample"
kind = "child"
role = "app"

[needs]
toys = ["logging", "measure"]
"#,
        );
        write(
            &root.join("children/sample/src/lib.rs"),
            "wit_bindgen::generate!({ path: \"wit\", world: \"sample\", generate_all, });",
        );
        write(
            &root.join("children/sample/wit/world.wit"),
            "package patina:sample@0.1.0;",
        );

        write(
            &root.join("wit/toys/deps/toys-registry.toml"),
            r#"[patina-logging]
file = "logging.wit"
version = "0.1.0"
source = "patina"
"#,
        );

        let snapshot = build_snapshot(root).unwrap();

        assert_eq!(snapshot.summary.spec_count, 2);
        assert_eq!(snapshot.summary.child_count, 1);
        assert_eq!(snapshot.summary.toy_count, 1);
        assert!(snapshot
            .spec_edges
            .iter()
            .any(|e| e.kind == "blocks" && e.from == "b" && e.to == "a"));
        assert!(snapshot
            .spec_edges
            .iter()
            .any(|e| e.kind == "related" && e.from == "a" && e.to == "b"));

        let a = snapshot.specs.iter().find(|s| s.id == "a").unwrap();
        assert_eq!(a.checked, 1);
        assert_eq!(a.total, 2);
        assert!(!a.passed);
    }

    #[test]
    fn build_snapshot_fails_closed_on_malformed_spec_frontmatter() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        write(
            &root.join("layer/surface/build/feat/bad/SPEC.md"),
            "---\nstatus: [unterminated\n---\n# bad\n",
        );

        let err = build_snapshot(root).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("parse spec frontmatter"));
        assert!(msg.contains("layer/surface/build/feat/bad/SPEC.md"));
    }

    #[test]
    fn render_html_embeds_snapshot_data() {
        let snapshot = AtlasSnapshot {
            generated_at: "now".to_string(),
            project_root: "/tmp/project".to_string(),
            summary: AtlasSummary {
                spec_count: 1,
                status_counts: BTreeMap::new(),
                lane_counts: BTreeMap::new(),
                edge_count: 0,
                child_count: 0,
                toy_count: 0,
            },
            specs: vec![SpecNode {
                id: "demo-spec".to_string(),
                title: "Demo".to_string(),
                path: "layer/surface/build/feat/demo/SPEC.md".to_string(),
                spec_type: "feat".to_string(),
                status: "draft".to_string(),
                target: None,
                lane: "general".to_string(),
                checked: 0,
                total: 0,
                passed: true,
                blocked_by: vec![],
                related: vec![],
            }],
            spec_edges: vec![],
            children: vec![],
            toys: vec![],
        };

        let html = render_html(&snapshot).unwrap();
        assert!(html.contains("Patina Atlas"));
        assert!(html.contains("demo-spec"));
        assert!(html.contains("const DATA ="));
    }
}
