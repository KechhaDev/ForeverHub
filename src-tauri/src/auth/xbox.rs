
use serde::Deserialize;
use serde_json::json;

const XBOX_AUTH_URL: &str =
    "https://user.auth.xboxlive.com/user/authenticate";

const XSTS_AUTH_URL: &str =
    "https://xsts.auth.xboxlive.com/xsts/authorize";

// ============================================================
// XBOX RESPONSES
// ============================================================

#[derive(Debug, Deserialize)]
pub(super) struct XboxAuthResponse {
    #[serde(rename = "Token")]
    pub(super) token: String,

    #[serde(rename = "DisplayClaims")]
    pub(super) display_claims: XboxDisplayClaims,
}

#[derive(Debug, Deserialize)]
pub(super) struct XboxDisplayClaims {
    pub(super) xui: Vec<XboxUserInfo>,
}

#[derive(Debug, Deserialize)]
pub(super) struct XboxUserInfo {
    pub(super) uhs: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct XstsAuthResponse {
    #[serde(rename = "Token")]
    pub(super) token: String,

    #[serde(rename = "DisplayClaims")]
    pub(super) display_claims: XboxDisplayClaims,
}

// ============================================================
// XBOX LIVE AUTHENTICATION
// ============================================================

pub(super) async fn authenticate_xbox(
    client: &reqwest::Client,
    microsoft_token: &str,
) -> Result<XboxAuthResponse, String> {
    let body = json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={}", microsoft_token)
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let response = client
        .post(XBOX_AUTH_URL)
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            format!("Xbox Live konnte nicht erreicht werden: {}", error)
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        return Err(format!(
            "Xbox Live-Authentifizierung fehlgeschlagen ({}): {}",
            status,
            body
        ));
    }

    response
        .json::<XboxAuthResponse>()
        .await
        .map_err(|error| {
            format!("Xbox Live-Antwort ungültig: {}", error)
        })
}

// ============================================================
// XSTS AUTHENTICATION
// ============================================================

pub(super) async fn authenticate_xsts(
    client: &reqwest::Client,
    xbox_token: &str,
) -> Result<XstsAuthResponse, String> {
    let body = json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbox_token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    let response = client
        .post(XSTS_AUTH_URL)
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            format!("Xbox XSTS konnte nicht erreicht werden: {}", error)
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        return Err(format!(
            "Xbox XSTS-Authentifizierung fehlgeschlagen ({}): {}",
            status,
            body
        ));
    }

    response
        .json::<XstsAuthResponse>()
        .await
        .map_err(|error| {
            format!("XSTS-Antwort ungültig: {}", error)
        })
}
