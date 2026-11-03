
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

    // Create the synchronization plan.
    let plan = sync::create_sync_plan(
        &manifest,
        &profile.install_dir,
    )?;

    let downloaded_files = plan.files_to_download.len();
    let up_to_date_files = plan.files_up_to_date.len();

    // Download missing or outdated files.
    sync::download_files(
        &github,
        &manifest,
        &plan,
        &profile.install_dir,
        modpack_id,
    )
    .await?;

    // Remove obsolete managed files and update inventory.
    let removed_files = sync::remove_obsolete_files(
        &manifest,
        &profile.install_dir,
    )?;

    // Save the successfully installed modpack version.
    crate::profiles::update_modpack_version(
        &profile.name,
        &manifest.version,
    )?;

    // Mark the initial installation as complete only after
    // all synchronization steps have succeeded.
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
