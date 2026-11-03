
use serde::Serialize;

use crate::modpack::github::GitHubClient;
use crate::modpack::sync;
use crate::profiles::{find_profile, Profile};

// ============================================================
// MODPACK SYNC RESULT
// ============================================================

#[derive(Serialize)]
pub(crate) struct ModpackSyncResult {
    pub(crate) profile_name: String,
    pub(crate) modpack_name: String,
    pub(crate) modpack_version: String,
    pub(crate) total_files: usize,
    pub(crate) downloaded_files: usize,
    pub(crate) up_to_date_files: usize,
}

// ============================================================
// SYNCHRONIZE PROFILE
// ============================================================

pub(crate) async fn synchronize_profile(
    profile: &Profile,
) -> Result<ModpackSyncResult, String> {

    // Vanilla profiles do not use a modpack.
    let modpack_id = profile.modpack_id.as_deref()
        .ok_or_else(|| {
            "Für dieses Profil ist kein Modpack konfiguriert.".to_string()
        })?;

    // Connect to GitHub.
    let github = GitHubClient::new()?;

    println!(
        "[Modpack] Profil: {} | Modpack-ID: {} | Installationspfad: {}",
        profile.name,
        modpack_id,
        profile.install_dir.display()
    );

    // Fetch and validate the selected modpack manifest.
    let manifest = github.fetch_manifest(modpack_id).await?;

    // Check Minecraft version compatibility.
    if manifest.minecraft_version != profile.minecraft_version {
        return Err(format!(
            "Minecraft-Version stimmt nicht überein. Profil: {}, Modpack: {}",
            profile.minecraft_version,
            manifest.minecraft_version
        ));
    }

    // Check Fabric version compatibility.
    if manifest.fabric_version != profile.fabric_version {
        return Err(format!(
            "Fabric-Version stimmt nicht überein. Profil: {}, Modpack: {}",
            profile.fabric_version,
            manifest.fabric_version
        ));
    }

    // --------------------------------------------------------
    // CONFIG IMPORT
    // --------------------------------------------------------

    // Decide whether default configs should be imported.
    // This decision is stored separately from the initial
    // installation marker.
    let import_configs = sync::should_import_configs(
        &profile.install_dir,
    )?;

    // --------------------------------------------------------
    // CREATE SYNC PLAN
    // --------------------------------------------------------

    let plan = sync::create_sync_plan(
        &manifest,
        &profile.install_dir,
        import_configs,
    )?;

    let downloaded_files = plan.files_to_download.len();
    let up_to_date_files = plan.files_up_to_date.len();

    // --------------------------------------------------------
    // DOWNLOAD FILES
    // --------------------------------------------------------

    sync::download_files(
        &github,
        &manifest,
        &plan,
        &profile.install_dir,
        modpack_id,
    )
    .await?;

    // --------------------------------------------------------
    // INSTALL DEFAULT OPTIONS
    // --------------------------------------------------------

    // Install default options.txt only if it does not exist.
    sync::install_default_options(
        &github,
        &manifest,
        &profile.install_dir,
        modpack_id,
    )
    .await?;

    // --------------------------------------------------------
    // REMOVE OBSOLETE FILES
    // --------------------------------------------------------

    let removed_files = sync::remove_obsolete_files(
        &manifest,
        &profile.install_dir,
    )?;

    // --------------------------------------------------------
    // UPDATE MODPACK VERSION
    // --------------------------------------------------------

    crate::profiles::update_modpack_version(
        &profile.name,
        &manifest.version,
    )?;

    // --------------------------------------------------------
    // COMPLETE CONFIG IMPORT
    // --------------------------------------------------------

    // Only mark the config import as complete after all
    // synchronization steps have succeeded.
    if import_configs {
        sync::mark_config_import_complete(
            &profile.install_dir,
        )?;
    }

    // --------------------------------------------------------
    // COMPLETE INITIAL INSTALLATION
    // --------------------------------------------------------

    sync::mark_initial_install_complete(
        &profile.install_dir,
    )?;

    println!(
        "Modpack {} synchronisiert. {} Dateien heruntergeladen, {} bereits aktuell, {} entfernt.",
        modpack_id,
        downloaded_files,
        up_to_date_files,
        removed_files
    );

    Ok(ModpackSyncResult {
        profile_name: profile.name.clone(),
        modpack_name: manifest.name,
        modpack_version: manifest.version,
        total_files: downloaded_files + up_to_date_files,
        downloaded_files,
        up_to_date_files,
    })
}

// ============================================================
// TAURI COMMAND
// ============================================================

#[tauri::command]
pub(crate) async fn sync_modpack(
    profile_name: String,
) -> Result<ModpackSyncResult, String> {

    let profile = find_profile(&profile_name)
        .ok_or_else(|| {
            format!(
                "Minecraft-Profil nicht gefunden: {}",
                profile_name
            )
        })?;

    synchronize_profile(&profile).await
}
