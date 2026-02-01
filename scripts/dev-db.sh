#!/usr/bin/env sh
set -eu

docker compose up -d marinvpn-db >/dev/null

retries=30
while [ "$retries" -gt 0 ]; do
  cid="$(docker compose ps -q marinvpn-db || true)"
  if [ -n "$cid" ]; then
    status="$(docker inspect -f '{{.State.Health.Status}}' "$cid" 2>/dev/null || true)"
    if [ "$status" = "healthy" ]; then
      echo "Postgres is healthy."
      exit 0
    fi
  fi
  sleep 1
  retries=$((retries - 1))
done

echo "Postgres did not become healthy in time." >&2
exit 1
