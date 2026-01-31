# MarinVPN Architecture & Security Roadmap

This document outlines the architectural choices and future improvements to ensure MarinVPN remains a TOP tier VPN in terms of privacy, security, and anonymity.

## 1. Privacy & Anonymity

### Blind Signature Authentication (Proposed)
To completely decouple account numbers from VPN sessions, we are moving towards a Blind Signature model:
1. **Issuance:** User authenticates with their account number and requests a blinded token. The server signs the blinded token.
2. **Redemption:** The client unblinds the token and presents it to the server when requesting a VPN configuration.
3. **Unlinkability:** The server can verify its own signature but cannot link the redeemed token back to the account that requested it.

### Ephemeral Peer Management
- **RAM-only Storage:** Peer public keys and assigned internal IPs are stored in an in-memory SQLite database (`ephemeral_pool`). They are wiped automatically on server restart.
- **Unlinkability:** The database does not maintain any relationship between `account_number` and `peer_pub_key`.

### Daita (Data Anonymization)
- **Traffic Masking:** MarinVPN implements Daita, which injects randomized noise traffic (UDP packets to common DNS providers) during idle periods.
- **Timing Anonymity:** Noise packets are sent at randomized intervals with varying payload sizes to defeat advanced traffic analysis.

## 2. Security

### Post-Quantum Cryptography (PQC)
- **Quantum Resistance:** WireGuard configurations support a Preshared Key (PSK) that can be derived from a PQC key exchange (e.g., ML-KEM).
- **Metadata:** Configuration responses include PQC handshake details for client-side verification.

### Kill Switch & Leak Protection
- **Multi-OS Support:** Platform-specific implementations for Linux (`iptables`) and Windows (`netsh`/`WFP`) ensure no traffic leaves the device outside the VPN tunnel.
- **IPv6 Protection:** All IPv6 traffic is blocked by default to prevent leakage, as most VPN servers currently operate on IPv4.

## 3. Censorship Circumvention (Stealth Mode)

### Advanced Obfuscation
- **UDP-over-TCP/TLS:** Future support for wrapping WireGuard traffic in a TLS layer (via WSTunnel or similar) to bypass Deep Packet Inspection (DPI) in restrictive regimes.
- **Bridge Support:** Ability to connect via bridge nodes before reaching the entry VPN server.

## 4. Usability

### Multi-hop (Double VPN)
- **Nested Tunnels:** Support for nesting an exit tunnel inside an entry tunnel directly within the client logic, providing an extra layer of anonymity.
- **UI Integration:** Simple toggle for "Double VPN" with easy location selection.
