import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  FolderSearch,
  Check,
  ArrowRight,
  LoaderCircle,
  RefreshCw,
  CircleAlert,
} from "lucide-react";
import type { Settings } from "./types";
type Detection = {
  games: string[];
  documents: string[];
  steam_roots: string[];
  notes: string[];
};
export default function Onboarding({
  initial,
  onComplete,
  onClose,
}: {
  initial: Settings;
  onComplete: (settings: Settings, count: number) => Promise<void>;
  onClose?: () => void;
}) {
  const [value, setValue] = useState(initial);
  const [detected, setDetected] = useState<Detection | null>(null);
  const [busy, setBusy] = useState(true);
  const [progress, setProgress] = useState("正在检测 Steam 游戏库与存档目录…");
  const [error, setError] = useState("");
  async function detect() {
    setBusy(true);
    setError("");
    setProgress("正在检测 Steam 游戏库与存档目录…");
    try {
      const d = await invoke<Detection>("rpc", {
        payload: { action: "detect" },
      });
      setDetected(d);
      setValue((v) => ({
        ...v,
        game:
          v.onboarding_version > 0 || detected
            ? v.game
            : d.games.length === 1
              ? d.games[0]
              : "",
        documents:
          v.onboarding_version > 0 || detected
            ? v.documents
            : d.documents.length === 1
              ? d.documents[0]
              : "",
      }));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  useEffect(() => {
    void detect();
    let stop: (() => void) | undefined;
    let disposed = false;
    void listen<string>("setup-progress", (e) => setProgress(e.payload)).then(
      (fn) => {
        if (disposed) fn();
        else stop = fn;
      },
    );
    return () => {
      disposed = true;
      stop?.();
    };
  }, []);
  async function finish() {
    setBusy(true);
    setError("");
    setProgress("正在准备本机环境…");
    try {
      const r = await invoke<{ settings: Settings; count: number }>("rpc", {
        payload: { action: "setup", settings: value },
      });
      await onComplete(r.settings, r.count);
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  }
  return (
    <div className="modal-backdrop onboarding-backdrop">
      <section
        className="onboarding"
        role="dialog"
        aria-modal="true"
        aria-label="首次启动向导"
      >
        <div className="welcome-icon">
          <FolderSearch size={38} />
        </div>
        <div className="eyebrow">WELCOME TO ETS2 WORKSHOP</div>
        <h1>确认位置，即可开始改装。</h1>
        <p>
          存档解码器已经内置。请选择本机的游戏和用户数据目录，工具会自动完成其余准备。
        </p>
        <div className="wizard-steps">
          <span className="active">1 检测路径</span>
          <ArrowRight size={15} />
          <span>2 确认并准备</span>
          <ArrowRight size={15} />
          <span>3 打开车库</span>
        </div>
        <fieldset disabled={busy}>
          <label>
            游戏安装目录
            <input
              list="detected-games"
              value={value.game}
              onChange={(e) => setValue({ ...value, game: e.target.value })}
              placeholder="包含 def.scs 的 Euro Truck Simulator 2 文件夹"
            />
            <datalist id="detected-games">
              {detected?.games.map((p) => (
                <option key={p} value={p} />
              ))}
            </datalist>
            <small>
              已检测到 {detected?.games.length || 0} 个安装位置，可手动修改。
            </small>
            {(detected?.games.length || 0) > 1 && (
              <select
                aria-label="选择游戏安装位置"
                value={detected?.games.includes(value.game) ? value.game : ""}
                onChange={(e) => setValue({ ...value, game: e.target.value })}
              >
                <option value="">检测到多个安装位置，请选择或手动填写</option>
                {detected?.games.map((p) => (
                  <option key={p} value={p}>
                    {p}
                  </option>
                ))}
              </select>
            )}
          </label>
          <label>
            ETS2 用户数据目录
            <input
              list="detected-docs"
              value={value.documents}
              onChange={(e) =>
                setValue({ ...value, documents: e.target.value })
              }
              placeholder="通常为文档中的 Euro Truck Simulator 2 文件夹"
            />
            <datalist id="detected-docs">
              {detected?.documents.map((p) => (
                <option key={p} value={p} />
              ))}
            </datalist>
            <small>
              包含 profiles、steam_profiles 或
              config.cfg，通常与游戏安装目录不同。
            </small>
            {(detected?.documents.length || 0) > 1 && (
              <select
                aria-label="选择用户数据位置"
                value={
                  detected?.documents.includes(value.documents)
                    ? value.documents
                    : ""
                }
                onChange={(e) =>
                  setValue({ ...value, documents: e.target.value })
                }
              >
                <option value="">
                  检测到多个用户数据位置，请选择或手动填写
                </option>
                {detected?.documents.map((p) => (
                  <option key={p} value={p}>
                    {p}
                  </option>
                ))}
              </select>
            )}
          </label>
          {detected?.notes.map((note) => (
            <p className="detection-note" key={note}>
              {note}
            </p>
          ))}
          <div className="wizard-facts">
            <span>
              <Check size={16} />
              内置存档解码器，无需 Truck Tools
            </span>
            <span>
              <Check size={16} />
              自动从 SCS 官方获取解包工具并校验
            </span>
            <span>
              <Check size={16} />
              确认前不修改设置或游戏存档
            </span>
          </div>
          <p className="inline-warning">
            <CircleAlert size={16} />
            仅支持原版及官方 DLC，不支持 Mod
            存档。首次准备需要网络及用于解包缓存的磁盘空间。
          </p>
          {error && (
            <p role="alert" className="notice error">
              {error}
            </p>
          )}
          <div className="button-row">
            <button onClick={detect}>
              <RefreshCw size={16} />
              重新检测
            </button>
            {onClose && <button onClick={onClose}>取消</button>}
            <button
              className="primary"
              disabled={!value.game || !value.documents}
              onClick={finish}
            >
              确认路径并准备 <ArrowRight size={16} />
            </button>
          </div>
        </fieldset>
        {busy && (
          <div className="wizard-progress" role="status">
            <LoaderCircle className="spin" size={18} />
            {progress}
          </div>
        )}
      </section>
    </div>
  );
}
