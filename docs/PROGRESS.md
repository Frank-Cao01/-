# 项目进度

- [x] 阶段 1：规划、Git 项目结构、架构与数据库文档
- [x] 阶段 2：Tauri/React 骨架、Design Tokens、窗口与数据库迁移
- [x] 阶段 3：笔记 MVP、搜索、快速窗口、托盘、设置和数据管理
- [x] 阶段 4：规则化自动整理、实体/建议持久化和自动化测试
- [x] 阶段 5：Windows MSI/NSIS/exe 构建与 Release 工作流配置
- [x] 阶段 6（本地部分）：macOS 构建、运行时冒烟与自动化质量门禁
- [x] 阶段 6（Windows Runner）：x64 MSI、NSIS、独立 exe 与 Release 资产构建证据
- [ ] 阶段 6（Windows 真机）：Windows 10/11 安装、多显示器、高 DPI 与中文输入验收

## 2026-07-17 v0.1.1 验证记录

- 修复无边框快速记录窗口缺少 `core:window:allow-start-dragging` 权限的问题。
- 将快速记录窗口整条顶部栏设为原生拖拽区域，操作按钮保持可点击。
- macOS 原生窗口实测：快速记录窗从 `(650, 227)` 移动到 `(1120, 190)`。
- 浅色、深色和跟随系统主题完成视觉检查，快速窗口可同步主窗口设置。
- `pnpm check`：通过（Lint、TypeScript、8 个前端测试）。
- `pnpm build`：通过。
- `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`：通过。
- `cargo test`：通过（13 个 Rust 测试）。
- [CI 运行 29564454658](https://github.com/Frank-Cao01/-/actions/runs/29564454658)：通过，生成 Windows x64 MSI、NSIS 和独立 exe。
- [Release 运行 29566076189](https://github.com/Frank-Cao01/-/actions/runs/29566076189)：通过。
- [GitHub Release v0.1.1](https://github.com/Frank-Cao01/-/releases/tag/v0.1.1)：已发布，包含安装包、独立 exe、完整 Release.zip、说明和 SHA-256。
- 本地 `Release/0.1.1`：完整包与三种 Windows 产物 SHA-256 校验通过，ZIP 结构测试通过。

## 2026-07-14 本地验证记录

- `pnpm check`：通过（Lint、TypeScript、7 个前端测试）。
- `pnpm build`：通过（Vite 生产构建）。
- `cargo test`：通过（13 个 Rust 测试）。
- `cargo clippy --all-targets -- -D warnings`：通过，零警告。
- `cargo fmt --check`：通过。
- `pnpm tauri build --debug --no-bundle`：通过，生成 macOS 调试可执行文件。
- `pnpm tauri build --debug --bundles app`：通过，生成可启动的 macOS 应用包。
- 后台启动冒烟：通过；两个窗口、插件和 SQLite WAL 数据库完成初始化。
- Windows 发布配置：已包含离线 WebView2、桌面/开始菜单入口和完整 Release 目录归档；原生产物只能由 GitHub Actions Windows Runner 生成，当前尚无远端执行证据。

## TODO

- [ ] 在 Windows 10 x64 真机完成安装、卸载、高 DPI 和中文输入测试。
- [ ] 在 Windows 11 x64 双显示器环境完成弹窗定位测试。
- [ ] 采购 Windows 代码签名证书并接入 CI secrets。
- [ ] 后续版本实现提醒通知、日历、自动更新和可选 AI provider。
