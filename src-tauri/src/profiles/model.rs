
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuthenticationMode {
    Microsoft,
    Offline,
}

impl Default for AuthenticationMode {
    fn default() -> Self {
        Self::Microsoft
    }
}

fn default_ram_gb() -> u32 {
    4
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Profile {
    pub(crate) name: String,

    pub(crate) minecraft_version: String,
    pub(crate) fabric_version: String,

    pub(crate) modpack_id: Option<String>,
    pub(crate) modpack_version: String,

    #[serde(default)]
    pub(crate) authentication_mode: AuthenticationMode,

    #[serde(default = "default_ram_gb")]
    pub(crate) ram_gb: u32,

    #[serde(skip)]
    pub(crate) install_dir: PathBuf,
}
