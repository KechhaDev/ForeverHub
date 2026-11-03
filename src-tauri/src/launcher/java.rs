use std::path::{Path, PathBuf};
use std::process::Command;

// ============================================================
// JAVA EXECUTABLE
// ============================================================

fn get_java_executable() -> &'static str {
    if cfg!(target_os = "windows") {
        "java.exe"
    } else {
        "java"
    }
}

// ============================================================
// JAVA VERSION
// ============================================================

fn get_java_version(java_path: &Path) -> Result<u32, String> {
    let output = Command::new(java_path)
        .arg("-version")
        .output()
        .map_err(|error| {
            format!("Java konnte nicht ausgeführt werden: {}", error)
        })?;

    if !output.status.success() {
        return Err(
            "Java-Version konnte nicht geprüft werden.".to_string()
        );
    }

    let version_output = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let version_string = version_output
        .split('"')
        .nth(1)
        .or_else(|| {
            version_output
                .split_whitespace()
                .find(|part| {
                    part.chars()
                        .next()
                        .is_some_and(|character| character.is_ascii_digit())
                })
        })
        .ok_or("Java-Version konnte nicht erkannt werden.")?;

    let mut parts = version_string.split('.');

    let first_part = parts
        .next()
        .ok_or("Ungültige Java-Versionsnummer.")?;

    let major = first_part
        .parse::<u32>()
        .map_err(|_| "Java-Versionsnummer ist ungültig.")?;

    // Java 8 uses the old version format 1.8.x.
    if major == 1 {
        let second_part = parts
            .next()
            .ok_or("Ungültige Java-8-Version.")?;

        return second_part
            .parse::<u32>()
            .map_err(|_| {
                "Java-Version konnte nicht gelesen werden.".to_string()
            });
    }

    Ok(major)
}

// ============================================================
// CHECK JAVA
// ============================================================

pub(super) fn check_java(
    java_path: &Path,
    required_major: u32,
) -> Result<PathBuf, String> {
    let actual_major = get_java_version(java_path)?;

    if actual_major != required_major {
        return Err(format!(
            "Java {} gefunden, aber Minecraft benötigt Java {}.",
            actual_major,
            required_major
        ));
    }

    println!(
        "[Java] Java {} erfolgreich erkannt: {:?}",
        actual_major,
        java_path
    );

    Ok(java_path.to_path_buf())
}

// ============================================================
// SYSTEM JAVA DETECTION
// ============================================================

pub(super) fn find_system_java(
    required_major: u32,
) -> Result<PathBuf, String> {
    // 1. Check JAVA_HOME.
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let java_path = PathBuf::from(java_home)
            .join("bin")
            .join(get_java_executable());

        if java_path.is_file() {
            if let Ok(path) = check_java(&java_path, required_major) {
                return Ok(path);
            }
        }
    }

    // 2. Check Java in PATH.
    let java_path = PathBuf::from("java");

    if let Ok(path) = check_java(&java_path, required_major) {
        return Ok(path);
    }

    Err(format!(
        "Keine passende System-Java-Version {} gefunden.",
        required_major
    ))
}