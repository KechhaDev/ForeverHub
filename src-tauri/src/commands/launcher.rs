
use serde::Serialize;
use std::process::{Child, ExitStatus};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, State};

use crate::dev_auth;
use crate::installer::prepare_installation;
use crate::launcher;
use crate::profiles;

use super::AuthState;

// ============================================================
// LAUNCHER STATUS
// ============================================================

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LauncherStatus {
    stage: String,
    profile_name: String,
    message: String,
}

fn emit_status(
    app: &AppHandle,
    stage: &str,
    profile_name: &str,
    message: &str,
) {
    let status = LauncherStatus {
        stage: stage.to_string(),
        profile_name: profile_name.to_string(),
        message: message.to_string(),
    };

    if let Err(error) = app.emit("launcher-status", status) {
        eprintln!("Launcher-Status konnte nicht gesendet werden: {}", error);
    }
}

// ============================================================
// LAUNCH GAME
// ============================================================

#[tauri::command]
pub(crate) async fn launch_game(
    profile_name: String,
    app: AppHandle,
    state: State<'_, AuthState>,
) -> Result<String, String> {
    let profile = profiles::find_profile(&profile_name)
        .ok_or_else(|| format!("Profil {} existiert nicht.", profile_name))?;

    let running_profiles = state.running_profiles.clone();

    // --------------------------------------------------------
    // PREVENT DUPLICATE LAUNCHES
    // --------------------------------------------------------

    {
        let mut running = running_profiles
            .lock()
            .map_err(|error| error.to_string())?;

        if !running.insert(profile_name.clone()) {
            let message = format!(
                "Minecraft läuft für das Profil {} bereits.",
                profile_name
            );

            emit_status(
                &app,
                "running",
                &profile_name,
                &message,
            );

            return Err(message);
        }
    }

    // --------------------------------------------------------
    // PREPARE AND START GAME
    // --------------------------------------------------------

    let result: Result<Child, String> = async {
        let session = match profile.authentication_mode {
            profiles::AuthenticationMode::Offline => {
                println!("Offline-Authentifizierung aktiviert.");
                dev_auth::create_session()
            }

            profiles::AuthenticationMode::Microsoft => {
                println!("Verwende Microsoft-Authentifizierung.");

                state.session
                    .lock()
                    .map_err(|error| error.to_string())?
                    .clone()
                    .ok_or_else(|| {
                        "Du bist nicht bei Microsoft angemeldet.".to_string()
                    })?
            }
        };

        // ----------------------------------------------------
        // MODPACK SYNCHRONIZATION
        // ----------------------------------------------------

        let fabric_enabled =
            profile.fabric_version != "/"
                && !profile.fabric_version.trim().is_empty();

        if fabric_enabled {
            emit_status(
                &app,
                "syncing",
                &profile_name,
                "ForeverWorld-Modpack wird synchronisiert...",
            );

            println!("Synchronisiere ForeverWorld-Modpack.");

            let result =
                crate::commands::github::synchronize_profile(&profile).await?;

            println!(
                "Modpack {} Version {} synchronisiert. \
                 {} Dateien heruntergeladen, {} bereits aktuell.",
                result.modpack_name,
                result.modpack_version,
                result.downloaded_files,
                result.up_to_date_files,
            );
        }

        // ----------------------------------------------------
        // INSTALLATION
        // ----------------------------------------------------

        emit_status(
            &app,
            "preparing",
            &profile_name,
            "Minecraft-Installation wird vorbereitet...",
        );

        println!("Bereite Minecraft-Installation vor.");

        prepare_installation(&profile).await?;

        // ----------------------------------------------------
        // START GAME
        // ----------------------------------------------------

        emit_status(
            &app,
            "starting",
            &profile_name,
            "Minecraft wird gestartet...",
        );

        println!("Starte Minecraft.");

        let child = launcher::start_game(
            &profile.install_dir,
            &profile.minecraft_version,
            fabric_enabled,
            profile.ram_gb,
            &session,
        ).await?;

        println!(
            "Minecraft-Prozess gestartet. PID: {}",
            child.id()
        );

        Ok(child)
    }
    .await;

    // --------------------------------------------------------
    // HANDLE RESULT
    // --------------------------------------------------------

    match result {
        Ok(child) => {
            emit_status(
                &app,
                "running",
                &profile_name,
                "Minecraft läuft.",
            );

            // Prozess unabhängig vom Command überwachen
            monitor_process(
                child,
                app,
                profile_name.clone(),
                running_profiles,
            );

            Ok(format!("{} wurde gestartet!", profile.name))
        }

        Err(error) => {
            if let Ok(mut running) = running_profiles.lock() {
                running.remove(&profile_name);
            }

            emit_status(
                &app,
                "error",
                &profile_name,
                &error,
            );

            Err(error)
        }
    }
}

// ============================================================
// PROCESS MONITORING
// ============================================================

fn monitor_process(
    mut child: Child,
    app: AppHandle,
    profile_name: String,
    running_profiles: std::sync::Arc<
        std::sync::Mutex<std::collections::HashSet<String>>
    >,
) {
    thread::spawn(move || {
        loop {
            match child.try_wait() {
                Ok(Some(exit_status)) => {
                    handle_process_exit(
                        &app,
                        &profile_name,
                        exit_status,
                    );

                    if let Ok(mut running) = running_profiles.lock() {
                        running.remove(&profile_name);
                    }

                    break;
                }

                Ok(None) => {
                    thread::sleep(Duration::from_secs(1));
                }

                Err(error) => {
                    let message = format!(
                        "Minecraft-Prozess konnte nicht überwacht werden: {}",
                        error
                    );

                    eprintln!("{}", message);

                    emit_status(
                        &app,
                        "error",
                        &profile_name,
                        &message,
                    );

                    if let Ok(mut running) = running_profiles.lock() {
                        running.remove(&profile_name);
                    }

                    break;
                }
            }
        }
    });
}

// ============================================================
// PROCESS EXIT
// ============================================================

fn handle_process_exit(
    app: &AppHandle,
    profile_name: &str,
    status: ExitStatus,
) {
    let message = match status.code() {
        Some(0) => "Minecraft wurde ordnungsgemäß beendet.".to_string(),

        Some(code) => {
            format!("Minecraft wurde mit Exit-Code {} beendet.", code)
        }

        None => "Minecraft wurde beendet.".to_string(),
    };

    println!("{}", message);

    emit_status(
        app,
        "stopped",
        profile_name,
        &message,
    );
}
