# User Guide

This guide describes DeployDesk from the operator's point of view.

## What DeployDesk Manages

Each project stores:

- Local project path.
- Build command.
- Build output directory.
- Deployment mode.
- SSH host, port, username, and authentication input.
- Remote deployment path.
- Symlink name, defaulting to `current`.
- Maximum number of releases to keep.

Configuration is stored locally in `projects.json` under the application config directory. On Windows, the directory is:

```text
%APPDATA%\com.deploydesk.app
```

## Deployment Modes

`deploy_mode` controls what happens before upload:

- `build`: run the configured local build command, then upload the configured output directory.
- `upload`: skip the build command and upload the configured output directory directly.

Use `build` when DeployDesk should create fresh static assets. Use `upload` when another process already produced the assets.

## Publish Flow

When publishing a project, DeployDesk:

1. Validates the local and remote settings.
2. Runs the local build command when `deploy_mode` is `build`.
3. Archives the configured output directory.
4. Uploads the archive over SFTP.
5. Extracts it into a new remote release directory.
6. Writes `RELEASE_NOTE.txt` when a release note is provided.
7. Updates the configured symlink.
8. Cleans up old releases according to `max_releases`.
9. Writes a local operation log entry.

The remote web server should point at the symlink directory, not at a specific timestamped release.

## Release History and Rollback

Release history reads timestamped directories under the remote `releases/` directory and detects the release currently pointed to by the configured symlink.

Rollback switches the symlink back to a selected release. It does not rebuild or re-upload local files.

## Local Logs

DeployDesk keeps the latest 100 operation log records locally. Logs are intended for operator review and should not be committed to the repository.

## Credentials

The `password_or_key` field may contain an SSH password or a private key path. Do not include real credentials, private keys, server addresses, or customer release notes in issues, screenshots, logs, or public documentation.
