pub fn counter(name: &str, delta: f64) -> Result<(), String> {
    crate::types::patina::measure::measure::counter(name, delta)
}

pub fn gauge(name: &str, value: f64) -> Result<(), String> {
    crate::types::patina::measure::measure::gauge(name, value)
}
