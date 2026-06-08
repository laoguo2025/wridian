mod cocreation;
mod chat_persistence;
mod memory;
mod model_accounts;
mod projects;
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
            workspace::wridian_create_work_file,
            workspace::wridian_create_work_folder,
            workspace::wridian_duplicate_work_node,
            workspace::wridian_rename_work_node,
            workspace::wridian_trash_work_node,
            model_accounts::wridian_get_custom_api_settings,
            model_accounts::wridian_save_custom_api_settings,
            model_accounts::wridian_test_custom_api,
            projects::wridian_get_project_state,
            projects::wridian_save_project,
            projects::wridian_select_project,
            projects::wridian_find_relevant_notes,
            cocreation::wridian_cocreate,
            chat_persistence::wridian_save_chat_transcript,
            memory::wridian_get_memory_state,
            memory::wridian_ingest_memory_wiki,
            memory::wridian_rebuild_memory_wiki_index,
            memory::wridian_search_memory_wiki,
            memory::wridian_get_memory_graph,
            memory::wridian_create_memory_candidate,
            memory::wridian_extract_memory_candidates,
            memory::wridian_update_memory_candidate,
            memory::wridian_accept_memory_candidate,
            memory::wridian_ignore_memory_candidate
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
