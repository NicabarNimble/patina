use patina_sdk::child::{Child, ChildHealth, HealthStatus};
use patina_sdk::granted::{self, Bundle as GrantedBundle};
use patina_sdk::register_child;

#[derive(Debug, Clone)]
struct DoctorToys {
    log: granted::Log,
    state: granted::State,
}

impl GrantedBundle for DoctorToys {
    fn granted() -> Self {
        Self {
            log: granted::log(),
            state: granted::state(),
        }
    }
}

#[derive(Default)]
struct DoctorChild {
    toys: Option<DoctorToys>,
}

impl DoctorChild {
    fn toys(&self) -> &DoctorToys {
        self.toys.as_ref().expect("doctor toys not initialized")
    }

    fn toys_mut(&mut self) -> &mut DoctorToys {
        self.toys.as_mut().expect("doctor toys not initialized")
    }

    fn run_count(&self) -> u64 {
        self.toys()
            .state
            .get("doctor.run-count")
            .and_then(|value| serde_json::from_str::<u64>(&value).ok())
            .unwrap_or(0)
    }
}

impl Child for DoctorChild {
    fn name(&self) -> String {
        "doctor".into()
    }

    fn on_load(&mut self) -> Result<(), String> {
        self.toys = Some(DoctorToys::granted());
        self.toys().log.info("doctor loaded");
        Ok(())
    }

    fn health(&self) -> ChildHealth {
        ChildHealth {
            status: HealthStatus::Healthy,
            reason: Some(format!("run-count={}", self.run_count())),
        }
    }

    fn handle(&mut self, action: &str, _payload: &str) -> Result<String, String> {
        match action {
            "run" => {
                let count = self.run_count() + 1;
                self.toys_mut().state.put(
                    "doctor.run-count",
                    &serde_json::to_string(&count).map_err(|e| e.to_string())?,
                )?;
                Ok(serde_json::json!({
                    "status": "ok",
                    "message": "doctor run complete",
                    "run_count": count
                })
                .to_string())
            }
            other => Err(format!("doctor: unknown action '{}'", other)),
        }
    }
}

register_child!(DoctorChild);
