
    use std::path::{Path, PathBuf};

    use std::io::Read;
    use sha1::{Digest, Sha1};

    use std::sync::atomic::{AtomicU64, Ordering};

    static DOWNLOAD_COUNTER: AtomicU64 = AtomicU64::new(0);

    // ============================================================
    // HASH AND FILE VERIFICATION
    // ============================================================

    fn calculate_sha1(bytes: &[u8]) -> String {
        let mut hasher = Sha1::new();
        hasher.update(bytes);

        format!("{:x}", hasher.finalize())
    }

    
    fn verify_file(
        path: &Path,
        expected_sha1: &str,
        expected_size: u64,
    ) -> bool {
        let Ok(metadata) = std::fs::metadata(path) else {
            return false;
        };

        if metadata.len() != expected_size {
            return false;
        }

        let Ok(mut file) = std::fs::File::open(path) else {
            return false;
        };

        let mut hasher = Sha1::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(bytes) => bytes,
                Err(_) => return false,
            };

            hasher.update(&buffer[..bytes_read]);
        }

        format!("{:x}", hasher.finalize()) == expected_sha1
    }


    // ============================================================
    // VERIFIED DOWNLOADS
    // ============================================================

    pub(crate) async fn download_verified(
        client: &reqwest::Client,
        url: &str,
        destination: &Path,
        expected_sha1: &str,
        expected_size: u64,
    ) -> Result<(), String> {
        if verify_file(destination, expected_sha1, expected_size) {
            return Ok(());
        }

        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| error.to_string())?;
        }

        println!("Download: {}", url);

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|error| error.to_string())?;

        if !response.status().is_success() {
            return Err(format!(
                "Download fehlgeschlagen: HTTP {} bei {}",
                response.status(),
                url
            ));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|error| error.to_string())?;

        if bytes.len() as u64 != expected_size {
            return Err(format!(
                "Falsche Dateigröße: erwartet {} Bytes, erhalten {} Bytes",
                expected_size,
                bytes.len()
            ));
        }

        if calculate_sha1(&bytes) != expected_sha1 {
            return Err(format!(
                "SHA-1-Prüfung fehlgeschlagen: {}",
                url
            ));
        }

        let counter = DOWNLOAD_COUNTER.fetch_add(1, Ordering::Relaxed);

        let file_name = destination
            .file_name()
            .ok_or("Ungültiger Dateiname")?
            .to_string_lossy();

        let temporary_path = destination.with_file_name(format!(
            "{}.{}.{}.part",
            file_name,
            std::process::id(),
            counter
        ));

        
        std::fs::write(&temporary_path, &bytes)
            .map_err(|error| error.to_string())?;

        let backup_path = destination.with_file_name(format!(
            "{}.{}.{}.backup",
            file_name,
            std::process::id(),
            counter
        ));

        let had_original = destination.exists();

        if had_original {
            std::fs::rename(destination, &backup_path)
                .map_err(|error| error.to_string())?;
        }

        if let Err(error) = std::fs::rename(&temporary_path, destination) {
            if had_original {
                let _ = std::fs::rename(&backup_path, destination);
            }

            let _ = std::fs::remove_file(&temporary_path);

            return Err(format!(
                "Download-Datei konnte nicht installiert werden: {}",
                error
            ));
        }

        if had_original {
            let _ = std::fs::remove_file(&backup_path);
        }


        Ok(())
    }

    // ============================================================
    // SAFE FILE PATHS
    // ============================================================

    pub(crate) fn safe_relative_path(
        path: &str,
    ) -> Result<PathBuf, String> {
        let relative = Path::new(path);

        if relative.as_os_str().is_empty()
            || relative.components().any(|component| {
                !matches!(
                    component,
                    std::path::Component::Normal(_)
                )
            })
        {
            return Err(format!(
                "Ungültiger Download-Pfad: {}",
                path
            ));
        }

        Ok(relative.to_path_buf())
    }

    // ============================================================
    // MAVEN PATHS
    // ============================================================

    pub(crate) fn maven_path(
        coordinate: &str,
    ) -> Result<PathBuf, String> {
        let (coordinate, extension) = match coordinate.split_once('@') {
            Some((coordinate, extension)) => (coordinate, extension),
            None => (coordinate, "jar"),
        };

        let parts: Vec<&str> = coordinate.split(':').collect();

        if parts.len() < 3 || parts.len() > 4 {
            return Err(format!(
                "Ungültige Maven-Koordinate: {}",
                coordinate
            ));
        }

        let group = parts[0];
        let artifact = parts[1];
        let version = parts[2];

        let valid_component = |value: &str| {
                !value.is_empty()
                    && value != "."
                    && value != ".."
                    && value.chars().all(|character| {
                        character.is_ascii_alphanumeric()
                            || matches!(character, '.' | '_' | '-' | '+')
                    })
            };

        if !group.split('.').all(valid_component)
            || !valid_component(artifact)
            || !valid_component(version)
            || (parts.len() == 4 && !valid_component(parts[3]))
        {
            return Err(format!(
                "Ungültige Maven-Koordinate: {}",
                coordinate
            ));
        }

        let classifier = if parts.len() == 4 {
            format!("-{}", parts[3])
        } else {
            String::new()
        };

        if !valid_component(extension) {
            return Err(format!(
                "Ungültige Maven-Erweiterung: {}",
                extension
            ));
        }

        let group_path = group.replace('.', "/");

        let file_name = format!(
            "{}-{}{}.{}",
            artifact,
            version,
            classifier,
            extension
        );

        let path = PathBuf::from(group_path)
            .join(artifact)
            .join(version)
            .join(file_name);

        safe_relative_path(&path.to_string_lossy())
    }
        