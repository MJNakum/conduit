mod auth;
#[cfg(target_os = "linux")]
mod filestore;
mod forwards;
mod hosts;
mod keys;
mod knownhosts;
mod logging;
mod putty;
mod secrets;
mod sftp;
mod snippets;
mod ssh;
mod sshconfig;
mod telnet;

/// Open a URL in the user's default browser. Every launcher below receives the
/// URL as a single CreateProcess/exec argument — none go through a shell, so
/// shell metacharacters are inert (macOS `open`, Linux `xdg-open`, and Windows
/// `explorer` which hands the URL to the default protocol handler; notably NOT
/// `cmd /C start`, which would interpret `& | ^ < > %`). Restricted to http(s)
/// with no control/whitespace chars as defense in depth.
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("refusing non-http(s) url".into());
    }
    if url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("refusing url with control/whitespace characters".into());
    }
    #[cfg(target_os = "macos")]
    let mut cmd = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut cmd = std::process::Command::new("explorer");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut cmd = std::process::Command::new("xdg-open");
    cmd.arg(&url).spawn().map_err(|e| format!("open url: {e}"))?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .manage(ssh::SshState::default())
        .manage(forwards::ForwardState::default())
        .manage(sftp::SftpState::default());

    // In-app auto-update (+ process for the relaunch after install). Desktop
    // only — the updater/process plugins don't apply on mobile targets.
    #[cfg(desktop)]
    {
        builder = builder
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_process::init());
    }

    builder
        // Resolve the app data dir once so `secrets.rs` can reach the encrypted
        // file store from the SSH/host layers, which hold no AppHandle.
        .setup(|app| {
            use tauri::Manager;
            if let Ok(dir) = app.path().app_data_dir() {
                secrets::init_data_dir(dir);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ssh::ssh_connect,
            ssh::ssh_write,
            ssh::ssh_resize,
            ssh::ssh_disconnect,
            ssh::ssh_reconnect,
            ssh::ssh_host_key_decision,
            ssh::ssh_prompt_response,
            ssh::telnet_connect,
            hosts::hosts_list,
            hosts::host_save,
            hosts::host_delete,
            knownhosts::known_hosts_list,
            keys::keys_list,
            keys::key_generate,
            keys::key_import,
            keys::key_delete,
            secrets::secret_set,
            secrets::secret_has,
            secrets::secret_delete,
            secrets::secrets_purge,
            secrets::secret_backend_status,
            secrets::secret_backend_pin,
            secrets::secret_store_create,
            secrets::secret_store_unlock,
            sshconfig::ssh_config_import,
            sshconfig::ssh_config_export,
            sshconfig::ssh_config_export_write,
            forwards::forwards_list,
            forwards::forward_save,
            forwards::forward_delete,
            forwards::forward_start,
            forwards::forward_stop,
            logging::logs_dir_path,
            logging::conn_log_save,
            logging::logs_list,
            logging::log_read,
            logging::log_reveal,
            logging::log_delete,
            snippets::snippets_list,
            snippets::snippet_save,
            snippets::snippet_delete,
            putty::putty_import,
            sftp::sftp_open,
            sftp::sftp_list,
            sftp::sftp_download,
            sftp::sftp_upload,
            sftp::sftp_close,
            auth::vault_authenticate,
            open_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
