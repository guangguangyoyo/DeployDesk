# Security Notes

DeployDesk can connect to servers and run deployment commands, so security
review matters before using it in production. The Chinese product name is 发布舱.

## Local Secrets

Project configuration is stored on the local machine under the app config
directory. The directory name remains `com.deploydesk.app` for compatibility
with the previous Tauri implementation. The `password_or_key` field may contain
an SSH password or a private key path. Do not commit real configuration files or
screenshots showing credentials.

## Remote Commands

The backend quotes remote paths before passing them to shell commands and rejects
obviously dangerous deployment paths such as `/`, `/root`, `/etc`, `/bin`, `/sbin`,
and `/usr`.

Before adding new remote commands:

- Quote user-controlled paths.
- Reject empty or root-level destructive targets.
- Avoid recursive deletion unless the target is derived from a validated release
  directory.
- Keep stdout and stderr handling explicit.

## Release Cleanup

Old release cleanup only considers directory names matching the timestamp format
`YYYYMMDD_HHMMSS`. Keep this constraint if changing retention behavior.

## Public Repository Checklist

- Remove real server addresses, usernames, passwords, private keys, and customer
  release notes.
- Keep local logs ignored.
- Keep `target/` and `dist/` ignored; publish packaged installers only as
  intentional release assets.
- Review screenshots before attaching them to issues or documentation.
