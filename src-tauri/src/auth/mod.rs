
mod microsoft;
mod xbox;
mod minecraft;
mod models;

use serde_json::json;
use tauri::Emitter;

use crate::account::LaunchSession;

use self::microsoft::{
    request_device_code,
    poll_for_microsoft_token,
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

    println!("Microsoft-Anmeldung erfolgreich.");

    // 4. Authenticate with Xbox Live.
    let xbox_response = authenticate_xbox(
        &client,
        &microsoft_token,
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

    // 5. Authenticate with XSTS.
    let xsts_response = authenticate_xsts(
        &client,
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

    // 6. Exchange XSTS token for a Minecraft token.
    let minecraft_token = authenticate_minecraft(
        &client,
        &user_hash,
        &xsts_response.token,
    )
    .await?;

    println!("Minecraft-Authentifizierung erfolgreich.");

    // 7. Fetch the actual Minecraft Java profile.
    let account = fetch_minecraft_profile(
        &client,
        &minecraft_token,
    )
    .await?;

    println!(
        "Minecraft-Profil geladen: {}",
        account.username
    );

    // 8. Create the session used by the launcher.
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
