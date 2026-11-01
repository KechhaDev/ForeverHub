
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::metadata::get_version_id;

// ============================================================
// MAVEN LIBRARY PATHS
// ============================================================

fn safe_relative_path(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);

    if path.is_absolute() {
        return Err("Absoluter Library-Pfad ist nicht erlaubt.".to_string());
    }

    for component in path.components() {
        match component {
            std::path::Component::Normal(_) => {}
            _ => {
                return Err("Ungültiger Library-Pfad.".to_string());
            }
        }
    }

    Ok(path.to_path_buf())
}

fn maven_path(coordinate: &str) -> Result<PathBuf, String> {
    let (coordinate, extension) = match coordinate.split_once('@') {
        Some((coordinate, extension)) => (coordinate, extension),
        None => (coordinate, "jar"),
    };

    let parts: Vec<&str> = coordinate.split(':').collect();

    if parts.len() < 3 || parts.len() > 4 {
        return Err(format!(
            "Ungültige Library-Koordinate: {}",
            coordinate
        ));
    }

    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];

    let classifier = if parts.len() == 4 {
        format!("-{}", parts[3])
    } else {
        String::new()
    };

    let filename = format!(
        "{}-{}{}.{}",
        artifact,
        version,
        classifier,
        extension
    );

    safe_relative_path(
        &format!(
            "{}/{}/{}/{}",
            group,
            artifact,
            version,
            filename
        )
    )
}

fn get_library_path(
    library: &Value,
) -> Result<Option<PathBuf>, String> {
    // Normal artifact supplied by Mojang or Fabric.
    if let Some(path) = library["downloads"]["artifact"]["path"].as_str() {
        return Ok(Some(safe_relative_path(path)?));
    }

    // Native-only libraries do not belong in the Java classpath.
    if library["downloads"]["classifiers"].is_object() {
        return Ok(None);
    }

    let name = library["name"]
        .as_str()
        .ok_or("Library ohne Namen.")?;

    Ok(Some(maven_path(name)?))
}

// ============================================================
// OPERATING SYSTEM
// ============================================================

fn current_os() -> &'static str {
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

// ============================================================
// LAUNCHER FEATURES
// ============================================================

    pub(super) fn launcher_features() -> HashMap<String, bool> {
        let mut features = HashMap::new();

        features.insert("has_custom_resolution".to_string(), false);
        features.insert("is_demo_user".to_string(), false);
        features.insert("has_quick_plays_support".to_string(), false);
        features.insert("is_quick_play_singleplayer".to_string(), false);
        features.insert("is_quick_play_multiplayer".to_string(), false);
        features.insert("is_quick_play_realms".to_string(), false);

        features
    }

// ============================================================
// RULE EVALUATION
// ============================================================

fn rule_matches(
    rule: &Value,
    features: &HashMap<String, bool>,
) -> bool {
    if let Some(os) = rule.get("os") {
        if let Some(name) = os["name"].as_str() {
            if name != current_os() {
                return false;
            }
        }

        if let Some(arch) = os["arch"].as_str() {
            let current_arch = std::env::consts::ARCH;

            if arch == "x86" && current_arch != "x86" {
                return false;
            }

            if arch == "x86_64" && current_arch != "x86_64" {
                return false;
            }

            if arch == "arm64" && current_arch != "aarch64" {
                return false;
            }
        }
    }

    if let Some(rule_features) = rule["features"].as_object() {
        for (name, expected) in rule_features {
            let expected = match expected.as_bool() {
                Some(value) => value,
                None => return false,
            };

            let actual = features
                .get(name)
                .copied()
                .unwrap_or(false);

            if actual != expected {
                return false;
            }
        }
    }

    true
}

pub(super) fn rules_allow(
    rules: &[Value],
    features: &HashMap<String, bool>,
) -> bool {
    let mut allowed = false;

    for rule in rules {
        if !rule_matches(rule, features) {
            continue;
        }

        allowed = match rule["action"].as_str() {
            Some("allow") => true,
            Some("disallow") => false,
            _ => false,
        };
    }

    allowed
}

fn library_allowed(library: &Value) -> bool {
    let Some(rules) = library["rules"].as_array() else {
        return true;
    };

    rules_allow(rules, &launcher_features())
}

// ============================================================
// CLASSPATH
// ============================================================

pub(super) fn build_classpath(
    base_metadata: &Value,
    fabric_metadata: Option<&Value>,
    profile_dir: &Path,
) -> Result<String, String> {
    let libraries_dir = profile_dir.join("libraries");

    let mut paths: Vec<PathBuf> = Vec::new();

    let metadata_list = match fabric_metadata {
        Some(fabric) => vec![base_metadata, fabric],
        None => vec![base_metadata],
    };

    for metadata in metadata_list {
        let Some(libraries) = metadata["libraries"].as_array() else {
            continue;
        };

        for library in libraries {
            if !library_allowed(library) {
                continue;
            }

            let Some(relative_path) = get_library_path(library)? else {
                continue;
            };

            let full_path = libraries_dir.join(relative_path);

            if !full_path.exists() {
                return Err(format!(
                    "Library fehlt: {}",
                    full_path.display()
                ));
            }

            let canonical_path = std::fs::canonicalize(&full_path)
                .map_err(|error| error.to_string())?;

            if !paths.contains(&canonical_path) {
                paths.push(canonical_path);
            }
        }
    }

    // Minecraft client JAR.
    let base_id = get_version_id(base_metadata)?;

    let client_jar = profile_dir
        .join("versions")
        .join(&base_id)
        .join(format!("{}.jar", base_id));

    if !client_jar.exists() {
        return Err(format!(
            "Minecraft-Client-JAR fehlt: {}",
            client_jar.display()
        ));
    }

    paths.push(
        std::fs::canonicalize(client_jar)
            .map_err(|error| error.to_string())?
    );

    Ok(std::env::join_paths(paths)
        .map_err(|error| error.to_string())?
        .to_string_lossy()
        .to_string())
}
