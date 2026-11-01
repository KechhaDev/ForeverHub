use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct LaunchSession {
    pub username: String,
    pub uuid: String,
    pub access_token: String,

    #[serde(default = "default_user_type")]
    pub user_type: String,

    #[serde(default)]
    pub client_id: String,

    #[serde(default)]
    pub xuid: String,
}

fn default_user_type() -> String {
    "msa".to_string()
}