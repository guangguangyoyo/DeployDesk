# Deployment Workflow

This document describes how DeployDesk works internally. DeployDesk is published
in Chinese as 发布舱.

## Project Configuration

Each project stores:

- Local project path.
- Build command.
- Build output directory.
- Deployment mode: `build` or `upload`.
- SSH host, port, username, and authentication method.
- Remote deployment path.
- Symlink name, defaulting to `current`.
- Maximum number of releases to keep.

Configuration is stored locally under the app config directory in `projects.json`.
The directory name remains `com.deploydesk.app` for compatibility with the
previous Tauri implementation.

## Publish Flow

1. Validate that the remote deployment path is not dangerous.
2. If `deploy_mode` is `build`, run the configured build command in the local
   project directory.
3. Compress the configured build output directory into a temporary `.tar.gz`.
4. Create an SSH session and SFTP connection.
5. Create a remote release directory named with a timestamp.
6. Upload and extract the archive on the server.
7. Write `RELEASE_NOTE.txt` when a publish remark is provided.
8. Update the configured symlink with `ln -sfn`.
9. Delete old releases when the count exceeds `max_releases`.
10. Write a local operation log entry.

## Rollback Flow

Rollback validates that the selected release name matches the timestamp format,
then updates the configured symlink to point at the selected release directory.

## Remote Directory Example

```text
/var/www/project/
├── releases/
│   ├── 20260306_150000/
│   ├── 20260306_160000/
│   └── 20260306_170000/
└── current -> releases/20260306_170000/
```

Point the web server root at the symlink directory.
