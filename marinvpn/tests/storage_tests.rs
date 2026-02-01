use keyring::Entry;
use marinvpn::storage::{load_config, save_config, AppConfig};
use serial_test::serial;
use std::fs;

const TEST_SERVICE: &str = "marinvpn";
const TEST_USER: &str = "active_user";

fn cleanup() {
    let path = marinvpn::storage::get_config_path();
    let _ = fs::remove_file(path);

    if let Ok(entry) = Entry::new(TEST_SERVICE, TEST_USER) {
        let _ = entry.delete_password();
    }
}

#[test]
#[serial]
fn test_save_and_load_config() {
    cleanup();

    let config = AppConfig {
        account_number: Some("1234 5678 1234 5678".to_string()),
        auth_token: None,
        refresh_token: None,
        account_expiry: Some(1738320000),
        device_name: Some("Test Device".to_string()),
        favorites: None,
        settings: None,
    };

    save_config(&config).expect("Failed to save config");

    let loaded = load_config();

    assert_eq!(
        loaded.account_number.as_deref(),
        Some("1234 5678 1234 5678")
    );
    assert_eq!(loaded.device_name.as_deref(), Some("Test Device"));
    assert_eq!(loaded.account_expiry, Some(1738320000));

    let path = marinvpn::storage::get_config_path();
    let content = fs::read_to_string(path).expect("Failed to read config file");
    assert!(
        !content.contains("1234 5678 1234 5678"),
        "Account number leaked to JSON!"
    );

    cleanup();
}

#[test]
#[serial]
fn test_migration_from_legacy_json() {
    cleanup();

    let legacy_json = r#"{
        "account_number": "LEAKY_ACCOUNT",
        "device_name": "Legacy Device"
    }"#;
    let path = marinvpn::storage::get_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("Failed to create config dir");
    }
    fs::write(&path, legacy_json).expect("Failed to write legacy json");

    let loaded = load_config();
    assert_eq!(loaded.account_number.as_deref(), Some("LEAKY_ACCOUNT"));

    let new_content = fs::read_to_string(&path).expect("Failed to read updated config");
    assert!(
        !new_content.contains("LEAKY_ACCOUNT"),
        "Migration failed to remove sensitive data from JSON"
    );

    let entry =
        Entry::new(TEST_SERVICE, TEST_USER).expect("Failed to access keyring for verification");
    let stored_pass = match entry.get_password() {
        Ok(pass) => pass,
        Err(err) => {
            eprintln!("Keyring unavailable; skipping keyring assertion: {err}");
            cleanup();
            return;
        }
    };
    assert_eq!(stored_pass, "LEAKY_ACCOUNT");

    cleanup();
}
