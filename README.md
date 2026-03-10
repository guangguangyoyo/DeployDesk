# Web 前端发布工具

基于 **Tauri + Vue 3 + Rust** 的桌面应用，用于自动化 Web 前端项目的构建、部署、回滚和管理。

## 功能特性

- 📦 **本地构建**：自动执行 `npm run build` 等构建命令
- 🚀 **SFTP 部署**：通过 SSH/SFTP 将构建产物上传至 Linux 服务器
- 🔗 **软链接发布**：基于 `releases/时间戳` + 软链接的零停机发布方案
- ⏪ **一键回滚**：查看历史版本列表，即时回滚到任意版本
- 📝 **发布备注**：每次发布可填写备注，保存至服务器用于版本追踪
- 🔐 **多种认证**：支持密码和私钥两种 SSH 认证方式
- 🎨 **美观界面**：深色主题 + 磨砂玻璃拟态设计

## 环境要求

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://www.rust-lang.org/tools/install) >= 1.70
- Windows 10 / 11

## 安装依赖

```bash
npm install
```

## 开发调试

```bash
npm run tauri dev
```

启动后会自动打开桌面窗口，前端热更新实时生效，Rust 代码修改后会自动重新编译。

## 打包构建

```bash
npm run tauri build
```

打包后的安装程序位于：

```
src-tauri/target/release/bundle/
├── msi/        # Windows MSI 安装包
└── nsis/       # Windows NSIS 安装包
```

## 项目结构

```
├── src/                    # Vue 3 前端代码
│   ├── App.vue             # 主布局（侧边栏 + 路由）
│   ├── views/              # 页面视图
│   │   ├── Projects.vue    # 项目管理（主页面）
│   │   ├── Logs.vue        # 操作日志
│   │   └── Settings.vue    # 系统设置
│   ├── components/         # 组件
│   │   ├── ProjectForm.vue # 项目配置表单
│   │   └── HistoryModal.vue# 发布历史 & 回滚
│   ├── stores/             # Pinia 状态管理
│   └── router/             # Vue Router 路由
├── src-tauri/              # Rust 后端代码
│   ├── src/
│   │   ├── lib.rs          # Tauri 入口 & 命令注册
│   │   ├── projects.rs     # 项目配置管理（CRUD）
│   │   └── deploy.rs       # 部署 / 回滚 / 历史逻辑
│   ├── Cargo.toml          # Rust 依赖
│   └── tauri.conf.json     # Tauri 配置
└── package.json            # 前端依赖 & 脚本
```

## 推荐 IDE 配置

- [VS Code](https://code.visualstudio.com/)
  - [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar)
  - [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
  - [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 部署原理

```
/var/www/project/
├── releases/
│   ├── 20260306_150000/    # 版本1
│   ├── 20260306_160000/    # 版本2（包含 RELEASE_NOTE.txt）
│   └── 20260306_170000/    # 版本3
└── dist -> releases/20260306_170000/   # 软链接（可配置名称）
```

Nginx 只需指向软链接目录即可，发布和回滚都是原子操作。
