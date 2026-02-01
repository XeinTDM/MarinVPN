# MarinVPN Auth Server

A top-tier, high-performance authentication and configuration server for MarinVPN.

## Features

- **Robust Architecture:** Modular design with separated handlers, services, and models.
- **Persistent Storage:** PostgreSQL database using `sqlx` for reliable data management.
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
- **Database:** PostgreSQL (sqlx)
- **Serialization:** Serde
- **Logging:** Tracing

## Getting Started

### Prerequisites

- Rust 1.75+
- PostgreSQL 14+
- `sqlx-cli` (optional, for migrations)

### Configuration

Create a `.env` file in this directory:

```env
PORT=3000
APP__DATABASE__URL=postgres://marinvpn:marinvpn@127.0.0.1:5432/marinvpn
```

See `docs/PRODUCTION.md` for production checklist and TLS examples.

Optional server settings:

```env
APP__SERVER__MAX_BODY_BYTES=262144
APP__SERVER__ADMIN_TOKEN=replace-with-a-real-token
APP__SERVER__METRICS_ALLOWLIST=127.0.0.1
APP__SERVER__TRUSTED_PROXY_HOPS=1
APP__AUTH__JWT_SECRET=replace-with-a-real-secret
APP__AUTH__ACCOUNT_SALT=replace-with-a-real-salt
APP__AUTH__PANIC_KEY=replace-with-a-real-key
MARIN_KEY_DIR=/var/lib/marinvpn/keys
```

Admin token rotation (Unix):
- Update env values and send `SIGHUP` to reload `ADMIN_TOKEN` and allowlist.

### Running the Server

```bash
cargo run
```

The server will automatically apply migrations to the configured PostgreSQL database.

## Local Development (Recommended)

Start a local Postgres instance via Docker:

```bash
docker compose up -d marinvpn-db
```

Or use the helper script (Windows: `scripts/dev-db.ps1`):

```bash
scripts/dev-db.sh
```

Then run the server (defaults already point at the local container):

```bash
cargo run
```

### Tests

Set the test database URL before running integration tests:

```bash
TEST_DATABASE_URL=postgres://marinvpn:marinvpn@127.0.0.1:5432/marinvpn cargo test -p marinvpn-server
```

Or use the helper script (Windows: `scripts/test-server.ps1`):

```bash
scripts/test-server.sh
```

## API Endpoints

### Account
- `POST /api/v1/account/generate` - Create a new account
- `POST /api/v1/account/login` - Authenticate and register device
- `POST /api/v1/account/devices` - List registered devices
- `POST /api/v1/account/devices/remove` - De-register a device

### Auth
- `POST /api/v1/auth/refresh` - Refresh access token

### VPN
- `POST /api/v1/vpn/config` - Get WireGuard configuration
- `POST /api/v1/vpn/report` - Report a connectivity problem

### System
- `GET /health` - Server health check
