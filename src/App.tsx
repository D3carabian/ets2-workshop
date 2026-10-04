import {
  LanguageProvider,
  LanguagePicker,
  SourceMessage,
  useLanguage,
  type TextValues,
} from "./i18n";
import Onboarding from "./Onboarding";
import "./garage.css";
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Truck as TruckIcon,
  Wrench,
  Layers3,
  History,
  Settings2,
  ArrowRight,
  Search,
  RefreshCw,
  FolderOpen,
  ShieldCheck,
  ChevronRight,
  X,
  Plus,
  CircleAlert,
  Check,
  LoaderCircle,
  Save,
  Database,
  Undo2,
} from "lucide-react";
import type {
  Settings,
  Definition,
  Accessory,
  Opened,
  SaveEntry,
  Operation,
  Preview,
  Receipt,
  Verification,
} from "./types";

import {
  labels,
  groups,
  groupOf,
  friendly,
  describePart,
  truckDefinition,
  categoryLabel,
  compareParts,
  partOrigin,
  replacementReason,
  candidateReason,
} from "./parts";
import { Metrics, Comparison } from "./PartMetrics";
const FLEET_PAGE_SIZE = 50;
const pathKey = (path: string) =>
  path
    .replaceAll("\\", "/")
    .replace(/^\/\/\?\/UNC\//i, "//")
    .replace(/^\/\/\?\//, "")
    .toLowerCase();
const short = (p: string) => p.split("/").slice(-2).join("/");
async function rpc<T>(
  action: string,
  extra: Record<string, unknown> = {},
): Promise<T> {
  return invoke("rpc", { payload: { action, ...extra } });
}

export default function App() {
  return (
    <LanguageProvider>
      <Workshop />
    </LanguageProvider>
  );
}

function Workshop() {
  const { t: tr, locale } = useLanguage();
  const partLocale = locale === "en" ? "en" : "zh";
  const [page, setPage] = useState("garage");
  const [settings, setSettings] = useState<Settings>({
    documents: "",
    game: "",
    extractor: "",
    onboarding_version: 0,
  });
  const [savedGame, setSavedGame] = useState("");
  const [discoveryWarnings, setDiscoveryWarnings] = useState<string[]>([]);
  const [catalogRebuildReason, setCatalogRebuildReason] = useState<
    string | null
  >(null);
  const [showSetup, setShowSetup] = useState(false);
  const [firstSetup, setFirstSetup] = useState(false);
  const [saves, setSaves] = useState<SaveEntry[]>([]);
  const [includeAutosaves, setIncludeAutosaves] = useState(() => {
    try {
      return localStorage.getItem("ets2-workshop.include-autosaves") === "true";
    } catch {
      return false;
    }
  });
  const [openedSaveLabel, setOpenedSaveLabel] = useState("");
  const [savePath, setSavePath] = useState("");
  const [manualPath, setManualPath] = useState("");
  const [opened, setOpened] = useState<Opened | null>(null);
  const garageRef = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const grid = garageRef.current;
    if (!grid) return;
    const resize = () => {
      const top = grid.getBoundingClientRect().top + window.scrollY;
      const height = `${Math.max(240, window.innerHeight - top - 16)}px`;
      if (grid.style.getPropertyValue("--garage-height") !== height)
        grid.style.setProperty("--garage-height", height);
    };
    resize();
    const observer = new ResizeObserver(resize);
    observer.observe(grid.closest("main")!);
    window.addEventListener("resize", resize);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", resize);
    };
  }, [page, !!opened]);
  const [truckId, setTruckId] = useState("");
  const [fleetQuery, setFleetQuery] = useState("");
  const [fleetPage, setFleetPage] = useState(0);
  const [partId, setPartId] = useState("");
  const [definitions, setDefinitions] = useState<Definition[]>([]);
  const definitionByPath = useMemo(
    () => new Map(definitions.map((def) => [def.path, def])),
    [definitions],
  );
  const displayPath = (path: string) => {
    const def = definitionByPath.get(path);
    return def ? describePart(def, undefined, partLocale).title : short(path);
  };
  const [count, setCount] = useState(0);
  const [dataDir, setDataDir] = useState("");
  const [operations, setOperations] = useState<Operation[]>([]);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [busy, setBusy] = useState("");
  const [notice, setNotice] = useState<{
    text: string;
    error: boolean;
    values?: TextValues;
    detail?: string;
    detailValues?: TextValues;
  } | null>(null);
  const [partQuery, setPartQuery] = useState("");
  const [filter, setFilter] = useState("all");
  const [candidateQuery, setCandidateQuery] = useState("");
  const [action, setAction] = useState<"replace" | "add">("replace");
  const [source, setSource] = useState("donor");
  const [chosen, setChosen] = useState("");
  const [addCategory, setAddCategory] = useState("beacon");
  const [receipts, setReceipts] = useState<Receipt[]>([]);
  const [verifyPath, setVerifyPath] = useState("");
  const [verification, setVerification] = useState<Verification[]>([]);
  const [saveModal, setSaveModal] = useState(false);
  const [saveMode, setSaveMode] = useState("new");
  const [newName, setNewName] = useState("workshop_test");
  const [restoreId, setRestoreId] = useState("");
  const [catalogQuery, setCatalogQuery] = useState("");
  const [catalogFilter, setCatalogFilter] = useState("all");
  const [catalogPage, setCatalogPage] = useState(0);
  const [candidatePage, setCandidatePage] = useState(0);
  const [nameSchema, setNameSchema] = useState(1);
  const [catalogWarnings, setCatalogWarnings] = useState<string[]>([]);
  async function refreshNameStatus() {
    const status = await rpc<{
      catalog_count: number;
      catalog_name_schema?: number;
      catalog_warnings?: string[];
      catalog_rebuild_reason?: string | null;
    }>("init");
    setCount(status.catalog_count);
    setCatalogRebuildReason(status.catalog_rebuild_reason || null);
    setNameSchema(status.catalog_name_schema || 0);
    setCatalogWarnings(status.catalog_warnings || []);
  }
  const catalogDefinitions = useMemo(
    () =>
      definitions
        .filter(truckDefinition)
        .sort((a, b) => compareParts(a, b, partLocale)),
    [definitions, partLocale],
  );
  const catalogResults = useMemo(
    () =>
      catalogDefinitions.filter(
        (d) =>
          (catalogFilter === "all" || groupOf(d) === catalogFilter) &&
          describePart(d, undefined, partLocale).search.includes(
            catalogQuery.toLowerCase(),
          ),
      ),
    [catalogDefinitions, catalogFilter, catalogQuery, partLocale],
  );
  useEffect(() => setCatalogPage(0), [catalogFilter, catalogQuery]);
  async function task(label: string, fn: () => Promise<void>) {
    setBusy(label);
    setNotice(null);
    try {
      await fn();
    } catch (e) {
      setNotice({ text: String(e), error: true });
    } finally {
      setBusy("");
    }
  }
  async function refresh(preferred?: string, include = includeAutosaves) {
    const result = await rpc<{ saves: SaveEntry[]; warnings: string[] }>(
      "discover",
      {
        include_autosaves: include,
      },
    );
    const list = result.saves;
    setDiscoveryWarnings(result.warnings);
    setSaves(list);
    setVerifyPath(
      (p) => list.find((s) => pathKey(s.path) === pathKey(p))?.path || "",
    );
    setSavePath(
      (p) =>
        list.find((s) => pathKey(s.path) === pathKey(preferred || p))?.path ||
        list[0]?.path ||
        "",
    );
  }
  async function changeAutosaves(include: boolean) {
    const previous = includeAutosaves;
    setIncludeAutosaves(include);
    try {
      await refresh(undefined, include);
    } catch (error) {
      setIncludeAutosaves(previous);
      throw error;
    }
    try {
      localStorage.setItem("ets2-workshop.include-autosaves", String(include));
    } catch {
      /* Optional preference storage. */
    }
  }
  const saveFilter = (
    <label className="save-filter">
      <input
        type="checkbox"
        checked={includeAutosaves}
        onChange={(e) => {
          const include = e.target.checked;
          void task("刷新存档列表", () => changeAutosaves(include));
        }}
      />
      {tr("包含自动存档")}
      <span>{tr("默认显示手动存档和快速存档")}</span>
    </label>
  );
  function clearEditor() {
    setOpened(null);
    setOpenedSaveLabel("");
    setOperations([]);
    setPreview(null);
    setTruckId("");
    setPartId("");
    setChosen("");
  }
  async function applySettings() {
    const result = await rpc<{ settings: Settings; catalog_changed: boolean }>(
      "settings",
      { settings },
    );
    if (result.catalog_changed) clearEditor();
    setSettings(result.settings);
    setSavedGame(result.settings.game);
    await refreshNameStatus();
    setDefinitions(await rpc<Definition[]>("catalog"));
    return result.catalog_changed;
  }
  const changingGame = pathKey(settings.game) !== pathKey(savedGame);
  async function open(path: string) {
    setOpened(null);
    setOperations([]);
    setPreview(null);
    const doc = await rpc<Opened>("open", { path });
    setOpened(doc);
    setOpenedSaveLabel(
      saves.find((s) => pathKey(s.path) === pathKey(path))?.name || path,
    );
    setFleetQuery("");
    setFleetPage(
      Math.floor(
        Math.max(
          0,
          doc.trucks.findIndex((t) => t.current),
        ) / FLEET_PAGE_SIZE,
      ),
    );
    setTruckId(
      doc.trucks.find((t) => t.current)?.id || doc.trucks[0]?.id || "",
    );
    setPartId("");
    setOperations([]);
    setPreview(null);
    setChosen("");
    setPage("garage");
  }
  useEffect(() => {
    void task("读取本地配置", async () => {
      const init = await rpc<{
        settings: Settings;
        catalog_count: number;
        catalog_name_schema?: number;
        catalog_warnings?: string[];
        catalog_rebuild_reason?: string | null;
        data_dir: string;
        needs_setup: boolean;
      }>("init");
      setSettings(init.settings);
      setSavedGame(init.settings.game);
      setCatalogRebuildReason(init.catalog_rebuild_reason || null);
      setCount(init.catalog_count);
      setNameSchema(init.catalog_name_schema || 0);
      setCatalogWarnings(init.catalog_warnings || []);
      setDataDir(init.data_dir);
      if (init.needs_setup) {
        setShowSetup(true);
        setFirstSetup(true);
        return;
      }
      setDefinitions(await rpc<Definition[]>("catalog"));
      setReceipts(await rpc<Receipt[]>("history"));
      await refresh();
    });
  }, []);
  const trucks = preview?.trucks || opened?.trucks || [];
  const filteredTrucks = useMemo(() => {
    const query = fleetQuery.trim().toLowerCase();
    return query
      ? trucks.filter((t) =>
          `${friendly(t.model)} ${t.plate} ${t.id}`
            .toLowerCase()
            .includes(query),
        )
      : trucks;
  }, [trucks, fleetQuery]);
  const fleetPages = Math.max(
    1,
    Math.ceil(filteredTrucks.length / FLEET_PAGE_SIZE),
  );
  const shownFleetPage = Math.min(fleetPage, fleetPages - 1);
  const truck = trucks.find((t) => t.id === truckId) || trucks[0];
  const part = truck?.accessories.find((a) => a.id === partId);
  const category = action === "add" ? addCategory : part?.category;
  const parts =
    truck?.accessories.filter(
      (a) =>
        (filter === "all" || groupOf(a) === filter) &&
        describePart(a.definition, a, partLocale).search.includes(
          partQuery.toLowerCase(),
        ),
    ) || [];
  const candidates = useMemo(() => {
    if (source === "donor")
      return trucks.flatMap((t) =>
        t.accessories
          .filter((a) => a.category === category && t.id !== truck?.id)
          .map((a) => ({
            key: a.id,
            path: a.path,
            donor: a.id,
            name: a.name,
            model: a.model,
            detail: `${friendly(t.model)} · ${t.plate || tr("无车牌")}`,
            definition: a.definition,
          })),
      );
    return catalogDefinitions
      .filter((d) => d.category === category)
      .map((d) => ({
        key: d.path,
        path: d.path,
        donor: null,
        name: d.name,
        model: d.model,
        detail: d.source,
        definition: d,
      }));
  }, [source, trucks, catalogDefinitions, category, truck?.id, locale]);
  const visibleCandidates = candidates.filter((c) =>
    `${describePart(c.definition, { name: c.name, path: c.path, model: c.model, category: category || "unknown" }, partLocale).search} ${c.detail}`
      .toLowerCase()
      .includes(candidateQuery.toLowerCase()),
  );
  const selected = candidates.find((c) => c.key === chosen);
  useEffect(
    () =>
      setCandidatePage((p) =>
        Math.min(p, Math.max(0, Math.ceil(visibleCandidates.length / 50) - 1)),
      ),
    [visibleCandidates.length],
  );
  useEffect(
    () =>
      setCatalogPage((p) =>
        Math.min(p, Math.max(0, Math.ceil(catalogResults.length / 60) - 1)),
      ),
    [catalogResults.length],
  );
  const readOnlyReason = action === "replace" ? replacementReason(part) : null;
  const blockedReason = candidateReason(
    part,
    selected?.definition,
    truck?.model || "",
    action,
    category,
  );
  useEffect(
    () => setCandidatePage(0),
    [candidateQuery, category, partId, source, truckId],
  );
  function selectPart(a: Accessory) {
    setPartId(a.id);
    setAction("replace");
    setChosen("");
    setCandidateQuery("");
  }
  async function stage() {
    if (!truck || !selected || blockedReason) return;
    const op: Operation = {
      truck_id: truck.id,
      action,
      accessory_id: action === "replace" ? part?.id || null : null,
      candidate_path: selected.path,
      donor_accessory: selected.donor,
    };
    const next = [
      ...operations.filter(
        (o) =>
          !(
            op.action === "replace" &&
            o.action === "replace" &&
            o.truck_id === op.truck_id &&
            o.accessory_id === op.accessory_id
          ),
      ),
      op,
    ];
    const p = await rpc<Preview>("preview", { operations: next });
    setOperations(next);
    setPreview(p);
    setChosen("");
    setNotice({ text: "已加入变更清单，尚未写入存档", error: false });
  }
  async function removeOperation(n: number) {
    const next = operations.filter((_, i) => i !== n);
    setPreview(await rpc<Preview>("preview", { operations: next }));
    setOperations(next);
  }
  async function save() {
    let r: Receipt;
    try {
      r = await rpc<Receipt>("commit", {
        operations,
        mode: saveMode,
        name: newName,
      });
    } catch (e) {
      setSaveModal(false);
      try {
        setReceipts(await rpc<Receipt[]>("history"));
      } catch {
        /* retry in history */
      }
      throw e;
    }
    setSaveModal(false);
    setOpened(null);
    setOperations([]);
    setPreview(null);
    const message =
      r.warning ||
      (saveMode === "new"
        ? "已保存为 {name}。请在游戏中手动加载；备份已保留。"
        : "已保存。请在游戏中手动加载；备份已保留。");
    const messageValues = { name: newName };
    try {
      setReceipts(await rpc<Receipt[]>("history"));
      await refresh(r.output);
      if (r.state === "completed") {
        await open(r.output);
      } else {
        setPage("history");
      }
      setNotice({ text: message, values: messageValues, error: !!r.warning });
    } catch (e) {
      setNotice({
        text: message,
        values: messageValues,
        detail: "刷新界面失败：{error}。备份：{backup}",
        detailValues: {
          error: String(e),
          backup: r.backup,
        },
        error: true,
      });
    }
  }
  const tabs = [
    ["garage", "车库", TruckIcon],
    ["catalog", "配件目录", Layers3],
    ["history", "改装记录", History],
    ["settings", "设置", Settings2],
  ] as const;
  return (
    <div className="app">
      <aside className="rail">
        <div className="brand">
          <div className="brand-icon">
            <Wrench size={23} />
          </div>
          <div>
            ETS2<span>WORKSHOP</span>
          </div>
        </div>
        <div className="rail-label">{tr("本地改装工作台")}</div>
        <nav>
          {tabs.map(([id, label, Icon]) => (
            <button
              key={id}
              className={page === id ? "nav active" : "nav"}
              onClick={() => setPage(id)}
            >
              <Icon size={19} />
              {tr(label)}
              {id === "garage" && opened && <small>{trucks.length}</small>}
            </button>
          ))}
        </nav>
        <div className="rail-bottom">
          <span className="dot" />
          {tr("本地处理 · Windows x64")}
          <div>0.2.0 / PUBLIC PREVIEW</div>
        </div>
      </aside>
      <main>
        <header>
          <div className="breadcrumb">
            ETS2 Workshop <ChevronRight size={14} />{" "}
            {tr(tabs.find((t) => t[0] === page)?.[1] || "")}
          </div>
          <div className="header-right">
            <LanguagePicker />
            <span className="chip">
              {count.toLocaleString()}
              {tr("个配件定义")}
            </span>
            <ShieldCheck size={17} />
            <span>{tr("自动备份")}</span>
          </div>
        </header>
        <fieldset disabled={!!busy} className="workspace">
          {notice && (
            <div
              role="status"
              className={"notice " + (notice.error ? "error" : "success")}
            >
              {notice.error ? <CircleAlert size={18} /> : <Check size={18} />}
              <span>
                <SourceMessage
                  text={notice.text}
                  values={notice.values}
                  error={notice.error}
                />
                {notice.detail && (
                  <p>
                    <SourceMessage
                      text={notice.detail}
                      values={notice.detailValues}
                      error
                    />
                  </p>
                )}
              </span>
              <button
                aria-label={tr("关闭消息")}
                onClick={() => setNotice(null)}
              >
                <X size={16} />
              </button>
            </div>
          )}
          {catalogRebuildReason && (
            <div className="notice">
              <span>
                {tr(
                  "配件目录需要更新，当前仅供查看。请在设置中重新建立目录后再修改。",
                )}
              </span>
              <button onClick={() => setPage("settings")}>
                {tr("前往设置")}
              </button>
            </div>
          )}
          {count > 0 && nameSchema < 1 && !catalogRebuildReason && (
            <div className="notice">
              <span>
                {tr(
                  "更新配件目录即可读取游戏内的中英文名称。现有存档不受影响。",
                )}
              </span>
              <button onClick={() => setPage("settings")}>
                {tr("前往设置")}
              </button>
            </div>
          )}
          {page === "garage" && (
            <>
              <div className="page-title">
                <div>
                  <div className="eyebrow">YOUR FLEET</div>
                  <h1>{tr("每辆卡车，都有自己的配置。")}</h1>
                  <p>
                    {tr(
                      "选择车辆，查看配件。修改先进入清单，保存时才会写入存档。",
                    )}
                  </p>
                </div>
                <button
                  className="primary"
                  disabled={!operations.length}
                  onClick={() => setSaveModal(true)}
                >
                  <Save size={17} />
                  {tr("保存修改")}
                  {operations.length > 0 && <b>{operations.length}</b>}
                </button>
              </div>
              <div className="savebar">
                <FolderOpen size={20} />
                <select
                  aria-label={tr("选择存档")}
                  value={savePath}
                  onChange={(e) => setSavePath(e.target.value)}
                >
                  <option value="">{tr("选择一个存档")}</option>
                  {saves.map((s) => (
                    <option key={s.path} value={s.path}>
                      {s.name} · {new Date(s.modified * 1000).toLocaleString()}{" "}
                      · {s.profile}
                      {s.is_autosave ? ` · ${tr("自动存档")}` : ""}
                    </option>
                  ))}
                </select>
                <button
                  onClick={() => task("读取并解密存档", () => open(savePath))}
                  disabled={!savePath}
                >
                  {tr("打开存档")}
                  <ArrowRight size={15} />
                </button>
                <button
                  className="icon-button"
                  title={tr("刷新存档列表")}
                  onClick={() => task("刷新存档列表", refresh)}
                >
                  <RefreshCw size={17} />
                </button>
                <details>
                  <summary>{tr("手动路径")}</summary>
                  <div className="path-entry">
                    <input
                      aria-label={tr("game.sii 路径")}
                      placeholder={tr("粘贴 game.sii 的完整路径")}
                      value={manualPath}
                      onChange={(e) => setManualPath(e.target.value)}
                    />
                    <button
                      disabled={!manualPath}
                      onClick={() =>
                        task("打开指定文件", () => open(manualPath))
                      }
                    >
                      {tr("打开")}
                    </button>
                  </div>
                </details>
              </div>
              {saveFilter}
              {discoveryWarnings.map((warning) => (
                <p className="inline-warning" key={warning}>
                  <SourceMessage text={warning} />
                </p>
              ))}
              {opened && (
                <p className="opened-save">
                  {tr("当前已打开：{name}", { name: openedSaveLabel })}
                </p>
              )}
              {opened?.warnings.map((w) => (
                <div className="inline-warning" key={w}>
                  <CircleAlert size={15} />
                  {<SourceMessage text={w} />}
                </div>
              ))}
              {!opened ? (
                <div className="empty welcome">
                  <div className="welcome-icon">
                    <TruckIcon size={52} />
                  </div>
                  <h2>{tr("从你的车库开始")}</h2>
                  <p>
                    {tr("打开一个存档，自动解密并读取所有自有卡车。")}
                    <br />
                    {tr("发动机、油箱、轮胎和外观附件，都在同一个工作台。")}
                  </p>
                  <div className="steps">
                    <span>
                      <b>01</b>
                      {tr("打开存档")}
                    </span>
                    <ArrowRight size={17} />
                    <span>
                      <b>02</b>
                      {tr("选择配件")}
                    </span>
                    <ArrowRight size={17} />
                    <span>
                      <b>03</b>
                      {tr("预览并保存")}
                    </span>
                  </div>
                  {!count && (
                    <button onClick={() => setPage("settings")}>
                      {tr("建立本机配件目录")}
                      <ChevronRight size={16} />
                    </button>
                  )}
                </div>
              ) : (
                <>
                  <div className="garage-grid" ref={garageRef}>
                    <section className="fleet" aria-label={tr("车库列表")}>
                      <div className="section-head">
                        <h2>{tr("我的卡车")}</h2>
                        <span>
                          {trucks.length}
                          {tr("辆")}
                        </span>
                      </div>
                      <label className="search">
                        <Search size={16} />
                        <input
                          aria-label={tr("搜索卡车")}
                          placeholder={tr("车型、车牌或编号")}
                          value={fleetQuery}
                          onChange={(e) => {
                            setFleetQuery(e.target.value);
                            setFleetPage(0);
                          }}
                        />
                      </label>
                      {fleetPages > 1 && (
                        <div className="button-row">
                          <button
                            disabled={shownFleetPage === 0}
                            onClick={() => setFleetPage(shownFleetPage - 1)}
                          >
                            {tr("上一页")}
                          </button>
                          <span>
                            {shownFleetPage + 1} / {fleetPages}
                          </span>
                          <button
                            disabled={shownFleetPage + 1 === fleetPages}
                            onClick={() => setFleetPage(shownFleetPage + 1)}
                          >
                            {tr("下一页")}
                          </button>
                        </div>
                      )}
                      {fleetQuery && filteredTrucks.length > 1 && (
                        <p className="micro">
                          {tr("匹配 {count} 辆，请选择目标车辆", {
                            count: filteredTrucks.length,
                          })}
                        </p>
                      )}
                      <div
                        className="fleet-list"
                        key={`${shownFleetPage}:${fleetQuery}`}
                        aria-label={tr("本页车辆")}
                      >
                        {!filteredTrucks.length && (
                          <p>{tr("没有匹配的卡车")}</p>
                        )}
                        {filteredTrucks
                          .slice(
                            shownFleetPage * FLEET_PAGE_SIZE,
                            (shownFleetPage + 1) * FLEET_PAGE_SIZE,
                          )
                          .map((t) => (
                            <button
                              className={
                                "truck-card " +
                                (truck?.id === t.id ? "selected" : "")
                              }
                              key={t.id}
                              onClick={() => {
                                setTruckId(t.id);
                                setPartId("");
                                setChosen("");
                                setFilter("all");
                              }}
                            >
                              <div className="truck-card-top">
                                <TruckIcon size={23} />
                                {t.current && (
                                  <span className="tag">{tr("正在驾驶")}</span>
                                )}
                              </div>
                              <strong>{friendly(t.model)}</strong>
                              <div className="plate">
                                {t.plate || tr("未设置车牌")}
                              </div>
                              <small>
                                {t.accessories.length}
                                {tr("个配件")} <ChevronRight size={13} />
                              </small>
                            </button>
                          ))}
                      </div>
                    </section>
                    <section className="parts-panel">
                      <div className="section-head">
                        <div>
                          <h2>{friendly(truck?.model || "")}</h2>
                          <span>
                            {truck?.plate}
                            {tr("· 完整配件列表")}
                          </span>
                        </div>
                        <button
                          className="small"
                          onClick={() => {
                            setAction("add");
                            setChosen("");
                            setSource("donor");
                          }}
                        >
                          <Plus size={14} />
                          {tr("添加附件")}
                        </button>
                      </div>
                      <div className="filters">
                        <div className="search">
                          <Search size={15} />
                          <input
                            aria-label={tr("搜索车辆配件")}
                            placeholder={tr("搜索名称、类别或路径")}
                            value={partQuery}
                            onChange={(e) => setPartQuery(e.target.value)}
                          />
                        </div>
                        <select
                          aria-label={tr("配件类别")}
                          value={filter}
                          onChange={(e) => setFilter(e.target.value)}
                        >
                          <option value="all">{tr("所有类别")}</option>
                          {groups.map(([id, name]) => (
                            <option key={id} value={id}>
                              {tr(name)}
                            </option>
                          ))}
                        </select>
                      </div>
                      <div
                        className="part-list"
                        key={truck?.id}
                        aria-label={tr("当前车辆配件")}
                      >
                        {parts.map((a) => (
                          <button
                            key={a.id}
                            className={
                              "part-row " +
                              (part?.id === a.id && action === "replace"
                                ? "selected"
                                : "")
                            }
                            onClick={() => selectPart(a)}
                          >
                            <span className="part-number">
                              {String(a.index).padStart(2, "0")}
                            </span>
                            <span className="part-description">
                              <strong>
                                {tr(
                                  categoryLabel(
                                    a.category,
                                    a.definition,
                                    partLocale,
                                  ),
                                )}
                                <em>
                                  {a.definition
                                    ? tr("已识别")
                                    : tr("只读 / 未索引")}
                                </em>
                              </strong>
                              <span>
                                {
                                  describePart(a.definition, a, partLocale)
                                    .title
                                }
                              </span>
                              <small>
                                {
                                  describePart(a.definition, a, partLocale)
                                    .summary
                                }
                              </small>
                            </span>
                            <ChevronRight size={15} />
                          </button>
                        ))}
                        {!parts.length && (
                          <div className="empty">{tr("没有匹配的配件")}</div>
                        )}
                      </div>
                      <div className="panel-foot">
                        {parts.length} / {truck?.accessories.length}
                        {tr("个配件 · 未识别字段仍会保留")}
                      </div>
                    </section>
                    <section
                      className="inspector"
                      aria-label={tr("配件详情与替换")}
                      key={truck?.id}
                    >
                      <div className="section-head">
                        <h2>
                          {action === "add"
                            ? tr("追加外观附件")
                            : tr("配件详情")}
                        </h2>
                        <span className="tag">
                          {action === "add" ? "ADD" : "REPLACE"}
                        </span>
                      </div>
                      {action === "replace" && !part ? (
                        <div className="empty inspector-empty">
                          <Wrench size={32} />
                          <p>
                            {tr("选择一个配件")}
                            <br />
                            {tr("查看参数与替换方案")}
                          </p>
                        </div>
                      ) : (
                        <>
                          {action === "add" ? (
                            <>
                              <p className="muted">
                                {tr(
                                  "只接受同车型、同驾驶室和底盘的供体附件；安装类别不能已被占用。",
                                )}
                              </p>
                              <label>
                                {tr("附件类别")}
                                <select
                                  value={addCategory}
                                  onChange={(e) => {
                                    setAddCategory(e.target.value);
                                    setChosen("");
                                  }}
                                >
                                  {[
                                    "beacon",
                                    "r_grill",
                                    "f_grill",
                                    "sunshld",
                                    "engine",
                                    "transmission",
                                    "chassis",
                                    "tank",
                                  ].map((c) => (
                                    <option value={c} key={c}>
                                      {tr(labels[c] || c)}
                                      {[
                                        "engine",
                                        "transmission",
                                        "chassis",
                                        "tank",
                                      ].includes(c)
                                        ? tr(" · 禁止追加")
                                        : ""}
                                    </option>
                                  ))}
                                </select>
                              </label>
                            </>
                          ) : (
                            <>
                              <div className="current-part">
                                <span className="eyebrow">
                                  {tr("当前")}{" "}
                                  {tr(
                                    categoryLabel(
                                      part!.category,
                                      part!.definition,
                                      partLocale,
                                    ),
                                  )}
                                </span>
                                <h3>
                                  {
                                    describePart(
                                      part!.definition,
                                      part!,
                                      partLocale,
                                    ).title
                                  }
                                </h3>
                                <b className="part-highlight">
                                  {
                                    describePart(
                                      part!.definition,
                                      part!,
                                      partLocale,
                                    ).summary
                                  }
                                </b>
                                <span>
                                  {
                                    describePart(
                                      part!.definition,
                                      part!,
                                      partLocale,
                                    ).origin
                                  }
                                </span>
                              </div>
                              <details className="current-metrics">
                                <summary>{tr("当前配件全部参数")}</summary>
                                <Metrics def={part!.definition} />
                              </details>
                              <details className="raw">
                                <summary>
                                  {tr("查看原始字段与定义路径")}
                                </summary>
                                <code className="break">{part!.path}</code>
                                <p>
                                  {tr("原始名称：")}
                                  {part!.name}
                                  {tr("· 原始分类：")}
                                  {part!.category}
                                </p>
                                <pre>{part!.raw}</pre>
                                <p>
                                  {tr("refund 是存档退款字段；替换时保留原值")}{" "}
                                  {part!.refund}。
                                </p>
                              </details>
                            </>
                          )}
                          {readOnlyReason && (
                            <p className="inline-warning">
                              {tr(readOnlyReason)}
                            </p>
                          )}
                          <div className="divider" />
                          <h3 className="subheading">
                            {action === "add"
                              ? tr("选择供体附件")
                              : tr("替换为")}
                          </h3>
                          <div className="segmented">
                            <button
                              className={source === "donor" ? "active" : ""}
                              onClick={() => {
                                setSource("donor");
                                setChosen("");
                              }}
                            >
                              {tr("从车库选择")}
                            </button>
                            <button
                              className={source === "catalog" ? "active" : ""}
                              disabled={action === "add"}
                              onClick={() => {
                                setSource("catalog");
                                setChosen("");
                              }}
                            >
                              {tr("本机配件库")}
                            </button>
                          </div>
                          <div className="search">
                            <Search size={15} />
                            <input
                              aria-label={tr("搜索候选配件")}
                              placeholder={tr("搜索品牌、型号、马力或配件")}
                              value={candidateQuery}
                              onChange={(e) =>
                                setCandidateQuery(e.target.value)
                              }
                            />
                          </div>
                          <div className="candidates">
                            {visibleCandidates
                              .slice(
                                candidatePage * 50,
                                (candidatePage + 1) * 50,
                              )
                              .map((c) => (
                                <button
                                  key={c.key}
                                  className={
                                    "candidate " +
                                    (chosen === c.key ? "selected" : "")
                                  }
                                  onClick={() => setChosen(c.key)}
                                >
                                  <span>
                                    <strong>
                                      {
                                        describePart(
                                          c.definition,
                                          {
                                            name: c.name,
                                            path: c.path,
                                            model: c.model,
                                            category: category || "unknown",
                                          },
                                          partLocale,
                                        ).title
                                      }
                                    </strong>
                                    <b className="candidate-spec">
                                      {
                                        describePart(
                                          c.definition,
                                          undefined,
                                          partLocale,
                                        ).summary
                                      }
                                    </b>
                                    <small>
                                      {
                                        describePart(
                                          c.definition,
                                          undefined,
                                          partLocale,
                                        ).origin
                                      }
                                      {source === "donor"
                                        ? tr(" · 供体 {detail}", {
                                            detail: c.detail || tr("无车牌"),
                                          })
                                        : ""}
                                    </small>
                                  </span>
                                  {chosen === c.key && <Check size={15} />}
                                </button>
                              ))}
                            {!visibleCandidates.length && (
                              <p className="muted">
                                {tr(
                                  "没有候选配件。可切换配件库，或先建立目录。",
                                )}
                              </p>
                            )}
                          </div>
                          <p className="micro">
                            {tr("共 {count} 项候选", {
                              count: visibleCandidates.length,
                            })}
                          </p>
                          {visibleCandidates.length > 50 && (
                            <div className="button-row candidate-pagination">
                              <button
                                disabled={candidatePage === 0}
                                onClick={() =>
                                  setCandidatePage(candidatePage - 1)
                                }
                              >
                                {tr("上一页")}
                              </button>
                              <span>
                                {candidatePage + 1} /{" "}
                                {Math.ceil(visibleCandidates.length / 50)}
                              </span>
                              <button
                                disabled={
                                  (candidatePage + 1) * 50 >=
                                  visibleCandidates.length
                                }
                                onClick={() =>
                                  setCandidatePage(candidatePage + 1)
                                }
                              >
                                {tr("下一页")}
                              </button>
                            </div>
                          )}
                          {selected && (
                            <div className="candidate-metrics">
                              <Comparison
                                before={
                                  action === "replace" ? part?.definition : null
                                }
                                after={selected.definition}
                              />
                            </div>
                          )}
                          {selected &&
                            blockedReason &&
                            blockedReason !== readOnlyReason && (
                              <p className="inline-warning">
                                {tr(blockedReason)}
                              </p>
                            )}
                          <button
                            className="primary full"
                            disabled={
                              !selected ||
                              !!blockedReason ||
                              !!catalogRebuildReason
                            }
                            onClick={() => task("校验改装规则", stage)}
                          >
                            <Plus size={16} />
                            {tr("加入变更清单")}
                          </button>
                          <p className="micro">
                            {tr(
                              "发动机、变速箱、底盘等核心部件禁止重复追加。所有操作由后台再次校验。",
                            )}
                          </p>
                        </>
                      )}
                    </section>
                  </div>
                  {preview && operations.length > 0 && (
                    <section className="changes">
                      <div className="section-head">
                        <h2>
                          {tr("待保存的修改")}{" "}
                          <span className="tag">{operations.length}</span>
                        </h2>
                        <span>{tr("尚未写入游戏存档")}</span>
                      </div>
                      {preview.changes.map((c, n) => (
                        <div className="change" key={n}>
                          <span>
                            {friendly(c.model)}
                            <small>
                              {tr(labels[c.category] || c.category)}
                            </small>
                          </span>
                          <code>
                            {c.before ? displayPath(c.before) : tr("新增")}
                          </code>
                          <ArrowRight size={16} />
                          <code>{displayPath(c.after)}</code>
                          <button
                            className="icon-button"
                            aria-label={tr("移除此修改")}
                            onClick={() =>
                              task("更新清单", () => removeOperation(n))
                            }
                          >
                            <X size={16} />
                          </button>
                        </div>
                      ))}
                      {preview.warnings.map((w, i) => (
                        <p className="inline-warning" key={i}>
                          <CircleAlert size={15} />
                          {<SourceMessage text={w} />}
                        </p>
                      ))}
                    </section>
                  )}
                </>
              )}
            </>
          )}
          {page === "settings" && (
            <>
              <div className="page-title">
                <div>
                  <div className="eyebrow">LOCAL CONFIGURATION</div>
                  <h1>{tr("连接你的游戏。")}</h1>
                  <p>
                    {tr(
                      "文件保留在这台电脑上。存档解码器已内置，解包工具由程序自动准备。",
                    )}
                  </p>
                </div>
              </div>
              <div className="settings-grid">
                <section className="card">
                  <h2>{tr("文件位置")}</h2>
                  <button
                    disabled={operations.length > 0}
                    onClick={() => setShowSetup(true)}
                  >
                    <RefreshCw size={16} />
                    {tr("重新运行配置向导")}
                  </button>
                  {(
                    [
                      ["documents", "ETS2 用户数据目录"],
                      ["game", "游戏安装目录"],
                    ] as const
                  ).map(([key, title]) => (
                    <label key={key}>
                      {tr(title)}
                      <input
                        value={settings[key]}
                        onChange={(e) =>
                          setSettings({ ...settings, [key]: e.target.value })
                        }
                      />
                    </label>
                  ))}
                  {operations.length > 0 && (
                    <p className="inline-warning">
                      {tr("请先保存或移除待保存修改，再更换游戏安装或重新配置")}
                    </p>
                  )}
                  <button
                    className="primary"
                    disabled={changingGame && operations.length > 0}
                    onClick={() =>
                      task("保存设置", async () => {
                        await applySettings();
                        await refresh();
                        setNotice({ text: "设置已保存", error: false });
                      })
                    }
                  >
                    {tr("保存设置")}
                  </button>
                </section>
                <section className="card">
                  <Database size={30} />
                  <h2>{tr("建立本机配件目录")}</h2>
                  <p>
                    {tr(
                      "读取 def.scs 及官方车型、轮胎与改装 DLC，获得真实定义路径、配件类型和性能参数。首次解包需要一些时间。",
                    )}
                  </p>
                  {catalogWarnings.length > 0 && (
                    <details>
                      <summary>{tr("目录提示")}</summary>
                      {catalogWarnings.map((warning, i) => (
                        <SourceMessage key={i} text={warning} />
                      ))}
                    </details>
                  )}
                  <div className="stat">
                    {count.toLocaleString()}
                    <span>{tr("已索引定义")}</span>
                  </div>
                  {operations.length > 0 && (
                    <p className="inline-warning">
                      {tr("请先保存或移除待保存修改，再更新目录。")}
                    </p>
                  )}
                  <button
                    className="primary"
                    disabled={operations.length > 0}
                    onClick={() =>
                      task(
                        "正在解包并索引游戏定义，首次运行可能需要数分钟",
                        async () => {
                          const changedGame = await applySettings();
                          const r = await rpc<{
                            count: number;
                            warnings: string[];
                          }>("index");
                          setCount(r.count);
                          await refreshNameStatus();
                          setDefinitions(await rpc<Definition[]>("catalog"));
                          if (opened && !changedGame) await open(opened.path);
                          setNotice({
                            text: "已索引 {count} 个配件定义。目录仅涵盖已识别的官方资源。",
                            values: { count: r.count },
                            error: false,
                          });
                        },
                      )
                    }
                  >
                    <RefreshCw size={16} />
                    {tr("建立 / 更新目录")}
                  </button>
                  <p className="micro">
                    {tr(
                      "当前不支持 Mod；检测到 Mod 或未知扩展依赖的存档将拒绝打开。",
                    )}
                  </p>
                  <details>
                    <summary>{tr("本地缓存与备份目录")}</summary>
                    <code className="break">{dataDir}</code>
                  </details>
                </section>
              </div>
            </>
          )}
          {page === "catalog" && (
            <>
              <div className="page-title">
                <div>
                  <div className="eyebrow">PARTS LIBRARY</div>
                  <h1>{tr("从定义认识性能。")}</h1>
                  <p>
                    {tr(
                      "参数来自本机游戏文件。显示马力与实际动力参数分开，最高速度不作精确预测。",
                    )}
                  </p>
                </div>
                <button onClick={() => setPage("settings")}>
                  <Database size={16} />
                  {tr("更新目录")}
                </button>
              </div>
              <div className="search catalog-search">
                <Search size={18} />
                <input
                  aria-label={tr("搜索配件目录")}
                  placeholder={tr("搜索品牌、型号、engine、tank…")}
                  value={catalogQuery}
                  onChange={(e) => setCatalogQuery(e.target.value)}
                />
              </div>
              <div className="catalog-filters">
                <select
                  aria-label={tr("目录分类")}
                  value={catalogFilter}
                  onChange={(e) => setCatalogFilter(e.target.value)}
                >
                  <option value="all">{tr("所有类别")}</option>
                  {groups.map(([id, name]) => (
                    <option key={id} value={id}>
                      {tr(name)}
                    </option>
                  ))}
                </select>
                <p>
                  {tr(
                    "车型专属件优先，其次为共享件，最后为未识别车型。共享并不代表适合所有卡车。",
                  )}
                </p>
              </div>
              <div className="catalog-grid">
                {catalogResults
                  .slice(catalogPage * 60, (catalogPage + 1) * 60)
                  .map((d) => (
                    <article className="card definition" key={d.path}>
                      <span className="tag">
                        {tr(categoryLabel(d.category, d, partLocale))}
                      </span>
                      <h3>{describePart(d, undefined, partLocale).title}</h3>
                      <span className="tag">
                        {tr(
                          partOrigin(d) === "brand"
                            ? "车型专属"
                            : partOrigin(d) === "shared"
                              ? "共享件"
                              : "未识别车型",
                        )}
                      </span>
                      <b className="part-highlight">
                        {describePart(d, undefined, partLocale).summary}
                      </b>
                      <p>{describePart(d, undefined, partLocale).origin}</p>
                      <Metrics def={d} />
                      <details>
                        <summary>{tr("路径与适配条件")}</summary>
                        <code className="break">{d.path}</code>
                        <p>{d.kind}</p>
                        <p>
                          {d.suitable.join(", ") ||
                            tr("定义未指定 suitable_for")}
                        </p>
                      </details>
                    </article>
                  ))}
              </div>
              {!catalogResults.length && (
                <div className="empty">
                  {tr(
                    count
                      ? "没有匹配的配件。请调整搜索词或分类。"
                      : "请先在设置中建立配件目录。",
                  )}
                </div>
              )}
              <div className="button-row catalog-pagination">
                <button
                  disabled={catalogPage === 0}
                  onClick={() => setCatalogPage(catalogPage - 1)}
                >
                  {tr("上一页")}
                </button>
                <span>
                  {catalogPage + 1} /{" "}
                  {Math.max(1, Math.ceil(catalogResults.length / 60))} ·{" "}
                  {tr("共 {count} 项", { count: catalogResults.length })}
                </span>
                <button
                  disabled={(catalogPage + 1) * 60 >= catalogResults.length}
                  onClick={() => setCatalogPage(catalogPage + 1)}
                >
                  {tr("下一页")}
                </button>
              </div>
            </>
          )}
          {page === "history" && (
            <>
              <div className="page-title">
                <div>
                  <div className="eyebrow">CHANGE HISTORY</div>
                  <h1>{tr("每次改装，都有据可查。")}</h1>
                  <p>
                    {tr(
                      "读取游戏另存的结果，检查配件是否保留。车辆身份变化时会提示手动核对。",
                    )}
                  </p>
                </div>
                <button
                  onClick={() =>
                    task("刷新记录", async () =>
                      setReceipts(await rpc<Receipt[]>("history")),
                    )
                  }
                >
                  <RefreshCw size={16} />
                  {tr("刷新")}
                </button>
              </div>
              <section className="card">
                <label>
                  {tr("用于复查的游戏存档")}
                  <select
                    value={verifyPath}
                    onChange={(e) => setVerifyPath(e.target.value)}
                  >
                    <option value="">{tr("选择游戏另存的结果")}</option>
                    {saves.map((s) => (
                      <option key={s.path} value={s.path}>
                        {s.name} · {s.profile}
                      </option>
                    ))}
                  </select>
                </label>
                {saveFilter}
                {discoveryWarnings.map((warning) => (
                  <p className="inline-warning" key={warning}>
                    <SourceMessage text={warning} />
                  </p>
                ))}
                <button onClick={() => task("刷新游戏存档", refresh)}>
                  {tr("刷新存档列表")}
                </button>
              </section>
              <div className="history-list">
                {receipts.map((r) => (
                  <article className="card" key={r.id}>
                    <div className="section-head">
                      <h3>
                        {r.changes.length}
                        {tr("项改装")}
                      </h3>
                      <span>
                        {new Date(
                          Number(r.id.split("-")[0]) * 1000,
                        ).toLocaleString()}
                      </span>
                    </div>
                    <code className="break">{r.output}</code>
                    {r.state === "preparing" && (
                      <p>
                        {tr(
                          "准备未完成，本工具尚未写入目标。备份可能不完整，可清理临时文件后重新保存。",
                        )}
                      </p>
                    )}
                    {r.state === "prepared" && (
                      <p>
                        {tr(
                          "写入未确认或已中断。备份保留在下方位置；恢复时会核对目标内容，拒绝覆盖新的进度。",
                        )}
                      </p>
                    )}
                    {r.changes.map((c, i) => (
                      <p key={i}>
                        {friendly(c.model)} ·{" "}
                        {tr(labels[c.category] || c.category)} →{" "}
                        {displayPath(c.after)}
                      </p>
                    ))}
                    <div className="button-row">
                      <button
                        disabled={!verifyPath}
                        onClick={() =>
                          task("检查改装是否保留", async () =>
                            setVerification(
                              await rpc<Verification[]>("verify", {
                                id: r.id,
                                path: verifyPath,
                              }),
                            ),
                          )
                        }
                      >
                        <ShieldCheck size={16} />
                        {tr("复查所选存档")}
                      </button>
                      <button onClick={() => setRestoreId(r.id)}>
                        <Undo2 size={16} />
                        {tr("恢复修改前")}
                      </button>
                    </div>
                    <button
                      onClick={() =>
                        task("清理临时文件", async () => {
                          await rpc("cleanup", { id: r.id });
                          setNotice({
                            text: "已清理该记录的临时文件，备份和存档已保留",
                            error: false,
                          });
                        })
                      }
                    >
                      {tr("清理临时文件")}
                    </button>
                    <details>
                      <summary>{tr("备份位置")}</summary>
                      <code className="break">{r.backup}</code>
                    </details>
                  </article>
                ))}
              </div>
              {!receipts.length && (
                <div className="empty">
                  {tr("保存第一次改装后，记录将显示在这里。")}
                </div>
              )}
              {verification.length > 0 && (
                <section className="card">
                  <h2>{tr("复查结果")}</h2>
                  {verification.map((v, i) => (
                    <p key={i}>
                      <strong>
                        {tr(labels[v.category] || v.category)}：
                        {<SourceMessage text={v.status} />}
                      </strong>
                      <br />
                      <code className="break">{v.matches.join("\n")}</code>
                    </p>
                  ))}
                </section>
              )}
            </>
          )}
        </fieldset>
        <footer>
          <span>ETS2 Workshop</span>
          <span>
            {tr("独立油箱与发动机跨品牌替换已实测 · 外观组合仍需游戏内验证")}
          </span>
        </footer>
      </main>
      {showSetup && (
        <Onboarding
          initial={settings}
          onClose={firstSetup ? undefined : () => setShowSetup(false)}
          onComplete={async (s, n) => {
            clearEditor();
            setSettings(s);
            setSavedGame(s.game);
            setCount(n);
            setShowSetup(false);
            setFirstSetup(false);
            // Setup is already committed. Keep post-setup read errors visible in the app.
            await task("正在读取车库…", async () => {
              await refreshNameStatus();
              setDefinitions(await rpc<Definition[]>("catalog"));
              setReceipts(await rpc<Receipt[]>("history"));
              await refresh();
            });
          }}
        />
      )}
      {busy && (
        <div className="busy" role="status">
          <LoaderCircle className="spin" size={18} />
          {tr(busy)}
        </div>
      )}
      {saveModal && (
        <div className="modal-backdrop">
          <section className="modal">
            <div className="section-head">
              <h2>{tr("保存这次改装")}</h2>
              <button
                aria-label={tr("关闭")}
                disabled={!!busy}
                onClick={() => setSaveModal(false)}
              >
                <X size={18} />
              </button>
            </div>
            <p>
              {operations.length}{" "}
              {tr("项变更已通过结构和操作规则检查。保存前自动备份。")}
            </p>
            <label>
              {tr("保存方式")}
              <select
                disabled={!!busy}
                value={saveMode}
                onChange={(e) => setSaveMode(e.target.value)}
              >
                <option value="new">{tr("另存为新存档（推荐）")}</option>
                <option value="overwrite">{tr("备份并覆盖当前存档")}</option>
              </select>
            </label>
            {saveMode === "new" && (
              <label>
                {tr("新存档名称")}
                <input
                  disabled={!!busy}
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  maxLength={80}
                />
              </label>
            )}
            <div className="inline-warning">
              <CircleAlert size={17} />
              {tr("保存后需要在游戏中手动加载。改装升级可能恢复原厂配件。")}
              {saveMode === "overwrite" && (
                <span>
                  {tr(
                    "覆盖或恢复前，请退出游戏并暂停会写入该存档的同步及其他程序。",
                  )}
                </span>
              )}
            </div>
            <button
              className="primary full"
              disabled={!!busy}
              onClick={() => task("备份、校验并写入存档", save)}
            >
              <Save size={17} />
              {tr("确认保存")}
            </button>
          </section>
        </div>
      )}
      {restoreId && (
        <div className="modal-backdrop">
          <section className="modal">
            <h2>{tr("恢复修改前的存档？")}</h2>
            <p>
              {tr(
                "覆盖或恢复前，请退出游戏并暂停会写入该存档的同步及其他程序。",
              )}
            </p>
            <p>
              {tr(
                "仅恢复车辆和游戏进度文件 game.sii，不删除存档槽，也不恢复名称、截图或 info.sii。 另存的槽位会保留新名称。恢复前会核对目标内容，新记录也会核对存档信息；发现更新则拒绝恢复。",
              )}
            </p>
            <div className="button-row">
              <button disabled={!!busy} onClick={() => setRestoreId("")}>
                {tr("取消")}
              </button>
              <button
                className="primary"
                disabled={!!busy}
                onClick={() =>
                  task("恢复备份", async () => {
                    const path = await rpc<string>("restore", {
                      id: restoreId,
                    });
                    setRestoreId("");
                    setOpened(null);
                    setOperations([]);
                    setPreview(null);
                    try {
                      await refresh(path);
                      await open(path);
                      setNotice({
                        text: "已恢复修改前的 game.sii；存档名称和信息保持不变",
                        error: false,
                      });
                    } catch (e) {
                      setNotice({
                        text: "恢复已完成，刷新界面失败：{error}",
                        values: { error: String(e) },
                        error: true,
                      });
                    }
                  })
                }
              >
                {tr("恢复")}
              </button>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}
