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

Configuration is stored locally in `projects.json` under the application config
directory. Server passwords are protected before they are written to this file.
On Windows, the directory is:

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

1. Validates the local project directory, build command, output directory, and remote settings.
2. Runs the local build command when `deploy_mode` is `build`.
3. Archives the configured output directory.
4. Uploads the archive over SFTP.
5. Extracts it into a new remote release directory.
6. Writes `RELEASE_NOTE.txt` when a release note is provided.
7. Updates the configured symlink.
8. Cleans up old releases according to `max_releases`.
9. Writes a local operation log entry.

The remote web server should point at the symlink directory, not at a specific
timestamped release.

The configured symlink name must not already be used by a normal directory on
the remote server. If a same-name directory exists, DeployDesk stops the publish
or rollback operation and asks you to delete or rename that directory, or change
the symlink name.

DeployDesk stops the publish task before connecting to the server when:

- the local project directory does not exist;
- the output path is missing or is not a directory;
- a common package-manager build script such as `npm run build` is not defined
  in `package.json`.

## Release History and Rollback

Release history reads timestamped directories under the remote `releases/`
directory and detects the release currently pointed to by the configured
symlink.

Rollback switches the symlink back to a selected release. It does not rebuild or
re-upload local files.

## Settings

The settings page can export all project configuration to a JSON file and import
a JSON file back into DeployDesk. Export requires an operator-entered
configuration package password. Importing project configuration replaces the
current local project list and requires the same password.

Password-based projects store passwords protected with the configuration package
password and restore them during import. Key-based projects include protected
private key file content and restore the key into the local app configuration
directory during import.

After import, DeployDesk checks each project's local project path. Projects
whose local directory is missing or points to a file are shown with a warning on
the project card.

## Local Logs

DeployDesk keeps the latest 100 operation log records locally. Logs are intended
for operator review and should not be committed to the repository.

## Credentials

The `password_or_key` field may contain a protected SSH password or a private
key path in local project configuration. Exported project configuration contains
credentials protected with the operator-entered configuration package password,
so keep exported files private and keep the password separate. Do not include
real credentials, private keys, server addresses, customer release notes,
exported configuration files, or logs in issues, screenshots, or public
documentation.
