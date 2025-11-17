use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub category: FileCategory,
    pub git_status: GitStatus,
    pub safety: SafetyLevel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileCategory {
    Source,
    Documentation,
    Config,
    BuildArtifact,
    Database,
    Session,
    Archive,
    Temporary,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitStatus {
    Tracked,
    Untracked,
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SafetyLevel {
    Critical,     // .git, .patina, Cargo.toml, src/
    Protected,    // Tracked files, layer/, sessions
    ReviewNeeded, // Untracked but looks important
    SafeToDelete, // Build artifacts, temp files
}

pub struct FileAudit {
    critical: Vec<FileInfo>,
    protected: Vec<FileInfo>,
    review_needed: Vec<FileInfo>,
    safe_to_delete: Vec<FileInfo>,
    layer_insights: Option<LayerInsights>,
}

#[derive(Debug)]
struct LayerInsights {
    core: LayerDirInfo,
    surface: LayerDirInfo,
    dust: LayerDirInfo,
    sessions: SessionInfo,
}

#[derive(Debug)]
struct LayerDirInfo {
    file_count: usize,
    total_size: u64,
    stale_files: Vec<StaleFile>,
    repos: Vec<RepoSummary>,
}

#[derive(Debug)]
struct RepoSummary {
    name: String,
    file_count: usize,
    total_size: u64,
}

#[derive(Debug)]
struct StaleFile {
    path: PathBuf,
    days_old: u64,
    size: u64,
}

#[derive(Debug)]
struct SessionInfo {
    file_count: usize,
    total_size: u64,
    oldest_date: Option<SystemTime>,
    newest_date: Option<SystemTime>,
}

pub fn execute(project_root: &Path) -> Result<()> {
    println!("🔍 Auditing project files...\n");

    let audit = scan_files(project_root)?;
    display_audit(&audit)?;

    Ok(())
}

fn scan_files(project_root: &Path) -> Result<FileAudit> {
    let mut critical = Vec::new();
    let mut protected = Vec::new();
    let mut review_needed = Vec::new();
    let mut safe_to_delete = Vec::new();

    // Get git-tracked files
    let tracked_files = get_tracked_files(project_root)?;
    let ignored_files = get_ignored_files(project_root)?;

    // Walk the directory tree
    for entry in walkdir::WalkDir::new(project_root)
        .into_iter()
        .filter_entry(|e| !is_hidden_dir(e.path()))
    {
        let entry = entry?;
        let path = entry.path();

        // Skip directories, we'll summarize them via their files
        if path.is_dir() {
            continue;
        }

        // Get relative path
        let rel_path = path.strip_prefix(project_root).unwrap_or(path);

        // Determine git status
        let git_status = if tracked_files.contains(rel_path) {
            GitStatus::Tracked
        } else if ignored_files.contains(rel_path) {
            GitStatus::Ignored
        } else {
            GitStatus::Untracked
        };

        // Get file metadata (skip files we can't read - broken symlinks, permission issues, etc)
        let metadata = match fs::metadata(path) {
            Ok(m) => m,
            Err(_) => continue, // Skip files we can't read
        };
        let size = metadata.len();
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

        // Categorize file
        let category = categorize_file(rel_path);
        let safety = determine_safety(rel_path, &git_status, &category);

        let file_info = FileInfo {
            path: rel_path.to_path_buf(),
            size,
            modified,
            category,
            git_status,
            safety: safety.clone(),
        };

        // Sort into safety buckets
        match safety {
            SafetyLevel::Critical => critical.push(file_info),
            SafetyLevel::Protected => protected.push(file_info),
            SafetyLevel::ReviewNeeded => review_needed.push(file_info),
            SafetyLevel::SafeToDelete => safe_to_delete.push(file_info),
        }
    }

    // Sort by size (descending)
    for bucket in [
        &mut critical,
        &mut protected,
        &mut review_needed,
        &mut safe_to_delete,
    ] {
        bucket.sort_by(|a, b| b.size.cmp(&a.size));
    }

    // Analyze layer directory if it exists
    let layer_insights = analyze_layer_directory(project_root)?;

    Ok(FileAudit {
        critical,
        protected,
        review_needed,
        safe_to_delete,
        layer_insights,
    })
}

fn get_tracked_files(project_root: &Path) -> Result<HashSet<PathBuf>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .arg("ls-files")
        .output()
        .context("Failed to run git ls-files")?;

    let files: HashSet<PathBuf> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(PathBuf::from)
        .collect();

    Ok(files)
}

fn get_ignored_files(project_root: &Path) -> Result<HashSet<PathBuf>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .arg("ls-files")
        .arg("--others")
        .arg("--ignored")
        .arg("--exclude-standard")
        .output()
        .context("Failed to run git ls-files for ignored files")?;

    let files: HashSet<PathBuf> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(PathBuf::from)
        .collect();

    Ok(files)
}

