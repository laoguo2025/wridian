mod bridge;
mod chat_persistence;
mod cocreation;
mod creative_skills;
mod e2e;
mod knowledge_graph;
mod knowledge_ops;
mod memory;
mod metadata_index;
mod model_accounts;
mod opener;
mod path_safety;
mod projects;
mod rule_router;
mod runtime;
mod text_index;
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
            workspace::wridian_preview_file,
            workspace::wridian_preview_asset,
            workspace::wridian_save_file,
            workspace::wridian_create_work_file,
            workspace::wridian_create_work_folder,
            workspace::wridian_duplicate_work_node,
            workspace::wridian_rename_work_node,
            workspace::wridian_trash_work_node,
            e2e::wridian_e2e_status,
            e2e::wridian_e2e_prepare_fixture,
            e2e::wridian_e2e_set_next_cocreation,
            creative_skills::wridian_get_creative_skill_sources,
            model_accounts::wridian_get_model_accounts,
            model_accounts::wridian_save_model_provider,
            model_accounts::wridian_select_active_model,
            model_accounts::wridian_delete_model_provider,
            model_accounts::wridian_test_model_provider_config,
            model_accounts::wridian_anthropic_oauth_start,
            model_accounts::wridian_anthropic_oauth_complete,
            model_accounts::wridian_openai_oauth_start,
            model_accounts::wridian_openai_oauth_complete,
            model_accounts::wridian_google_gemini_oauth_login,
            opener::wridian_open_local_path,
            opener::wridian_open_memory_tree_folder,
            projects::wridian_get_project_state,
            projects::wridian_select_project,
            projects::wridian_find_relevant_notes,
            knowledge_graph::wridian_get_knowledge_graph,
            knowledge_ops::wridian_search_knowledge_bm25,
            knowledge_ops::wridian_run_knowledge_health_check,
            knowledge_ops::wridian_fix_knowledge_health_low_risk,
            bridge::wridian_apply_bridge_relation,
            cocreation::wridian_cocreate,
            cocreation::wridian_apply_chat_file_operations,
            cocreation::wridian_abort_cocreate,
            chat_persistence::wridian_load_chat_continuity,
            chat_persistence::wridian_save_chat_transcript,
            memory::wridian_get_memory_tree,
            memory::wridian_delete_memory_tree_file,
            memory::wridian_save_memory_tree_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
