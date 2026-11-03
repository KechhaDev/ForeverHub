
use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::github::GitHubClient;
use super::manifest::{self, ModpackManifest, ModpackFile};

// ============================================================
// TEMPORARY FILE COUNTER
// ============================================================

static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

// ============================================================
// INITIAL INSTALLATION
// ============================================================

fn is_config_file(path: &str) -> bool {
    path.starts_with("config/")
}

fn initial_install_marker(install_dir: &Path) -> PathBuf {
    install_dir
        .join(".foreverhub")
        .join("initial-install-complete")
}

fn is_initial_install_complete(
    install_dir: &Path,
) -> Result<bool, String> {
    let marker = initial_install_marker(install_dir);

    match std::fs::symlink_metadata(marker) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(
            "Der Erstinstallationsmarker ist keine reguläre Datei.".to_string()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(false)
        }
        Err(error) => Err(format!(
            "Installationsstatus konnte nicht überprüft werden: {}",
            error
        )),
    }
}

pub(crate) fn mark_initial_install_complete(
    install_dir: &Path,
) -> Result<(), String> {
    let marker = initial_install_marker(install_dir);

    let parent = marker
        .parent()
        .ok_or_else(|| "Ungültiger Installationspfad.".to_string())?;

    std::fs::create_dir_all(parent).map_err(|error| {
        format!(
            "ForeverHub-Metadatenordner konnte nicht erstellt werden: {}",
            error
        )
    })?;

    std::fs::write(&marker, b"complete").map_err(|error| {
        format!(
            "Erstinstallationsstatus konnte nicht gespeichert werden: {}",
            error
        )
    })
}

// ============================================================
// SYNC PLAN
// ============================================================

pub(crate) struct SyncPlan {
    pub(crate) files_to_download: Vec<String>,
    pub(crate) files_up_to_date: Vec<String>,
}

// ============================================================
// FILE VERIFICATION
// ============================================================

fn verify_file(
    path: &Path,
    expected_sha256: &str,
    expected_size: u64,
) -> Result<bool, String> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(false);
        }
        Err(error) => {
            return Err(format!(
                "Datei konnte nicht überprüft werden: {}",
                error
            ));
        }
    };

    if !metadata.is_file() || metadata.len() != expected_size {
        return Ok(false);
    }

    let mut file = std::fs::File::open(path)
        .map_err(|error| error.to_string())?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    let actual_hash = format!("{:x}", hasher.finalize());

    Ok(actual_hash.eq_ignore_ascii_case(expected_sha256))
}

// ============================================================
// CREATE SYNC PLAN
// ============================================================

pub(crate) fn create_sync_plan(
    manifest: &ModpackManifest,
    install_dir: &Path,
) -> Result<SyncPlan, String> {
    let mut plan = SyncPlan {
        files_to_download: Vec::new(),
        files_up_to_date: Vec::new(),
    };

    let initial_install_complete =
        is_initial_install_complete(install_dir)?;

    for file in &manifest.files {
        manifest::validate_file(file)?;

        let local_path = install_dir.join(&file.path);

        // Config files are only installed during the first
        // successful installation. Existing files are never
        // overwritten, even during the initial installation.
        if is_config_file(&file.path) {
            if initial_install_complete {
                plan.files_up_to_date.push(file.path.clone());
                continue;
            }

            match std::fs::symlink_metadata(&local_path) {
                Ok(_) => {
                    plan.files_up_to_date.push(file.path.clone());
                    continue;
                }
                Err(error)
                    if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(format!(
                        "Config-Datei konnte nicht überprüft werden ({}): {}",
                        file.path,
                        error
                    ));
                }
            }
        }

        let is_up_to_date = verify_file(
            &local_path,
            &file.sha256,
            file.size,
        )?;

        if is_up_to_date {
            plan.files_up_to_date.push(file.path.clone());
        } else {
            plan.files_to_download.push(file.path.clone());
        }
    }

    Ok(plan)
}

// ============================================================
// DOWNLOAD FILES
// ============================================================

pub(crate) async fn download_files(
    github: &GitHubClient,
    manifest: &ModpackManifest,
    plan: &SyncPlan,
    install_dir: &Path,
    modpack_id: &str,
) -> Result<(), String> {
    let paths_to_download: HashSet<&str> = plan
        .files_to_download
        .iter()
        .map(String::as_str)
        .collect();

    for file in &manifest.files {
        if !paths_to_download.contains(file.path.as_str()) {
            continue;
        }

        let destination = install_dir.join(&file.path);

        // Never overwrite an existing config file.
        if is_config_file(&file.path) {
            match std::fs::symlink_metadata(&destination) {
                Ok(_) => continue,
                Err(error)
                    if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(format!(
                        "Config-Datei konnte nicht überprüft werden ({}): {}",
                        file.path,
                        error
                    ));
                }
            }
        }

        manifest::validate_file(file)?;

        download_file(
            github,
            &destination,
            file,
            modpack_id,
        )
        .await?;
    }

    Ok(())
}

