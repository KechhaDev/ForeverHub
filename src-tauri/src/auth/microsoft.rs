
use serde::Deserialize;

const MICROSOFT_CLIENT_ID: &str =
    "890798b4-1d3a-4040-8eec-fe7fcb543aae";

const DEVICE_CODE_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";

const TOKEN_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";

// ============================================================
// MICROSOFT RESPONSES
// ============================================================

#[derive(Debug, Deserialize)]
pub(super) struct DeviceCodeResponse {
    pub(super) device_code: String,
    pub(super) user_code: String,
    pub(super) verification_uri: String,
    pub(super) expires_in: u64,

    #[serde(default = "default_poll_interval")]
    pub(super) interval: u64,

    #[serde(default)]
    pub(super) message: String,
}

fn default_poll_interval() -> u64 {
    5
}

#[derive(Debug, Deserialize)]
pub(super) struct MicrosoftTokenResponse {
    pub(super) access_token: String,

    #[serde(default)]
    pub(super) refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MicrosoftTokenError {
    error: String,

    #[serde(default)]
    error_description: String,
}

// ============================================================
// DEVICE CODE FLOW
// ============================================================

pub(super) async fn request_device_code(
    client: &reqwest::Client,
) -> Result<DeviceCodeResponse, String> {
    let response = client
        .post(DEVICE_CODE_URL)
        .form(&[
            ("client_id", MICROSOFT_CLIENT_ID),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await
        .map_err(|error| {
            format!(
                "Microsoft-Anmeldung konnte nicht gestartet werden: {}",
                error
            )
        })?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();

        return Err(format!(
            "Microsoft hat den Anmeldecode abgelehnt: {}",
            body
        ));
    }

    response
        .json::<DeviceCodeResponse>()
        .await
        .map_err(|error| {
            format!(
                "Microsoft-Anmeldecode konnte nicht gelesen werden: {}",
                error
            )
        })
}

// ============================================================
// TOKEN POLLING
// ============================================================

pub(super) async fn poll_for_microsoft_token(
    client: &reqwest::Client,
    device: &DeviceCodeResponse,
) -> Result<MicrosoftTokenResponse, String> {
    let start_time = std::time::Instant::now();
    let mut interval = device.interval.max(1);

    loop {
        if start_time.elapsed().as_secs() >= device.expires_in {
            return Err(
                "Der Microsoft-Anmeldecode ist abgelaufen. Bitte versuche es erneut."
                    .to_string()
            );
        }

        tokio::time::sleep(
            std::time::Duration::from_secs(interval)
        )
        .await;

        let response = client
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", MICROSOFT_CLIENT_ID),
                ("device_code", device.device_code.as_str()),
            ])
            .send()
            .await
            .map_err(|error| {
                format!(
                    "Microsoft-Anmeldestatus konnte nicht abgefragt werden: {}",
                    error
                )
            })?;

        let status = response.status();
        let body = response.text().await
            .map_err(|error| error.to_string())?;

        if status.is_success() {
            return serde_json::from_str::<MicrosoftTokenResponse>(&body)
                .map_err(|error| {
                    format!(
                        "Microsoft-Token konnte nicht gelesen werden: {}",
                        error
                    )
                });
        }

        let error = serde_json::from_str::<MicrosoftTokenError>(&body)
            .map_err(|_| {
                format!(
                    "Unbekannter Microsoft-Fehler: HTTP {}",
                    status
                )
            })?;

        match error.error.as_str() {
            "authorization_pending" => {}

            "slow_down" => {
                interval += 5;
            }

            "authorization_declined" => {
                return Err(
                    "Die Microsoft-Anmeldung wurde abgebrochen."
                        .to_string()
                );
            }

            "expired_token" => {
                return Err(
                    "Der Microsoft-Anmeldecode ist abgelaufen. Bitte versuche es erneut."
                        .to_string()
                );
            }

            "bad_verification_code" => {
                return Err(
                    "Der Microsoft-Anmeldecode ist ungültig."
                        .to_string()
                );
            }

            _ => {
                return Err(format!(
                    "Microsoft-Anmeldung fehlgeschlagen: {} ({})",
                    error.error,
                    error.error_description
                ));
            }
        }
    }
}

// ============================================================
// REFRESH TOKEN FLOW
// ============================================================
#[derive(Debug)]
pub(super) enum MicrosoftRefreshError {
    Network(String),
    InvalidToken(String),
    Other(String),
}

pub(super) async fn refresh_microsoft_token(
    client: &reqwest::Client,
    refresh_token: &str,
) -> Result<MicrosoftTokenResponse, MicrosoftRefreshError> {
    let response = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", MICROSOFT_CLIENT_ID),
            ("refresh_token", refresh_token),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await
        .map_err(|error| {
            MicrosoftRefreshError::Network(format!(
                "Microsoft konnte nicht erreicht werden: {}",
                error
            ))
        })?;

    let status = response.status();

    let body = response
        .text()
        .await
        .map_err(|error| {
            MicrosoftRefreshError::Network(format!(
                "Microsoft-Antwort konnte nicht gelesen werden: {}",
                error
            ))
        })?;

    if !status.is_success() {
        let error = serde_json::from_str::<MicrosoftTokenError>(&body)
            .map_err(|_| {
                MicrosoftRefreshError::Other(format!(
                    "Microsoft-Token-Erneuerung fehlgeschlagen: HTTP {}",
                    status
                ))
            })?;

        if matches!(
            error.error.as_str(),
            "invalid_grant" | "invalid_request"
        ) {
            return Err(MicrosoftRefreshError::InvalidToken(format!(
                "{} ({})",
                error.error,
                error.error_description
            )));
        }

        return Err(MicrosoftRefreshError::Other(format!(
            "{} ({})",
            error.error,
            error.error_description
        )));
    }

    serde_json::from_str::<MicrosoftTokenResponse>(&body)
        .map_err(|error| {
            MicrosoftRefreshError::Other(format!(
                "Erneuertes Microsoft-Token konnte nicht gelesen werden: {}",
                error
            ))
        })
}

