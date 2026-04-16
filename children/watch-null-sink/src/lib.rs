wit_bindgen::generate!({
    path: "wit",
    world: "watch-null-sink",
    generate_all,
});

use patina_sdk::toys;

struct WatchNullSink;

fn kind_label(kind: patina::watch::types::ChangeKind) -> &'static str {
    match kind {
        patina::watch::types::ChangeKind::Created => "created",
        patina::watch::types::ChangeKind::Modified => "modified",
        patina::watch::types::ChangeKind::Deleted => "deleted",
    }
}

impl exports::patina::watch::events::Guest for WatchNullSink {
    fn emit(change: patina::watch::types::FileChange) -> Result<(), String> {
        let kind = kind_label(change.change_kind);
        toys::measure::counter("watch_events_dropped", 1.0)?;
        toys::log::info(
            "watch-null-sink",
            &format!("drop kind={} path={}", kind, change.absolute_path),
        );
        Ok(())
    }
}

export!(WatchNullSink);
