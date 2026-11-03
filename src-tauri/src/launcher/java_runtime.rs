use std::fs::{self, File};
use std::io::Cursor;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::profiles;

// ============================================================
// ADOPTIUM API
// ============================================================

const ADOPTIUM_API: &str =
    "https://api.adoptium.net/v3/assets/latest";

#[derive(Deserialize)]
struct AdoptiumAsset {
    binaries: Vec<AdoptiumBinary>,
}

#[derive(Deserialize)]
struct AdoptiumBinary {
    package: AdoptiumPackage,
}

#[derive(Deserialize)]
struct AdoptiumPackage {
    link: String,
    checksum: String,
}

// ============================================================
// JAVA EXECUTABLE
// ============================================================

fn java_executable_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "java.exe"
    } else {
        "java"
    }
}

// ============================================================
// FIND JAVA HOME
// ============================================================

fn find_java_home(
    directory: &Path,
) -> Option<PathBuf> {
    let direct_executable =
        directory.join("bin").join(java_executable_name());

    if direct_executable.is_file() {
        return Some(directory.to_path_buf());
    }

    for entry in fs::read_dir(directory).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(java_home) = find_java_home(&path) {
                return Some(java_home);
            }
        }
    }

    None
}

// ============================================================
// HASH
// ============================================================

fn calculate_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);

    format!("{:x}", hasher.finalize())
}

// ============================================================
// DOWNLOAD METADATA
// ============================================================

async fn fetch_runtime_package(
    required_major: u32,
) -> Result<AdoptiumPackage, String> {
    let url = format!(
        "{}/{}/hotspot?architecture=x64&heap_size=normal&image_type=jre&jvm_impl=hotspot&os=windows&page_size=1&project=jdk",
        ADOPTIUM_API,
        required_major
    );

    println!(
        "[Java] Suche Temurin Java {}...",
        required_major
    );

    let response = reqwest::Client::new()
        .get(&url)
        .header("User-Agent", "ForeverHub")
        .send()
        .await
        .map_err(|error| {
            format!(
                "Java-Informationen konnten nicht geladen werden: {}",
                error
            )
        })?;

    if !response.status().is_success() {
        return Err(format!(
            "Temurin-API antwortete mit HTTP {}.",
            response.status()
        ));
    }

    let assets = response
        .json::<Vec<AdoptiumAsset>>()
        .await
        .map_err(|error| {
            format!(
                "Java-Informationen sind ungültig: {}",
                error
            )
        })?;

    let package = assets
        .into_iter()
        .next()
        .and_then(|asset| asset.binaries.into_iter().next())
        .map(|binary| binary.package)
        .ok_or_else(|| {
            format!(
                "Für Java {} wurde keine passende Windows-JRE gefunden.",
                required_major
            )
        })?;

    Ok(package)
}

// ============================================================
// DOWNLOAD RUNTIME
// ============================================================

async fn download_runtime(
    package: &AdoptiumPackage,
) -> Result<Vec<u8>, String> {
    println!("[Java] Lade Runtime herunter...");

    let response = reqwest::Client::new()
        .get(&package.link)
        .header("User-Agent", "ForeverHub")
        .send()
        .await
        .map_err(|error| {
            format!(
                "Java-Download fehlgeschlagen: {}",
                error
            )
        })?;

    if !response.status().is_success() {
        return Err(format!(
            "Java-Download antwortete mit HTTP {}.",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|error| {
            format!(
                "Java-Download konnte nicht gelesen werden: {}",
                error
            )
        })?
        .to_vec();

    let actual_hash = calculate_sha256(&bytes);

    if !actual_hash.eq_ignore_ascii_case(&package.checksum) {
        return Err(
            "SHA-256-Prüfung der Java-Runtime fehlgeschlagen.".to_string()
        );
    }

    println!("[Java] SHA-256 erfolgreich geprüft.");

    Ok(bytes)
}

// ============================================================
// EXTRACT RUNTIME
// ============================================================

fn extract_runtime(
    bytes: &[u8],
    temporary_dir: &Path,
) -> Result<(), String> {
    let cursor = Cursor::new(bytes);

    let mut archive = ZipArchive::new(cursor)
        .map_err(|error| {
            format!(
                "Java-Archiv ist ungültig: {}",
                error
            )
        })?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)
            .map_err(|error| {
                format!(
                    "Java-Archiv konnte nicht gelesen werden: {}",
                    error
                )
            })?;

        let Some(relative_path) = entry.enclosed_name() else {
            return Err(
                "Java-Archiv enthält einen ungültigen Dateipfad.".to_string()
            );
        };

        let destination = temporary_dir.join(relative_path);

        if entry.is_dir() {
            fs::create_dir_all(&destination)
                .map_err(|error| error.to_string())?;

            continue;
        }

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| error.to_string())?;
        }

        let mut output = File::create(&destination)
            .map_err(|error| {
                format!(
                    "Java-Datei konnte nicht erstellt werden: {}",
                    error
                )
            })?;

        std::io::copy(&mut entry, &mut output)
            .map_err(|error| {
                format!(
                    "Java-Datei konnte nicht entpackt werden: {}",
                    error
                )
            })?;
    }

    Ok(())
}

