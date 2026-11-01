
use tauri::State;

use crate::auth;
use super::AuthState;

// ============================================================
// MICROSOFT AUTHENTICATION
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
    *state.session.lock()
        .map_err(|error| error.to_string())? = None;

    *state.account.lock()
        .map_err(|error| error.to_string())? = None;

    Ok(())
}
