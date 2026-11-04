#[tauri::command]
pub(crate) fn get_data_directory() -> Result<String, String> {
    let path = crate::profiles::get_foreverhub_dir()
        .ok_or_else(|| {
            "ForeverHub-Datenverzeichnis konnte nicht gefunden werden.".to_string()
        })?;

    Ok(path.to_string_lossy().into_owned())
}