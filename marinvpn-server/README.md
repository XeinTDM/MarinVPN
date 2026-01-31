# MarinVPN Auth Server

A top-tier, high-performance authentication and configuration server for MarinVPN.

## Features

- **Robust Architecture:** Modular design with separated handlers, services, and models.
- **Persistent Storage:** SQLite database using `sqlx` for reliable data management.
- **Security:**
  - Rate limiting via `tower-governor`.
  - Account existence and expiration validation.
  - CORS and Timeout middleware.
- **Performance:** Asynchronous request handling with Axum and Tokio.
- **Observability:** Structured logging and HTTP request tracing.
- **API Versioning:** Future-proofed with `/api/v1` nesting.

## Tech Stack

- **Framework:** Axum (Rust)
- **Runtime:** Tokio
- **Database:** SQLite (sqlx)
- **Serialization:** Serde
- **Logging:** Tracing

## Getting Started

### Prerequisites

- Rust 1.75+
- `sqlx-cli` (optional, for migrations)

### Configuration

Create a `.env` file in this directory:

```env
PORT=3000
DATABASE_URL=sqlite:marinvpn.db
```

### Running the Server

```bash
cargo run
```

The server will automatically initialize the SQLite database (`marinvpn.db`) if it doesn't exist.

## API Endpoints

### Account
- `POST /api/v1/account/generate` - Create a new account
- `POST /api/v1/account/login` - Authenticate and register device
- `POST /api/v1/account/devices` - List registered devices
- `POST /api/v1/account/devices/remove` - De-register a device

### VPN
- `POST /api/v1/vpn/config` - Get WireGuard configuration
- `POST /api/v1/vpn/report` - Report a connectivity problem

### System
- `GET /health` - Server health check
