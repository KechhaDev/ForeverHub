use std::path::Path;

use serde_json::Value;

use crate::account::LaunchSession;

use super::libraries::{launcher_features, rules_allow};

// ============================================================
// LAUNCH PATHS
// ============================================================

pub(super) struct LaunchPaths<'a> {
    pub(super) profile_dir: &'a Path,
    pub(super) assets_dir: &'a Path,
    pub(super) natives_dir: &'a Path,
    pub(super) libraries_dir: &'a Path,
    pub(super) version_id: &'a str,
    pub(super) version_type: &'a str,
    pub(super) asset_index: &'a str,
    pub(super) classpath: &'a str,
}

// ============================================================
// PLACEHOLDER RESOLUTION
// ============================================================

fn replace_placeholders(
    argument: &str,
    paths: &LaunchPaths<'_>,
    session: &LaunchSession,
) -> String {
    let replacements = [
        ("${auth_player_name}", session.username.as_str()),
        ("${auth_uuid}", session.uuid.as_str()),
        ("${auth_access_token}", session.access_token.as_str()),
        ("${user_type}", session.user_type.as_str()),
        ("${clientid}", session.client_id.as_str()),
        ("${auth_xuid}", session.xuid.as_str()),
        ("${version_name}", paths.version_id),
        ("${version_type}", paths.version_type),
        ("${assets_index_name}", paths.asset_index),
        ("${classpath}", paths.classpath),
        (
            "${classpath_separator}",
            if cfg!(target_os = "windows") { ";" } else { ":" },
        ),
        ("${launcher_name}", "ForeverHub"),
        ("${launcher_version}", "1.0.0"),
        ("${resolution_width}", "854"),
        ("${resolution_height}", "480"),
    ];

    let mut result = argument.to_string();

    for (placeholder, value) in replacements {
        result = result.replace(placeholder, value);
    }

    result = result.replace(
        "${game_directory}",
        &paths.profile_dir.to_string_lossy(),
    );

    result = result.replace(
        "${assets_root}",
        &paths.assets_dir.to_string_lossy(),
    );

    result = result.replace(
        "${natives_directory}",
        &paths.natives_dir.to_string_lossy(),
    );

    result = result.replace(
        "${library_directory}",
        &paths.libraries_dir.to_string_lossy(),
    );

    result
}

// ============================================================
// ARGUMENT COLLECTION
// ============================================================

pub(super) fn collect_arguments(
    metadata: &Value,
    argument_type: &str,
    paths: &LaunchPaths<'_>,
    session: &LaunchSession,
) -> Vec<String> {
    let Some(arguments) = metadata["arguments"][argument_type].as_array() else {
        return Vec::new();
    };

    let features = launcher_features();
    let mut result = Vec::new();

    for argument in arguments {
        let mut values = Vec::new();

        if let Some(value) = argument.as_str() {
            values.push(value.to_string());
        } else if let Some(rules) = argument["rules"].as_array() {
            if !rules_allow(rules, &features) {
                continue;
            }

            let value = &argument["value"];

            if let Some(value) = value.as_str() {
                values.push(value.to_string());
            } else if let Some(array) = value.as_array() {
                for item in array {
                    if let Some(item) = item.as_str() {
                        values.push(item.to_string());
                    }
                }
            }
        }

        for value in values {
            result.push(replace_placeholders(
                &value,
                paths,
                session,
            ));
        }
    }

    result
}