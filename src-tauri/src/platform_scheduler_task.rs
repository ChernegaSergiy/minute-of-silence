use std::process::Command;

/// Returns true when the current process is running from an MSIX package
/// (i.e. installed via Microsoft Store or `.msix`/`.msixbundle`).
///
/// The reliable indicator is that the executable path contains `\WindowsApps\`,
/// which is the protected directory where the OS unpacks MSIX packages.
/// We intentionally avoid `GetCurrentPackageFullName` WinAPI here to keep the
/// dependency surface minimal — the path check is sufficient for our purposes.
#[allow(dead_code)]
pub fn is_msix_package() -> bool {
    std::env::current_exe()
        .map(|p| {
            let s = p.to_string_lossy().to_ascii_lowercase();
            s.contains("\\windowsapps\\")
        })
        .unwrap_or(false)
}

/// Create a Windows Task Scheduler task that runs the app at user logon.
///
/// Important: for MSIX packages this function must **not** be called — use the
/// `StartupTask` manifest extension instead (see `appxmanifest.xml`).
/// The function is therefore only reached for plain MSI / portable installs.
#[allow(dead_code)]
pub fn create_autostart_task(exe_path: &str) -> Result<(), String> {
    let task_name = "MinuteOfSilence";
    log::info!("Creating Windows Task Scheduler task: {}", exe_path);

    let _ = Command::new("schtasks")
        .args(["/Delete", "/TN", task_name, "/F"])
        .output();

    let quoted = format!("\"{}\"", exe_path);

    let output = Command::new("schtasks")
        .args([
            "/Create", "/TN", task_name, "/TR", &quoted, "/SC", "ONLOGON", "/RL", "LIMITED", "/F",
        ])
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        log::info!("Successfully created autostart task");
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        log::error!("Failed to create autostart task: {}", error);
        Err(error)
    }
}

#[allow(dead_code)]
pub fn remove_autostart_task() -> Result<(), String> {
    let task_name = "MinuteOfSilence";
    log::info!("Removing Windows Task Scheduler task");

    let output = Command::new("schtasks")
        .args(["/Delete", "/TN", task_name, "/F"])
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        log::info!("Successfully removed autostart task");
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        log::error!("Failed to remove autostart task: {}", error);
        Err(error)
    }
}

#[allow(dead_code)]
pub fn is_autostart_task_exists() -> bool {
    let task_name = "MinuteOfSilence";
    Command::new("schtasks")
        .args(["/Query", "/TN", task_name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
