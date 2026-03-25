use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSnapshot {
    pub name: String,
    pub available: bool,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolChange {
    pub name: String,
    pub old_version: Option<String>,
    pub new_version: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EnvironmentChanges {
    pub missing_tools: Vec<ToolChange>,
    pub new_tools: Vec<ToolChange>,
    pub version_changes: Vec<ToolChange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStatus {
    pub llm: String,
    pub adapter_version: Option<String>,
    pub layer_patterns: usize,
    pub sessions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionDurability {
    pub uncommitted: bool,
    pub dirty_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EmissionCoverage {
    pub types_checked: usize,
    pub types_with_data: usize,
    pub types_empty: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventsDbStatus {
    pub exists: bool,
    pub integrity_ok: bool,
    pub event_count: i64,
    pub max_seq: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JsonlReplicaStatus {
    pub exists: bool,
    pub max_seq: i64,
    pub gap: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DataIntegrity {
    pub events_db: EventsDbStatus,
    pub jsonl_replica: JsonlReplicaStatus,
    pub emission_coverage: EmissionCoverage,
    pub session_durability: SessionDurability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthCheck {
    pub status: HealthStatus,
    pub environment_changes: EnvironmentChanges,
    pub project_config: ProjectStatus,
    pub data_integrity: DataIntegrity,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorRunResult {
    pub health: HealthCheck,
    pub exit_code: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSnapshot {
    pub llm: String,
    pub adapter_version: Option<String>,
    pub layer_patterns: usize,
    pub sessions: usize,
    pub stored_tools: Vec<String>,
}

pub trait EnvironmentPort {
    fn detect_tools(&self) -> Result<Vec<ToolSnapshot>, String>;
}

pub trait ProjectRepoPort {
    fn snapshot(&self) -> Result<ProjectSnapshot, String>;
}

pub trait EventStorePort {
    fn data_integrity(&self) -> Result<DataIntegrity, String>;
    fn data_recommendations(&self) -> Result<Vec<String>, String>;
    fn session_recommendations(&self) -> Result<Vec<String>, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoctorServiceError {
    Environment(String),
    ProjectRepo(String),
    EventStore(String),
}

impl fmt::Display for DoctorServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Environment(message) | Self::ProjectRepo(message) | Self::EventStore(message) => {
                write!(f, "{}", message)
            }
        }
    }
}

impl Error for DoctorServiceError {}

pub fn run_doctor(
    environment: &dyn EnvironmentPort,
    project_repo: &dyn ProjectRepoPort,
    event_store: &dyn EventStorePort,
) -> Result<DoctorRunResult, DoctorServiceError> {
    let project = project_repo
        .snapshot()
        .map_err(DoctorServiceError::ProjectRepo)?;
    let tools = environment
        .detect_tools()
        .map_err(DoctorServiceError::Environment)?;

    let mut recommendations = Vec::new();
    let environment_changes =
        analyze_environment_changes(&tools, &project.stored_tools, &mut recommendations);
    let mut status = status_from_environment(&environment_changes);

    let data_integrity = event_store
        .data_integrity()
        .map_err(DoctorServiceError::EventStore)?;
    recommendations.extend(
        event_store
            .data_recommendations()
            .map_err(DoctorServiceError::EventStore)?,
    );
    recommendations.extend(
        event_store
            .session_recommendations()
            .map_err(DoctorServiceError::EventStore)?,
    );

    if has_data_warnings(&data_integrity) && status == HealthStatus::Healthy {
        status = HealthStatus::Warning;
    }
    if data_integrity.session_durability.uncommitted && status == HealthStatus::Healthy {
        status = HealthStatus::Warning;
    }
    if data_integrity.events_db.exists && !data_integrity.events_db.integrity_ok {
        status = HealthStatus::Critical;
    }

    if data_integrity.emission_coverage.types_checked > 0
        && !data_integrity.emission_coverage.types_empty.is_empty()
    {
        recommendations.push(format!(
            "No events for: {}. Run the corresponding commands to populate.",
            data_integrity.emission_coverage.types_empty.join(", ")
        ));
    }

    let exit_code = match status {
        HealthStatus::Healthy => 0,
        HealthStatus::Warning => 2,
        HealthStatus::Critical => 3,
    };

    Ok(DoctorRunResult {
        health: HealthCheck {
            status,
            environment_changes,
            project_config: ProjectStatus {
                llm: project.llm,
                adapter_version: project.adapter_version,
                layer_patterns: project.layer_patterns,
                sessions: project.sessions,
            },
            data_integrity,
            recommendations,
        },
        exit_code,
    })
}

fn analyze_environment_changes(
    tools: &[ToolSnapshot],
    stored_tools: &[String],
    recommendations: &mut Vec<String>,
) -> EnvironmentChanges {
    let tool_map: HashMap<&str, &ToolSnapshot> = tools
        .iter()
        .map(|tool| (tool.name.as_str(), tool))
        .collect();

    let mut missing_tools = Vec::new();
    for tool_name in stored_tools {
        let available = tool_map
            .get(tool_name.as_str())
            .map(|tool| tool.available)
            .unwrap_or(false);
        if !available {
            let required = matches!(tool_name.as_str(), "cargo" | "rust" | "git");
            missing_tools.push(ToolChange {
                name: tool_name.clone(),
                old_version: Some("detected".to_string()),
                new_version: None,
                required,
            });

            if required {
                let install = match tool_name.as_str() {
                    "cargo" | "rust" => "curl https://sh.rustup.rs -sSf | sh",
                    "docker" => "Visit https://docker.com/get-started",
                    "git" => "brew install git (macOS) or apt install git (Linux)",
                    _ => "Check your package manager",
                };
                recommendations.push(format!("Install {tool_name}: {install}"));
            }
        }
    }

    let mut new_tools = Vec::new();
    for tool in tools {
        if tool.available && !stored_tools.contains(&tool.name) {
            new_tools.push(ToolChange {
                name: tool.name.clone(),
                old_version: None,
                new_version: tool.version.clone(),
                required: false,
            });
        }
    }

    EnvironmentChanges {
        missing_tools,
        new_tools,
        version_changes: Vec::new(),
    }
}

fn status_from_environment(environment_changes: &EnvironmentChanges) -> HealthStatus {
    if environment_changes
        .missing_tools
        .iter()
        .any(|tool| tool.required)
    {
        HealthStatus::Critical
    } else if !environment_changes.missing_tools.is_empty() {
        HealthStatus::Warning
    } else {
        HealthStatus::Healthy
    }
}

fn has_data_warnings(data_integrity: &DataIntegrity) -> bool {
    !data_integrity.events_db.warnings.is_empty()
        || !data_integrity.jsonl_replica.warnings.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeEnvironment {
        tools: Vec<ToolSnapshot>,
    }

    impl EnvironmentPort for FakeEnvironment {
        fn detect_tools(&self) -> Result<Vec<ToolSnapshot>, String> {
            Ok(self.tools.clone())
        }
    }

    struct FakeProjectRepo {
        snapshot: ProjectSnapshot,
    }

    impl ProjectRepoPort for FakeProjectRepo {
        fn snapshot(&self) -> Result<ProjectSnapshot, String> {
            Ok(self.snapshot.clone())
        }
    }

    struct FakeEventStore {
        integrity: DataIntegrity,
        data_recommendations: Vec<String>,
        session_recommendations: Vec<String>,
    }

    impl EventStorePort for FakeEventStore {
        fn data_integrity(&self) -> Result<DataIntegrity, String> {
            Ok(self.integrity.clone())
        }

        fn data_recommendations(&self) -> Result<Vec<String>, String> {
            Ok(self.data_recommendations.clone())
        }

        fn session_recommendations(&self) -> Result<Vec<String>, String> {
            Ok(self.session_recommendations.clone())
        }
    }

    fn base_project_snapshot() -> ProjectSnapshot {
        ProjectSnapshot {
            llm: "opencode".to_string(),
            adapter_version: Some("1.0.0".to_string()),
            layer_patterns: 10,
            sessions: 5,
            stored_tools: vec!["git".to_string(), "cargo".to_string()],
        }
    }

    #[test]
    fn required_missing_tool_marks_critical() {
        let env = FakeEnvironment {
            tools: vec![ToolSnapshot {
                name: "git".to_string(),
                available: false,
                version: None,
            }],
        };
        let project = FakeProjectRepo {
            snapshot: base_project_snapshot(),
        };
        let events = FakeEventStore {
            integrity: DataIntegrity::default(),
            data_recommendations: Vec::new(),
            session_recommendations: Vec::new(),
        };

        let result = run_doctor(&env, &project, &events).unwrap();
        assert_eq!(result.health.status, HealthStatus::Critical);
        assert_eq!(result.exit_code, 3);
    }

    #[test]
    fn data_warning_escalates_to_warning() {
        let env = FakeEnvironment {
            tools: vec![
                ToolSnapshot {
                    name: "git".to_string(),
                    available: true,
                    version: Some("2.0".to_string()),
                },
                ToolSnapshot {
                    name: "cargo".to_string(),
                    available: true,
                    version: Some("1.0".to_string()),
                },
            ],
        };
        let project = FakeProjectRepo {
            snapshot: base_project_snapshot(),
        };
        let mut integrity = DataIntegrity::default();
        integrity
            .events_db
            .warnings
            .push("events.db not found".to_string());
        let events = FakeEventStore {
            integrity,
            data_recommendations: vec!["Run any command to initialize events.db".to_string()],
            session_recommendations: Vec::new(),
        };

        let result = run_doctor(&env, &project, &events).unwrap();
        assert_eq!(result.health.status, HealthStatus::Warning);
        assert_eq!(result.exit_code, 2);
    }
}
