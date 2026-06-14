# Contributing

Thanks for considering a contribution to DeployDesk.

## Development Setup

```bash
cargo run
```

## Before Submitting

Please run the relevant checks before opening a pull request:

```bash
cargo check
cargo build --release
```

## Pull Request Guidelines

- Keep changes focused on one problem or feature.
- Update `README.md` or `docs/` when behavior, configuration, or workflows change.
- Do not commit local logs, build outputs, generated bundles, server credentials,
  private keys, or screenshots that expose sensitive information.
- Explain user-visible behavior changes in the pull request description.
- Include manual verification notes for deployment and rollback changes.

## Security-Sensitive Changes

Deployment, SSH authentication, release cleanup, rollback, and local credential
handling are security-sensitive areas. Keep changes small and explain the failure
modes you considered.