// ============================================================
// INSTALL RUNTIME
// ============================================================

async fn install_runtime(
    required_major: u32,
    runtime_dir: &Path,
) -> Result<PathBuf, String> {
    let package = fetch_runtime_package(required_major).await?;
    let bytes = download_runtime(&package).await?;

    let temporary_dir = runtime_dir.join(format!(
        "java-{}.part",
        required_major
    ));

    let installation_dir = runtime_dir.join(format!(
        "java-{}",
        required_major
    ));

    if temporary_dir.exists() {
        fs::remove_dir_all(&temporary_dir)
            .map_err(|error| error.to_string())?;
    }

    fs::create_dir_all(&temporary_dir)
        .map_err(|error| {
            format!(
                "Temporärer Java-Ordner konnte nicht erstellt werden: {}",
                error
            )
        })?;

    let extraction_result =
        extract_runtime(&bytes, &temporary_dir);

    if let Err(error) = extraction_result {
        let _ = fs::remove_dir_all(&temporary_dir);
        return Err(error);
    }

    let java_home = find_java_home(&temporary_dir)
        .ok_or_else(|| {
            "Java-Archiv enthält keine ausführbare Runtime.".to_string()
        })?;

    if installation_dir.exists() {
        fs::remove_dir_all(&installation_dir)
            .map_err(|error| error.to_string())?;
    }

    fs::rename(&java_home, &installation_dir)
        .map_err(|error| {
            format!(
                "Java-Runtime konnte nicht installiert werden: {}",
                error
            )
        })?;

    let _ = fs::remove_dir_all(&temporary_dir);

    println!(
        "[Java] Java {} installiert: {:?}",
        required_major,
        installation_dir
    );

    Ok(
        installation_dir
            .join("bin")
            .join(java_executable_name())
    )
}

// ============================================================
// ENSURE JAVA
// ============================================================

pub(super) async fn ensure_java(
    required_major: u32,
) -> Result<PathBuf, String> {
    let runtime_dir = profiles::get_runtimes_dir()?;

    let installation_dir = runtime_dir.join(format!(
        "java-{}",
        required_major
    ));

    // 1. Check ForeverHub-managed Java.
    if installation_dir.exists() {
        if let Some(java_home) = find_java_home(&installation_dir) {
            let executable = java_home
                .join("bin")
                .join(java_executable_name());

            if super::java::check_java(
                &executable,
                required_major,
            ).is_ok() {
                println!(
                    "[Java] Verwende ForeverHub-Runtime: {:?}",
                    executable
                );

                return Ok(executable);
            }
        }

        println!(
            "[Java] Vorhandene Runtime ist ungültig. Installiere erneut."
        );

        fs::remove_dir_all(&installation_dir)
            .map_err(|error| {
                format!(
                    "Ungültige Java-Runtime konnte nicht entfernt werden: {}",
                    error
                )
            })?;
    }

    // 2. Check system Java.
    if let Ok(java) = super::java::find_system_java(required_major) {
        println!(
            "[Java] Verwende vorhandene System-Runtime: {:?}",
            java
        );

        return Ok(java);
    }

    // 3. Download and install Java.
    println!(
        "[Java] Keine passende Java-Version gefunden. Installiere Java {} automatisch.",
        required_major
    );

    let java = install_runtime(
        required_major,
        &runtime_dir,
    ).await?;

    // 4. Verify installation.
    super::java::check_java(
        &java,
        required_major,
    )?;

    Ok(java)
}