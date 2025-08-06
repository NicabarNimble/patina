//! Benchmarking utilities for the indexer

use super::*;
use std::time::Instant;

impl PatternIndexer {
    /// Index directory sequentially for benchmarking
    pub fn index_directory_sequential(&self, dir: &Path) -> Result<()> {
        use walkdir::WalkDir;

        let start = Instant::now();
        let mut count = 0;
        let mut errors = Vec::new();

        for entry in WalkDir::new(dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md")
                && !path.to_string_lossy().contains("/sessions/")
            {
                count += 1;
                if let Err(e) = self.index_document(path) {
                    errors.push((path.to_path_buf(), e));
                }
            }
        }

        let duration = start.elapsed();
        println!("Sequential indexing: {} files in {:.2?}", count, duration);

        if !errors.is_empty() {
            println!("  {} errors occurred", errors.len());
        }

        Ok(())
    }

    /// Benchmark parallel vs sequential indexing
    pub fn benchmark_indexing(&self, dir: &Path) -> Result<()> {
        println!("\n=== Indexing Benchmark ===\n");

        // Clear cache first
        {
            let mut cache = self.cache.lock().unwrap();
            *cache = GitAwareNavigationMap::new();
        }

        // Sequential
        println!("Testing sequential indexing...");
        let seq_start = Instant::now();
        self.index_directory_sequential(dir)?;
        let seq_duration = seq_start.elapsed();

        // Clear cache again
        {
            let mut cache = self.cache.lock().unwrap();
            *cache = GitAwareNavigationMap::new();
        }

        // Parallel
        println!("\nTesting parallel indexing...");
        let par_start = Instant::now();
        self.index_directory(dir)?;
        let par_duration = par_start.elapsed();

        // Results
        println!("\n=== Results ===");
        println!("Sequential: {:.2?}", seq_duration);
        println!("Parallel:   {:.2?}", par_duration);
        println!(
            "Speedup:    {:.2}x",
            seq_duration.as_secs_f64() / par_duration.as_secs_f64()
        );

        Ok(())
    }
}
