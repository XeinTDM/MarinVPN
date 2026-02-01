# MarinVPN Production Checklist

This checklist captures the minimum steps for a production‑ready deployment.

## 1) Secrets and Configuration

- Set strong secrets (64+ chars each):
  - `APP__AUTH__JWT_SECRET`
  - `APP__AUTH__ACCOUNT_SALT`
  - `APP__AUTH__PANIC_KEY`
- Use a real Postgres database:
  - `APP__DATABASE__URL=postgres://user:pass@db-host:5432/marinvpn`
- Bind to a public interface:
  - `APP__SERVER__HOST=0.0.0.0`
- Enable `RUN_MODE=production` (or `APP_ENV=production`)
- Set a strict body limit if needed:
  - `APP__SERVER__MAX_BODY_BYTES=262144`
- Protect admin endpoints:
  - `APP__SERVER__ADMIN_TOKEN=<long random token>`
- Lock down metrics by IP:
  - `APP__SERVER__METRICS_ALLOWLIST=10.0.0.5,10.0.0.6`
  - `APP__SERVER__TRUSTED_PROXY_HOPS=1` (if behind a proxy)
  - `APP__SERVER__TRUSTED_PROXY_CIDRS=10.0.0.0/24,192.168.0.0/16`
- Admin endpoints (`/metrics`, `/swagger-ui`, `/api-docs`) require:
  - `X-Admin-Token: <token>` or `Authorization: Bearer <token>`
  - Client IP on the allowlist (if set)

## 2) TLS/HTTPS (Required)

Terminate TLS in front of the API. Use a reverse proxy:
- Caddy: `docs/Caddyfile.example`
- Nginx: `docs/nginx.example.conf`

## 3) Database

- Run migrations on deploy:
  - `sqlx migrate run` (or rely on server start‑up migrations)
- Ensure backups + monitoring

## 4) Logging & Observability

- Set `APP__SERVER__LOG_LEVEL=info` (or stricter)
- Capture structured logs and metrics (`/metrics` protected by network policy)

## 5) Hardening

- Run as a non‑root service account
- Firewall restricts inbound to 443 (proxy) and health checks
- Restrict access to `/metrics`, `/swagger-ui`, `/api-docs`
- Store blind/support keys in a locked‑down directory:
  - `MARIN_KEY_DIR=/var/lib/marinvpn/keys` (ensure 700 on dir, 600 on files)
- To rotate admin token without restart (Unix):
  - Update env values and send `SIGHUP` to the server process

## 6) Operational

- Health checks: `/health`
- Smoke test: login, config fetch, connect/disconnect
