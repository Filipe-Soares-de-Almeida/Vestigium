//! Prints the phases of the recent boots, read without elevation.

fn secs(ms: Option<u32>) -> String {
    ms.map_or_else(
        || "-".to_string(),
        |ms| format!("{:.1}", f64::from(ms) / 1000.0),
    )
}

fn main() {
    let boots = match vestigium_collect::boot_timeline(30) {
        Ok(boots) => boots,
        Err(err) => {
            eprintln!("boot timeline: {err}");
            std::process::exit(1);
        }
    };
    println!(
        "kernel_start_ms  type  fw  signin  wait  profile  tpm_wait  desktop  to_desktop  from_power  apps  logger_full"
    );
    for boot in &boots {
        let p = boot.phases();
        println!(
            "{}  {:?}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}",
            boot.kernel_start_ms,
            boot.boot_type,
            secs(p.firmware_ms),
            secs(p.to_signin_ms),
            p.probable_wait,
            secs(p.profile_ms),
            secs(p.tpm_wait_ms),
            secs(p.desktop_ms),
            secs(p.to_desktop_ms),
            secs(p.from_power_ms),
            boot.apps.len(),
            boot.logger_full,
        );
    }
}
