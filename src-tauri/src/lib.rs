mod hosts;
mod knownhosts;
mod secrets;
mod ssh;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ssh::SshState::default())
        .invoke_handler(tauri::generate_handler![
            ssh::ssh_connect,
            ssh::ssh_write,
            ssh::ssh_resize,
            ssh::ssh_disconnect,
            ssh::ssh_reconnect,
            ssh::ssh_host_key_decision,
            hosts::hosts_list,
            hosts::host_save,
            hosts::host_delete,
            knownhosts::known_hosts_list,
            secrets::secret_set,
            secrets::secret_has,
            secrets::secret_delete
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
