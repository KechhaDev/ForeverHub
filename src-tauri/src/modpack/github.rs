
use reqwest::{header::HeaderMap, Client, StatusCode, Url};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::manifest::ModpackManifest;

// ============================================================
// GITHUB REPOSITORY
// ============================================================

const GITHUB_OWNER: &str = "KechhaDev";
const GITHUB_REPOSITORY: &str = "ForeverWorld-Modpack";
const GITHUB_BRANCH: &str = "main";

const GITHUB_API: &str = "https://api.github.com/";

const MAX_ATTEMPTS: u32 = 4;
const MAX_RETRY_WAIT: u64 = 60;

// ============================================================
// GITHUB TOKEN
// ============================================================

const GITHUB_TOKEN: &str = env!("FOREVERHUB_GITHUB_TOKEN");

// ============================================================
// GITHUB CLIENT
// ============================================================

pub(crate) struct GitHubClient {
    client: Client,
    token: String,
}

impl GitHubClient {
    pub(crate) fn new() -> Result<Self, String> {
        let client = Client::builder()
            .user_agent("ForeverHub")
            .build()
            .map_err(|error| {
                format!("GitHub-Client konnte nicht erstellt werden: {}", error)
            })?;

        Ok(Self {
            client,
            token: GITHUB_TOKEN.to_string(),
        })
    }

    // ========================================================
    // CREATE FILE URL
    // ========================================================

    fn create_file_url(
        &self,
        repository_path: &str,
    ) -> Result<Url, String> {
        if repository_path.is_empty()
            || repository_path.starts_with('/')
            || repository_path.contains('\\')
            || repository_path.split('/').any(|part| {
                part.is_empty() || part == "." || part == ".."
            })
        {
            return Err(format!(
                "Ungültiger GitHub-Dateipfad: {}",
                repository_path
            ));
        }

        let mut url = Url::parse(GITHUB_API)
            .map_err(|error| error.to_string())?;

        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| {
                    "GitHub-URL konnte nicht erstellt werden.".to_string()
                })?;

            segments
                .push("repos")
                .push(GITHUB_OWNER)
                .push(GITHUB_REPOSITORY)
                .push("contents");

            for segment in repository_path.split('/') {
                segments.push(segment);
            }
        }

        url.query_pairs_mut()
            .append_pair("ref", GITHUB_BRANCH);

        Ok(url)
    }

    // ========================================================
    // RETRY HELPERS
    // ========================================================

    fn is_rate_limit_response(
        headers: &HeaderMap,
        body: &str,
    ) -> bool {
        let remaining_is_zero = headers
            .get("x-ratelimit-remaining")
            .and_then(|value| value.to_str().ok())
            == Some("0");

        let body_indicates_limit = {
            let body = body.to_lowercase();
            body.contains("rate limit")
                || body.contains("secondary rate limit")
        };

        remaining_is_zero || body_indicates_limit
    }

    fn should_retry(
        status: StatusCode,
        headers: &HeaderMap,
        body: &str,
    ) -> bool {
        matches!(
            status.as_u16(),
            408 | 429 | 500 | 502 | 503 | 504
        ) || (
            status == StatusCode::FORBIDDEN
                && Self::is_rate_limit_response(headers, body)
        )
    }

    fn retry_delay(
        headers: &HeaderMap,
        attempt: u32,
        rate_limited: bool,
    ) -> Duration {
        if let Some(seconds) = headers
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
        {
            return Duration::from_secs(seconds);
        }

        if rate_limited {
            if let Some(reset) = headers
                .get("x-ratelimit-reset")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
            {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                return Duration::from_secs(reset.saturating_sub(now));
            }
        }

        Duration::from_secs(1 << (attempt - 1))
    }

    // ========================================================
    // DOWNLOAD FILE
    // ========================================================

    pub(crate) async fn download_file(
        &self,
        repository_path: &str,
    ) -> Result<Vec<u8>, String> {
        let url = self.create_file_url(repository_path)?;

        println!("[GitHub] Request: {}", url);

        for attempt in 1..=MAX_ATTEMPTS {
            let response = self.client
                .get(url.clone())
                .bearer_auth(&self.token)
                .header("Accept", "application/vnd.github.raw+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .send()
                .await;

            match response {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        return response
                            .bytes()
                            .await
                            .map(|bytes| bytes.to_vec())
                            .map_err(|error| {
                                format!(
                                    "GitHub-Datei konnte nicht gelesen werden: {}",
                                    error
                                )
                            });
                    }

                    let headers = response.headers().clone();
                    let details = response.text().await.unwrap_or_default();

                    let rate_limited =
                        status == StatusCode::TOO_MANY_REQUESTS
                        || (
                            status == StatusCode::FORBIDDEN
                            && Self::is_rate_limit_response(&headers, &details)
                        );

                    if !Self::should_retry(status, &headers, &details) {
                        return Err(format!(
                            "GitHub-Download fehlgeschlagen (HTTP {}): {}",
                            status,
                            details
                        ));
                    }

                    if attempt == MAX_ATTEMPTS {
                        return Err(format!(
                            "GitHub-Download nach {} Versuchen fehlgeschlagen (HTTP {}): {}",
                            MAX_ATTEMPTS,
                            status,
                            details
                        ));
                    }

                    let delay = Self::retry_delay(
                        &headers,
                        attempt,
                        rate_limited,
                    );

                    if delay.as_secs() > MAX_RETRY_WAIT {
                        return Err(format!(
                            "GitHub-Rate-Limit erreicht. Bitte warte mindestens {} Sekunden und versuche es erneut.",
                            delay.as_secs()
                        ));
                    }

                    println!(
                        "[GitHub] HTTP {}. Versuch {}/{}. Neuer Versuch in {} Sekunden.",
                        status,
                        attempt + 1,
                        MAX_ATTEMPTS,
                        delay.as_secs()
                    );

                    tokio::time::sleep(delay).await;
                }

                Err(error) => {
                    if attempt == MAX_ATTEMPTS {
                        return Err(format!(
                            "GitHub-Verbindung nach {} Versuchen fehlgeschlagen: {}",
                            MAX_ATTEMPTS,
                            error
                        ));
                    }

                    let delay = Duration::from_secs(1 << (attempt - 1));

                    println!(
                        "[GitHub] Verbindungsfehler. Versuch {}/{}. Neuer Versuch in {} Sekunden: {}",
                        attempt + 1,
                        MAX_ATTEMPTS,
                        delay.as_secs(),
                        error
                    );

                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err("GitHub-Download unerwartet abgebrochen.".to_string())
    }

    // ========================================================
    // FETCH MANIFEST
    // ========================================================

    pub(crate) async fn fetch_manifest(
        &self,
        modpack_id: &str,
    ) -> Result<ModpackManifest, String> {
        if modpack_id.is_empty()
            || modpack_id.contains('/')
            || modpack_id.contains('\\')
            || modpack_id == "."
            || modpack_id == ".."
        {
            return Err(format!(
                "Ungültige Modpack-ID: {}",
                modpack_id
            ));
        }

        let manifest_path = format!("{}/manifest.json", modpack_id);

        let bytes = self.download_file(&manifest_path).await?;

        let json = String::from_utf8(bytes)
            .map_err(|error| {
                format!(
                    "GitHub-Manifest enthält ungültiges UTF-8: {}",
                    error
                )
            })?;

        super::manifest::parse_manifest(&json)
    }
}
