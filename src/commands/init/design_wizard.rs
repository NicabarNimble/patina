//! User interaction utilities for init command

use anyhow::Result;
use std::io::{self, Write};

/// Confirm prompt (Y/n)
pub fn confirm(prompt: &str) -> Result<bool> {
    print!("{prompt} [Y/n]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let trimmed = input.trim().to_lowercase();
    Ok(trimmed.is_empty() || trimmed == "y" || trimmed == "yes")
}
