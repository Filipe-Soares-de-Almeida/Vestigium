//! Prints the health findings of a history database, read-only: `findings <path-to-db>`.

use std::path::PathBuf;

use vestigium_store::Store;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path: PathBuf = std::env::args_os()
        .nth(1)
        .ok_or("usage: findings <path-to-db>")?
        .into();
    let store = Store::open_read_only(&path)?;
    let all = store.findings(i64::MIN, i64::MAX)?;
    println!("findings: {}", all.len());
    for record in &all {
        println!(
            "#{} opened {} updated {} closed {:?}\n  {:?}",
            record.id, record.opened_ms, record.updated_ms, record.closed_ms, record.finding
        );
    }
    for row in store.dumps(i64::MIN, i64::MAX)? {
        let r = &row.record;
        println!(
            "dump {:?} {} {} MB crash {} file {} deleted {:?} analyzed {}",
            r.kind,
            r.path,
            r.size_bytes >> 20,
            r.crash_ms,
            r.file_ms,
            row.deleted_ms,
            r.analysis.is_some()
        );
        if !matches!(r.kind, vestigium_core::DumpKind::App)
            && let Some(a) = &r.analysis
        {
            println!(
                "  fault {:?} module {:?} process {:?} stack {:?} culprit {:?}",
                a.fault,
                a.fault_module,
                a.process,
                a.stack_modules.iter().take(8).collect::<Vec<_>>(),
                vestigium_core::dump_culprit(a)
            );
        }
    }
    let episodes = store.episodes(i64::MIN, i64::MAX)?;
    let occurrences = store.occurrences(i64::MIN, i64::MAX)?;
    let events = store.events(i64::MIN, i64::MAX)?;
    println!(
        "episodes {} occurrences {} events {}",
        episodes.len(),
        occurrences.len(),
        events.len()
    );
    if let Some(first) = events.first() {
        println!(
            "first event {} last event {}",
            first.ts_ms,
            events.last().map_or(0, |e| e.ts_ms)
        );
    }
    Ok(())
}
