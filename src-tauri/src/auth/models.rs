
use serde::Serialize;

// ============================================================
// PUBLIC ACCOUNT DATA
// ============================================================

#[derive(Debug, Clone, Serialize)]
pub struct AccountInfo {
    pub username: String,
    pub uuid: String,
}
