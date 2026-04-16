pub fn info(context: &str, message: &str) {
    crate::types::wasi::logging::logging::log(
        crate::types::wasi::logging::logging::Level::Info,
        context,
        message,
    );
}

pub fn warn(context: &str, message: &str) {
    crate::types::wasi::logging::logging::log(
        crate::types::wasi::logging::logging::Level::Warn,
        context,
        message,
    );
}

pub fn error(context: &str, message: &str) {
    crate::types::wasi::logging::logging::log(
        crate::types::wasi::logging::logging::Level::Error,
        context,
        message,
    );
}