// ============================================================
// INSTALL DEFAULT OPTIONS
// ============================================================

pub(crate) async fn install_default_options(
    github: &GitHubClient,
    manifest: &ModpackManifest,
    install_dir: &Path,
    modpack_id: &str,
) -> Result<bool, String> {
    let Some(options) = &manifest.default_options else {
        return Ok(false);
    };

    let destination = install_dir.join("options.txt");

    // Never overwrite an existing file or filesystem entry.
    match std::fs::symlink_metadata(&destination) {
        Ok(_) => {
            println!(
                "[Modpack] Vorhandene options.txt bleibt unverändert."
            );
            return Ok(false);
        }
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "options.txt konnte nicht überprüft werden: {}",
                error
            ));
        }
    }

    // Download the default options file.
    let repository_path = format!(
        "{}/{}",
        modpack_id,
        options.path
    );

    println!(
        "[GitHub] Downloading default options: {}",
        repository_path
    );

    let bytes = github
        .download_file(&repository_path)
        .await?;

    // Verify file size.
    if bytes.len() as u64 != options.size {
        return Err(
            "Dateigröße der Standard-options.txt stimmt nicht überein."
                .to_string()
        );
    }

    // Verify SHA-256.
    let actual_hash = format!(
        "{:x}",
        Sha256::digest(&bytes)
    );

    if !actual_hash.eq_ignore_ascii_case(&options.sha256) {
        return Err(
            "SHA-256-Prüfung der Standard-options.txt fehlgeschlagen."
                .to_string()
        );
    }

    // Create the file exclusively.
    // An existing file can never be overwritten.
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
    {
        Ok(file) => file,
        Err(error)
            if error.kind() == std::io::ErrorKind::AlreadyExists =>
        {
            return Ok(false);
        }
        Err(error) => {
            return Err(format!(
                "Standard-options.txt konnte nicht angelegt werden: {}",
                error
            ));
        }
    };

    if let Err(error) = file.write_all(&bytes) {
        drop(file);
        let _ = std::fs::remove_file(&destination);

        return Err(format!(
            "Standard-options.txt konnte nicht geschrieben werden: {}",
            error
        ));
    }

    println!(
        "[Modpack] Standard-options.txt erfolgreich installiert."
    );

    Ok(true)
}

// ============================================================
// DOWNLOAD SINGLE FILE
// ============================================================

async fn download_file(
    github: &GitHubClient,
    destination: &Path,
    file: &ModpackFile,
    modpack_id: &str,
) -> Result<(), String> {
    // Build the complete path inside the GitHub repository.
    let repository_path = format!(
        "{}/{}",
        modpack_id,
        file.path
    );

    println!(
        "[GitHub] Downloading modpack file: {}",
        repository_path
    );

    let bytes = github
        .download_file(&repository_path)
        .await?;

    // Verify file size.
    if bytes.len() as u64 != file.size {
        return Err(format!(
            "Dateigröße stimmt nicht überein: {}",
            file.path
        ));
    }

    // Verify SHA-256.
    let actual_hash = format!(
        "{:x}",
        Sha256::digest(&bytes)
    );

    if !actual_hash.eq_ignore_ascii_case(&file.sha256) {
        return Err(format!(
            "SHA-256-Prüfung fehlgeschlagen: {}",
            file.path
        ));
    }

    install_downloaded_file(destination, &bytes)?;

    Ok(())
}

// ============================================================
// INSTALL DOWNLOADED FILE
// ============================================================

fn install_downloaded_file(
    destination: &Path,
    bytes: &[u8],
) -> Result<(), String> {
    let parent = destination
        .parent()
        .ok_or_else(|| {
            "Ungültiger Installationspfad.".to_string()
        })?;

    std::fs::create_dir_all(parent)
        .map_err(|error| {
            format!(
                "Installationsverzeichnis konnte nicht erstellt werden: {}",
                error
            )
        })?;

    let file_name = destination
        .file_name()
        .ok_or_else(|| {
            "Ungültiger Dateiname.".to_string()
        })?;

    let id = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let suffix = format!("{}.{}", std::process::id(), id);

    let mut temporary_name = file_name.to_os_string();
    temporary_name.push(format!(".{}.part", suffix));

    let mut backup_name = file_name.to_os_string();
    backup_name.push(format!(".{}.backup", suffix));

    let temporary_path: PathBuf = parent.join(temporary_name);
    let backup_path: PathBuf = parent.join(backup_name);

    // Write the complete file before replacing the old one.
    std::fs::write(&temporary_path, bytes)
        .map_err(|error| {
            format!(
                "Temporäre Datei konnte nicht geschrieben werden: {}",
                error
            )
        })?;

    let had_existing_file = destination.exists();

    if had_existing_file {
        if let Err(error) = std::fs::rename(
            destination,
            &backup_path,
        ) {
            let _ = std::fs::remove_file(&temporary_path);

            return Err(format!(
                "Vorhandene Datei konnte nicht gesichert werden: {}",
                error
            ));
        }
    }

    if let Err(error) = std::fs::rename(
        &temporary_path,
        destination,
    ) {
        if had_existing_file {
            let _ = std::fs::rename(
                &backup_path,
                destination,
            );
        }

        let _ = std::fs::remove_file(&temporary_path);

        return Err(format!(
            "Neue Datei konnte nicht installiert werden: {}",
            error
        ));
    }

    if had_existing_file {
        let _ = std::fs::remove_file(&backup_path);
    }

    Ok(())
}

