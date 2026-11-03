
use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

// ============================================================
// MODPACK MANIFEST
// ============================================================

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ModpackManifest {
    pub(crate) manifest_version: u32,
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) minecraft_version: String,
    pub(crate) fabric_version: String,

    #[serde(default)]
    pub(crate) default_options: Option<ModpackFile>,

    pub(crate) files: Vec<ModpackFile>,
}

// ============================================================
// MODPACK FILES
// ============================================================

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ModpackFile {
    pub(crate) path: String,
    pub(crate) sha256: String,
    pub(crate) size: u64,
}

// ============================================================
// MANIFEST PARSING
// ============================================================

pub(crate) fn parse_manifest(
    json: &str,
) -> Result<ModpackManifest, String> {
    let manifest = serde_json::from_str::<ModpackManifest>(json)
        .map_err(|error| {
            format!("Modpack-Manifest ist ungültig: {}", error)
        })?;

    validate_manifest(&manifest)?;

    Ok(manifest)
}

// ============================================================
// LOAD MANIFEST FROM FILE
// ============================================================

pub(crate) fn load_manifest(
    path: &Path,
) -> Result<ModpackManifest, String> {
    let json = std::fs::read_to_string(path)
        .map_err(|error| {
            format!(
                "Modpack-Manifest konnte nicht gelesen werden: {}",
                error
            )
        })?;

    parse_manifest(&json)
}

// ============================================================
// MANIFEST VALIDATION
// ============================================================

fn validate_manifest(
    manifest: &ModpackManifest,
) -> Result<(), String> {
    if manifest.manifest_version != 1 {
        return Err(format!(
            "Nicht unterstützte Manifest-Version: {}",
            manifest.manifest_version
        ));
    }

    if manifest.name.trim().is_empty()
        || manifest.version.trim().is_empty()
        || manifest.minecraft_version.trim().is_empty()
        || manifest.fabric_version.trim().is_empty()
    {
        return Err(
            "Das Manifest enthält leere Pflichtfelder.".to_string()
        );
    }

    let mut registered_paths = HashSet::new();

    for file in &manifest.files {
        validate_file(file)?;

        let normalized_path = file.path.to_lowercase();

        if !registered_paths.insert(normalized_path) {
            return Err(format!(
                "Dateipfad ist mehrfach vorhanden: {}",
                file.path
            ));
        }
    }

    // Validate the optional default options file.
    if let Some(default_options) = &manifest.default_options {
        validate_default_options(default_options)?;
    }

    Ok(())
}

// ============================================================
// FILE VALIDATION
// ============================================================

pub(super) fn validate_file(
    file: &ModpackFile,
) -> Result<(), String> {
    let path = file.path.as_str();

    if path.is_empty()
        || path.contains('\\')
        || path.contains(':')
        || path.starts_with('/')
    {
        return Err(format!(
            "Ungültiger Dateipfad: {}",
            path
        ));
    }

    let components: Vec<&str> = path.split('/').collect();

    if components.iter().any(|component| {
        component.is_empty()
            || *component == "."
            || *component == ".."
    }) {
        return Err(format!(
            "Ungültiger Dateipfad: {}",
            path
        ));
    }

    let allowed_directories = [
        "mods",
        "shaderpacks",
        "resourcepacks",
        "config",
    ];

    if !allowed_directories.contains(&components[0]) {
        return Err(format!(
            "Nicht erlaubtes Modpack-Verzeichnis: {}",
            components[0]
        ));
    }

    if components.len() < 2 {
        return Err(format!(
            "Dateipfad enthält keinen Dateinamen: {}",
            path
        ));
    }

    if !is_valid_sha256(&file.sha256) {
        return Err(format!(
            "Ungültiger SHA-256-Hash für: {}",
            path
        ));
    }

    Ok(())
}

// ============================================================
// DEFAULT OPTIONS VALIDATION
// ============================================================

fn validate_default_options(
    file: &ModpackFile,
) -> Result<(), String> {
    if file.path != "options.txt" {
        return Err(
            "Der Pfad für default_options muss options.txt sein."
                .to_string()
        );
    }

    if !is_valid_sha256(&file.sha256) {
        return Err(
            "Ungültiger SHA-256-Hash für default_options."
                .to_string()
        );
    }

    Ok(())
}

// ============================================================
// SHA-256 VALIDATION
// ============================================================

fn is_valid_sha256(hash: &str) -> bool {
    hash.len() == 64
        && hash.chars().all(|character| {
            character.is_ascii_hexdigit()
        })
}
