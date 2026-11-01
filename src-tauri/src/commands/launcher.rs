
use tauri::State;

use crate::dev_auth;
use crate::installer::prepare_installation;
use crate::launcher;
use crate::profiles;

use super::AuthState;

// ============================================================
// GAME LAUNCH COMMAND
// ============================================================

#[tauri::command]
pub(crate) async fn launch_game(
    profile_name: String,
    state: State<'_, AuthState>,
) -> Result<String, String> {

    // --------------------------------------------------------
    // FIND PROFILE
    // --------------------------------------------------------

    let profile = profiles::find_profile(&profile_name)
        .ok_or_else(|| {
            format!("Profil {} existiert nicht.", profile_name)
        })?;

    // --------------------------------------------------------
    // SELECT AUTHENTICATION
    // --------------------------------------------------------

    let session = match profile.authentication_mode {
        profiles::AuthenticationMode::Offline => {
            println!("Offline-Authentifizierung aktiviert.");
            dev_auth::create_session()
        }

        profiles::AuthenticationMode::Microsoft => {
            println!("Verwende Microsoft-Authentifizierung.");

            state.session.lock()
                .map_err(|error| error.to_string())?
                .clone()
                .ok_or_else(|| {
                    "Du bist nicht bei Microsoft angemeldet.".to_string()
                })?
        }
    };

    // --------------------------------------------------------
    // PREPARE INSTALLATION
    // --------------------------------------------------------

    prepare_installation(&profile).await?;

    // --------------------------------------------------------
    // DETERMINE LOADER
    // --------------------------------------------------------

    let fabric_enabled =
        profile.fabric_version != "/"
            && !profile.fabric_version.trim().is_empty();

    // --------------------------------------------------------
    // START GAME
    // --------------------------------------------------------

    let child = launcher::start_game(
        &profile.install_dir,
        &profile.minecraft_version,
        fabric_enabled,
        &session,
    )?;

    println!(
        "Minecraft-Prozess gestartet. PID: {}",
        child.id()
    );

    Ok(format!(
        "{} wurde gestartet!",
        profile.name
    ))
}
