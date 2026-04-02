use std::sync::{Mutex, OnceLock};

pub fn env_test_mutex() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
pub fn with_temp_patina_home<T>(f: impl FnOnce(std::path::PathBuf) -> T) -> T {
    let _guard = env_test_mutex()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let temp = tempfile::TempDir::new().unwrap();
    let patina_home = temp.path().join("patina-home");
    std::fs::create_dir_all(&patina_home).unwrap();
    let old_patina_home = std::env::var_os("PATINA_HOME");
    unsafe {
        std::env::set_var("PATINA_HOME", &patina_home);
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(patina_home)));
    match old_patina_home {
        Some(value) => unsafe {
            std::env::set_var("PATINA_HOME", value);
        },
        None => unsafe {
            std::env::remove_var("PATINA_HOME");
        },
    }
    match result {
        Ok(value) => value,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}
