# 发布舱

中文 | [English](README.en.md)

发布舱（DeployDesk）是一个 **纯 Rust** 桌面发布工具，用于管理 Web 前端项目的本地构建、静态产物上传、远程版本发布和一键回滚。当前实现使用 Slint 构建原生桌面界面，不再依赖 Node.js、Vue、Vite 或 Tauri 作为运行时。

它采用 `releases/<timestamp>` 加软链接的发布模型：每次发布都会生成一个独立版本目录，Web 服务只需要指向稳定软链接，例如 `current`。发布和回滚都通过切换软链接完成，便于保留历史版本并快速恢复。

完整文档入口见 [docs/index.md](docs/index.md)。

## 功能特性

- 项目管理：维护多个前端项目的本地路径、构建命令、产物目录和远端服务器配置。
- 本地构建：发布前可自动执行自定义构建命令，例如 `npm run build`、`pnpm build` 或任意 shell 命令。
- 直接上传：也可跳过构建，直接上传已有静态文件目录。
- SSH/SFTP 部署：支持密码认证和私钥路径认证，通过 SFTP 上传构建产物。
- 软链接发布：按时间戳创建 release 目录，并更新可配置软链接。
- 发布备注：每次发布可保存备注到远端 `RELEASE_NOTE.txt`。
- 历史版本：读取远端 release 列表，显示当前版本和发布备注。
- 一键回滚：将远端软链接切换到指定历史版本。
- 操作日志：本地保存最近 100 条发布和回滚记录。
- Windows 安装包：可通过 NSIS 生成离线安装包，安装后创建桌面、开始菜单和卸载入口。

## 技术栈

- 桌面界面：Rust、Slint
- 部署能力：ssh2、tar、flate2、chrono
- 本地数据：serde、serde_json、uuid
- 文件选择：rfd
- Windows 打包：NSIS

## 环境要求

- Rust 1.80 或更高版本
- Windows 10/11、macOS 或 Linux 桌面环境
- 目标服务器需要支持 SSH、SFTP、`tar` 和 `ln -sfn`
- 构建 Windows 安装包时需要安装 NSIS，并确保 `makensis.exe` 在 PATH 中，或位于默认安装目录

## 快速开始

```bash
cargo run
```

## 常用命令

```bash
# 检查 Rust 项目
cargo check

# 运行桌面应用
cargo run

# 构建发布二进制
cargo build --release

# 构建 Windows NSIS 安装包
powershell -ExecutionPolicy Bypass -File scripts/build-nsis.ps1
```

构建产物位于 `target/release/`。
NSIS 安装包产物位于 `dist/DeployDesk-Setup-<version>.exe`，该目录已被 `.gitignore` 忽略。
安装包构建和离线安装说明见 [docs/windows-installer.md](docs/windows-installer.md)。

## 项目结构

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

主要文档：

- [docs/user-guide.md](docs/user-guide.md)：项目配置、发布模式、历史版本、回滚和本地数据。
- [docs/deployment-workflow.md](docs/deployment-workflow.md)：构建、上传、发布、清理和回滚流程。
- [docs/development.md](docs/development.md)：开发命令、项目结构和验证说明。
- [docs/windows-installer.md](docs/windows-installer.md)：NSIS 安装包构建和安装行为。
- [docs/security-notes.md](docs/security-notes.md)：凭据、远程命令和公开仓库检查项。

## 部署模型

远端目录结构示例：

```text
/var/www/project/
├── releases/
│   ├── 20260306_150000/
│   ├── 20260306_160000/
│   └── 20260306_170000/
└── current -> releases/20260306_170000/
```

Nginx 或其他 Web 服务只需要指向 `current` 这样的软链接目录。发布新版本时，发布舱会上传到新的 release 目录后更新软链接；回滚时只切换软链接。

更多细节见 [docs/deployment-workflow.md](docs/deployment-workflow.md)。

## 配置和数据

项目配置和操作日志保存在本地应用配置目录中。Windows 下为：

```text
%APPDATA%\com.deploydesk.app
```

该路径沿用旧 Tauri 版本的应用标识，便于继续读取既有数据：

- `projects.json`：项目配置。
- `logs.json`：最近 100 条操作日志。

当前版本的 `password_or_key` 字段可能保存 SSH 密码或私钥路径。公开仓库、issue、截图和日志中不要包含真实服务器地址、账号、密码或私钥信息。

更多说明见 [docs/security-notes.md](docs/security-notes.md)。

## 开发和贡献

欢迎提交 issue 和 pull request。提交前请至少运行：

```bash
cargo check
```

Windows 下如遇到正在运行的 `deploydesk.exe` 锁住默认 `target/` 目录，可参考 [docs/development.md](docs/development.md) 使用独立 target 目录验证。

贡献流程见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 许可证

本项目使用 [MIT License](LICENSE)。
