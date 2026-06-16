# Security Notes

DeployDesk can connect to servers and run deployment commands, so security
review matters before using it in production. The Chinese product name is 发布舱.

## Local Secrets

Project configuration is stored on the local machine under the app config
directory. The directory name remains `com.deploydesk.app` for compatibility
with the previous Tauri implementation. Password-based project credentials are
stored in `projects.json` using local protection instead of plaintext. On
Windows, protection is bound to the current Windows user through the system data
protection API. The app does not read legacy project entries that still contain
plaintext server passwords. The `password_or_key` field may still contain a
private key path for key-based projects.

Non-Windows builds should be reviewed against the target platform's secret
storage expectations before production use. Do not commit real configuration
files or screenshots showing credentials.

Project configuration exports require an operator-entered configuration package
password. Password-based credentials and private key file content are protected
with that password. Treat exported configuration files and their package
passwords as sensitive secrets, store them separately, and keep
`deploydesk-projects*.json` out of source control.

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
- Remove exported project configuration bundles such as `deploydesk-projects.json`.
- Keep local logs ignored.
- Keep `target/` and `dist/` ignored; publish packaged installers only as
  intentional release assets.
- Review screenshots before attaching them to issues or documentation.
