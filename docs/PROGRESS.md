# 项目进度

- [x] 阶段 1：规划、Git 项目结构、架构与数据库文档
- [x] 阶段 2：Tauri/React 骨架、Design Tokens、窗口与数据库迁移
- [x] 阶段 3：笔记 MVP、搜索、快速窗口、托盘、设置和数据管理
- [x] 阶段 4：规则化自动整理、实体/建议持久化和自动化测试
- [x] 阶段 5：Windows MSI/NSIS/exe 构建与 Release 工作流配置
- [x] 阶段 6（本地部分）：macOS 构建、运行时冒烟与自动化质量门禁
- [ ] 阶段 6（Windows 部分）：Windows Runner 构建证据与 Windows 10/11 真实设备验收

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
- [ ] 推送到 GitHub 后记录首个 Windows Runner 成功构建链接和产物校验值。
- [ ] 采购 Windows 代码签名证书并接入 CI secrets。
- [ ] 后续版本实现提醒通知、日历、自动更新和可选 AI provider。
