
use std::fs;

use crate::profiles::Profile;

use super::{
    download_assets,
    download_libraries,
    download_verified,
    fetch_json,
    find_minecraft_version,
    MinecraftVersionDetails,
};

// ============================================================
// MINECRAFT INSTALLATION
// ============================================================

pub(crate) async fn install_minecraft(
    client: &reqwest::Client,
    profile: &Profile,
) -> Result<(), String> {
    println!(
        "Bereite Minecraft {} für {} vor...",
        profile.minecraft_version,
        profile.name
    );

    // --------------------------------------------------------
    // VERSION METADATA
    // --------------------------------------------------------

    let version = find_minecraft_version(
        client,
        &profile.minecraft_version,
    )
    .await
    .map_err(|error| {
        format!(
            "Minecraft-Version {} konnte nicht gefunden werden: {}",
            profile.minecraft_version,
            error
        )
    })?;

    let (details, metadata_bytes) =
        fetch_json::<MinecraftVersionDetails>(
            client,
            &version.url,
        )
        .await
        .map_err(|error| {
            format!(
                "Minecraft-Metadaten konnten nicht geladen werden: {}",
                error
            )
        })?;

    if details.id != profile.minecraft_version {
        return Err(format!(
            "Versions-ID stimmt nicht überein: erwartet {}, erhalten {}",
            profile.minecraft_version,
            details.id
        ));
    }

    // --------------------------------------------------------
    // VERSION DIRECTORY
    // --------------------------------------------------------

    let version_dir = profile
        .install_dir
        .join("versions")
        .join(&profile.minecraft_version);

    fs::create_dir_all(&version_dir)
        .map_err(|error| {
            format!(
                "Versionsverzeichnis konnte nicht erstellt werden: {}",
                error
            )
        })?;

    // --------------------------------------------------------
    // VERSION JSON
    // --------------------------------------------------------

    let version_json_path = version_dir.join(format!(
        "{}.json",
        profile.minecraft_version
    ));

    fs::write(&version_json_path, metadata_bytes)
        .map_err(|error| {
            format!(
                "Minecraft-Versionsmetadaten konnten nicht gespeichert werden: {}",
                error
            )
        })?;

    // --------------------------------------------------------
    // CLIENT JAR
    // --------------------------------------------------------

    let client_download = &details.downloads.client;

    if client_download.url.is_empty()
        || client_download.sha1.is_empty()
        || client_download.size == 0
    {
        return Err(format!(
            "Ungültige Client-Download-Metadaten für Minecraft {}",
            profile.minecraft_version
        ));
    }

    let client_jar_path = version_dir.join(format!(
        "{}.jar",
        profile.minecraft_version
    ));

    download_verified(
        client,
        &client_download.url,
        &client_jar_path,
        &client_download.sha1,
        client_download.size,
    )
    .await
    .map_err(|error| {
        format!(
            "Minecraft-Client konnte nicht heruntergeladen werden: {}",
            error
        )
    })?;

    // --------------------------------------------------------
    // LIBRARIES
    // --------------------------------------------------------

    download_libraries(
        client,
        profile,
        &details,
    )
    .await
    .map_err(|error| {
        format!(
            "Minecraft-Bibliotheken konnten nicht installiert werden: {}",
            error
        )
    })?;

    // --------------------------------------------------------
    // ASSETS
    // --------------------------------------------------------

    download_assets(
        client,
        profile,
        &details.asset_index,
    )
    .await
    .map_err(|error| {
        format!(
            "Minecraft-Assets konnten nicht installiert werden: {}",
            error
        )
    })?;

    println!(
        "Minecraft {} für {} erfolgreich installiert.",
        profile.minecraft_version,
        profile.name
    );

    Ok(())
}
