use std::process::Command;

use crate::error::AppError;

const KEYCHAIN_SERVICE: &str = "gemini-api-key-secrets";
const KEYCHAIN_ACCOUNT: &str = "gemini";

pub fn read_api_key_from_keychain() -> Result<String, AppError> {
    let output = Command::new("security")
        .args(["find-generic-password", "-s", KEYCHAIN_SERVICE, "-w"])
        .output()
        .map_err(|_| AppError::ApiKeyMissing)?;

    if !output.status.success() {
        return Err(AppError::ApiKeyMissing);
    }

    let key = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if key.is_empty() {
        Err(AppError::ApiKeyMissing)
    } else {
        Ok(key)
    }
}

pub fn save_api_key_to_keychain(key: &str) -> Result<(), AppError> {
    let status = Command::new("security")
        .args([
            "add-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            KEYCHAIN_ACCOUNT,
            "-w",
            key,
            "-U",
        ])
        .status()
        .map_err(|err| AppError::Message(format!("Error: failed to run security: {err}")))?;

    if status.success() {
        Ok(())
    } else {
        Err(AppError::Message(String::from(
            "Error: failed to save API key to keychain",
        )))
    }
}

pub fn mask_api_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    let last_len = chars.len().min(4);
    let first_len = chars.len().min(8).min(chars.len().saturating_sub(last_len));
    let first: String = chars[..first_len].iter().collect();
    let last: String = chars[chars.len().saturating_sub(last_len)..]
        .iter()
        .collect();
    format!("{first}...{last}")
}
