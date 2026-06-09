mod chat_persistence;
mod cocreation;
mod knowledge_graph;
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
            workspace::wridian_set_knowledge_root,
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
            knowledge_graph::wridian_get_knowledge_graph,
            cocreation::wridian_cocreate,
            chat_persistence::wridian_save_chat_transcript,
            memory::wridian_get_memory_state,
            memory::wridian_get_memory_state_for_source,
            memory::wridian_get_memory_tree,
            memory::wridian_plant_memory_leaf,
            memory::wridian_propose_memory_leaf,
            memory::wridian_save_memory_tree_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
