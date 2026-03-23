use std::path::{Path, PathBuf};

pub fn patina_home() -> PathBuf {
    if let Some(override_path) = std::env::var_os("PATINA_HOME") {
        return PathBuf::from(override_path);
    }

    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".patina")
}

pub mod secrets {
    use super::*;

    pub fn registry_path() -> PathBuf {
        patina_home().join("secrets.toml")
    }

    pub fn vault_path() -> PathBuf {
        patina_home().join("vault.age")
    }

    pub fn recipient_path() -> PathBuf {
        patina_home().join("recipient.txt")
    }

    pub fn project_registry_path(root: &Path) -> PathBuf {
        root.join(".patina").join("secrets.toml")
    }

    pub fn project_vault_path(root: &Path) -> PathBuf {
        root.join(".patina").join("vault.age")
    }

    pub fn project_recipients_path(root: &Path) -> PathBuf {
        root.join(".patina").join("recipients.txt")
    }
}

pub mod serve {
    use super::*;

    pub fn run_dir() -> PathBuf {
        patina_home().join("run")
    }

    pub fn socket_path() -> PathBuf {
        run_dir().join("serve.sock")
    }

    pub fn token_path() -> PathBuf {
        run_dir().join("serve.token")
    }
}
