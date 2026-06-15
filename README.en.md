# DeployDesk

[中文](README.md) | English

DeployDesk, known in Chinese as 发布舱, is a **pure Rust** desktop app for managing web frontend builds, static asset uploads, remote releases, and one-click rollbacks. The current implementation uses Slint for the native desktop UI and no longer uses Node.js, Vue, Vite, or Tauri as runtime dependencies.

It uses a `releases/<timestamp>` plus symlink deployment model. Each publish creates an isolated release directory, while the web server points to a stable symlink such as `current`. Publishing and rollback are both handled by updating that symlink, making historical releases easy to keep and restore.

See [docs/index.md](docs/index.md) for the full documentation map.

## Features

- Project management: manage local paths, build commands, output directories, and remote server settings for multiple frontend projects.
- Local builds: run a custom build command such as `npm run build`, `pnpm build`, or any shell command before publishing.
- Direct uploads: skip the build step and upload an existing static asset directory.
- SSH/SFTP deployment: connect with password authentication or a private key path and upload artifacts over SFTP.
- Symlink-based publishing: create timestamped release directories and update a configurable symlink.
- Release notes: write optional publish notes to remote `RELEASE_NOTE.txt` files.
- Release history: list remote releases, show the current release, and display notes.
- One-click rollback: point the remote symlink back to a selected historical release.
- Operation logs: keep the latest 100 publish and rollback records locally.
- Windows installer: build an offline NSIS installer with desktop, start menu, and uninstall entries.

## Tech Stack

- Desktop UI: Rust, Slint
- Deployment: ssh2, tar, flate2, chrono
- Local data: serde, serde_json, uuid
- File dialogs: rfd
- Windows packaging: NSIS

## Requirements

- Rust 1.80 or later
- Windows 10/11, macOS, or a Linux desktop environment
- A target server with SSH, SFTP, `tar`, and `ln -sfn`
- NSIS with `makensis.exe` in PATH, or in the default install location, when building the Windows installer

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

# Build the Windows NSIS installer
powershell -ExecutionPolicy Bypass -File scripts/build-nsis.ps1
```

Release artifacts are written under `target/release/`.
NSIS installers are written under `dist/DeployDesk-Setup-<version>.exe`, and that directory is ignored by `.gitignore`.
See [docs/windows-installer.md](docs/windows-installer.md) for packaging and offline installation details.

## Project Structure

```text
.
├── src/
│   ├── main.rs
│   ├── app.rs
│   ├── app.slint
│   ├── models.rs
│   ├── storage.rs
│   ├── deploy.rs
│   └── logs.rs
├── docs/
├── packaging/
│   └── windows/
├── scripts/
├── Cargo.toml
└── README.md
```

Main documents:

- [docs/user-guide.md](docs/user-guide.md): project configuration, deployment modes, release history, rollback, and local data.
- [docs/deployment-workflow.md](docs/deployment-workflow.md): build, upload, release, cleanup, and rollback flow.
- [docs/development.md](docs/development.md): development commands, project structure, and validation notes.
- [docs/windows-installer.md](docs/windows-installer.md): NSIS installer build and install behavior.
- [docs/security-notes.md](docs/security-notes.md): credentials, remote commands, and public repository checklist.

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

On Windows, if a running `deploydesk.exe` locks the default `target/` directory, see [docs/development.md](docs/development.md) for validation with a separate target directory.

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

## License

This project is released under the [MIT License](LICENSE).
