#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
#[cfg(target_os = "windows")]
mod desktop;
mod model;
fn main() {
    #[cfg(target_os = "windows")]
    if let Err(error) = desktop::run() {
        rfd::MessageDialog::new()
            .set_title("Social Workspace")
            .set_description(error)
            .show();
    }
    #[cfg(not(target_os = "windows"))]
    eprintln!("This desktop preview targets Windows. Core tests run with cargo test.");
}
