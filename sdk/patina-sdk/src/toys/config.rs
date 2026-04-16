pub fn get(key: &str) -> Result<String, String> {
    crate::types::patina::config::config::get(key)
}
