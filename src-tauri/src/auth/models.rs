
use serde::Serialize;

// ============================================================
// PUBLIC ACCOUNT DATA
// ============================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    pub username: String,
    pub uuid: String,
    pub skin_url: Option<String>,
}
