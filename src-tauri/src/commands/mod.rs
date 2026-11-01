
pub(crate) mod auth;
pub(crate) mod launcher;

use std::sync::Mutex;

use crate::account::LaunchSession;

pub(crate) struct AuthState {
    pub(crate) session: Mutex<Option<LaunchSession>>,
    pub(crate) account: Mutex<Option<crate::auth::AccountInfo>>,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            session: Mutex::new(None),
            account: Mutex::new(None),
        }
    }
}
