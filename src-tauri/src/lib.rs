mod hosts;
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
            hosts::hosts_list,
            hosts::host_save,
            hosts::host_delete
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
