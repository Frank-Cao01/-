import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { ArrowLeft, Database, Download, FolderArchive, RotateCcw, Upload } from "lucide-react";
import { APP_VERSION, PRODUCT_NAME } from "../../config/app";
import { api } from "../../services/api";
import { useNotesStore } from "../../stores/notesStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useUiStore } from "../../stores/uiStore";
import type { AppSettings } from "../../types";

export function SettingsPage() {
  const store = useSettingsStore();
  const notes = useNotesStore();
  const ui = useUiStore();
  const [form, setForm] = useState<AppSettings>(store.settings);
  const [message, setMessage] = useState<string | null>(null);
  const [databaseInfo, setDatabaseInfo] = useState<{ path: string; fts5: boolean } | null>(null);

  useEffect(() => setForm(store.settings), [store.settings]);
  useEffect(() => { void api.databaseInfo().then(setDatabaseInfo); }, []);

  const patch = (value: Partial<AppSettings>) => setForm((current) => ({ ...current, ...value }));
  const persist = async () => {
    setMessage(null);
    try {
      await store.save(form);
      setMessage("设置已保存");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const exportJson = async () => {
    const path = await save({ defaultPath: `${PRODUCT_NAME}-${new Date().toISOString().slice(0, 10)}.json`, filters: [{ name: `${PRODUCT_NAME} JSON`, extensions: ["json"] }] });
    if (!path) return;
    await api.exportJson(path);
    setMessage(`JSON 已导出到 ${path}`);
  };

  const exportMarkdown = async () => {
    const directory = await open({ directory: true, multiple: false, title: "选择 Markdown 导出文件夹" });
    if (!directory || Array.isArray(directory)) return;
    const count = await api.exportMarkdown(directory);
    setMessage(`已导出 ${count} 条 Markdown 笔记`);
  };

  const importJson = async () => {
    const path = await open({ multiple: false, filters: [{ name: `${PRODUCT_NAME} JSON`, extensions: ["json"] }] });
    if (!path || Array.isArray(path)) return;
    const preview = await api.previewImport(path);
    if (!window.confirm(`将新增 ${preview.notesToCreate} 条，跳过 ${preview.duplicatesToSkip} 条，并将 ${preview.conflictsAsCopies} 条冲突记录另存为副本。确定导入吗？`)) return;
    const result = await api.importJson(path);
    await Promise.all([notes.loadView(), notes.loadMetadata()]);
    setMessage(`导入完成：新增 ${result.created}，跳过 ${result.skipped}，冲突副本 ${result.copiedConflicts}`);
  };

  const createBackup = async () => {
    const path = await save({ defaultPath: `${PRODUCT_NAME}备份-${new Date().toISOString().slice(0, 10)}.sjbackup`, filters: [{ name: `${PRODUCT_NAME}备份`, extensions: ["sjbackup"] }] });
    if (!path) return;
    await api.createBackup(path);
    setMessage(`备份已保存到 ${path}`);
  };

  const restoreBackup = async () => {
    const path = await open({ multiple: false, filters: [{ name: `${PRODUCT_NAME}备份`, extensions: ["sjbackup"] }] });
    if (!path || Array.isArray(path)) return;
    if (!window.confirm(`恢复备份会替换当前数据库。${PRODUCT_NAME}会先自动生成一份回滚备份，确定继续吗？`)) return;
    const rollback = await api.restoreBackup(path);
    await Promise.all([notes.loadView("all"), notes.loadMetadata(), store.load()]);
    setMessage(`恢复完成；恢复前数据保存在 ${rollback}`);
  };

  return (
    <section className="settings-page">
      <div className="settings-inner">
        <button className="button small" onClick={() => ui.setPanel("notes")}><ArrowLeft size={14} />返回笔记</button>
        <h1 className="settings-heading">设置</h1>
        {message && <div className={message.includes("失败") || message.includes("不可用") ? "error-banner" : "suggestion-panel"}>{message}</div>}

        <div className="settings-card">
          <h2>通用</h2>
          <div className="settings-row"><div className="settings-copy"><strong>主题</strong><p>默认使用浅色，也可以跟随系统。</p></div><select className="select" style={{ width: 150 }} value={form.theme} onChange={(event) => patch({ theme: event.target.value as AppSettings["theme"] })}><option value="light">浅色</option><option value="dark">深色</option><option value="system">跟随系统</option></select></div>
          <div className="settings-row"><div className="settings-copy"><strong>关闭主窗口</strong><p>选择隐藏到托盘或真正退出程序。</p></div><select className="select" style={{ width: 170 }} value={form.closeBehavior} onChange={(event) => patch({ closeBehavior: event.target.value as AppSettings["closeBehavior"] })}><option value="tray">最小化到托盘</option><option value="quit">退出软件</option></select></div>
          <div className="settings-row"><div className="settings-copy"><strong>开机启动</strong><p>开机后在后台启动并驻留托盘。</p></div><input className="switch" type="checkbox" checked={form.autostart} onChange={(event) => patch({ autostart: event.target.checked })} /></div>
        </div>

        <div className="settings-card">
          <h2>快速记录</h2>
          <div className="settings-row"><div className="settings-copy"><strong>全局快捷键</strong><p>保存时会检测系统快捷键冲突，失败则保留旧设置。</p></div><input className="input shortcut-input" value={form.shortcut} onChange={(event) => patch({ shortcut: event.target.value })} /></div>
          <div className="settings-row"><div className="settings-copy"><strong>固定快速窗口</strong><p>保留拖动后的位置，并在切换到其他软件时继续显示。</p></div><input className="switch" type="checkbox" checked={form.quickPinned} onChange={(event) => patch({ quickPinned: event.target.checked })} /></div>
          <div className="settings-row"><div className="settings-copy"><strong>点击窗口外部时隐藏</strong><p>隐藏前会先刷新自动保存。</p></div><input className="switch" type="checkbox" checked={form.hideOnBlur} onChange={(event) => patch({ hideOnBlur: event.target.checked })} /></div>
        </div>

        <div className="settings-card">
          <h2>自动整理</h2>
          <div className="settings-row"><div className="settings-copy"><strong>启用规则建议</strong><p>识别日期、待办、标签、分类和联系信息，不会自动覆盖内容。</p></div><input className="switch" type="checkbox" checked={form.organizerEnabled} onChange={(event) => patch({ organizerEnabled: event.target.checked })} /></div>
          <div className="settings-row"><div className="settings-copy"><strong>归档建议天数</strong><p>超过指定天数未修改时给出建议。</p></div><input className="input" style={{ width: 100 }} type="number" min={7} max={3650} value={form.archiveDays} onChange={(event) => patch({ archiveDays: Number(event.target.value) || 90 })} /></div>
        </div>

        <div className="settings-card">
          <h2>数据管理</h2>
          <div className="settings-actions">
            <button className="button" onClick={exportJson}><Download size={15} />导出 JSON</button>
            <button className="button" onClick={exportMarkdown}><Download size={15} />导出 Markdown</button>
            <button className="button" onClick={importJson}><Upload size={15} />从 JSON 导入</button>
            <button className="button" onClick={createBackup}><FolderArchive size={15} />创建备份</button>
            <button className="button danger" onClick={restoreBackup}><RotateCcw size={15} />恢复备份</button>
          </div>
          {databaseInfo && <div className="settings-row"><Database size={16} /><div className="settings-copy"><strong>本地数据库</strong><p>{databaseInfo.path}<br />全文检索：{databaseInfo.fts5 ? "FTS5 已启用" : "兼容搜索模式"}</p></div></div>}
        </div>

        <div className="settings-card"><h2>关于</h2><p>{PRODUCT_NAME} {APP_VERSION} · 本地优先，不依赖服务器。</p><p style={{ color: "var(--color-text-tertiary)", fontSize: 12 }}>当前数据库为本地明文存储。Windows 首版安装包未签名，安装时可能显示 SmartScreen 提示。</p></div>
        <div className="settings-save"><button className="button primary" onClick={persist} disabled={store.loading}>{store.loading ? "保存中…" : "保存设置"}</button></div>
      </div>
    </section>
  );
}
