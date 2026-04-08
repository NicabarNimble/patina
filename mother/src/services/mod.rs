pub mod health;
pub mod secrets;
pub mod sessions;

use health::HealthService;
use secrets::SecretsService;
use sessions::SessionStateService;

pub struct MotherServices {
    pub health: HealthService,
    pub secrets: SecretsService,
    pub sessions: SessionStateService,
}

impl MotherServices {
    pub fn new(runtime_store: crate::MotherRuntimeStore) -> Self {
        Self {
            health: HealthService::new(),
            secrets: SecretsService::new(),
            sessions: SessionStateService::new(runtime_store),
        }
    }
}
