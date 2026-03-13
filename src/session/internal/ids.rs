use chrono::{DateTime, Local};

const SUFFIX_ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

pub fn generate_runtime_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn generate_file_id(now: DateTime<Local>) -> String {
    let mut value = fastrand::u32(..) & 0x000f_ffff;
    let mut suffix = ['A'; 4];
    for idx in (0..4).rev() {
        let alphabet_idx = (value & 0b1_1111) as usize;
        suffix[idx] = SUFFIX_ALPHABET[alphabet_idx] as char;
        value >>= 5;
    }
    format!(
        "{}-{}",
        now.format("%Y%m%d-%H%M%S"),
        suffix.iter().collect::<String>()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};

    #[test]
    fn test_runtime_id_is_uuid() {
        let id = generate_runtime_id();
        assert!(uuid::Uuid::parse_str(&id).is_ok());
    }

    #[test]
    fn test_file_id_includes_timestamp_prefix_and_suffix() {
        let now = Local.with_ymd_and_hms(2026, 3, 11, 9, 30, 45).unwrap();
        let id = generate_file_id(now);
        assert!(id.starts_with("20260311-093045-"));
        assert_eq!(id.len(), "20260311-093045-ABCD".len());
    }
}
