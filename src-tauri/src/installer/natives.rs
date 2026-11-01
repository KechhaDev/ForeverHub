
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use crate::profiles::Profile;

// ============================================================
// NATIVE LIBRARY DIRECTORY
// ============================================================

pub(crate) fn get_natives_directory(profile: &Profile) -> PathBuf {
    profile
        .install_dir
        .join("versions")
        .join(&profile.minecraft_version)
        .join(format!(
            "{}-natives",
            profile.minecraft_version
        ))
}

pub(crate) fn prepare_natives_directory(
    profile: &Profile,
) -> Result<PathBuf, String> {
    let natives_dir = get_natives_directory(profile);

    if natives_dir.exists() {
        std::fs::remove_dir_all(&natives_dir)
            .map_err(|error| {
                format!(
                    "Altes Native-Verzeichnis konnte nicht entfernt werden: {}",
                    error
                )
            })?;
    }

    std::fs::create_dir_all(&natives_dir)
        .map_err(|error| error.to_string())?;

    Ok(natives_dir)
}

// ============================================================
// NATIVE LIBRARY EXTRACTION
// ============================================================

pub(crate) fn extract_native_library(
    archive_path: &Path,
    destination: &Path,
    excludes: &[String],
) -> Result<(), String> {
    let file = File::open(archive_path)
        .map_err(|error| error.to_string())?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| {
            format!(
                "Native-Archiv konnte nicht geöffnet werden: {}",
                error
            )
        })?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| error.to_string())?;

        let Some(relative_path) = entry.enclosed_name() else {
            continue;
        };

        let relative_path = relative_path.to_path_buf();

        let normalized_path = relative_path
            .to_string_lossy()
            .replace('\\', "/");

        let excluded = excludes.iter().any(|excluded_path| {
            let excluded_path = excluded_path
                .trim_matches('/')
                .replace('\\', "/");

            normalized_path == excluded_path
                || normalized_path.starts_with(
                    &format!("{}/", excluded_path)
                )
        });

        if excluded {
            continue;
        }

        let output_path = destination.join(&relative_path);

        if entry.is_dir() {
            std::fs::create_dir_all(&output_path)
                .map_err(|error| error.to_string())?;

            continue;
        }

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| error.to_string())?;
        }

        let mut output_file = File::create(&output_path)
            .map_err(|error| error.to_string())?;

        io::copy(&mut entry, &mut output_file)
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}
