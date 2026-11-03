
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// ============================================================
// LOG FILE
// ============================================================

fn create_log_file(profile_dir: &Path) -> Result<(std::fs::File, PathBuf), String> {
    let appdata = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .ok_or("Windows-APPDATA-Verzeichnis konnte nicht gefunden werden.")?;

    let log_dir = appdata.join("ForeverHub").join("logs");

    fs::create_dir_all(&log_dir)
        .map_err(|error| format!("Log-Verzeichnis konnte nicht erstellt werden: {}", error))?;

    let profile_name = profile_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Minecraft");

    let safe_name: String = profile_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect();

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let log_path = log_dir.join(format!(
        "{}-{}.log",
        safe_name,
        timestamp
    ));

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|error| format!("Minecraft-Logdatei konnte nicht erstellt werden: {}", error))?;

    writeln!(file, "ForeverHub Minecraft Log")
        .map_err(|error| error.to_string())?;

    writeln!(file, "Profil: {}", profile_name)
        .map_err(|error| error.to_string())?;

    writeln!(file, "----------------------------------------")
        .map_err(|error| error.to_string())?;

    Ok((file, log_path))
}

// ============================================================
// GAME PROCESS
// ============================================================

pub(super) fn start_process(
    java: &Path,
    main_class: &str,
    arguments: &[String],
    game_arguments: &[String],
    natives_dir: &Path,
    profile_dir: &Path,
) -> Result<Child, String> {
    let (log_file, log_path) = create_log_file(profile_dir)?;

    let stderr_file = log_file
        .try_clone()
        .map_err(|error| format!("Logdatei konnte nicht geöffnet werden: {}", error))?;

    let mut command = Command::new(java);

    command.arg(format!(
        "-Djava.library.path={}",
        natives_dir.to_string_lossy()
    ));

    command.arg(format!(
        "-Djna.tmpdir={}",
        natives_dir.to_string_lossy()
    ));

    command.arg(format!(
        "-Dorg.lwjgl.system.SharedLibraryExtractPath={}",
        natives_dir.to_string_lossy()
    ));

    command.arg(format!(
        "-Dio.netty.native.workdir={}",
        natives_dir.to_string_lossy()
    ));

    command
        .args(arguments)
        .arg(main_class)
        .args(game_arguments)
        .current_dir(profile_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(stderr_file));

    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);

    let child = command
        .spawn()
        .map_err(|error| {
            format!(
                "Minecraft konnte nicht gestartet werden: {}",
                error
            )
        })?;

    println!(
        "Minecraft-Prozess gestartet. PID: {}",
        child.id()
    );

    println!(
        "Minecraft-Logdatei: {}",
        log_path.display()
    );

    Ok(child)
}
