
use crate::profiles::Profile;

use super::{
    install_fabric,
    install_minecraft,
    prepare_natives_directory,
};

// ============================================================
// INSTALLATION PREPARATION
// ============================================================

pub(crate) async fn prepare_installation(
    profile: &Profile,
) -> Result<(), String> {
    println!(
        "Bereite Installation für {} vor...",
        profile.name
    );

    let client = reqwest::Client::new();

    prepare_natives_directory(profile)
        .map_err(|error| {
            format!("Native-Dateien konnten nicht vorbereitet werden: {}", error)
        })?;

    install_minecraft(&client, profile)
        .await
        .map_err(|error| {
            format!("Minecraft-Installation fehlgeschlagen: {}", error)
        })?;

    if has_fabric(profile) {
        install_fabric(&client, profile)
            .await
            .map_err(|error| {
                format!("Fabric-Installation fehlgeschlagen: {}", error)
            })?;
    }

    println!(
        "Installation für {} erfolgreich vorbereitet.",
        profile.name
    );

    Ok(())
}

// ============================================================
// INSTALLATION CONFIGURATION
// ============================================================

fn has_fabric(profile: &Profile) -> bool {
    !profile.fabric_version.trim().is_empty()
        && profile.fabric_version != "/"
}
