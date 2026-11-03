
use serde::Serialize;

use crate::profiles::{
    self,
    AuthenticationMode,
};

// ============================================================
// PROFILE RESPONSE
// ============================================================

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProfileInfo {
    name: String,
    minecraft_version: String,
    fabric_version: String,
    modpack_version: String,
    dev_mode: bool,
}

impl From<profiles::Profile> for ProfileInfo {
    fn from(profile: profiles::Profile) -> Self {
        Self {
            name: profile.name,
            minecraft_version: profile.minecraft_version,
            fabric_version: if profile.fabric_version == "/" {
                "Nicht installiert".to_string()
            } else {
                profile.fabric_version
            },
            modpack_version: profile.modpack_version,
            dev_mode: matches!(
                profile.authentication_mode,
                AuthenticationMode::Offline
            ),
        }
    }
}

// ============================================================
// GET PROFILES
// ============================================================

#[tauri::command]
pub(crate) fn get_profiles() -> Result<Vec<ProfileInfo>, String> {
    let profiles = profiles::list_profiles()
        .ok_or_else(|| {
            "Minecraft-Profile konnten nicht geladen werden.".to_string()
        })?;

    Ok(profiles.into_iter().map(ProfileInfo::from).collect())
}

// ============================================================
// REFRESH PROFILE METADATA
// ============================================================

#[tauri::command]
pub(crate) async fn refresh_profile_metadata() -> Result<(), String> {
    let profiles = profiles::list_profiles()
        .ok_or_else(|| {
            "Minecraft-Profile konnten nicht geladen werden.".to_string()
        })?;

    let client = match crate::modpack::github::GitHubClient::new() {
        Ok(client) => client,
        Err(error) => {
            log::warn!(
                "[Profiles] GitHub-Client konnte nicht erstellt werden: {}",
                error
            );
            return Ok(());
        }
    };

    for profile in profiles {
        let Some(modpack_id) = profile.modpack_id.as_deref() else {
            continue;
        };

        match client.fetch_manifest(modpack_id).await {
            Ok(manifest) => {
                if let Err(error) = profiles::update_profile_metadata(
                    &profile.name,
                    &manifest.minecraft_version,
                    &manifest.fabric_version,
                    &manifest.version,
                ) {
                    log::warn!(
                        "[Profiles] Metadaten für {} konnten nicht gespeichert werden: {}",
                        profile.name,
                        error
                    );
                } else {
                    log::info!(
                        "[Profiles] Metadaten für {} aktualisiert: MC {}, Fabric {}, Modpack {}",
                        profile.name,
                        manifest.minecraft_version,
                        manifest.fabric_version,
                        manifest.version
                    );
                }
            }

            Err(error) => {
                log::warn!(
                    "[Profiles] Manifest für {} konnte nicht geladen werden: {}",
                    profile.name,
                    error
                );
            }
        }
    }

    Ok(())
}
