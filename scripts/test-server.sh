#!/usr/bin/env sh
set -eu

export TEST_DATABASE_URL="${TEST_DATABASE_URL:-postgres://marinvpn:marinvpn@127.0.0.1:5432/marinvpn}"

cargo test -p marinvpn-server --verbose
