
use std::path::PathBuf;

use super::AuthenticationMode;
use super::Profile;

const MIN_RAM_GB: u32 = 2;
const MAX_RAM_GB: u32 = 12;
const DEFAULT_RAM_GB: u32 = 4;

// ============================================================
// STORAGE DIRECTORIES
// ============================================================

fn get_appdata_dir() -> Option<PathBuf> {
    std::env::var("APPDATA")
        .ok()
        .map(PathBuf::from)
}



pub(crate) fn get_foreverhub_dir() -> Option<PathBuf> {
    let appdata = get_appdata_dir()?;
    let foreverhub_dir = appdata.join("ForeverHub");

    std::fs::create_dir_all(&foreverhub_dir).ok()?;

    Some(foreverhub_dir)
}

pub(crate) fn get_runtimes_dir() -> Result<PathBuf, String> {
    let base_dir = get_foreverhub_dir()
        .ok_or_else(|| {
            "ForeverHub-Speicherverzeichnis ist nicht verfügbar.".to_string()
        })?;

    let runtimes_dir = base_dir.join("runtimes");

    std::fs::create_dir_all(&runtimes_dir)
        .map_err(|error| {
            format!(
                "Java-Runtime-Verzeichnis konnte nicht erstellt werden: {}",
                error
            )
        })?;

    Ok(runtimes_dir)
}

// ============================================================
// PROFILE MANAGEMENT
// ============================================================

fn get_profiles() -> Vec<Profile> {
    let base_dir = get_foreverhub_dir()
        .expect("ForeverHub-Ordner nicht verfügbar");

    let installations_dir = base_dir.join("installations");

    vec![
        Profile {
            name: "Vanilla".to_string(),
            minecraft_version: "26.3".to_string(),
            fabric_version: "/".to_string(),
            modpack_id: None,
            modpack_version: "Vanilla".to_string(),
            install_dir: installations_dir.join("Vanilla"),
            authentication_mode: AuthenticationMode::Microsoft,
            ram_gb: DEFAULT_RAM_GB,
        },
        Profile {
            name: "ForeverWorld".to_string(),
            minecraft_version: "26.3".to_string(),
            fabric_version: "0.19.5".to_string(),
            modpack_id: Some("ForeverWorld".to_string()),
            modpack_version: "1.0.0".to_string(),
            install_dir: installations_dir.join("ForeverWorld"),
            authentication_mode: AuthenticationMode::Microsoft,
            ram_gb: DEFAULT_RAM_GB,
        },
        Profile {
            name: "ForeverWorld-DEVELOPMENT".to_string(),
            minecraft_version: "26.3".to_string(),
            fabric_version: "0.19.5".to_string(),
            modpack_id: Some("ForeverWorldDev".to_string()),
            modpack_version: "1.0.1-dev".to_string(),
            install_dir: installations_dir.join("ForeverWorld-DEVELOPMENT"),
            authentication_mode: AuthenticationMode::Offline,
            ram_gb: DEFAULT_RAM_GB,
        },
        Profile {
            name: "NewCraft".to_string(),
            minecraft_version: "26.3".to_string(),
            fabric_version: "0.19.5".to_string(),
            modpack_id: Some("NewCraft".to_string()),
            modpack_version: "1.0.0".to_string(),
            authentication_mode: AuthenticationMode::Offline,
            install_dir: installations_dir.join("NewCraft"),
            ram_gb: DEFAULT_RAM_GB,
        },
    ]
}

// ============================================================
// SAVE PROFILES
// ============================================================

fn save_profiles(profiles: &[Profile]) -> bool {
    let Some(base_dir) = get_foreverhub_dir() else {
        return false;
    };

    let profiles_dir = base_dir.join("launcher");

    if std::fs::create_dir_all(&profiles_dir).is_err() {
        return false;
    }

    let profiles_file = profiles_dir.join("profiles.json");

    let json = match serde_json::to_string_pretty(profiles) {
        Ok(json) => json,
        Err(_) => return false,
    };

    std::fs::write(profiles_file, json).is_ok()
}

// ============================================================
// LOAD PROFILES
// ============================================================

fn load_profiles() -> Option<Vec<Profile>> {
    let base_dir = get_foreverhub_dir()?;

    let profiles_file = base_dir
        .join("launcher")
        .join("profiles.json");

    let json = std::fs::read_to_string(profiles_file).ok()?;

    serde_json::from_str::<Vec<Profile>>(&json).ok()
}

fn load_profiles_with_paths() -> Option<Vec<Profile>> {
    let mut profiles = load_profiles()?;

    let base_dir = get_foreverhub_dir()?;
    let installations_dir = base_dir.join("installations");

    for profile in &mut profiles {
        profile.install_dir =
            installations_dir.join(&profile.name);
    }

    Some(profiles)
}

// ============================================================
// INITIALIZE PROFILES
// ============================================================

