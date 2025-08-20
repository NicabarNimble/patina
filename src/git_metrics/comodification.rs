//! Co-modification pattern detection - finding files that change together

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::git_metrics::{CommitMetrics, ComodificationCluster};

/// Find clusters of files that frequently change together
pub fn find_clusters(commits: &[CommitMetrics]) -> Result<Vec<ComodificationCluster>> {
    let mut comod_pairs: HashMap<(PathBuf, PathBuf), usize> = HashMap::new();
    let mut file_commit_count: HashMap<PathBuf, usize> = HashMap::new();
    
    // Count co-modifications
    for commit in commits {
        let files = &commit.files_changed;
        
        // Update individual file counts
        for file in files {
            *file_commit_count.entry(file.clone()).or_insert(0) += 1;
        }
        
        // Count pairs
        for i in 0..files.len() {
            for j in (i + 1)..files.len() {
                let pair = if files[i] < files[j] {
                    (files[i].clone(), files[j].clone())
                } else {
                    (files[j].clone(), files[i].clone())
                };
                *comod_pairs.entry(pair).or_insert(0) += 1;
            }
        }
    }
    
    // Calculate confidence scores
    let mut clusters: Vec<ComodificationCluster> = Vec::new();
    let mut processed_files: HashSet<PathBuf> = HashSet::new();
    
    for ((file1, file2), count) in &comod_pairs {
        // Skip if already part of a cluster
        if processed_files.contains(file1) || processed_files.contains(file2) {
            continue;
        }
        
        // Calculate confidence (Jaccard similarity)
        let count1 = file_commit_count.get(file1).unwrap_or(&0);
        let count2 = file_commit_count.get(file2).unwrap_or(&0);
        let union = count1 + count2 - count;
        let confidence = if union > 0 {
            *count as f64 / union as f64
        } else {
            0.0
        };
        
        // Only create cluster if confidence is high enough
        if confidence > 0.3 && *count > 2 {
            // Find all related files
            let mut cluster_files = HashSet::new();
            cluster_files.insert(file1.clone());
            cluster_files.insert(file2.clone());
            
            // Expand cluster to include strongly related files
            expand_cluster(&mut cluster_files, &comod_pairs, &file_commit_count, 0.25);
            
            // Mark files as processed
            for file in &cluster_files {
                processed_files.insert(file.clone());
            }
            
            // Find last modification
            let last_seen = commits.iter()
                .filter(|c| c.files_changed.iter().any(|f| cluster_files.contains(f)))
                .map(|c| c.timestamp)
                .max()
                .unwrap_or_else(chrono::Utc::now);
            
            clusters.push(ComodificationCluster {
                files: cluster_files,
                frequency: *count,
                confidence,
                last_seen,
            });
        }
    }
    
    // Sort by confidence
    clusters.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    
    Ok(clusters)
}

/// Expand a cluster to include related files
fn expand_cluster(
    cluster: &mut HashSet<PathBuf>,
    comod_pairs: &HashMap<(PathBuf, PathBuf), usize>,
    file_counts: &HashMap<PathBuf, usize>,
    min_confidence: f64,
) {
    let mut added = true;
    
    while added {
        added = false;
        let current_files: Vec<PathBuf> = cluster.iter().cloned().collect();
        
        for file in &current_files {
            // Find all files that co-modify with this file
            for ((f1, f2), count) in comod_pairs {
                let other = if f1 == file {
                    f2
                } else if f2 == file {
                    f1
                } else {
                    continue;
                };
                
                // Skip if already in cluster
                if cluster.contains(other) {
                    continue;
                }
                
                // Calculate confidence with this file
                let count1 = file_counts.get(file).unwrap_or(&0);
                let count2 = file_counts.get(other).unwrap_or(&0);
                let union = count1 + count2 - count;
                let confidence = if union > 0 {
                    *count as f64 / union as f64
                } else {
                    0.0
                };
                
                // Add if confidence is high enough
                if confidence > min_confidence {
                    cluster.insert(other.clone());
                    added = true;
                }
            }
        }
    }
}

/// Generate co-modification report
pub fn generate_comodification_report(clusters: &[ComodificationCluster]) -> String {
    let mut report = String::from("# Co-modification Patterns\n\n");
    report.push_str("Files that frequently change together indicate architectural coupling.\n\n");
    
    for (i, cluster) in clusters.iter().enumerate() {
        report.push_str(&format!("## Cluster {} (Confidence: {:.1}%)\n", 
            i + 1, 
            cluster.confidence * 100.0
        ));
        report.push_str(&format!("Frequency: {} co-modifications\n", cluster.frequency));
        report.push_str(&format!("Last seen: {}\n", 
            cluster.last_seen.format("%Y-%m-%d")
        ));
        report.push_str("Files:\n");
        
        for file in &cluster.files {
            report.push_str(&format!("- {}\n", file.display()));
        }
        
        // Suggest architectural insight
        let insight = suggest_architectural_insight(cluster);
        if !insight.is_empty() {
            report.push_str(&format!("\n**Insight**: {}\n", insight));
        }
        
        report.push_str("\n");
    }
    
    report
}

/// Suggest architectural insights based on co-modification patterns
fn suggest_architectural_insight(cluster: &ComodificationCluster) -> String {
    let files: Vec<String> = cluster.files.iter()
        .map(|f| f.to_string_lossy().to_string())
        .collect();
    
    // Check for test/implementation coupling
    let has_tests = files.iter().any(|f| f.contains("test") || f.contains("spec"));
    let has_impl = files.iter().any(|f| !f.contains("test") && !f.contains("spec"));
    
    if has_tests && has_impl {
        return "Strong test-implementation coupling (good practice!)".to_string();
    }
    
    // Check for cross-module coupling
    let modules: HashSet<String> = files.iter()
        .filter_map(|f| {
            let parts: Vec<&str> = f.split('/').collect();
            if parts.len() > 1 {
                Some(parts[0].to_string())
            } else {
                None
            }
        })
        .collect();
    
    if modules.len() > 2 {
        return format!("Cross-module coupling detected across {} modules. Consider refactoring to reduce coupling.", 
            modules.len());
    }
    
    // Check for documentation coupling
    if files.iter().any(|f| f.ends_with(".md")) {
        return "Documentation kept in sync with code (excellent!)".to_string();
    }
    
    // Check for configuration coupling
    if files.iter().any(|f| f.contains("config") || f.ends_with(".toml") || f.ends_with(".json")) {
        return "Configuration changes trigger code changes. Ensure backwards compatibility.".to_string();
    }
    
    String::new()
}