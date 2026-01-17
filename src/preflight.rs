//! Preflight checks - ensure system is ready to run patina.

use std::process::Command;

const STALE_THRESHOLD_MINUTES: u64 = 24 * 60; // 24 hours

/// Ensure system is ready to run patina. Kills stale processes (>24h).
pub fn ensure_clean_state() {
    for proc in find_stale_processes(STALE_THRESHOLD_MINUTES) {
        if Command::new("kill")
            .arg(proc.pid.to_string())
            .output()
            .is_ok()
        {
            eprintln!(
                "Cleaned up stale {} (PID {}, running {})",
                proc.cmd,
                proc.pid,
                format_elapsed(proc.mins)
            );
        }
    }
}

struct StaleProcess {
    pid: u32,
    cmd: String,
    mins: u64,
}

fn find_stale_processes(threshold: u64) -> Vec<StaleProcess> {
    let output = match Command::new("ps")
        .args(["-eo", "pid,etime,command"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };
    let current_pid = std::process::id();
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .skip(1)
        .filter(|l| l.contains("patina") && !l.contains("grep"))
        .filter_map(|line| {
            let p: Vec<&str> = line.split_whitespace().collect();
            let pid: u32 = p.first()?.parse().ok()?;
            if pid == current_pid || p.len() < 3 {
                return None;
            }
            let mins = parse_elapsed(p.get(1)?);
            if mins < threshold {
                return None;
            }
            Some(StaleProcess {
                pid,
                cmd: p[2..].join(" "),
                mins,
            })
        })
        .collect()
}

fn parse_elapsed(s: &str) -> u64 {
    let (days, rest) = s
        .find('-')
        .map_or((0, s), |i| (s[..i].parse().unwrap_or(0), &s[i + 1..]));
    let p: Vec<u64> = rest.split(':').filter_map(|x| x.parse().ok()).collect();
    days * 1440
        + match p.len() {
            3 => p[0] * 60 + p[1],
            2 => p[0],
            _ => 0,
        }
}

fn format_elapsed(m: u64) -> String {
    match m {
        m if m >= 1440 => format!("{}d{}h", m / 1440, (m % 1440) / 60),
        m if m >= 60 => format!("{}h{}m", m / 60, m % 60),
        m => format!("{}m", m),
    }
}
