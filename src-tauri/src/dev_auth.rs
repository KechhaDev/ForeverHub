use crate::account::LaunchSession;

pub(crate) fn create_session() -> LaunchSession {
    LaunchSession {
        username: "ForeverHubTest".to_string(),
        uuid: "d2f1a4b7c8e943b5a6d7e8f901234567".to_string(),
        access_token: "0".to_string(),
        user_type: "legacy".to_string(),
        client_id: String::new(),
        xuid: String::new(),
    }
}