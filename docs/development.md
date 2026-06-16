# Development Guide

DeployDesk is a pure Rust desktop application built with Slint.

## Project Map

- `src/main.rs`: Rust application entry point and Windows release subsystem setting.
- `src/app.rs`: Slint UI controller, callbacks, dialogs, and background task orchestration.
- `src/app.slint`: Slint window markup, pages, forms, and modal layouts.
- `src/models.rs`: Shared project, release, and log data structures.
- `src/storage.rs`: Local JSON persistence under the application config directory.
- `src/deploy.rs`: SSH connection, deployment, release listing, cleanup, and rollback.
- `src/logs.rs`: Local operation log persistence.
- `packaging/windows/deploydesk.nsi`: NSIS installer definition.
- `scripts/build-nsis.ps1`: Windows packaging script that builds release output and invokes NSIS.

## Commands

```bash
# Run the desktop app
cargo run

# Check the Rust project
cargo check

# Run unit tests
cargo test

# Check formatting
cargo fmt -- --check

# Build a release binary
cargo build --release

# Build the Windows NSIS installer
powershell -ExecutionPolicy Bypass -File scripts/build-nsis.ps1
```

## Windows Build Notes

Windows may lock files under the default `target/` directory when a running `deploydesk.exe` exists. If validation fails with access denied or locked output errors, use a separate target directory:

```powershell
cargo build --target-dir "$env:TEMP\deploydesk-codex-target"
```

The NSIS packaging script already builds with a separate target directory by default.

Release builds use the Windows GUI subsystem, so double-clicking `deploydesk.exe` should not open a console window. `cargo run` still runs through Cargo and may show terminal output during development.

## Repository Hygiene

- Keep generated build output under `target/`.
- Keep local installer output under `dist/`.
- Keep local runtime logs under `logs/`.
- Keep exported project configuration bundles, including `deploydesk-projects.json`, out of the repository.
- Do not commit credentials, private keys, customer release notes, generated installers, screenshots with secrets, or local machine paths.
- Update README or `docs/` when behavior, configuration, deployment, or packaging changes.
