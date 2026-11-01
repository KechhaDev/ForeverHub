
use std::path::Path;

use crate::profiles::Profile;

use super::{
    download_verified,
    extract_native_library,
    get_natives_directory,
    maven_path,
    safe_relative_path,
};

use super::metadata::{
    Library,
    LibraryRule,
    MinecraftVersionDetails,
};

// ============================================================
// OPERATING SYSTEM DETECTION
// ============================================================

fn current_os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "unknown"
    }
}

fn current_architecture() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "64",
        "x86" => "32",
        "aarch64" => "64",
        _ => "64",
    }
}

fn library_rule_matches(rule: &LibraryRule) -> bool {
    let Some(os) = &rule.os else {
        return true;
    };

    if let Some(name) = &os.name {
        if name != current_os_name() {
            return false;
        }
    }

    if let Some(arch) = &os.arch {
        let current_arch = std::env::consts::ARCH;

        if arch != current_arch {
            return false;
        }
    }

    true
}

fn library_is_allowed(library: &Library) -> bool {
    let Some(rules) = &library.rules else {
        return true;
    };

    let mut allowed = false;

    for rule in rules {
        if library_rule_matches(rule) {
            allowed = rule.action == "allow";
        }
    }

    allowed
}

// ============================================================
// MINECRAFT LIBRARY INSTALLATION
// ============================================================

pub(crate) async fn download_libraries(
    client: &reqwest::Client,
    profile: &Profile,
    details: &MinecraftVersionDetails,
) -> Result<(), String> {
    let libraries_dir = profile.install_dir.join("libraries");
    let natives_dir = get_natives_directory(profile);

    std::fs::create_dir_all(&libraries_dir)
        .map_err(|error| error.to_string())?;

    std::fs::create_dir_all(&natives_dir)
        .map_err(|error| error.to_string())?;

    for library in &details.libraries {
        if !library_is_allowed(library) {
            continue;
        }

        let Some(downloads) = &library.downloads else {
            continue;
        };

        if let Some(artifact) = &downloads.artifact {
            let relative_path =
                safe_relative_path(&artifact.path)?;

            let destination =
                libraries_dir.join(relative_path);

            download_verified(
                client,
                &artifact.url,
                &destination,
                &artifact.sha1,
                artifact.size,
            )
            .await?;
        }

        let Some(natives) = &library.natives else {
            continue;
        };

        let Some(native_template) =
            natives.get(current_os_name())
        else {
            continue;
        };

        let classifier = native_template.replace(
            "${arch}",
            current_architecture(),
        );

        let Some(native_artifact) =
            downloads.classifiers.get(&classifier)
        else {
            continue;
        };

        let relative_path =
            safe_relative_path(&native_artifact.path)?;

        let native_jar =
            libraries_dir.join(relative_path);

        download_verified(
            client,
            &native_artifact.url,
            &native_jar,
            &native_artifact.sha1,
            native_artifact.size,
        )
        .await?;

        let excludes = library
            .extract
            .as_ref()
            .map(|extract| extract.exclude.as_slice())
            .unwrap_or(&[]);

        println!(
            "Entpacke Minecraft-Native: {}",
            native_artifact.path
        );

        extract_native_library(
            &native_jar,
            &natives_dir,
            excludes,
        )?;
    }

    println!(
        "Minecraft-Libraries für {} vorbereitet.",
        profile.name
    );

    Ok(())
}

// ============================================================
// FABRIC LIBRARY DOWNLOADS
// ============================================================

pub(crate) async fn download_fabric_library(
    client: &reqwest::Client,
    libraries_dir: &Path,
    library: &serde_json::Value,
) -> Result<(), String> {
    let coordinate = library["name"]
        .as_str()
        .ok_or("Fabric-Library ohne Namen")?;

    let relative_path = maven_path(coordinate)?;

    let destination = libraries_dir.join(&relative_path);

    if destination.exists() {
        let metadata = std::fs::metadata(&destination)
            .map_err(|error| error.to_string())?;

        if metadata.len() > 0 {
            return Ok(());
        }
    }

    let base_url = library["url"]
        .as_str()
        .unwrap_or("https://libraries.minecraft.net/");

    let url = format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        relative_path.to_string_lossy().replace('\\', "/")
    );

    println!("Fabric-Library: {}", coordinate);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Err(format!(
            "Fabric-Library konnte nicht heruntergeladen werden: {} (HTTP {})",
            coordinate,
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|error| error.to_string())?;

    if bytes.is_empty() {
        return Err(format!(
            "Fabric-Library ist leer: {}",
            coordinate
        ));
    }

    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| error.to_string())?;
    }

    let temporary_path = destination.with_extension("part");

    std::fs::write(&temporary_path, &bytes)
        .map_err(|error| error.to_string())?;

    if destination.exists() {
        std::fs::remove_file(&destination)
            .map_err(|error| error.to_string())?;
    }

    std::fs::rename(&temporary_path, &destination)
        .map_err(|error| error.to_string())?;

    Ok(())
}

// ============================================================
// FABRIC NATIVE LIBRARIES
// ============================================================

pub(crate) async fn download_fabric_natives(
    client: &reqwest::Client,
    libraries_dir: &Path,
    natives_dir: &Path,
    library: &serde_json::Value,
) -> Result<(), String> {
    let Some(natives) = library["natives"].as_object() else {
        return Ok(());
    };

    let Some(native_template) =
        natives.get(current_os_name()).and_then(|value| value.as_str())
    else {
        return Ok(());
    };

    let classifier = native_template.replace(
        "${arch}",
        current_architecture(),
    );

    let Some(artifact) = library["downloads"]["classifiers"]
        [&classifier]
        .as_object()
    else {
        return Ok(());
    };

    let path = artifact["path"]
        .as_str()
        .ok_or("Fabric-Native ohne Pfad")?;

    let url = artifact["url"]
        .as_str()
        .ok_or("Fabric-Native ohne Download-URL")?;

    let sha1 = artifact["sha1"]
        .as_str()
        .ok_or("Fabric-Native ohne SHA-1")?;

    let size = artifact["size"]
        .as_u64()
        .ok_or("Fabric-Native ohne Dateigröße")?;

    let relative_path = safe_relative_path(path)?;
    let native_jar = libraries_dir.join(relative_path);

    download_verified(
        client,
        url,
        &native_jar,
        sha1,
        size,
    )
    .await?;

    let excludes: Vec<String> = library["extract"]["exclude"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    println!("Entpacke Fabric-Native: {}", path);

    extract_native_library(
        &native_jar,
        natives_dir,
        &excludes,
    )?;

    Ok(())
}
