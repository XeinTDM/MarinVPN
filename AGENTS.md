# Repository Guidelines

- `marinvpn-common/` holds shared models, helpers, and test utilities consumed by both client and server crates.
- `marinvpn-server/` is the Rust backend API; its `src/` folder contains handlers (auth, vpn), services, and models with additional integration tests under `tests/`.
- `marinvpn/` is the desktop application built with Dioxus (Rust + webview) where front-end components, hooks, services, and views live under `src/`, plus UI/state helpers.
- `docs/` and `gemini.md` capture architecture notes, reference flows, and reproducible Docker builds that should stay in sync with code changes.
- Keep workspace manifests (`Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`) in the root aligned with the versions used during local development (`stable` toolchain as of this repo’s lockfile).

## Build, Test, and Development Commands
- `cargo fmt --all`: format every crate; run it before commits.
- `cargo clippy --all -- -Dwarnings`: catch lints that could block CI.
- `cargo test --workspace`: run unit and integration tests across client, server, and common crates.
- `cargo build --workspace`: verify that both server and client compile with current dependencies.
- `cargo run -p marinvpn`: start the desktop client when iterating on UI/service changes.
- `cargo run -p marinvpn-server`: exercise API handlers when mocking back-end behavior.

## Coding Style & Naming Conventions
- Follow Rust idioms: snake_case for functions/modules, PascalCase for structs/enums, expressive constant names, and `use` statements grouped by std/third-party/local.
- Always run `cargo fmt` after editing Rust code to keep spacing consistent; the repo sticks with the default 4-space indentation and LF line endings (Git will warn otherwise).
- Keep shared code in `marinvpn-common` to avoid duplication (helpers for WireGuard configs, stats, etc.).
- Prefix UI components and hooks with their area (`components/`, `views/`, `services/`) and keep files focused on single responsibilities.

## Testing Guidelines
- Use `cargo test --workspace` for automated verification; address failing tests before submitting a change.
- Place unit tests alongside the source (`mod tests`) for small helpers; larger scenarios belong under `tests/` directories inside each crate.
- Name test functions to describe the expected behavior (e.g., `config_defaults_to_public_server`) and keep mock data deterministic.

## Commit & Pull Request Guidelines
- Mirror existing history by keeping the subject short and imperative—`feat: improve vpn dns handling`, `fix: cleanup systemd-resolved state`. Favor Conventional Commit prefixes when possible.
- Include a concise PR description, list key changes, mention related issues, and capture manual verification steps (screenshots for UI tweaks, command output for services).
- Reference the reviewed paths (e.g., `marinvpn/src/services/vpn.rs`) when documenting regressions or regressions fixed in the description.

## Security & Configuration Tips
- Never check in private keys, tokens, or production credentials; use environment variables or secure storage when configuring the WireGuard helper and DNS logic.
- Update `docs/` whenever configuration expectations change (new env vars, resolvectl behavior, etc.) so contributors have a single source of truth.