// ============================================================
// MANAGED FILE INVENTORY
// ============================================================

#[derive(Serialize, Deserialize)]
struct ManagedFiles {
    files: Vec<String>,
}

// ============================================================
// VALIDATE MANAGED PATH
// ============================================================

fn validate_managed_path(path: &str) -> Result<(), String> {
    let parts: Vec<&str> = path.split('/').collect();

    if path.contains('\\')
        || path.contains(':')
        || parts.len() < 2
        || parts.iter().any(|part| {
            part.is_empty() || *part == "." || *part == ".."
        })
    {
        return Err(format!(
            "Ungültiger verwalteter Dateipfad: {}",
            path
        ));
    }

    match parts[0] {
        "mods" | "shaderpacks" | "resourcepacks" | "config" => Ok(()),
        _ => Err(format!(
            "Nicht erlaubter verwalteter Dateipfad: {}",
            path
        )),
    }
}

// ============================================================
// REMOVE OBSOLETE FILES
// ============================================================

pub(crate) fn remove_obsolete_files(
    manifest: &ModpackManifest,
    install_dir: &Path,
) -> Result<usize, String> {
    let metadata_dir = install_dir.join(".foreverhub");
    let inventory_path = metadata_dir.join("managed-files.json");

    // Load previously managed files.
    let previous_files: Vec<String> =
        match std::fs::read_to_string(&inventory_path) {
            Ok(json) => {
                let inventory: ManagedFiles =
                    serde_json::from_str(&json).map_err(|error| {
                        format!(
                            "Dateiverwaltung konnte nicht gelesen werden: {}",
                            error
                        )
                    })?;

                inventory.files
            }

            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                Vec::new()
            }

            Err(error) => {
                return Err(format!(
                    "Dateiverwaltung konnte nicht geöffnet werden: {}",
                    error
                ));
            }
        };

    // Build the current manifest file list.
    let current_files: HashSet<String> = manifest.files
        .iter()
        .map(|file| file.path.to_ascii_lowercase())
        .collect();

    let mut removed_files = 0;

    // Remove files that were previously managed but are no
    // longer part of the current manifest.
    for old_path in &previous_files {
        validate_managed_path(old_path)?;

        // Config files are user-managed after installation.
        // Never automatically delete them.
        if is_config_file(old_path) {
            continue;
        }

        if current_files.contains(&old_path.to_ascii_lowercase()) {
            continue;
        }

        let file_path = install_dir.join(old_path);

        match std::fs::symlink_metadata(&file_path) {
            Ok(metadata) if metadata.file_type().is_file() => {
                std::fs::remove_file(&file_path).map_err(|error| {
                    format!(
                        "Veraltete Modpack-Datei konnte nicht gelöscht werden ({}): {}",
                        old_path,
                        error
                    )
                })?;

                removed_files += 1;
            }

            Ok(_) => {
                return Err(format!(
                    "Verwalteter Pfad ist keine reguläre Datei: {}",
                    old_path
                ));
            }

            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                // File was already removed.
            }

            Err(error) => {
                return Err(format!(
                    "Datei konnte nicht überprüft werden ({}): {}",
                    old_path,
                    error
                ));
            }
        }
    }

    // Store the current inventory after successful cleanup.
    std::fs::create_dir_all(&metadata_dir).map_err(|error| {
        format!(
            "ForeverHub-Metadatenordner konnte nicht erstellt werden: {}",
            error
        )
    })?;

    let mut managed_paths: Vec<String> = manifest.files
        .iter()
        .map(|file| file.path.clone())
        .collect();

    managed_paths.sort();

    let inventory = ManagedFiles {
        files: managed_paths,
    };

    let json = serde_json::to_string_pretty(&inventory)
        .map_err(|error| error.to_string())?;

    std::fs::write(&inventory_path, json).map_err(|error| {
        format!(
            "Dateiverwaltung konnte nicht gespeichert werden: {}",
            error
        )
    })?;

    Ok(removed_files)
}
