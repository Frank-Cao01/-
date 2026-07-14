# 闪记

闪记是一款轻量、本地优先的桌面备忘录。用户可以在其他软件中通过全局快捷键唤出快速记录窗口，内容在停止输入约 500ms 后自动保存到本机 SQLite 数据库。

## 当前能力

- 笔记创建、编辑、Markdown 预览和原生撤销/重做
- 分类、标签、收藏、归档、待办状态、优先级和截止日期
- 软删除、最近删除、恢复和永久删除
- SQLite FTS5 中文子串检索与 1–2 字兼容搜索
- 可拖动、可固定的快速记录窗口，全局快捷键、系统托盘和开机启动
- 浅色、深色和跟随系统主题
- 规则化日期/待办/标签/分类/联系信息/相似标题建议
- JSON 与 Markdown 导出、JSON 导入、数据库备份恢复
- GitHub Actions 构建 Windows x64 MSI、NSIS 和独立 exe

## 技术栈

- Tauri 2、Rust stable（项目锁定 `stable-2026-05-28` / Rust 1.96.0）
- React 19、TypeScript、Vite 7
- Tailwind CSS 4、Zustand
- SQLite（Rust `rusqlite` bundled + FTS5）
- Vitest、Testing Library、Cargo test

## 开发环境

### macOS

1. 安装 Xcode Command Line Tools：

   ```bash
   xcode-select --install
   ```

2. 从 [Node.js 官网](https://nodejs.org/)安装 Node.js 24 LTS（项目通过 `.nvmrc` 锁定 24.14.0），然后启用项目指定的 pnpm：

   ```bash
   corepack enable
   corepack prepare pnpm@11.7.0 --activate
   ```

3. 安装 rustup。项目内的 `rust-toolchain.toml` 会自动选择已锁定的稳定版本：

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh
   sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain none
   ```

4. 安装依赖并启动：

   ```bash
   pnpm install
   pnpm tauri dev
   ```

快速记录默认快捷键是 `Command + Shift + Space`。

### Windows

Windows 本地开发需要 Node.js 24 LTS、Rust stable MSVC、Microsoft C++ Build Tools 和 WebView2。默认快捷键为 `Ctrl + Shift + Space`。

## 常用命令

```bash
pnpm dev                 # 只启动 Vite 外壳；数据库与桌面能力需使用下一条命令
pnpm tauri dev           # 启动完整桌面应用
pnpm typecheck           # TypeScript 检查
pnpm lint                # ESLint
pnpm test                # 前端单元测试
pnpm check               # 前端完整检查
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## 数据位置与隐私

数据只保存在本机，不依赖服务器；断网后笔记、搜索和自动整理均可工作。

- macOS：`~/Library/Application Support/com.shanji.desktop/shanji.db` 附近的 Tauri本地数据目录
- Windows：当前用户 `%LOCALAPPDATA%` 下由 Tauri 根据 `com.shanji.desktop` 创建的目录

实际路径可在“设置 → 数据管理 → 本地数据库”查看。数据库当前为明文 SQLite；敏感内容请配合系统磁盘加密和账户权限使用。

## 自动保存与冲突

- 停止输入 500ms 后保存。
- 中文输入法组合输入期间暂停计时。
- 切换笔记、隐藏快速窗口和 `Ctrl/Cmd + Enter` 会立即刷新保存。
- 主窗口和快速窗口同时修改同一记录时使用 revision 乐观锁；发生冲突后可加载最新内容或另存副本，不会静默覆盖。

## 搜索

bundled SQLite 默认启用 FTS5，并使用 trigram 索引支持中文子串。所有查询词不少于 3 个字符时使用 FTS5 和权重排序；任意查询词少于 3 个字符时使用参数化兼容搜索。搜索支持分类、标签、日期、收藏和待办状态筛选。

## 导入、导出和备份

- JSON：版本化、可重新导入的完整内容格式，不包含设备设置和搜索历史。
- Markdown：每条笔记生成一个带 YAML frontmatter 的 `.md` 文件。
- JSON 导入：先预览；相同内容跳过，ID 冲突但内容不同的记录另存为副本。
- `.sjbackup`：包含 SQLite 一致性快照、版本信息和 SHA-256 校验。恢复前自动创建回滚备份。

永久删除、JSON 批量导入和恢复备份都要求二次确认。

## Windows 构建

不要依赖 macOS 交叉编译验证 Windows 原生能力。推送代码后，`.github/workflows/ci.yml` 会在 `windows-latest` 上执行：

```powershell
pnpm tauri build --target x86_64-pc-windows-msvc --bundles msi,nsis
```

产物包括：

- `bundle/msi/*.msi`
- `bundle/nsis/*-setup.exe`
- `release/shanji.exe`

NSIS 默认按当前用户安装，不需要管理员权限；安装完成后创建桌面快捷方式和开始菜单入口。安装器内置 WebView2 离线运行时，最终用户不需要 Node.js、Python、Rust 或其他开发环境。独立 exe 仍把数据保存到 AppData，不是“数据随程序移动”的完全便携模式。

每次 Windows 构建还会生成统一整理的 `Release/闪记-<版本>-windows-x64/`：

```text
Release/
├── 闪记-<版本>-windows-x64/
│   ├── 闪记-<版本>-x64-Setup.exe   # 推荐分发
│   ├── 闪记-<版本>-x64.msi
│   ├── 闪记-<版本>-x64.exe         # 独立程序
│   ├── 使用说明.txt
│   └── SHA256SUMS.txt
├── 闪记-<版本>-windows-x64-Release.zip
└── 闪记-<版本>-windows-x64-Release.zip.sha256
```

在 GitHub 仓库的 Actions 页面手动运行 `CI`，或直接推送代码，即可下载名为 `shanji-windows-x64-Release` 的完整发布目录。对外发送时优先发送 `Setup.exe`；若希望一次发送整个目录，则发送 `Release.zip`。

## Release 发布

1. 同步修改 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 版本。
2. 执行 `pnpm version:check`。
3. 更新 `CHANGELOG.md`。
4. 创建并推送标签，例如 `v0.1.0`。
5. `release.yml` 在 Windows Runner 构建并上传 MSI、NSIS、独立 exe、完整 Release ZIP 和 SHA-256 文件。

当前首版未配置 Windows 代码签名，安装时可能出现 SmartScreen 提示。正式公开发行前需要采购证书并接入 GitHub Actions secrets。

## Windows 验收

托盘、快捷键、开机启动、安装卸载、多显示器、高 DPI、文件权限和中文输入法必须在真实 Windows 10/11 环境测试。测试矩阵见 [docs/WINDOWS_VALIDATION.md](docs/WINDOWS_VALIDATION.md)。

## 项目文档

- [架构说明](docs/ARCHITECTURE.md)
- [产品与实施规格](docs/PRODUCT_SPEC.md)
- [数据库设计](docs/DATABASE.md)
- [开发进度](docs/PROGRESS.md)
- [Windows 验收矩阵](docs/WINDOWS_VALIDATION.md)
- [更新日志](CHANGELOG.md)

## 尚未包含

云同步、付费 AI、提醒通知、日历、自动更新、数据库加密和完全便携数据模式已保留扩展方向，但不属于 0.1.0。
