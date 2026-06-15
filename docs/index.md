# Documentation

DeployDesk is published in Chinese as 发布舱. This directory keeps workflow, packaging, development, and security documentation out of the root README so each document has a clear purpose.

## Start Here

- [User guide](user-guide.md): how to configure projects, choose deployment modes, publish, review release history, and roll back.
- [Deployment workflow](deployment-workflow.md): the internal build, archive, upload, release, symlink update, cleanup, and rollback sequence.
- [Windows installer](windows-installer.md): how to build the NSIS installer, what it installs, and what offline installation requires.
- [Development guide](development.md): project structure, commands, validation, and repository hygiene.
- [Security notes](security-notes.md): local secrets, remote commands, release cleanup, and public repository checklist.

## Root Documents

- [Chinese README](../README.md): concise project overview and common commands.
- [English README](../README.en.md): English overview and common commands.
- [Contributing](../CONTRIBUTING.md): contribution workflow and pull request expectations.
- [Repository guide](../AGENTS.md): short navigation map for coding agents.

## Generated Artifacts

Generated files should stay out of source control unless they are intentional release assets:

- Rust build output: `target/`
- NSIS installers and local bundles: `dist/`
- Local runtime logs: `logs/`, `*.log`, `*.err.log`
