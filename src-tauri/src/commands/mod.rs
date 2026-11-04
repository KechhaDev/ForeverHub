
pub(crate) mod auth;
pub(crate) mod github;
pub(crate) mod launcher;
pub(crate) mod profiles;
pub(crate) mod storage;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::account::LaunchSession;

pub(crate) struct AuthState {
    pub(crate) session: Mutex<Option<LaunchSession>>,
    pub(crate) account: Mutex<Option<crate::auth::AccountInfo>>,

    // Profile, für die aktuell ein Minecraft-Prozess läuft
    pub(crate) running_profiles: Arc<Mutex<HashSet<String>>>,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            session: Mutex::new(None),
            account: Mutex::new(None),
            running_profiles: Arc::new(Mutex::new(HashSet::new())),
        }
    }
}