fn is_hidden_dir(path: &Path) -> bool {
    let path_str = path.to_str().unwrap_or("");

    // Skip .git directory
    if path.ends_with(".git") {
        return true;
    }

    // Skip target directory (Rust build artifacts)
    if path.ends_with("target") {
        return true;
    }

    // Skip .cargo directory (Rust registry and build cache)
    if path_str.contains("/.cargo/") || path.ends_with(".cargo") {
        return true;
    }

    // Skip layer/dust/repos (analyzed separately in layer insights)
    if path_str.contains("layer/dust/repos") {
        return true;
    }

    // Skip node_modules if present
    if path.ends_with("node_modules") {
        return true;
    }

    // Skip Python venvs if present
    if path.ends_with(".venv") || path.ends_with("venv") {
        return true;
    }

    false
}

fn categorize_file(path: &Path) -> FileCategory {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

    match ext {
        "rs" | "toml" => FileCategory::Source,
        "md" => FileCategory::Documentation,
        "db" | "duckdb" => FileCategory::Database,
        "zip" | "tar" | "gz" | "tgz" => FileCategory::Archive,
        "tmp" | "swp" | "log" => FileCategory::Temporary,
        _ => {
            // Check by path
            if path.starts_with("layer/sessions") {
                FileCategory::Session
            } else if path.starts_with("target/") {
                FileCategory::BuildArtifact
            } else if filename == ".DS_Store"
                || filename.starts_with('.')
                || filename.ends_with('~')
            {
                FileCategory::Temporary
            } else if filename == "Cargo.lock" || filename.starts_with(".cargo") {
                FileCategory::Config
            } else {
                FileCategory::Unknown
            }
        }
    }
}

fn determine_safety(path: &Path, git_status: &GitStatus, category: &FileCategory) -> SafetyLevel {
    let path_str = path.to_str().unwrap_or("");

    // Critical: Core project files that should never be deleted
    if path_str.starts_with(".patina/")
        || path_str.starts_with("src/")
        || path_str.starts_with("Cargo.toml")
        || path_str.starts_with("Cargo.lock")
        || path_str == "README.md"
        || path_str == "CLAUDE.md"
    {
        return SafetyLevel::Critical;
    }

    // Protected: Git-tracked files or important project artifacts
    if *git_status == GitStatus::Tracked {
        return SafetyLevel::Protected;
    }

    if path_str.starts_with("layer/core/")
        || path_str.starts_with("layer/surface/")
        || path_str.starts_with("layer/sessions/")
    {
        return SafetyLevel::Protected;
    }

    // Safe to delete: Build artifacts, temp files, ignored files
    if matches!(
        category,
        FileCategory::BuildArtifact | FileCategory::Temporary
    ) {
        return SafetyLevel::SafeToDelete;
    }

    if *git_status == GitStatus::Ignored {
        // Some ignored files might be important (like .cargo/)
        if path_str.starts_with(".cargo/") {
            return SafetyLevel::ReviewNeeded;
        }
        return SafetyLevel::SafeToDelete;
    }

    // Untracked files: Check if they look important
    if matches!(
        category,
        FileCategory::Source
            | FileCategory::Documentation
            | FileCategory::Config
            | FileCategory::Session
    ) {
        return SafetyLevel::ReviewNeeded;
    }

    // Archives and databases: usually safe to delete if untracked
    if matches!(category, FileCategory::Archive | FileCategory::Database) {
        return SafetyLevel::SafeToDelete;
    }

    // Unknown untracked files: needs review
    SafetyLevel::ReviewNeeded
}

