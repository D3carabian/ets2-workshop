// src/TitleBar.tsx  (new file, copy verbatim)
// Custom 40px titlebar for the frameless window (tauri.conf.json: "decorations": false).
// The <header> element inside it must contain exactly one <select> (the language picker);
// smoke tests assert this. Do not put the save picker or chips inside <header>.
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Save } from "lucide-react";
import { LanguagePicker, useLanguage } from "./i18n";

function withWindow(
  action: (w: ReturnType<typeof getCurrentWindow>) => Promise<void>,
) {
  try {
    void action(getCurrentWindow()).catch(() => {});
  } catch {
    /* Not running inside Tauri (plain browser preview). */
  }
}

export function TitleBar({
  pageLabel,
  saveLabel,
  pending,
  saveDisabled,
  onSave,
  onClose,
  summary,
}: {
  pageLabel: string;
  saveLabel: string;
  pending: number;
  saveDisabled: boolean;
  onSave: () => void;
  onClose: () => void;
  summary: { model: string; category: string; before: string; after: string }[];
}) {
  const { t } = useLanguage();
  return (
    <div className="titlebar" data-tauri-drag-region>
      <div className="brand" data-tauri-drag-region>
        <div className="brand-icon">
          <img src="/logo-c.png" alt="" />
        </div>
        <div className="brand-name">
          ETS2<span>WORKSHOP</span>
        </div>
      </div>
      <div className="titlebar-context" data-tauri-drag-region>
        <span className="context-page">{pageLabel}</span>
        <span className="context-save">{saveLabel || t("未打开存档")}</span>
      </div>
      <div className="titlebar-drag" data-tauri-drag-region />
      <header>
        <div className="header-right">
          <LanguagePicker />
        </div>
      </header>
      <div className="save-menu">
        <button
          className="primary titlebar-save"
          disabled={saveDisabled}
          onClick={onSave}
        >
          <Save size={15} />
          {t("保存修改")}
          {pending > 0 && <b>{pending}</b>}
        </button>
        {pending > 0 && !saveDisabled && (
          <div className="save-peek" role="region" aria-label={t("变更预览")}>
            <div className="section-head">
              <strong>{t("待保存的修改")}</strong>
              <span className="tag">{pending}</span>
            </div>
            {summary.slice(0, 3).map((change, index) => (
              <div className="peek-change" key={index}>
                <span>
                  {change.model} · {change.category}
                </span>
                <strong>{change.after}</strong>
              </div>
            ))}
            {pending > 3 && (
              <p>{t("另有 {count} 项修改", { count: pending - 3 })}</p>
            )}
            <button className="primary full" onClick={onSave}>
              {t("查看全部变更")}
            </button>
          </div>
        )}
      </div>
      <div className="window-controls">
        <button
          aria-label={t("最小化")}
          onClick={() => withWindow((w) => w.minimize())}
        >
          <svg viewBox="0 0 10 10" aria-hidden="true">
            <path d="M0 5h10" stroke="currentColor" fill="none" />
          </svg>
        </button>
        <button
          aria-label={t("最大化")}
          onClick={() => withWindow((w) => w.toggleMaximize())}
        >
          <svg viewBox="0 0 10 10" aria-hidden="true">
            <rect
              x=".5"
              y=".5"
              width="9"
              height="9"
              stroke="currentColor"
              fill="none"
            />
          </svg>
        </button>
        <button className="close" aria-label={t("关闭窗口")} onClick={onClose}>
          <svg viewBox="0 0 10 10" aria-hidden="true">
            <path d="M0 0l10 10M10 0L0 10" stroke="currentColor" fill="none" />
          </svg>
        </button>
      </div>
    </div>
  );
}
