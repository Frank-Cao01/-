# 产品与实施规格

## 产品目标

闪记面向需要在当前工作流中即时记录的人群。核心路径是：按全局快捷键、在鼠标所在显示器中央打开快速窗口、输入、自动保存、再次按快捷键或 Esc 隐藏。数据默认只保存在本机，离线可用。

首版包含笔记、分类、标签、全文搜索、待办、收藏、归档、最近删除、快速窗口、托盘、开机启动、规则化整理、导入导出与备份恢复。云同步、付费 AI、提醒、日历、自动更新、数据库加密和完全便携数据模式不在 0.1.0 范围内。

## 页面与组件

- 主窗口：`Sidebar`、`NoteList`、`SearchBar`、`NoteEditor`、`SettingsPage`。
- 快速窗口：`QuickWindow` 与独立草稿/自动保存队列。
- 通用内容：Markdown 安全预览、关键词高亮、Design Tokens 和主题。
- 三栏、双栏、单栏分别在 1100px 与 720px 断点切换。

## 状态与数据真源

- `notesStore`：列表、选中项、分类与标签缓存。
- `searchStore`：关键词、结构化筛选、结果和最近搜索。
- `settingsStore`：主题、快捷键、关闭行为、开机启动等设置。
- `uiStore`：当前面板、移动端式单栏页和侧栏抽屉。
- Zustand 只保存界面状态；SQLite 是唯一持久化真源。

所有保存通过每条笔记独立的 500ms 串行队列。IME 组合输入时暂停调度；切换、隐藏和退出会强制刷新。更新携带 `expectedRevision`，冲突时不得静默覆盖。

## 后端边界

- `commands`：类型化 IPC，不向 WebView 暴露 SQL。
- `db`：连接、迁移、仓储、搜索、导入导出与备份。
- `domain`：DTO、规则整理和提醒扩展接口。
- `platform`：窗口定位、托盘、快捷键、单实例与开机启动。

SQLite 启用外键、WAL、NORMAL synchronous 和 busy timeout；bundled 构建统一提供 FTS5。三字符以上查询优先使用 trigram FTS，短查询降级为参数化兼容搜索。

## Windows 兼容重点

- 使用原生窗口边框/阴影，快速窗口单独使用无标题栏轻量外观。
- 所有定位基于 Tauri 物理像素工作区，兼容负坐标、多显示器与不同 DPI。
- 数据写入 `app_local_data_dir`，不依赖程序安装目录权限。
- NSIS 为当前用户安装；MSI 与 NSIS 只能在 Windows Runner 生成。
- 托盘、快捷键、开机启动、中文输入法、高 DPI、升级/卸载数据保留必须以真实 Windows 10/11 证据验收。

## 发布门禁

本地必须通过前端检查/测试/构建、Rust fmt/test/clippy 和 macOS Tauri 无 bundle 构建。标签必须等于三个配置文件中的版本号。Windows Release 必须包含 MSI、NSIS、独立 exe ZIP 和 `SHA256SUMS.txt`。