fn analyze_layer_directory(project_root: &Path) -> Result<Option<LayerInsights>> {
    let layer_path = project_root.join("layer");
    if !layer_path.exists() {
        return Ok(None);
    }

    let core = analyze_layer_subdir(&layer_path.join("core"), 90)?;
    let surface = analyze_layer_subdir(&layer_path.join("surface"), 60)?;
    let dust = analyze_layer_subdir(&layer_path.join("dust"), 0)?; // No stale file detection for dust
    let sessions = analyze_sessions(&layer_path.join("sessions"))?;

    Ok(Some(LayerInsights {
        core,
        surface,
        dust,
        sessions,
    }))
}

fn analyze_layer_subdir(dir_path: &Path, stale_days: u64) -> Result<LayerDirInfo> {
    let mut file_count = 0;
    let mut total_size = 0;
    let mut stale_files = Vec::new();
    let mut repos = Vec::new();

    if !dir_path.exists() {
        return Ok(LayerDirInfo {
            file_count: 0,
            total_size: 0,
            stale_files: vec![],
            repos: vec![],
        });
    }

    // Check if this is the dust directory - if so, analyze repos separately
    let is_dust = dir_path.ends_with("dust");
    if is_dust {
        let repos_dir = dir_path.join("repos");
        if repos_dir.exists() {
            repos = analyze_repos(&repos_dir)?;
        }
    }

    let now = SystemTime::now();
    let stale_threshold = Duration::from_secs(stale_days * 24 * 60 * 60);

    // Walk the directory, but skip repos/ in dust
    for entry in walkdir::WalkDir::new(dir_path)
        .into_iter()
        .filter_entry(|e| {
            // Skip repos directory in dust - we analyze it separately
            if is_dust && e.path().ends_with("repos") && e.path().parent() == Some(dir_path) {
                return false;
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        if !entry.path().is_file() {
            continue;
        }

        if let Ok(metadata) = fs::metadata(entry.path()) {
            file_count += 1;
            let size = metadata.len();
            total_size += size;

            // Check if file is stale (only if stale_days > 0)
            if stale_days > 0 {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age > stale_threshold {
                            let days_old = age.as_secs() / (24 * 60 * 60);
                            stale_files.push(StaleFile {
                                path: entry
                                    .path()
                                    .strip_prefix(dir_path)
                                    .unwrap_or(entry.path())
                                    .to_path_buf(),
                                days_old,
                                size,
                            });
                        }
                    }
                }
            }
        }
    }

    // Sort stale files by age (oldest first)
    stale_files.sort_by(|a, b| b.days_old.cmp(&a.days_old));

    Ok(LayerDirInfo {
        file_count,
        total_size,
        stale_files,
        repos,
    })
}

fn analyze_repos(repos_dir: &Path) -> Result<Vec<RepoSummary>> {
    let mut repos = Vec::new();

    if let Ok(entries) = fs::read_dir(repos_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            // Check if it's a git repository
            if !path.join(".git").exists() {
                continue;
            }

            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Count files and size in this repo
            let mut file_count = 0;
            let mut total_size = 0;

            for entry in walkdir::WalkDir::new(&path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.path().is_file() {
                    if let Ok(metadata) = fs::metadata(entry.path()) {
                        file_count += 1;
                        total_size += metadata.len();
                    }
                }
            }

            repos.push(RepoSummary {
                name,
                file_count,
                total_size,
            });
        }
    }

    // Sort by size (largest first)
    repos.sort_by(|a, b| b.total_size.cmp(&a.total_size));

    Ok(repos)
}

