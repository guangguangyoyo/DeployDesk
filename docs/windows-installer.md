# Windows Installer

DeployDesk uses NSIS for the Windows installer.

## Prerequisites

- Rust toolchain for building the release binary.
- NSIS with `makensis.exe` available in `PATH`, or installed in one of the default locations:
  - `C:\Program Files\NSIS\makensis.exe`
  - `C:\Program Files (x86)\NSIS\makensis.exe`

## Build Command

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-nsis.ps1
```

The script:

1. Reads the package version from `Cargo.toml`.
2. Builds `deploydesk.exe` in release mode using a separate target directory.
3. Invokes `makensis.exe` with generated version and path definitions.
4. Writes the installer to `dist/DeployDesk-Setup-<version>.exe`.

To use a custom Rust target directory:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-nsis.ps1 -TargetDir "$env:TEMP\deploydesk-nsis-target"
```

## Installer Behavior

The installer is per-user and does not require administrator privileges by default.

It installs:

- `deploydesk.exe`
- `LICENSE`
- `uninstall.exe`
- Start menu shortcut
- Desktop shortcut
- Windows uninstall registry entries under the current user

The default install directory is:

```text
%LOCALAPPDATA%\Programs\DeployDesk
```

Uninstall removes installed program files and shortcuts. It does not remove application data under:

```text
%APPDATA%\com.deploydesk.app
```

## Offline Installation

The generated NSIS installer is an offline installer. It contains the built application executable and license file, so the target machine does not need Rust, Cargo, NSIS, Node.js, Vue, Vite, or Tauri.

The target machine must still be compatible with the built Windows binary. Publishing from DeployDesk also requires network access to the configured SSH/SFTP server.

## Release Hygiene

`dist/` is ignored by Git. Do not commit generated installers unless they are intentionally added as release assets through a release process.

Unsigned installers may trigger Windows SmartScreen or antivirus warnings. Code signing should be handled as a separate release step if public distribution requires it.
