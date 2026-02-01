#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_account_serialization() {
        let account = Account {
            account_number: "1234 5678 1234 5678".to_string(),
            expiry_date: 1738320000,
            created_at: 1738320000,
        };
        let serialized = serde_json::to_string(&account).unwrap();
        let deserialized: Account = serde_json::from_str(&serialized).unwrap();
        assert_eq!(account, deserialized);
    }

    #[test]
    fn test_login_request_validation() {
        #[cfg(feature = "validation")]
        {
            use validator::Validate;

            let req = LoginRequest {
                account_number: "ABCD".to_string(),
                device_pubkey: None,
                kick_device: None,
            };
            assert!(req.validate().is_err());

            let req = LoginRequest {
                account_number: "ABCD E2GH JK7M NPQR".to_string(),
                device_pubkey: Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string()),
                kick_device: None,
            };
            assert!(req.validate().is_ok());
        }
    }
}