fn analyze_sessions(sessions_path: &Path) -> Result<SessionInfo> {
    let mut file_count = 0;
    let mut total_size = 0;
    let mut oldest_date: Option<SystemTime> = None;
    let mut newest_date: Option<SystemTime> = None;

    if !sessions_path.exists() {
        return Ok(SessionInfo {
            file_count: 0,
            total_size: 0,
            oldest_date: None,
            newest_date: None,
        });
    }

    for entry in walkdir::WalkDir::new(sessions_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.path().is_file() {
            continue;
        }

        // Only count .md files as sessions
        if entry.path().extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }

        if let Ok(metadata) = fs::metadata(entry.path()) {
            file_count += 1;
            total_size += metadata.len();

            if let Ok(modified) = metadata.modified() {
                oldest_date = Some(oldest_date.map_or(modified, |old| old.min(modified)));
                newest_date = Some(newest_date.map_or(modified, |new| new.max(modified)));
            }
        }
    }

    Ok(SessionInfo {
        file_count,
        total_size,
        oldest_date,
        newest_date,
    })
}

fn display_audit(audit: &FileAudit) -> Result<()> {
    // Summary
    let total_size: u64 = audit
        .critical
        .iter()
        .chain(&audit.protected)
        .chain(&audit.review_needed)
        .chain(&audit.safe_to_delete)
        .map(|f| f.size)
        .sum();

    println!("📊 Summary:");
    println!(
        "  Total files scanned: {}",
        audit.critical.len()
            + audit.protected.len()
            + audit.review_needed.len()
            + audit.safe_to_delete.len()
    );
    println!("  Total size: {}\n", format_size(total_size));

    // Display layer insights if available
    if let Some(ref layer_insights) = audit.layer_insights {
        display_layer_insights(layer_insights)?;
    }

    // Show category counts (simplified)
    println!("📋 Project Status:");
    println!(
        "  ✅ Protected: {} files, {} (git tracked + core files)",
        audit.protected.len(),
        format_size(audit.protected.iter().map(|f| f.size).sum())
    );
    println!(
        "  ⚠️  Untracked: {} files, {} (not in git)",
        audit.review_needed.len(),
        format_size(audit.review_needed.iter().map(|f| f.size).sum())
    );
    println!();

    // Cleanup suggestions
    if !audit.safe_to_delete.is_empty() {
        let deletable_size: u64 = audit.safe_to_delete.iter().map(|f| f.size).sum();
        println!("🗑️  Cleanup Opportunities:");
        println!(
            "  {} files can be safely deleted to reclaim {}",
            audit.safe_to_delete.len(),
            format_size(deletable_size)
        );
        println!();

        // Group deletables by category
        let mut by_category: std::collections::BTreeMap<String, Vec<&FileInfo>> =
            std::collections::BTreeMap::new();

        for file in &audit.safe_to_delete {
            // Categorize by top-level directory
            let category = if file.path.starts_with(".backup") {
                "Old backups"
            } else if file.path.starts_with("resources/models") {
                "Model files"
            } else if file.path.to_str().unwrap_or("").contains(".db") {
                "Database files"
            } else if file.path.starts_with("layer/dust") {
                "Dust archive files"
            } else {
                "Other"
            };

            by_category
                .entry(category.to_string())
                .or_default()
                .push(file);
        }

        for (category, files) in &by_category {
            let cat_size: u64 = files.iter().map(|f| f.size).sum();
            println!(
                "  • {}: {} files, {}",
                category,
                files.len(),
                format_size(cat_size)
            );

            // Show largest files in this category
            let mut sorted = files.clone();
            sorted.sort_by(|a, b| b.size.cmp(&a.size));
            for file in sorted.iter().take(3) {
                println!(
                    "      - {} ({})",
                    file.path.display(),
                    format_size(file.size)
                );
            }
        }
    } else {
        println!("✨ No obvious cleanup needed!");
    }

    Ok(())
}

