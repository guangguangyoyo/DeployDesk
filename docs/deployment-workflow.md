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

Password-auth projects are saved with locally protected passwords instead of
plaintext values. Key-auth projects keep a private key path. Project
configuration exports use a separate package format and are protected by an
operator-entered package password.

## Publish Flow

1. Validate that the remote deployment path is not dangerous.
2. Validate that the local project path exists and is a directory.
3. If `deploy_mode` is `build`, validate and run the configured build command in
   the local project directory. Common package-manager commands such as
   `npm run build` fail early when the matching `package.json` script is missing.
4. Validate and compress the configured build output directory into a temporary
   `.tar.gz`.
5. Create an SSH session.
6. Verify that the configured symlink path is not already an ordinary directory.
7. Create a remote release directory named with a timestamp.
8. Upload and extract the archive on the server through SFTP and `tar`.
9. Write `RELEASE_NOTE.txt` when a publish remark is provided.
10. Update the configured symlink with `ln -sfn`.
11. Delete old releases when the count exceeds `max_releases`.
12. Write a local operation log entry.

## Preflight Failures

DeployDesk stops before connecting to the remote server when:

- the local project path is empty, missing, or points to a file;
- the configured output directory is empty, missing, or points to a file;
- `deploy_mode` is `build` and a known package-manager command references a
  missing `package.json` script.

After connecting to the remote server, DeployDesk also stops before creating a
new release when the configured symlink path already exists as an ordinary
directory.

## Rollback Flow

Rollback validates that the selected release name matches the timestamp format,
checks that the configured symlink path is not an ordinary directory, then
updates the symlink to point at the selected release directory.

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

The configured symlink path, such as `current`, must not already exist as an
ordinary directory. If DeployDesk finds a same-name directory on the remote
server, it stops the publish or rollback operation and asks the operator to
remove or rename that directory, or to choose a different symlink name.
