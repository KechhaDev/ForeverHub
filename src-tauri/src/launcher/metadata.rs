
use std::path::Path;

use serde_json::Value;

// ============================================================
// JSON METADATA
// ============================================================

pub(super) fn read_json(path: &Path) -> Result<Value, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|error| {
            format!(
                "Datei konnte nicht gelesen werden ({}): {}",
                path.display(),
                error
            )
        })?;

    serde_json::from_str(&contents)
        .map_err(|error| {
            format!(
                "Ungültige JSON-Datei ({}): {}",
                path.display(),
                error
            )
        })
}

pub(super) fn get_version_id(metadata: &Value) -> Result<String, String> {
    metadata["id"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "Versions-ID fehlt.".to_string())
}

// ============================================================
// FABRIC METADATA
// ============================================================

pub(super) fn find_fabric_metadata(
    profile_dir: &Path,
    minecraft_version: &str,
) -> Result<Value, String> {
    let versions_dir = profile_dir.join("versions");

    for entry in std::fs::read_dir(&versions_dir)
        .map_err(|error| error.to_string())?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let Some(file_name) = path.file_name() else {
            continue;
        };

        let file_name = file_name.to_string_lossy();

        if !file_name.starts_with("fabric-loader-") {
            continue;
        }

        let json_path = path.join(format!("{}.json", file_name));

        if !json_path.exists() {
            continue;
        }

        let metadata = read_json(&json_path)?;

        if metadata["inheritsFrom"].as_str() == Some(minecraft_version) {
            return Ok(metadata);
        }
    }

    Err(format!(
        "Keine passende Fabric-Installation für Minecraft {} gefunden.",
        minecraft_version
    ))
}
