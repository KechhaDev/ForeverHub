
use crate::profiles::Profile;

use super::{
    download_verified,
    AssetIndex,
    AssetIndexInfo,
};

// ============================================================
// ASSET INSTALLATION
// ============================================================

pub(crate) async fn download_assets(
    client: &reqwest::Client,
    profile: &Profile,
    asset_info: &AssetIndexInfo,
) -> Result<(), String> {
    let assets_dir = profile.install_dir.join("assets");

    let indexes_dir = assets_dir.join("indexes");
    let objects_dir = assets_dir.join("objects");

    std::fs::create_dir_all(&indexes_dir)
        .map_err(|error| error.to_string())?;

    std::fs::create_dir_all(&objects_dir)
        .map_err(|error| error.to_string())?;

    // --------------------------------------------------------
    // ASSET INDEX
    // --------------------------------------------------------

    let index_path = indexes_dir.join(format!(
        "{}.json",
        asset_info.id
    ));

    download_verified(
        client,
        &asset_info.url,
        &index_path,
        &asset_info.sha1,
        asset_info.size,
    )
    .await
    .map_err(|error| {
        format!("Asset-Index konnte nicht geladen werden: {}", error)
    })?;

    let index_bytes = std::fs::read(&index_path)
        .map_err(|error| error.to_string())?;

    let asset_index =
        serde_json::from_slice::<AssetIndex>(&index_bytes)
            .map_err(|error| {
                format!("Asset-Index ist ungültig: {}", error)
            })?;

    // --------------------------------------------------------
    // CHECK EXISTING ASSETS
    // --------------------------------------------------------

    let mut missing_assets = Vec::new();

    for asset in asset_index.objects.values() {
        if asset.hash.len() != 40
            || !asset.hash.chars().all(|character| {
                character.is_ascii_hexdigit()
            })
        {
            return Err(format!(
                "Ungültiger Asset-Hash: {}",
                asset.hash
            ));
        }

        let prefix = &asset.hash[..2];

        let asset_path = objects_dir
            .join(prefix)
            .join(&asset.hash);

        let is_valid = std::fs::metadata(&asset_path)
            .map(|metadata| metadata.is_file() && metadata.len() == asset.size)
            .unwrap_or(false);

        if !is_valid {
            missing_assets.push(asset);
        }
    }

    // --------------------------------------------------------
    // DOWNLOAD MISSING ASSETS
    // --------------------------------------------------------

    if missing_assets.is_empty() {
        println!(
            "Alle {} Assets für {} sind bereits vorhanden.",
            asset_index.objects.len(),
            profile.name
        );

        return Ok(());
    }

    println!(
        "Lade {} fehlende Assets von {} für {} herunter...",
        missing_assets.len(),
        asset_index.objects.len(),
        profile.name
    );

    for asset in missing_assets {
        let prefix = &asset.hash[..2];

        let asset_path = objects_dir
            .join(prefix)
            .join(&asset.hash);

        let url = format!(
            "https://resources.download.minecraft.net/{}/{}",
            prefix,
            asset.hash
        );

        download_verified(
            client,
            &url,
            &asset_path,
            &asset.hash,
            asset.size,
        )
        .await
        .map_err(|error| {
            format!(
                "Asset {} konnte nicht geladen werden: {}",
                asset.hash,
                error
            )
        })?;
    }

    println!(
        "Assets für {} erfolgreich vorbereitet.",
        profile.name
    );

    Ok(())
}
