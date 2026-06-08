mod memory;
mod model_accounts;
mod runtime;
mod workspace;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            workspace::wridian_init_workspace,
            workspace::wridian_set_work_root,
            workspace::wridian_open_file,
            workspace::wridian_save_file,
            model_accounts::wridian_get_custom_api_settings,
            model_accounts::wridian_save_custom_api_settings,
            model_accounts::wridian_test_custom_api,
            memory::wridian_get_memory_state,
            memory::wridian_create_memory_candidate,
            memory::wridian_accept_memory_candidate,
            memory::wridian_ignore_memory_candidate
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
