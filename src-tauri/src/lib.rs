mod commands;
mod core;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::accounts::list_accounts,
            commands::accounts::add_offline_account,
            commands::accounts::remove_account,
            commands::accounts::ms_login_start,
            commands::accounts::ms_login_finish,
            commands::game::list_versions,
            commands::game::launch_game,
            commands::game::get_data_dir,
            commands::game::set_data_dir,
            commands::game::list_runtimes,
            commands::game::remove_runtime,
            commands::game::apply_skin,
            commands::mods::search_mods,
            commands::mods::install_mod,
            commands::mods::install_boost,
            commands::mods::list_mods,
            commands::mods::toggle_mod,
            commands::mods::apply_graphics,
            commands::mods::search_curseforge,
            commands::mods::install_curseforge,
            commands::ai::ai_chat,
            commands::ai::ai_autofix,
        ])
        .run(tauri::generate_context!())
        .expect("lỗi khi chạy ứng dụng Tauri");
}
