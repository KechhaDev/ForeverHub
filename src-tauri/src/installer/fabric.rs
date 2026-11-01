
use std::fs;

use crate::profiles::Profile;

use super::{
    download_fabric_library,
    download_fabric_natives,
    get_natives_directory,
};

// ============================================================
// FABRIC INSTALLATION
// ============================================================

pub(crate) async fn install_fabric(
    client: &reqwest::Client,
    profile: &Profile,
) -> Result<(), String> {
    println!(
        "Bereite Fabric {} für Minecraft {} vor...",
        profile.fabric_version,
        profile.minecraft_version
    );

    // --------------------------------------------------------
    // FABRIC METADATA
    // --------------------------------------------------------

    let fabric_url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        profile.minecraft_version,
        profile.fabric_version
    );

    println!("Rufe Fabric-Metadaten ab...");

    let fabric_json = client
        .get(&fabric_url)
        .send()
        .await
        .map_err(|error| {
            format!("Fabric-Anfrage fehlgeschlagen: {}", error)
        })?
        .error_for_status()
        .map_err(|error| {
            format!("Fabric-Server meldet einen Fehler: {}", error)
        })?
        .bytes()
        .await
        .map_err(|error| {
            format!("Fabric-Metadaten konnten nicht gelesen werden: {}", error)
        })?;

    println!("Fabric-Metadaten erfolgreich geladen.");

    let fabric_version: serde_json::Value =
        serde_json::from_slice(&fabric_json)
            .map_err(|error| {
                format!("Fabric-Metadaten sind ungültig: {}", error)
            })?;

    // --------------------------------------------------------
    // VERSION VALIDATION
    // --------------------------------------------------------

    let inherits_from = fabric_version["inheritsFrom"]
        .as_str()
        .ok_or("Fabric-Metadaten enthalten keine inheritsFrom-Angabe")?;

    if inherits_from != profile.minecraft_version {
        return Err(format!(
            "Fabric basiert auf Minecraft {}, erwartet wurde {}",
            inherits_from,
            profile.minecraft_version
        ));
    }

    let fabric_id = fabric_version["id"]
        .as_str()
        .filter(|id| !id.trim().is_empty())
        .ok_or("Fabric-Metadaten enthalten keine gültige Versions-ID")?;

    let libraries = fabric_version["libraries"]
        .as_array()
        .ok_or("Fabric-Metadaten enthalten keine Libraries")?;

    println!(
        "Fabric-Version bestätigt: {} mit {} Libraries.",
        fabric_id,
        libraries.len()
    );

    // --------------------------------------------------------
    // VERSION DIRECTORY
    // --------------------------------------------------------

    let version_dir = profile
        .install_dir
        .join("versions")
        .join(fabric_id);

    fs::create_dir_all(&version_dir)
        .map_err(|error| {
            format!(
                "Fabric-Versionsverzeichnis konnte nicht erstellt werden: {}",
                error
            )
        })?;

    let libraries_dir = profile.install_dir.join("libraries");

    fs::create_dir_all(&libraries_dir)
        .map_err(|error| {
            format!(
                "Library-Verzeichnis konnte nicht erstellt werden: {}",
                error
            )
        })?;

    // --------------------------------------------------------
    // FABRIC LIBRARIES
    // --------------------------------------------------------

    println!(
        "Verarbeite {} Fabric-Libraries...",
        libraries.len()
    );

    let mut library_count = 0;

    for library in libraries {
        download_fabric_library(
            client,
            &libraries_dir,
            library,
        )
        .await
        .map_err(|error| {
            format!(
                "Fabric-Library {} konnte nicht installiert werden: {}",
                library_count + 1,
                error
            )
        })?;

        library_count += 1;
    }

    println!(
        "{} Fabric-Libraries erfolgreich vorbereitet.",
        library_count
    );

    // --------------------------------------------------------
    // FABRIC NATIVES
    // --------------------------------------------------------

    println!("Bereite Fabric-Natives vor...");

    let natives_dir = get_natives_directory(profile);

    for (index, library) in libraries.iter().enumerate() {
        download_fabric_natives(
            client,
            &libraries_dir,
            &natives_dir,
            library,
        )
        .await
        .map_err(|error| {
            format!(
                "Fabric-Natives bei Library {} konnten nicht vorbereitet werden: {}",
                index + 1,
                error
            )
        })?;
    }

    println!("Fabric-Natives erfolgreich vorbereitet.");

    // --------------------------------------------------------
    // FABRIC VERSION JSON
    // --------------------------------------------------------

    println!("Speichere Fabric-Versionsmetadaten...");

    let version_json_path = version_dir.join(format!(
        "{}.json",
        fabric_id
    ));

    fs::write(&version_json_path, &fabric_json)
        .map_err(|error| {
            format!(
                "Fabric-Versionsmetadaten konnten nicht gespeichert werden: {}",
                error
            )
        })?;

    println!(
        "Fabric {} für {} erfolgreich installiert.",
        profile.fabric_version,
        profile.name
    );

    Ok(())
}
