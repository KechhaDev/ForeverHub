
use std::path::Path;
use std::process::{Child, Command, Stdio};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

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
        .stdin(Stdio::null());

    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NEW_PROCESS_GROUP);

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

    Ok(child)
}
