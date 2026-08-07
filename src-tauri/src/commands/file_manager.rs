use std::{fmt::Display, path::Path};

use mangodisk_core::diagnostic_path;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use super::error::{into_command_result, run_blocking, CommandResult};

#[tauri::command]
pub async fn reveal_in_file_manager(path: String) -> CommandResult<()> {
    run_blocking("reveal_in_file_manager", move || {
        let path_log = diagnostic_path(Path::new(&path));
        log::info!("file_manager_reveal_requested path={path_log}");
        match tauri_plugin_opener::reveal_item_in_dir(Path::new(&path)) {
            Ok(()) => {
                log::info!("file_manager_reveal_finished path={path_log}");
                Ok(())
            }
            Err(error) => {
                let diagnostic = opener_error_diagnostic(&error);
                log::warn!("file_manager_reveal_failed path={path_log} {diagnostic}");
                Err(diagnostic)
            }
        }
    })
    .await
}

/// Opens MangoDisk's application-owned log directory without exposing its
/// platform-specific path to the webview. Directory resolution stays in the
/// Tauri adapter because it depends on the installed application identity.
#[tauri::command]
pub fn open_application_log_directory(app: AppHandle) -> CommandResult<()> {
    // Resolve and encode the application-owned path entirely in the backend.
    // The supported Windows and macOS directory providers return Unicode
    // application paths, while the webview never sees the resolved value.
    let directory = app
        .path()
        .app_log_dir()
        .map_err(|error| log_directory_error_diagnostic(&error))
        .and_then(|path| {
            path.into_os_string().into_string().map_err(|path| {
                log_directory_error_diagnostic(&format!("non-unicode path: {path:?}"))
            })
        });
    let result: Result<(), String> = (|| {
        let directory = directory?;
        log::info!("application_log_directory_open_requested");
        app.opener()
            .open_path(directory, None::<String>)
            .map_err(|error| log_directory_error_diagnostic(&error))?;
        log::info!("application_log_directory_open_finished");
        Ok(())
    })();
    into_command_result("open_application_log_directory", result)
}

fn opener_error_diagnostic(error: &tauri_plugin_opener::Error) -> String {
    let digest = blake3::hash(error.to_string().as_bytes()).to_hex();
    format!("opener_reveal_failed error_digest={}", &digest[..12])
}

fn log_directory_error_diagnostic(error: &dyn Display) -> String {
    let digest = blake3::hash(error.to_string().as_bytes()).to_hex();
    format!(
        "application_log_directory_open_failed error_digest={}",
        &digest[..12]
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn opener_diagnostic_does_not_expose_private_paths() {
        let private_path = PathBuf::from(r"C:\Users\Developer\Private\project\target");
        let error = tauri_plugin_opener::Error::NoParent(private_path.clone());

        let diagnostic = opener_error_diagnostic(&error);

        assert!(diagnostic.starts_with("opener_reveal_failed error_digest="));
        assert!(!diagnostic.contains(private_path.to_string_lossy().as_ref()));
        assert!(!diagnostic.contains("Developer"));
        assert!(!diagnostic.contains("Private"));
    }

    #[test]
    fn log_directory_diagnostic_does_not_expose_private_paths() {
        let private_path = r"C:\Users\Developer\AppData\Local\MangoDisk\logs";

        let diagnostic = log_directory_error_diagnostic(&format!("cannot open {private_path}"));

        assert!(diagnostic.starts_with("application_log_directory_open_failed error_digest="));
        assert!(!diagnostic.contains(private_path));
        assert!(!diagnostic.contains("Developer"));
    }
}
