
use keyring::Entry;

const SERVICE_NAME: &str = "ForeverHub";
const CREDENTIAL_NAME: &str = "MicrosoftRefreshToken";

// ============================================================
// CREDENTIAL ENTRY
// ============================================================

fn credential_entry() -> Result<Entry, String> {
    Entry::new(SERVICE_NAME, CREDENTIAL_NAME)
        .map_err(|error| {
            format!(
                "Windows Credential Manager konnte nicht geöffnet werden: {}",
                error
            )
        })
}

// ============================================================
// SAVE REFRESH TOKEN
// ============================================================

pub(super) fn save_refresh_token(
    refresh_token: &str,
) -> Result<(), String> {
    let entry = credential_entry()?;

    entry.set_password(refresh_token)
        .map_err(|error| {
            format!(
                "Microsoft-Anmeldedaten konnten nicht gespeichert werden: {}",
                error
            )
        })
}

// ============================================================
// LOAD REFRESH TOKEN
// ============================================================

pub(super) fn load_refresh_token() -> Result<Option<String>, String> {
    let entry = credential_entry()?;

    match entry.get_password() {
        Ok(token) => Ok(Some(token)),

        Err(keyring::Error::NoEntry) => Ok(None),

        Err(error) => Err(format!(
            "Gespeicherte Microsoft-Anmeldedaten konnten nicht gelesen werden: {}",
            error
        )),
    }
}

// ============================================================
// DELETE REFRESH TOKEN
// ============================================================

pub(super) fn delete_refresh_token() -> Result<(), String> {
    let entry = credential_entry()?;

    match entry.delete_credential() {
        Ok(()) => Ok(()),

        Err(keyring::Error::NoEntry) => Ok(()),

        Err(error) => Err(format!(
            "Gespeicherte Microsoft-Anmeldedaten konnten nicht gelöscht werden: {}",
            error
        )),
    }
}
