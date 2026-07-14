# 闪记技术架构

## 数据流

React 组件通过 Zustand 管理窗口内临时状态，通过类型化 `invoke` 调用 Rust command。Rust 仓储层是唯一可访问 SQLite 的模块；写操作在受控连接上串行执行，并通过 revision 乐观锁防止主窗口和快速窗口互相覆盖。

保存成功后 Rust 广播 `note:changed`，其他窗口按需刷新列表。快速窗口的隐藏事件由 Rust 发起，前端先执行 `flush()`，成功后再调用隐藏命令。

## 数据库

- 数据库位于 Tauri `app_local_data_dir`，Windows 对应当前用户的本地应用数据目录。
- 启用外键、WAL、NORMAL synchronous 和 5 秒 busy timeout。
- `rusqlite` bundled SQLite 在 macOS 与 Windows 使用相同 SQLite 功能集，并启用 FTS5。
- FTS5 trigram 用于三字符以上查询；短查询使用参数化兼容检索。

## 安全边界

- WebView 不获得原始 SQL 或任意文件系统权限。
- 文件路径必须来自原生文件选择器，再交给 Rust 执行导入导出。
- Markdown 不解析原始 HTML，链接只允许通过系统浏览器打开 HTTP/HTTPS。
- Tauri capability 只开放核心安全默认项、原生对话框和外部链接。

## 扩展接口

- `OrganizerProvider`：当前为 `RuleOrganizer`，以后可接入本地模型或经 Rust 调用的 AI 服务。
- `ReminderScheduler`：当前为空实现，以后可接入系统通知和日历。
