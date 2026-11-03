
mod microsoft;
mod xbox;
mod minecraft;
mod models;
mod token_storage;

use serde_json::json;
use tauri::Emitter;

use crate::account::LaunchSession;

use self::microsoft::{
    request_device_code,
    poll_for_microsoft_token,
    refresh_microsoft_token,
    MicrosoftTokenResponse,
    MicrosoftRefreshError,
};

use self::xbox::{
    authenticate_xbox,
    authenticate_xsts,
};

use self::minecraft::{
    authenticate_minecraft,
    fetch_minecraft_profile,
};

pub use self::models::AccountInfo;

// ============================================================
// LOGIN ENTRY POINT
// ============================================================

pub async fn login(
    app: &tauri::AppHandle,
) -> Result<(LaunchSession, AccountInfo), String> {
    let client = reqwest::Client::new();

    println!("Starte Microsoft-Anmeldung...");

    // 1. Request a Microsoft device code.
    let device_code = request_device_code(&client).await?;

    // 2. Send the code to the React frontend.
    app.emit(
        "microsoft-device-code",
        &json!({
            "user_code": device_code.user_code.clone(),
            "verification_uri": device_code.verification_uri.clone(),
            "message": device_code.message.clone()
        })
    )
    .map_err(|error| {
        format!(
            "Anmeldecode konnte nicht angezeigt werden: {}",
            error
        )
    })?;

    println!(
        "Microsoft-Anmeldung wartet auf Code {}",
        device_code.user_code
    );

    // 3. Wait for the user to finish signing in.
    let microsoft_token = poll_for_microsoft_token(
        &client,
        &device_code,
    )
    .await?;

    // 4. Save the refresh token securely.
    let refresh_token = microsoft_token.refresh_token
        .as_deref()
        .ok_or(
            "Microsoft hat keinen Refresh Token geliefert."
        )?;

    token_storage::save_refresh_token(refresh_token)?;

    println!("Microsoft-Anmeldung erfolgreich.");

    // 5. Continue with Xbox and Minecraft authentication.
    authenticate_account(&client, microsoft_token).await
}

// ============================================================
// RESTORE SESSION
// ============================================================


pub async fn restore_login(
) -> Result<Option<(LaunchSession, AccountInfo)>, String> {
    let Some(refresh_token) = token_storage::load_refresh_token()? else {
        println!("Kein gespeicherter Microsoft-Login vorhanden.");
        return Ok(None);
    };

    println!("Stelle Microsoft-Sitzung wieder her...");

    let client = reqwest::Client::new();

    // 1. Microsoft-Token erneuern.
    let microsoft_token = match refresh_microsoft_token(
        &client,
        &refresh_token,
    ).await {
        Ok(token) => token,

        Err(MicrosoftRefreshError::Network(error)) => {
            return Err(format!(
                "Keine Verbindung zu Microsoft. Deine gespeicherte Anmeldung bleibt erhalten. {}",
                error
            ));
        }

        Err(MicrosoftRefreshError::InvalidToken(error)) => {
            token_storage::delete_refresh_token()?;

            return Err(format!(
                "Deine Microsoft-Anmeldung ist nicht mehr gültig. Bitte melde dich erneut an. {}",
                error
            ));
        }

        Err(MicrosoftRefreshError::Other(error)) => {
            return Err(format!(
                "Microsoft-Anmeldung konnte nicht wiederhergestellt werden: {}",
                error
            ));
        }
    };

    // 2. Rotierten Refresh Token speichern.
    if let Some(new_refresh_token) = &microsoft_token.refresh_token {
        token_storage::save_refresh_token(new_refresh_token)?;
    }

    println!("Microsoft-Sitzung erfolgreich erneuert.");

    // 3. Xbox Live, XSTS und Minecraft authentifizieren.
    let result = authenticate_account(
        &client,
        microsoft_token,
    )
    .await?;

    Ok(Some(result))
}


// ============================================================
// SHARED ACCOUNT AUTHENTICATION
// ============================================================

async fn authenticate_account(
    client: &reqwest::Client,
    microsoft_token: MicrosoftTokenResponse,
) -> Result<(LaunchSession, AccountInfo), String> {
    // 1. Authenticate with Xbox Live.
    let xbox_response = authenticate_xbox(
        client,
        &microsoft_token.access_token,
    )
    .await?;

    let user_hash = xbox_response
        .display_claims
        .xui
        .first()
        .ok_or("Xbox Live hat keine Benutzerinformationen geliefert.")?
        .uhs
        .clone();

    println!("Xbox Live-Anmeldung erfolgreich.");

    // 2. Authenticate with XSTS.
    let xsts_response = authenticate_xsts(
        client,
        &xbox_response.token,
    )
    .await?;

    let xsts_user_hash = xsts_response
        .display_claims
        .xui
        .first()
        .ok_or("XSTS hat keine Benutzerinformationen geliefert.")?
        .uhs
        .clone();

    if user_hash != xsts_user_hash {
        return Err(
            "Die Xbox-Benutzerinformationen stimmen nicht überein."
                .to_string()
        );
    }

    println!("XSTS-Anmeldung erfolgreich.");

    // 3. Exchange XSTS token for a Minecraft token.
    let minecraft_token = authenticate_minecraft(
        client,
        &user_hash,
        &xsts_response.token,
    )
    .await?;

    println!("Minecraft-Authentifizierung erfolgreich.");

    // 4. Fetch the actual Minecraft Java profile.
    let account = fetch_minecraft_profile(
        client,
        &minecraft_token,
    )
    .await?;

    println!(
        "Minecraft-Profil geladen: {}",
        account.username
    );

    // 5. Create the session used by the launcher.
    let session = LaunchSession {
        username: account.username.clone(),
        uuid: account.uuid.clone(),
        access_token: minecraft_token,
        user_type: "msa".to_string(),
        client_id: String::new(),
        xuid: String::new(),
    };

    Ok((session, account))
}

// ============================================================
// LOGOUT
// ============================================================

pub fn logout() -> Result<(), String> {
    token_storage::delete_refresh_token()
}
