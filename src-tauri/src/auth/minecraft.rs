
use serde::Deserialize;
use serde_json::json;

use super::models::AccountInfo;

const MINECRAFT_LOGIN_URL: &str =
    "https://api.minecraftservices.com/authentication/login_with_xbox";

const MINECRAFT_PROFILE_URL: &str =
    "https://api.minecraftservices.com/minecraft/profile";

// ============================================================
// MINECRAFT RESPONSES
// ============================================================

#[derive(Debug, Deserialize)]
struct MinecraftLoginResponse {
    access_token: String,
}

// ============================================================
// MINECRAFT AUTHENTICATION
// ============================================================

pub(super) async fn authenticate_minecraft(
    client: &reqwest::Client,
    user_hash: &str,
    xsts_token: &str,
) -> Result<String, String> {
    let identity_token = format!(
        "XBL3.0 x={};{}",
        user_hash,
        xsts_token
    );

    let body = json!({
        "identityToken": identity_token
    });

    let response = client
        .post(MINECRAFT_LOGIN_URL)
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            format!(
                "Minecraft-Authentifizierung konnte nicht gestartet werden: {}",
                error
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        return Err(format!(
            "Minecraft-Login fehlgeschlagen ({}): {}",
            status,
            body
        ));
    }

    response
        .json::<MinecraftLoginResponse>()
        .await
        .map(|result| result.access_token)
        .map_err(|error| {
            format!("Minecraft-Token ungültig: {}", error)
        })
}

// ============================================================
// MINECRAFT PROFILE
// ============================================================

pub(super) async fn fetch_minecraft_profile(
    client: &reqwest::Client,
    minecraft_token: &str,
) -> Result<AccountInfo, String> {
    let response = client
        .get(MINECRAFT_PROFILE_URL)
        .bearer_auth(minecraft_token)
        .send()
        .await
        .map_err(|error| {
            format!(
                "Minecraft-Profil konnte nicht abgerufen werden: {}",
                error
            )
        })?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(
            "Für dieses Microsoft-Konto wurde kein Minecraft-Java-Profil gefunden. Bitte prüfe, ob du Minecraft Java besitzt."
                .to_string()
        );
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        return Err(format!(
            "Minecraft-Profil konnte nicht geladen werden ({}): {}",
            status,
            body
        ));
    }

    let profile: MinecraftProfileResponse = response
        .json()
        .await
        .map_err(|error| {
            format!("Minecraft-Profilantwort ungültig: {}", error)
        })?;

    Ok(AccountInfo {
        username: profile.name,
        uuid: profile.id,
    })
}

// ============================================================
// PROFILE RESPONSE
// ============================================================

#[derive(Debug, Deserialize)]
struct MinecraftProfileResponse {
    name: String,
    id: String,
}
