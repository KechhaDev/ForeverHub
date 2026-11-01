
use std::collections::HashMap;

// ============================================================
// MINECRAFT VERSION METADATA
// ============================================================

#[derive(serde::Deserialize)]
pub(crate) struct VersionManifest {
    pub(crate) versions: Vec<VersionInfo>,
}

#[derive(serde::Deserialize)]
pub(crate) struct VersionInfo {
    pub(crate) id: String,
    pub(crate) url: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct MinecraftVersionDetails {
    pub(crate) id: String,
    pub(crate) downloads: VersionDownloads,

    #[serde(default)]
    pub(crate) libraries: Vec<Library>,

    #[serde(rename = "assetIndex")]
    pub(crate) asset_index: AssetIndexInfo,
}

#[derive(serde::Deserialize)]
pub(crate) struct VersionDownloads {
    pub(crate) client: DownloadInfo,
}

#[derive(serde::Deserialize)]
pub(crate) struct DownloadInfo {
    pub(crate) url: String,
    pub(crate) sha1: String,
    pub(crate) size: u64,
}

// ============================================================
// LIBRARY METADATA
// ============================================================

#[derive(serde::Deserialize)]
pub(crate) struct Library {
    #[serde(default)]
    pub(crate) downloads: Option<LibraryDownloads>,

    #[serde(default)]
    pub(crate) rules: Option<Vec<LibraryRule>>,

    #[serde(default)]
    pub(crate) natives: Option<HashMap<String, String>>,

    #[serde(default)]
    pub(crate) extract: Option<LibraryExtract>,
}

#[derive(serde::Deserialize)]
pub(crate) struct LibraryExtract {
    #[serde(default)]
    pub(crate) exclude: Vec<String>,
}

#[derive(serde::Deserialize)]
pub(crate) struct LibraryDownloads {
    pub(crate) artifact: Option<LibraryArtifact>,

    #[serde(default)]
    pub(crate) classifiers: HashMap<String, LibraryArtifact>,
}

#[derive(serde::Deserialize)]
pub(crate) struct LibraryArtifact {
    pub(crate) path: String,
    pub(crate) url: String,
    pub(crate) sha1: String,
    pub(crate) size: u64,
}

#[derive(serde::Deserialize)]
pub(crate) struct LibraryRule {
    pub(crate) action: String,

    #[serde(default)]
    pub(crate) os: Option<LibraryRuleOs>,
}

#[derive(serde::Deserialize)]
pub(crate) struct LibraryRuleOs {
    pub(crate) name: Option<String>,
    pub(crate) arch: Option<String>,
}

// ============================================================
// ASSET METADATA
// ============================================================

#[derive(serde::Deserialize)]
pub(crate) struct AssetIndexInfo {
    pub(crate) id: String,
    pub(crate) url: String,
    pub(crate) sha1: String,
    pub(crate) size: u64,
}

#[derive(serde::Deserialize)]
pub(crate) struct AssetIndex {
    pub(crate) objects: HashMap<String, AssetObject>,
}

#[derive(serde::Deserialize)]
pub(crate) struct AssetObject {
    pub(crate) hash: String,
    pub(crate) size: u64,
}

// ============================================================
// JSON FETCHING
// ============================================================

pub(crate) async fn fetch_json<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
) -> Result<(T, Vec<u8>), String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Err(format!(
            "HTTP-Fehler: {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|error| error.to_string())?
        .to_vec();

    let parsed = serde_json::from_slice::<T>(&bytes)
        .map_err(|error| error.to_string())?;

    Ok((parsed, bytes))
}

// ============================================================
// MINECRAFT VERSION LOOKUP
// ============================================================

pub(crate) async fn find_minecraft_version(
    client: &reqwest::Client,
    version_id: &str,
) -> Result<VersionInfo, String> {
    let (manifest, _) = fetch_json::<VersionManifest>(
        client,
        "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json",
    )
    .await?;

    manifest
        .versions
        .into_iter()
        .find(|version| version.id == version_id)
        .ok_or_else(|| {
            format!(
                "Minecraft-Version {} nicht gefunden.",
                version_id
            )
        })
}
