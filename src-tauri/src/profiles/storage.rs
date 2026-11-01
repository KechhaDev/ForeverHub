use std::path::PathBuf;

use super::Profile;
use super::AuthenticationMode;

// ============================================================
// STORAGE DIRECTORIES
// ============================================================

fn get_appdata_dir() -> Option<PathBuf> {
    std::env::var("APPDATA")
        .ok()
        .map(PathBuf::from)
}

fn get_foreverhub_dir() -> Option<PathBuf> {
    let appdata = get_appdata_dir()?;
    let foreverhub_dir = appdata.join("ForeverHub");

    std::fs::create_dir_all(&foreverhub_dir).ok()?;

    Some(foreverhub_dir)
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
            modpack_version: "Vanilla".to_string(),
            install_dir: installations_dir.join("Vanilla"),
            authentication_mode: AuthenticationMode::Microsoft,
        },
        Profile {
            name: "ForeverWorld".to_string(),
            minecraft_version: "26.3".to_string(),
            fabric_version: "0.19.5".to_string(),
            modpack_version: "1.0.0".to_string(),
            install_dir: installations_dir.join("ForeverWorld"),
            authentication_mode: AuthenticationMode::Microsoft,
        },
        Profile {
            name: "ForeverWorld-DEVELOPMENT".to_string(),
            minecraft_version: "26.3".to_string(),
            fabric_version: "0.19.5".to_string(),
            modpack_version: "1.0.1-dev".to_string(),
            install_dir: installations_dir.join("ForeverWorld-DEVELOPMENT"),
            authentication_mode: AuthenticationMode::Offline,
        },
    ]
}

fn save_profiles(profiles: &[Profile]) -> bool {
    let Some(base_dir) = get_foreverhub_dir() else {
        return false;
    };

    let profiles_file = base_dir
        .join("launcher")
        .join("profiles.json");

    let json = match serde_json::to_string_pretty(profiles) {
        Ok(json) => json,
        Err(_) => return false,
    };

    std::fs::write(profiles_file, json).is_ok()
}

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
        } else {
            profiles.push(default_profile);
        }
    }

    save_profiles(&profiles)
}



pub(crate) fn find_profile(profile_name: &str) -> Option<Profile> {
    let profiles = load_profiles_with_paths()?;

    profiles
        .into_iter()
        .find(|profile| profile.name == profile_name)
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