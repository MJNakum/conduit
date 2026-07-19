mod forwards;
mod hosts;
mod keys;
mod knownhosts;
mod logging;
mod secrets;
mod snippets;
mod ssh;
mod sshconfig;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ssh::SshState::default())
        .manage(forwards::ForwardState::default())
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
            keys::keys_list,
            keys::key_generate,
            keys::key_import,
            keys::key_delete,
            secrets::secret_set,
            secrets::secret_has,
            secrets::secret_delete,
            sshconfig::ssh_config_import,
            sshconfig::ssh_config_export,
            sshconfig::ssh_config_export_write,
            forwards::forwards_list,
            forwards::forward_save,
            forwards::forward_delete,
            forwards::forward_start,
            forwards::forward_stop,
            logging::logs_dir_path,
            snippets::snippets_list,
            snippets::snippet_save,
            snippets::snippet_delete
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
