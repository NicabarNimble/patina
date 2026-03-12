use chrono::{DateTime, Local};

pub fn generate_runtime_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn generate_file_id(now: DateTime<Local>) -> String {
    now.format("%Y%m%d-%H%M%S").to_string()
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
    fn test_file_id_is_timestamp_only() {
        let now = Local.with_ymd_and_hms(2026, 3, 11, 9, 30, 45).unwrap();
        let id = generate_file_id(now);
        assert_eq!(id, "20260311-093045");
    }
}
