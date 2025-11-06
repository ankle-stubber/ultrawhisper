//! Telegram-related Tauri commands
//!
//! This module provides commands for:
//! - Storing/retrieving Telegram credentials via OS keychain
//! - Testing Telegram bot connections
//! - Managing Telegram destination configurations

use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Keychain service name for Telegram credentials
const KEYCHAIN_SERVICE: &str = "com.ultrawhisper.telegram";

/// Telegram credentials stored in keychain
#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramCredentials {
    pub bot_token: String,
}

/// Result of testing Telegram connection
#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramTestResult {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bot_username: Option<String>,
}

/// Store Telegram credentials in OS keychain
#[tauri::command]
pub fn store_telegram_credentials(
    credential_id: String,
    bot_token: String,
) -> Result<(), String> {
    log::info!("Storing Telegram credentials for: {}", credential_id);

    let entry = Entry::new(KEYCHAIN_SERVICE, &credential_id)
        .map_err(|e| format!("Failed to create keychain entry: {}", e))?;

    entry
        .set_password(&bot_token)
        .map_err(|e| format!("Failed to store credentials in keychain: {}", e))?;

    log::info!("Telegram credentials stored successfully");
    Ok(())
}

/// Retrieve Telegram credentials from OS keychain
#[tauri::command]
pub fn get_telegram_credentials(credential_id: String) -> Result<TelegramCredentials, String> {
    log::debug!("Retrieving Telegram credentials for: {}", credential_id);

    let entry = Entry::new(KEYCHAIN_SERVICE, &credential_id)
        .map_err(|e| format!("Failed to create keychain entry: {}", e))?;

    let bot_token = entry
        .get_password()
        .map_err(|e| format!("Failed to retrieve credentials from keychain: {}", e))?;

    Ok(TelegramCredentials { bot_token })
}

/// Delete Telegram credentials from OS keychain
#[tauri::command]
pub fn delete_telegram_credentials(credential_id: String) -> Result<(), String> {
    log::info!("Deleting Telegram credentials for: {}", credential_id);

    let entry = Entry::new(KEYCHAIN_SERVICE, &credential_id)
        .map_err(|e| format!("Failed to create keychain entry: {}", e))?;

    entry
        .delete_credential()
        .map_err(|e| format!("Failed to delete credentials from keychain: {}", e))?;

    log::info!("Telegram credentials deleted successfully");
    Ok(())
}

/// Test Telegram bot connection
#[tauri::command]
pub async fn test_telegram_connection(
    bot_token: String,
    chat_id: String,
) -> Result<TelegramTestResult, String> {
    log::info!("Testing Telegram connection for chat_id: {}", chat_id);

    // First, try to get bot info to validate the token
    let bot_info_url = format!("https://api.telegram.org/bot{}/getMe", bot_token);

    let client = reqwest::Client::new();

    // Get bot info
    let bot_response = client
        .get(&bot_info_url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Telegram API: {}", e))?;

    if !bot_response.status().is_success() {
        return Ok(TelegramTestResult {
            success: false,
            message: format!("Invalid bot token (HTTP {})", bot_response.status()),
            bot_username: None,
        });
    }

    let bot_data: serde_json::Value = bot_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse bot info: {}", e))?;

    let bot_username = bot_data["result"]["username"]
        .as_str()
        .map(|s| s.to_string());

    // Try to send a test message
    let send_url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

    let mut payload = HashMap::new();
    payload.insert("chat_id", chat_id.as_str());
    payload.insert("text", "✅ UltraWhisper connection test successful!");

    let send_response = client
        .post(&send_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to send test message: {}", e))?;

    if !send_response.status().is_success() {
        let error_text = send_response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        return Ok(TelegramTestResult {
            success: false,
            message: format!("Failed to send message to chat. Check chat_id. Error: {}", error_text),
            bot_username,
        });
    }

    log::info!("Telegram connection test successful");

    Ok(TelegramTestResult {
        success: true,
        message: format!(
            "Connection successful! Test message sent to chat {}",
            chat_id
        ),
        bot_username,
    })
}

/// Check if credentials exist for a given credential_id
#[tauri::command]
pub fn telegram_credentials_exist(credential_id: String) -> Result<bool, String> {
    let entry = Entry::new(KEYCHAIN_SERVICE, &credential_id)
        .map_err(|e| format!("Failed to create keychain entry: {}", e))?;

    match entry.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(format!("Failed to check credentials: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keychain_service_name() {
        assert_eq!(KEYCHAIN_SERVICE, "com.ultrawhisper.telegram");
    }

    // Note: Integration tests for keychain operations require actual OS keychain
    // and should be run manually or in a dedicated test environment
}
