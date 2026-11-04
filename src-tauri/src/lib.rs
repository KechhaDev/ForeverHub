
mod launcher;
mod account;
mod profiles;
mod dev_auth;
mod installer;
mod auth;
mod commands;
mod modpack;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(commands::AuthState::default())
        .invoke_handler(
            tauri::generate_handler![
                commands::launcher::launch_game,
                commands::auth::login_microsoft,
                commands::auth::restore_microsoft_session,
                commands::auth::get_account,
                commands::auth::logout_microsoft,
                commands::github::sync_modpack,
                commands::profiles::get_profiles,
                commands::profiles::update_profile_ram,
                commands::profiles::refresh_profile_metadata,
                commands::storage::get_data_directory,
            ]
        )
        .setup(|app| {
            if !profiles::setup_directories() {
                return Err(
                    "ForeverHub-Verzeichnisse konnten nicht erstellt werden.".into()
                );
            }

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
