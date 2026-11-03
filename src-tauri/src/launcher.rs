
mod metadata;
mod java;
mod java_runtime;
mod libraries;
mod arguments;
mod process;

use self::metadata::{
    find_fabric_metadata,
    get_version_id,
    read_json,
};

use self::libraries::build_classpath;

use self::arguments::{
    collect_arguments,
    LaunchPaths,
};

use self::process::start_process;

use std::path::Path;
use std::process::Child;

use crate::account::LaunchSession;

// ============================================================
// GAME START
// ============================================================

pub async fn start_game(
    profile_dir: &Path,
    minecraft_version: &str,
    fabric_enabled: bool,
    ram_gb: u32,
    session: &LaunchSession,
) -> Result<Child, String> {
    println!("Erstelle Minecraft-Startkonfiguration...");

    if !(2..=12).contains(&ram_gb) {
        return Err(
            "Die RAM-Einstellung muss zwischen 2 und 12 GB liegen."
                .to_string()
        );
    }

    // --------------------------------------------------------
    // LOAD MINECRAFT METADATA
    // --------------------------------------------------------

    let base_json = profile_dir
        .join("versions")
        .join(minecraft_version)
        .join(format!("{}.json", minecraft_version));

    let base_metadata = read_json(&base_json)?;

    // --------------------------------------------------------
    // LOAD FABRIC METADATA
    // --------------------------------------------------------

    let fabric_metadata = if fabric_enabled {
        Some(find_fabric_metadata(
            profile_dir,
            minecraft_version,
        )?)
    } else {
        None
    };

    let launch_metadata = fabric_metadata
        .as_ref()
        .unwrap_or(&base_metadata);

    let version_id = get_version_id(launch_metadata)?;

    let main_class = launch_metadata["mainClass"]
        .as_str()
        .ok_or("Main-Class fehlt.")?;

    let asset_index = base_metadata["assetIndex"]["id"]
        .as_str()
        .ok_or("Asset-Index fehlt.")?;

    let version_type = base_metadata["type"]
        .as_str()
        .unwrap_or("release");

    // --------------------------------------------------------
    // DIRECTORIES
    // --------------------------------------------------------

    let assets_dir = profile_dir.join("assets");
    let libraries_dir = profile_dir.join("libraries");

    let natives_dir = profile_dir
        .join("versions")
        .join(minecraft_version)
        .join(format!("{}-natives", minecraft_version));

    if !natives_dir.exists() {
        std::fs::create_dir_all(&natives_dir)
            .map_err(|error| error.to_string())?;
    }

    // --------------------------------------------------------
    // CLASSPATH
    // --------------------------------------------------------

    let classpath = build_classpath(
        &base_metadata,
        fabric_metadata.as_ref(),
        profile_dir,
    )?;

    let paths = LaunchPaths {
        profile_dir,
        assets_dir: &assets_dir,
        natives_dir: &natives_dir,
        libraries_dir: &libraries_dir,
        version_id: &version_id,
        version_type,
        asset_index,
        classpath: &classpath,
    };

    // --------------------------------------------------------
    // REQUIRED JAVA VERSION
    // --------------------------------------------------------

    let required_java = fabric_metadata
        .as_ref()
        .and_then(|fabric| {
            fabric["javaVersion"]["majorVersion"].as_u64()
        })
        .or_else(|| {
            base_metadata["javaVersion"]["majorVersion"].as_u64()
        })
        .unwrap_or(8) as u32;

    // --------------------------------------------------------
    // ENSURE JAVA RUNTIME
    // --------------------------------------------------------

    let java = java_runtime::ensure_java(required_java).await?;

    // --------------------------------------------------------
    // JVM ARGUMENTS
    // --------------------------------------------------------

    let mut arguments = collect_arguments(
        &base_metadata,
        "jvm",
        &paths,
        session,
    );

    if let Some(fabric) = &fabric_metadata {
        arguments.extend(collect_arguments(
            fabric,
            "jvm",
            &paths,
            session,
        ));
    }

    // Remove RAM arguments supplied by Minecraft or Fabric metadata.
    arguments.retain(|argument| {
        !argument.starts_with("-Xms")
            && !argument.starts_with("-Xmx")
    });

    // Use a small initial heap and the selected maximum heap.
    arguments.push("-Xms1G".to_string());
    arguments.push(format!("-Xmx{}G", ram_gb));

    // --------------------------------------------------------
    // GAME ARGUMENTS
    // --------------------------------------------------------

    let mut game_arguments = collect_arguments(
        &base_metadata,
        "game",
        &paths,
        session,
    );

    if let Some(fabric) = &fabric_metadata {
        game_arguments.extend(collect_arguments(
            fabric,
            "game",
            &paths,
            session,
        ));
    }

    // --------------------------------------------------------
    // START PROCESS
    // --------------------------------------------------------

    println!("Java: {:?}", java);
    println!("Java-Version: {}", required_java);
    println!("RAM: {} GB", ram_gb);
    println!("Main-Class: {}", main_class);
    println!("Minecraft-Version: {}", minecraft_version);
    println!("Launch-Version: {}", version_id);
    println!("Starte Minecraft...");

    start_process(
        &java,
        main_class,
        &arguments,
        &game_arguments,
        &natives_dir,
        profile_dir,
    )
}
