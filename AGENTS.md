# Repository Guide

This repository contains DeployDesk, known in Chinese as 发布舱.

## Project Map

- `src/main.rs`: Rust application entry and native window setup.
- `src/app.rs`: Slint UI controller, callbacks, dialogs, and background task orchestration.
- `src/app.slint`: Slint window markup, pages, forms, and modal layouts.
- `src/models.rs`: Shared project, release, and log data structures.
- `src/storage.rs`: Local JSON persistence under the application config directory.
- `src/deploy.rs`: SSH connection, deployment, release listing, cleanup, and rollback.
- `src/logs.rs`: Local operation log persistence.
- `docs/deployment-workflow.md`: Build, upload, release, cleanup, and rollback workflow.
- `docs/security-notes.md`: Credential, remote command, and release cleanup notes.

## Commands

- Run the desktop app: `cargo run`
- Rust check: `cargo check`
- Release build: `cargo build --release`

## Notes

- This is a pure Rust desktop app using Slint; do not add Node, Vue, Vite,
  or Tauri back as runtime dependencies unless explicitly requested.
- Project config and operation logs are stored under the local app config
  directory, using `com.deploydesk.app` for compatibility with the older Tauri
  build.
- Deployment uses `releases/<timestamp>` directories and updates a configurable
  symlink, defaulting to `current`.
- `deploy_mode` controls whether the app runs the configured local build command
  before uploading the configured output directory.
- `password_or_key` can contain a password or private key path. Do not log it or
  expose it outside the intended form field.
- Keep generated output, build artifacts, local logs, and screenshots out of the
  repository unless they are intentionally documented artifacts.
