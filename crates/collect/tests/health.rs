use vestigium_collect::{antivirus, disk_health, page_files, system_drive, volumes};

#[test]
fn volumes_include_the_system_volume() {
    let found = volumes();

    let system: Vec<_> = found.iter().filter(|v| v.system).collect();
    assert_eq!(system.len(), 1, "{found:?}");
    assert!(system[0].total_bytes > 0);
    assert!(system[0].free_bytes <= system[0].total_bytes);
}

#[test]
fn system_drive_is_found_without_elevation() {
    assert!(system_drive().is_some());
}

#[test]
fn page_files_are_read_without_elevation() {
    let files = page_files().expect("the page file list");

    for file in &files {
        assert!(file.size_bytes > 0, "{file:?}");
        assert!(file.path.ends_with(".sys"), "{file:?}");
    }
}

#[test]
fn health_reads_never_panic() {
    if let Some(health) = system_drive().and_then(|drive| disk_health(drive, "0 C:", 1)) {
        assert_eq!(health.name, "0 C:");
        assert_eq!(health.read_ms, 1);
    }
    assert_eq!(disk_health(250, "250", 1), None);
    if let Some(products) = antivirus() {
        assert!(products.iter().all(|p| !p.name.is_empty()));
    }
}
