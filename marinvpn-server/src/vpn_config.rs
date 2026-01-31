pub use marinvpn_common::WireGuardConfig;

pub fn get_config_for_location(location: &str) -> WireGuardConfig {
    let country = location.split(',').next().unwrap_or("Sweden").trim();

    let endpoint = match country {
        "Sweden" => "se-sto.marinvpn.net:51820",
        "United States" => "us-nyc.marinvpn.net:51820",
        "Germany" => "de-fra.marinvpn.net:51820",
        "United Kingdom" => "gb-lon.marinvpn.net:51820",
        "Netherlands" => "nl-ams.marinvpn.net:51820",
        _ => "default.marinvpn.net:51820",
    };

    WireGuardConfig {
        private_key: "".to_string(),
        public_key: "SERVER_PUBLIC_KEY_FOR_PEER".to_string(),
        preshared_key: None,
        endpoint: endpoint.to_string(),
        allowed_ips: "0.0.0.0/0, ::/0".to_string(),
        address: "10.0.0.2/32, fc00::2/128".to_string(),
        dns: Some("1.1.1.1".to_string()),
        pqc_handshake: None,
        pqc_provider: None,
        pqc_ciphertext: None,
        obfuscation_key: None,
    }
}
