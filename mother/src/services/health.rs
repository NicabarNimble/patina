use crate::registry::ChildRegistry;

#[derive(Debug, Clone)]
pub struct HealthService;

impl Default for HealthService {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthService {
    pub fn new() -> Self {
        Self
    }

    pub fn child_health_all(&self, registry: &ChildRegistry) -> Vec<(String, crate::ChildHealth)> {
        registry.health_all()
    }
}