fn initialize_profiles() -> bool {
    let Some(base_dir) = get_foreverhub_dir() else {
        return false;
    };

    let profiles_file = base_dir
        .join("launcher")
        .join("profiles.json");

    let mut profiles = if profiles_file.exists() {
        match load_profiles() {
            Some(profiles) => profiles,
            None => return false,
        }
    } else {
        Vec::new()
    };

    let default_profiles = get_profiles();

    for default_profile in default_profiles {
        if let Some(existing_profile) = profiles
            .iter_mut()
            .find(|profile| profile.name == default_profile.name)
        {
            existing_profile.authentication_mode =
                default_profile.authentication_mode;

            existing_profile.modpack_id =
                default_profile.modpack_id;
        } else {
            profiles.push(default_profile);
        }
    }

    save_profiles(&profiles)
}

// ============================================================
// FIND PROFILE
// ============================================================

pub(crate) fn find_profile(
    profile_name: &str,
) -> Option<Profile> {
    let profiles = load_profiles_with_paths()?;

    profiles
        .into_iter()
        .find(|profile| profile.name == profile_name)
}

// ============================================================
// UPDATE PROFILE RAM
// ============================================================

pub(crate) fn update_profile_ram(
    profile_name: &str,
    ram_gb: u32,
) -> Result<(), String> {
    if !(MIN_RAM_GB..=MAX_RAM_GB).contains(&ram_gb) {
        return Err(format!(
            "RAM muss zwischen {} und {} GB liegen.",
            MIN_RAM_GB,
            MAX_RAM_GB
        ));
    }

    let mut profiles = load_profiles()
        .ok_or_else(|| {
            "Profile konnten nicht geladen werden.".to_string()
        })?;

    let profile = profiles
        .iter_mut()
        .find(|profile| profile.name == profile_name)
        .ok_or_else(|| {
            format!("Profil {} wurde nicht gefunden.", profile_name)
        })?;

    profile.ram_gb = ram_gb;

    if !save_profiles(&profiles) {
        return Err(
            "RAM-Einstellung konnte nicht gespeichert werden.".to_string()
        );
    }

    Ok(())
}

// ============================================================
// UPDATE MODPACK VERSION
// ============================================================

pub(crate) fn update_modpack_version(
    profile_name: &str,
    modpack_version: &str,
) -> Result<(), String> {
    let mut profiles = load_profiles()
        .ok_or_else(|| {
            "Profile konnten nicht geladen werden.".to_string()
        })?;

    let profile = profiles
        .iter_mut()
        .find(|profile| profile.name == profile_name)
        .ok_or_else(|| {
            format!(
                "Profil {} wurde nicht gefunden.",
                profile_name
            )
        })?;

    profile.modpack_version = modpack_version.to_string();

    if !save_profiles(&profiles) {
        return Err(
            "Modpack-Version konnte nicht gespeichert werden.".to_string()
        );
    }

    Ok(())
}

// ============================================================
// UPDATE PROFILE METADATA
// ============================================================

pub(crate) fn update_profile_metadata(
    profile_name: &str,
    minecraft_version: &str,
    fabric_version: &str,
    modpack_version: &str,
) -> Result<(), String> {
    let mut profiles = load_profiles()
        .ok_or_else(|| {
            "Profile konnten nicht geladen werden.".to_string()
        })?;

    let profile = profiles
        .iter_mut()
        .find(|profile| profile.name == profile_name)
        .ok_or_else(|| {
            format!("Profil {} wurde nicht gefunden.", profile_name)
        })?;

    profile.minecraft_version = minecraft_version.to_string();
    profile.fabric_version = fabric_version.to_string();
    profile.modpack_version = modpack_version.to_string();

    if !save_profiles(&profiles) {
        return Err(
            "Profil-Metadaten konnten nicht gespeichert werden.".to_string()
        );
    }

    Ok(())
}

// ============================================================
// LIST PROFILES
// ============================================================

pub(crate) fn list_profiles() -> Option<Vec<Profile>> {
    load_profiles_with_paths()
}

// ============================================================
// INSTALLATION DIRECTORIES
// ============================================================

fn setup_installations() -> bool {
    let Some(profiles) = load_profiles_with_paths() else {
        return false;
    };

    let directories = [
        "versions",
        "libraries",
        "assets",
        "mods",
        "resourcepacks",
        "shaderpacks",
        "config",
        "saves",
        "logs",
    ];

    for profile in profiles {
        if std::fs::create_dir_all(&profile.install_dir).is_err() {
            return false;
        }

        for directory in directories {
            let path = profile.install_dir.join(directory);

            if std::fs::create_dir_all(path).is_err() {
                return false;
            }
        }
    }

    true
}

// ============================================================
// APPLICATION DIRECTORY SETUP
// ============================================================

pub(crate) fn setup_directories() -> bool {
    let Some(base_dir) = get_foreverhub_dir() else {
        return false;
    };

    let directories = [
        "installations",
        "logs",
        "launcher",
        "runtimes",
    ];

    for directory in directories {
        if std::fs::create_dir_all(base_dir.join(directory)).is_err() {
            return false;
        }
    }

    if !initialize_profiles() {
        return false;
    }

    if !setup_installations() {
        return false;
    }

    true
}