fn display_layer_insights(insights: &LayerInsights) -> Result<()> {
    println!("📚 Layer Directory Analysis:\n");

    // Core patterns
    println!("  🔷 core/ - Eternal Patterns");
    println!(
        "    {} files, {}",
        insights.core.file_count,
        format_size(insights.core.total_size)
    );
    if !insights.core.stale_files.is_empty() {
        println!(
            "    ⚠️  {} files not modified in 90+ days:",
            insights.core.stale_files.len()
        );
        for stale in insights.core.stale_files.iter().take(5) {
            println!(
                "       • {} ({} days old, {})",
                stale.path.display(),
                stale.days_old,
                format_size(stale.size)
            );
        }
        if insights.core.stale_files.len() > 5 {
            println!(
                "       ... and {} more",
                insights.core.stale_files.len() - 5
            );
        }
    }
    println!();

    // Surface patterns
    println!("  🔶 surface/ - Active Development Patterns");
    println!(
        "    {} files, {}",
        insights.surface.file_count,
        format_size(insights.surface.total_size)
    );
    if !insights.surface.stale_files.is_empty() {
        println!(
            "    ⚠️  {} files not modified in 60+ days:",
            insights.surface.stale_files.len()
        );
        for stale in insights.surface.stale_files.iter().take(5) {
            println!(
                "       • {} ({} days old, {})",
                stale.path.display(),
                stale.days_old,
                format_size(stale.size)
            );
        }
        if insights.surface.stale_files.len() > 5 {
            println!(
                "       ... and {} more",
                insights.surface.stale_files.len() - 5
            );
        }
    }
    println!();

    // Dust archive
    println!("  📦 dust/ - Archival Storage");

    // Show repo breakdown if repos exist
    if !insights.dust.repos.is_empty() {
        let total_repo_size: u64 = insights.dust.repos.iter().map(|r| r.total_size).sum();
        let total_repo_files: usize = insights.dust.repos.iter().map(|r| r.file_count).sum();

        println!(
            "    Research Repositories: {} repos, {} files, {}",
            insights.dust.repos.len(),
            total_repo_files,
            format_size(total_repo_size)
        );

        // Show top repos
        for repo in insights.dust.repos.iter().take(10) {
            println!(
                "      • {} ({}, {} files)",
                repo.name,
                format_size(repo.total_size),
                repo.file_count
            );
        }
        if insights.dust.repos.len() > 10 {
            println!(
                "      ... and {} more repos",
                insights.dust.repos.len() - 10
            );
        }
        println!();
    }

    // Show non-repo files (archived docs, etc.)
    if insights.dust.file_count > 0 {
        println!(
            "    Archived docs: {} files, {}",
            insights.dust.file_count,
            format_size(insights.dust.total_size)
        );
    }
    println!();

    // Sessions
    println!("  📝 sessions/ - Development History");
    println!(
        "    {} sessions, {}",
        insights.sessions.file_count,
        format_size(insights.sessions.total_size)
    );
    if let (Some(oldest), Some(newest)) =
        (insights.sessions.oldest_date, insights.sessions.newest_date)
    {
        println!(
            "    Date range: {} → {}",
            format_date(oldest),
            format_date(newest)
        );
        if let Ok(duration) = newest.duration_since(oldest) {
            let days = duration.as_secs() / (24 * 60 * 60);
            println!("    Span: {} days", days);
        }
    }
    println!();

    Ok(())
}

fn format_date(time: SystemTime) -> String {
    use std::time::UNIX_EPOCH;
    if let Ok(duration) = time.duration_since(UNIX_EPOCH) {
        let secs = duration.as_secs();
        let datetime =
            chrono::DateTime::from_timestamp(secs as i64, 0).unwrap_or_else(chrono::Utc::now);
        datetime.format("%Y-%m-%d").to_string()
    } else {
        "unknown".to_string()
    }
}

fn display_category(title: &str, files: &[FileInfo]) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    let total_size: u64 = files.iter().map(|f| f.size).sum();
    println!(
        "{} ({} files, {})",
        title,
        files.len(),
        format_size(total_size)
    );

    // Group by directory or category
    let mut by_dir: std::collections::BTreeMap<String, Vec<&FileInfo>> =
        std::collections::BTreeMap::new();

    for file in files {
        let dir = file
            .path
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or(".")
            .to_string();
        by_dir.entry(dir).or_default().push(file);
    }

    // Show top directories
    for (dir, dir_files) in by_dir.iter().take(10) {
        let dir_size: u64 = dir_files.iter().map(|f| f.size).sum();
        println!(
            "  {:<40} {} files, {}",
            format!("{}/*", dir),
            dir_files.len(),
            format_size(dir_size)
        );
    }

    if by_dir.len() > 10 {
        println!("  ... and {} more directories", by_dir.len() - 10);
    }

    println!();
    Ok(())
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
