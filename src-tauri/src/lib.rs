
mod launcher;
mod account;
mod profiles;
mod dev_auth;
mod installer;
mod auth;
mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::AuthState::default())
        .invoke_handler(
            tauri::generate_handler![
                commands::launcher::launch_game,
                commands::auth::login_microsoft,
                commands::auth::get_account,
                commands::auth::logout_microsoft
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
