
use tauri::State;

use crate::auth;
use super::AuthState;

// ============================================================
// MICROSOFT LOGIN
// ============================================================

#[tauri::command]
pub(crate) async fn login_microsoft(
    app: tauri::AppHandle,
    state: State<'_, AuthState>,
) -> Result<auth::AccountInfo, String> {
    let (session, account) = auth::login(&app).await?;

    *state.session.lock()
        .map_err(|error| error.to_string())? = Some(session);

    *state.account.lock()
        .map_err(|error| error.to_string())? = Some(account.clone());

    Ok(account)
}

// ============================================================
// RESTORE MICROSOFT SESSION
// ============================================================

#[tauri::command]
pub(crate) async fn restore_microsoft_session(
    state: State<'_, AuthState>,
) -> Result<Option<auth::AccountInfo>, String> {
    let Some((session, account)) = auth::restore_login().await? else {
        return Ok(None);
    };

    *state.session.lock()
        .map_err(|error| error.to_string())? = Some(session);

    *state.account.lock()
        .map_err(|error| error.to_string())? = Some(account.clone());

    Ok(Some(account))
}

// ============================================================
// ACCOUNT INFORMATION
// ============================================================

#[tauri::command]
pub(crate) fn get_account(
    state: State<'_, AuthState>,
) -> Result<Option<auth::AccountInfo>, String> {
    Ok(
        state.account.lock()
            .map_err(|error| error.to_string())?
            .clone()
    )
}

// ============================================================
// LOGOUT
// ============================================================


#[tauri::command]
pub(crate) fn logout_microsoft(
    state: State<'_, AuthState>,
) -> Result<(), String> {
    // Lokale Sitzung immer zuerst leeren.
    *state.session.lock()
        .map_err(|error| error.to_string())? = None;

    *state.account.lock()
        .map_err(|error| error.to_string())? = None;

    // Anschließend gespeicherte Microsoft-Anmeldung löschen.
    auth::logout()
}

