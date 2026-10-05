import {
  ArrowRight,
  Check,
  CircleAlert,
  Save,
  ShieldCheck,
  X,
} from "lucide-react";
import { SourceMessage, useLanguage } from "./i18n";
import { friendly, labels } from "./parts";
import type { Preview } from "./types";
import { Dialog } from "./Dialog";

export function ChangeReview({
  preview,
  busy,
  saveMode,
  setSaveMode,
  newName,
  setNewName,
  displayPath,
  onRemove,
  onSave,
  onDismiss,
  error,
}: {
  preview: Preview | null;
  busy: boolean;
  saveMode: string;
  setSaveMode: (mode: string) => void;
  newName: string;
  setNewName: (name: string) => void;
  displayPath: (path: string) => string;
  onRemove: (index: number) => void;
  onSave: () => void;
  onDismiss: () => void;
  error?: React.ReactNode;
}) {
  const { t: tr } = useLanguage();
  const changes = preview?.changes || [];
  return (
    <Dialog
      titleId="review-title"
      className="change-review"
      onDismiss={() => {
        if (!busy) onDismiss();
      }}
    >
      <div className="section-head review-heading">
        <div>
          <div className="eyebrow">{tr("最后确认")}</div>
          <h2 id="review-title">{tr("保存这次改装")}</h2>
          <p>{tr("确认全部变更后，一并保存到存档。")}</p>
        </div>
        <button aria-label={tr("关闭")} disabled={busy} onClick={onDismiss}>
          <X size={20} />
        </button>
      </div>
      <div className="review-body">
        <section className="changes review-changes">
          <div className="section-head">
            <h3>
              {tr("待保存的修改")} <span className="tag">{changes.length}</span>
            </h3>
            <span className="backup-reminder">
              <ShieldCheck size={16} />
              {tr("保存时自动备份")}
            </span>
          </div>
          <div className="change-list">
            {changes.map((change, index) => (
              <div className="change" key={index}>
                <span>
                  <strong>{friendly(change.model)}</strong>
                  <small>
                    {change.plate} ·{" "}
                    {tr(labels[change.category] || change.category)}
                  </small>
                </span>
                <code>
                  {change.before ? displayPath(change.before) : tr("新增")}
                </code>
                <ArrowRight size={16} />
                <code>{displayPath(change.after)}</code>
                <button
                  className="icon-button"
                  aria-label={tr("移除此修改")}
                  disabled={busy}
                  onClick={() => onRemove(index)}
                >
                  <X size={16} />
                </button>
              </div>
            ))}
            {!changes.length && (
              <div className="empty">
                <Check size={30} />
                <p>{tr("没有待保存的修改")}</p>
              </div>
            )}
          </div>
          {preview?.warnings.map((warning, index) => (
            <p className="inline-warning" key={index}>
              <CircleAlert size={16} />
              <SourceMessage text={warning} />
            </p>
          ))}
        </section>
        <section className="review-save-options">
          <h3>{tr("保存方式")}</h3>
          <label>
            {tr("保存方式")}
            <select
              disabled={busy}
              value={saveMode}
              onChange={(event) => setSaveMode(event.target.value)}
            >
              <option value="new">{tr("另存为新存档（推荐）")}</option>
              <option value="overwrite">{tr("备份并覆盖当前存档")}</option>
            </select>
          </label>
          {saveMode === "new" && (
            <label>
              {tr("新存档名称")}
              <input
                disabled={busy}
                value={newName}
                onChange={(event) => setNewName(event.target.value)}
                maxLength={80}
              />
            </label>
          )}
          <p className="inline-warning">
            <CircleAlert size={17} />
            {tr("保存后需要在游戏中手动加载。改装升级可能恢复原厂配件。")}
          </p>
          {saveMode === "overwrite" && (
            <p className="inline-warning">
              {tr(
                "覆盖或恢复前，请退出游戏并暂停会写入该存档的同步及其他程序。",
              )}
            </p>
          )}
          {error && (
            <div className="review-error" role="alert">
              {error}
            </div>
          )}
        </section>
      </div>
      <div className="review-actions">
        <button disabled={busy} onClick={onDismiss}>
          {tr("继续改装")}
        </button>
        <button
          className="primary"
          disabled={
            busy || !changes.length || (saveMode === "new" && !newName.trim())
          }
          onClick={onSave}
        >
          <Save size={17} />
          {tr("确认保存")} <b>{changes.length}</b>
        </button>
      </div>
    </Dialog>
  );
}
