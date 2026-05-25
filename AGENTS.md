# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust workspace for the `rs` CLI plus a Roblox Studio Luau plugin.

- `crates/rs/src/main.rs` defines the CLI entrypoint.
- `crates/rs/src/cli/` contains command implementations such as `list`, `doctor`, `import_asset`, and `package`.
- `crates/rs/src/bridge/` contains the local HTTP bridge and Studio registration logic.
- `crates/rs/src/protocol/` contains shared request/response message types and protocol tests.
- `plugin/src/` contains the Rojo-built Studio plugin handlers and helpers.
- `docs/` holds architecture, protocol, transfer-format, and feature documentation.
- `examples/` contains sample assets and helper scripts.

Generated outputs belong in `target/` or `plugin/rs-bridge-plugin.rbxmx`; do not treat them as source of truth.

## Build, Test, and Development Commands

- `cargo build` builds the Rust workspace in debug mode.
- `cargo build --release` builds `target/release/rs.exe` for local use.
- `cargo test` runs Rust unit and integration tests.
- `cargo fmt` formats Rust code with rustfmt.
- `rojo build plugin/default.project.json --output plugin/rs-bridge-plugin.rbxmx` builds the Studio plugin bundle.
- `target\release\rs.exe doctor` checks bridge, plugin, connected Studio sessions, and protocol compatibility.

Use `RS_BRIDGE_PORT` or CLI `--port` only where existing commands support it.

## Coding Style & Naming Conventions

Follow Rust 2021 conventions and rustfmt output. Use `snake_case` for modules, functions, and CLI implementation files; use `PascalCase` for Rust types and enum variants. Keep command modules small and explicit.

Luau plugin files use clear `PascalCase.lua` module names and handler modules under `plugin/src/Handlers/`. Keep plugin behavior deterministic and return structured JSON-compatible responses to the bridge.

## Testing Guidelines

Add Rust tests close to the behavior being changed, following existing module-test patterns such as `protocol/tests.rs`. Prefer tests that validate protocol shape, CLI parsing, serialization, and error paths without requiring a live Studio session. For live Studio behavior, document the manual command used, for example `rs smoke regression --studio <name> --out smoke.json`.

## Commit & Pull Request Guidelines

Recent history uses concise Conventional Commit-style subjects such as `feat: import png images as studio ui` and `fix: preserve meshpart refs during transfer`. Keep commits scoped to one logical change.

Pull requests should include a short description, validation commands and results, linked issues when relevant, and screenshots or Studio output only for visible plugin or import behavior.

## Security & Configuration Tips

Never commit Roblox API keys, local profile data, generated package contents with secrets, or user-specific Studio paths. Keep bridge access local to `127.0.0.1` unless an explicit, reviewed change requires otherwise.
