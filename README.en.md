# DeployDesk

[中文](README.md) | English

DeployDesk, known in Chinese as 发布舱, is a **pure Rust** desktop app for managing web frontend builds, static asset uploads, remote releases, and one-click rollbacks. The current implementation uses Slint for the native desktop UI and no longer depends on Node.js, Vue, Vite, or Tauri.

It uses a `releases/<timestamp>` plus symlink deployment model. Each publish creates an isolated release directory, while the web server points to a stable symlink such as `current`. Publishing and rollback are both handled by updating that symlink, making historical releases easy to keep and restore.

## Features

- Project management: manage local paths, build commands, output directories, and remote server settings for multiple frontend projects.
- Local builds: run a custom build command such as `npm run build`, `pnpm build`, or any shell command before publishing.
- Direct uploads: skip the build step and upload an existing static asset directory.
- SSH/SFTP deployment: connect with password or private key authentication and upload artifacts over SFTP.
- Symlink-based publishing: create timestamped release directories and update a configurable symlink.
- Release notes: write optional publish notes to remote `RELEASE_NOTE.txt` files.
- Release history: list remote releases, show the current release, and display notes.
- One-click rollback: point the remote symlink back to a selected historical release.
- Operation logs: keep the latest 100 publish and rollback records locally.

## Tech Stack

- Desktop UI: Rust, Slint
- Deployment: ssh2, tar, flate2, chrono
- Local data: serde, serde_json, uuid
- File dialogs: rfd

## Requirements

- Rust 1.80 or later
- Windows 10/11, macOS, or a Linux desktop environment
- A target server with SSH, SFTP, `tar`, and `ln -sfn`

## Quick Start

```bash
cargo run
```

## Common Commands

```bash
# Check the Rust project
cargo check

# Run the desktop app
cargo run

# Build a release binary
cargo build --release
```

Release artifacts are written under `target/release/`.

## Project Structure

```text
.
├── src/
│   ├── main.rs
│   ├── app.rs
│   ├── models.rs
│   ├── storage.rs
│   ├── deploy.rs
│   └── logs.rs
├── docs/
├── Cargo.toml
└── README.md
```

## Deployment Model

Example remote directory layout:

```text
/var/www/project/
├── releases/
│   ├── 20260306_150000/
│   ├── 20260306_160000/
│   └── 20260306_170000/
└── current -> releases/20260306_170000/
```

Nginx or another web server only needs to point at the symlink directory. When publishing a new version, DeployDesk uploads files into a new release directory and updates the symlink. Rollback only switches the symlink back.

See [docs/deployment-workflow.md](docs/deployment-workflow.md) for details.

## Configuration and Data

Project configuration and operation logs are stored in the local app configuration directory. On Windows:

```text
%APPDATA%\com.deploydesk.app
```

This keeps compatibility with the previous Tauri app identifier:

- `projects.json`: project configuration.
- `logs.json`: latest 100 operation records.

The current `password_or_key` field may contain an SSH password or a private key path. Do not publish real server addresses, usernames, passwords, private keys, screenshots, or logs that expose sensitive infrastructure details.

See [docs/security-notes.md](docs/security-notes.md) for more information.

## Development and Contributing

Issues and pull requests are welcome. Before submitting changes, run at least:

```bash
cargo check
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

## License

This project is released under the [MIT License](LICENSE).
