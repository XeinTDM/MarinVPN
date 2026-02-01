# MarinVPN Architecture & Security Roadmap

This document outlines the architectural choices and implemented security measures that ensure MarinVPN remains a TOP tier VPN in terms of privacy, security, and anonymity.

## 1. Privacy & Anonymity

### Blind Signature Authentication (Implemented)
Account numbers are completely decoupled from VPN sessions using a Blind Signature model:
1. **Issuance:** User authenticates with their account number and requests a blinded token. The server signs the blinded token.
2. **Redemption:** The client unblinds the token and presents it to the server when requesting a VPN configuration.
3. **Unlinkability:** The server verifies its signature but cannot link the redeemed token back to the account that requested it.

### Peer Management & Lifecycle
- **Shared Session Store:** Peer public keys, assigned internal IPs, used blind tokens, and attestation nonces are stored in the primary database so replay protection survives restarts and multi-instance deployments.
- **Session Lifecycle:** A background task purges stale VPN sessions, tokens, and nonces every 24 hours to keep retention minimal.
- **Unlinkability:** The database maintains no relationship between `account_number` and `peer_pub_key`.

### Daita (Defense Against AI-guided Traffic Analysis)
- **Realistic Traffic Shaping:** Unlike simple noise injection, MarinVPN's Daita mimics real-world traffic patterns (Browsing, Media Streaming, and Heartbeats) with variable packet sizes and randomized timing to defeat advanced statistical analysis.
- **Target Obfuscation:** Noise traffic is routed to common public DNS providers and various infrastructure endpoints to blend in with standard background internet noise.

## 2. Security

### Dynamic Client Attestation
- **Ed25519 Request Signing:** Each request is signed with a device attestation key and verified server-side.
- **Replay Protection:** The server enforces a strict 60-second validity window and one-time nonce usage.

### Admin Endpoint Guarding
- **Admin Token Enforcement:** Metrics and API docs require an admin token via `X-Admin-Token` or `Authorization: Bearer`.
- **Proxy-Aware Allowlisting:** When deployed behind a trusted proxy, client IPs are checked against CIDR allowlists to prevent spoofed `X-Forwarded-For` headers.

### Token Lifecycle
- **Short-Lived Access Tokens:** Access tokens expire quickly to reduce blast radius.
- **Refresh Tokens:** Long-lived refresh tokens are rotated on use and stored hashed per device in the database.

### Fail-Closed Kill Switch & Leak Protection
- **Windows Lockdown:** Implements a strict "Fail-Closed" policy using the Windows Filtering Platform (WFP). All outbound traffic is blocked by default, with an explicit whitelist only for the VPN endpoint and tunnel interfaces.
- **Linux Nftables:** Uses `nftables` to enforce a drop-by-default policy, including explicit IPv6 blocking.
- **DNS Leak Protection:** Forcefully blocks outbound traffic on port 53 (UDP/TCP) for all physical network adapters, ensuring DNS queries *must* traverse the encrypted tunnel.

### Post-Quantum Cryptography (PQC)
- **Quantum Resistance:** Supports ML-KEM-768 for hybrid key exchange. WireGuard PSKs are derived from a quantum-resistant handshake to protect today's traffic against future decryption by quantum computers.

## 3. Censorship Circumvention (Stealth Mode)

### Advanced Obfuscation (Implemented)
- **Automatic Stealth Discovery:** An intelligent failover system that automatically cycles through available obfuscation methods (LWO → QUIC → WebSocket) to find the most effective path for the current network.
- **LWO (Lightweight WireGuard Obfuscation):** A low-overhead header shuffling technique designed to bypass protocol-based fingerprinting without the latency penalties of full TCP encapsulation.
- **WireGuard-over-WSS:** Supports wrapping WireGuard traffic in a WebSocket/TLS layer using `wstunnel`.
- **UDP-over-TCP:** Provides raw TCP encapsulation for WireGuard packets using `wstunnel` in TCP mode. This is useful for networks where all UDP traffic is blocked but non-HTTPS TCP is allowed.
- **Shadowsocks (AEAD):** Integrated support for Shadowsocks (using AES-256-GCM) as a secondary stealth layer.
- **QUIC (UDP-over-QUIC):** Leverages the QUIC protocol (HTTP/3) to wrap VPN traffic. This is highly effective against ISP throttling of standard UDP and provides better performance on lossy networks by utilizing QUIC's superior congestion control and stream multiplexing.

### DNS-over-HTTPS (DoH) Fallback
- **Censorship Resilience:** The client includes a built-in DoH resolver (using Cloudflare/Google infrastructure) to resolve MarinVPN API endpoints. This bypasses ISP-level DNS hijacking or blocking.

### Failover & Server Hopping
- **Health Monitoring:** Continuous end-to-end health checks verify tunnel connectivity. If a "Silent Dead" tunnel is detected, the client automatically re-scans for the best available server and hops to a new entry point.

## 4. Usability

### Multi-hop (Double VPN)
- **Nested Tunnels:** Support for nesting an exit tunnel inside an entry tunnel directly within the client logic, providing an extra layer of anonymity (Entry → Exit).
