mod chat_persistence;
mod cocreation;
mod creative_skills;
mod knowledge_graph;
mod memory;
mod model_accounts;
mod opener;
mod path_safety;
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
            creative_skills::wridian_get_creative_skill_sources,
            model_accounts::wridian_get_model_accounts,
            model_accounts::wridian_save_model_provider,
            model_accounts::wridian_select_active_model,
            model_accounts::wridian_delete_model_provider,
            model_accounts::wridian_test_model_provider,
            model_accounts::wridian_test_model_provider_config,
            model_accounts::wridian_anthropic_oauth_start,
            model_accounts::wridian_anthropic_oauth_complete,
            model_accounts::wridian_openai_oauth_login,
            model_accounts::wridian_google_gemini_oauth_login,
            opener::wridian_open_local_path,
            projects::wridian_get_project_state,
            projects::wridian_save_project,
            projects::wridian_select_project,
            projects::wridian_find_relevant_notes,
            knowledge_graph::wridian_get_knowledge_graph,
            cocreation::wridian_cocreate,
            cocreation::wridian_abort_cocreate,
            chat_persistence::wridian_load_chat_continuity,
            chat_persistence::wridian_save_chat_transcript,
            memory::wridian_get_memory_state,
            memory::wridian_get_memory_state_for_source,
            memory::wridian_get_memory_tree,
            memory::wridian_delete_memory_tree_file,
            memory::wridian_save_memory_tree_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
