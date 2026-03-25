pub mod secrets;

use secrets::SecretsService;

pub struct MotherServices {
    pub secrets: SecretsService,
}

impl Default for MotherServices {
    fn default() -> Self {
        Self::new()
    }
}

impl MotherServices {
    pub fn new() -> Self {
        Self {
            secrets: SecretsService::new(),
        }
    }
}
