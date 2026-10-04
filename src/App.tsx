import Onboarding from "./Onboarding";
import { useEffect, useMemo, useState } from "react";
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

const labels: Record<string, string> = {
  engine: "发动机",
  transmission: "变速箱",
  tank: "独立油箱",
  chassis: "底盘",
  cabin: "驾驶室",
  interior: "内饰",
  vehicle: "车辆基础",
  paint_job: "涂装",
  f_tire: "前轮轮胎",
  r_tire: "后轮轮胎",
  f_disc: "前轮轮毂",
  r_disc: "后轮轮毂",
  beacon: "警示灯",
  r_grill: "车顶灯架",
  f_grill: "防撞杆",
  sunshld: "遮阳板",
  sunshield: "遮阳板",
  head_light: "前大灯",
  badge: "车型铭牌",
  unknown: "未识别",
};
const metrics: Record<string, string> = {
  type: "动力类型",
  price: "商店价格 (€)",
  torque: "峰值扭矩 (Nm)",
  rpm_limit: "转速上限 (rpm)",
  rpm_limit_neutral: "空挡上限 (rpm)",
  volume: "排量 (L)",
  fuel_tank_size: "燃油容量 (L)",
  tank_size: "底盘油箱 (L)",
  differential_ratio: "主减速比",
  retarder: "缓速器级数",
  info: "游戏显示信息",
  ratios_forward: "前进挡齿比",
  ratios_reverse: "倒挡齿比",
  torque_curve: "扭矩曲线",
  consumption_coef: "油耗系数",
};
const friendly = (s: string) =>
  s
    .replace("scania.s_2016", "Scania S")
    .replace("volvo.fh_2024", "Volvo FH6")
    .replace("daf.2021", "DAF XG / XG+")
    .replace("daf.xf", "DAF XF105")
    .replaceAll("_", " ")
    .replaceAll(".", " ");
const short = (p: string) => p.split("/").slice(-2).join("/");
async function rpc<T>(
  action: string,
  extra: Record<string, unknown> = {},
): Promise<T> {
  return invoke("rpc", { payload: { action, ...extra } });
}

export default function App() {
  const [page, setPage] = useState("garage");
  const [settings, setSettings] = useState<Settings>({
    documents: "",
    game: "",
    extractor: "",
    onboarding_version: 0,
  });
  const [showSetup, setShowSetup] = useState(false);
  const [firstSetup, setFirstSetup] = useState(false);
  const [saves, setSaves] = useState<SaveEntry[]>([]);
  const [savePath, setSavePath] = useState("");
  const [manualPath, setManualPath] = useState("");
  const [opened, setOpened] = useState<Opened | null>(null);
  const [truckId, setTruckId] = useState("");
  const [partId, setPartId] = useState("");
  const [definitions, setDefinitions] = useState<Definition[]>([]);
  const [count, setCount] = useState(0);
  const [dataDir, setDataDir] = useState("");
  const [operations, setOperations] = useState<Operation[]>([]);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [busy, setBusy] = useState("");
  const [notice, setNotice] = useState<{ text: string; error: boolean } | null>(
    null,
  );
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
  async function refresh() {
    const list = await rpc<SaveEntry[]>("discover");
    setSaves(list);
    setSavePath((p) =>
      list.some((s) => s.path === p) ? p : list[0]?.path || "",
    );
  }
  async function open(path: string) {
    setOpened(null);
    setOperations([]);
    setPreview(null);
    const doc = await rpc<Opened>("open", { path });
    setOpened(doc);
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
        data_dir: string;
        needs_setup: boolean;
      }>("init");
      setSettings(init.settings);
      setCount(init.catalog_count);
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
  const truck = trucks.find((t) => t.id === truckId) || trucks[0];
  const part = truck?.accessories.find((a) => a.id === partId);
  const category = action === "add" ? addCategory : part?.category;
  const parts =
    truck?.accessories.filter(
      (a) =>
        (filter === "all" || a.category === filter) &&
        `${a.name} ${a.path} ${labels[a.category] || ""}`
          .toLowerCase()
          .includes(partQuery.toLowerCase()),
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
            model: friendly(t.model),
            detail: t.plate,
            definition: a.definition,
          })),
      );
    return definitions
      .filter((d) => d.category === category)
      .map((d) => ({
        key: d.path,
        path: d.path,
        donor: null,
        name: d.name,
        model: friendly(d.model),
        detail: d.source,
        definition: d,
      }));
  }, [source, trucks, definitions, category, truck?.id]);
  const visibleCandidates = candidates.filter((c) =>
    `${c.name} ${c.model} ${c.path} ${c.detail}`
      .toLowerCase()
      .includes(candidateQuery.toLowerCase()),
  );
  const selected = candidates.find((c) => c.key === chosen);
  function selectPart(a: Accessory) {
    setPartId(a.id);
    setAction("replace");
    setChosen("");
    setCandidateQuery("");
  }
  async function stage() {
    if (!truck || !selected) return;
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
    const r = await rpc<Receipt>("commit", {
      operations,
      mode: saveMode,
      name: newName,
    });
    setSaveModal(false);
    setReceipts(await rpc<Receipt[]>("history"));
    await refresh();
    await open(r.output);
    setSavePath(r.output);
    setNotice({
      text: `已保存${saveMode === "new" ? "为 " + newName : ""}。请在游戏中手动加载；备份已保留。`,
      error: false,
    });
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
        <div className="rail-label">本地改装工作台</div>
        <nav>
          {tabs.map(([id, label, Icon]) => (
            <button
              key={id}
              className={page === id ? "nav active" : "nav"}
              onClick={() => setPage(id)}
            >
              <Icon size={19} />
              {label}
              {id === "garage" && opened && <small>{trucks.length}</small>}
            </button>
          ))}
        </nav>
        <div className="rail-bottom">
          <span className="dot" />
          本地处理 · Windows x64<div>0.2.0 / PUBLIC PREVIEW</div>
        </div>
      </aside>
      <main>
        <header>
          <div className="breadcrumb">
            ETS2 Workshop <ChevronRight size={14} />{" "}
            {tabs.find((t) => t[0] === page)?.[1]}
          </div>
          <div className="header-right">
            <span className="chip">{count.toLocaleString()} 个配件定义</span>
            <ShieldCheck size={17} />
            <span>自动备份</span>
          </div>
        </header>
        <fieldset disabled={!!busy} className="workspace">
          {notice && (
            <div
              role="status"
              className={"notice " + (notice.error ? "error" : "success")}
            >
              {notice.error ? <CircleAlert size={18} /> : <Check size={18} />}
              <span>{notice.text}</span>
              <button aria-label="关闭消息" onClick={() => setNotice(null)}>
                <X size={16} />
              </button>
            </div>
          )}
          {page === "garage" && (
            <>
              <div className="page-title">
                <div>
                  <div className="eyebrow">YOUR FLEET</div>
                  <h1>每辆卡车，都有自己的配置。</h1>
                  <p>
                    选择车辆，查看配件。修改先进入清单，保存时才会写入存档。
                  </p>
                </div>
                <button
                  className="primary"
                  disabled={!operations.length}
                  onClick={() => setSaveModal(true)}
                >
                  <Save size={17} />
                  保存修改 {operations.length > 0 && <b>{operations.length}</b>}
                </button>
              </div>
              <div className="savebar">
                <FolderOpen size={20} />
                <select
                  aria-label="选择存档"
                  value={savePath}
                  onChange={(e) => setSavePath(e.target.value)}
                >
                  <option value="">选择一个存档</option>
                  {saves.map((s) => (
                    <option key={s.path} value={s.path}>
                      {s.name} · {new Date(s.modified * 1000).toLocaleString()}{" "}
                      · {s.profile}
                    </option>
                  ))}
                </select>
                <button
                  onClick={() => task("读取并解密存档", () => open(savePath))}
                  disabled={!savePath}
                >
                  打开存档
                  <ArrowRight size={15} />
                </button>
                <button
                  className="icon-button"
                  title="刷新存档列表"
                  onClick={() => task("刷新存档列表", refresh)}
                >
                  <RefreshCw size={17} />
                </button>
                <details>
                  <summary>手动路径</summary>
                  <div className="path-entry">
                    <input
                      aria-label="game.sii 路径"
                      placeholder="粘贴 game.sii 的完整路径"
                      value={manualPath}
                      onChange={(e) => setManualPath(e.target.value)}
                    />
                    <button
                      disabled={!manualPath}
                      onClick={() =>
                        task("打开指定文件", () => open(manualPath))
                      }
                    >
                      打开
                    </button>
                  </div>
                </details>
              </div>
              {opened?.warnings.map((w) => (
                <div className="inline-warning" key={w}>
                  <CircleAlert size={15} />
                  {w}
                </div>
              ))}
              {!opened ? (
                <div className="empty welcome">
                  <div className="welcome-icon">
                    <TruckIcon size={52} />
                  </div>
                  <h2>从你的车库开始</h2>
                  <p>
                    打开一个存档，自动解密并读取所有自有卡车。
                    <br />
                    发动机、油箱、轮胎和外观附件，都在同一个工作台。
                  </p>
                  <div className="steps">
                    <span>
                      <b>01</b>打开存档
                    </span>
                    <ArrowRight size={17} />
                    <span>
                      <b>02</b>选择配件
                    </span>
                    <ArrowRight size={17} />
                    <span>
                      <b>03</b>预览并保存
                    </span>
                  </div>
                  {!count && (
                    <button onClick={() => setPage("settings")}>
                      建立本机配件目录 <ChevronRight size={16} />
                    </button>
                  )}
                </div>
              ) : (
                <>
                  <div className="garage-grid">
                    <section className="fleet">
                      <div className="section-head">
                        <h2>我的卡车</h2>
                        <span>{trucks.length} 辆</span>
                      </div>
                      {trucks.map((t) => (
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
                            {t.current && <span className="tag">正在驾驶</span>}
                          </div>
                          <strong>{friendly(t.model)}</strong>
                          <div className="plate">{t.plate || "未设置车牌"}</div>
                          <small>
                            {t.accessories.length} 个配件{" "}
                            <ChevronRight size={13} />
                          </small>
                        </button>
                      ))}
                    </section>
                    <section className="parts-panel">
                      <div className="section-head">
                        <div>
                          <h2>{friendly(truck?.model || "")}</h2>
                          <span>{truck?.plate} · 完整配件列表</span>
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
                          添加附件
                        </button>
                      </div>
                      <div className="filters">
                        <div className="search">
                          <Search size={15} />
                          <input
                            aria-label="搜索车辆配件"
                            placeholder="搜索名称、类别或路径"
                            value={partQuery}
                            onChange={(e) => setPartQuery(e.target.value)}
                          />
                        </div>
                        <select
                          aria-label="配件类别"
                          value={filter}
                          onChange={(e) => setFilter(e.target.value)}
                        >
                          <option value="all">所有类别</option>
                          {Array.from(
                            new Set(truck?.accessories.map((a) => a.category)),
                          ).map((c) => (
                            <option key={c} value={c}>
                              {labels[c] || c}
                            </option>
                          ))}
                        </select>
                      </div>
                      <div className="part-list">
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
                                {labels[a.category] || a.category}
                                <em>
                                  {a.definition ? "已识别" : "只读 / 未索引"}
                                </em>
                              </strong>
                              <span>{a.name}</span>
                              <small title={a.path}>{short(a.path)}</small>
                            </span>
                            <ChevronRight size={15} />
                          </button>
                        ))}
                        {!parts.length && (
                          <div className="empty">没有匹配的配件</div>
                        )}
                      </div>
                      <div className="panel-foot">
                        {parts.length} / {truck?.accessories.length} 个配件 ·
                        未识别字段仍会保留
                      </div>
                    </section>
                    <section className="inspector">
                      <div className="section-head">
                        <h2>
                          {action === "add" ? "追加外观附件" : "配件详情"}
                        </h2>
                        <span className="tag">
                          {action === "add" ? "ADD" : "REPLACE"}
                        </span>
                      </div>
                      {action === "replace" && !part ? (
                        <div className="empty inspector-empty">
                          <Wrench size={32} />
                          <p>
                            选择一个配件
                            <br />
                            查看参数与替换方案
                          </p>
                        </div>
                      ) : (
                        <>
                          {action === "add" ? (
                            <>
                              <p className="muted">
                                只接受同车型、同驾驶室和底盘的供体附件；安装类别不能已被占用。
                              </p>
                              <label>
                                附件类别
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
                                      {labels[c] || c}
                                      {[
                                        "engine",
                                        "transmission",
                                        "chassis",
                                        "tank",
                                      ].includes(c)
                                        ? " · 禁止追加"
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
                                  当前{" "}
                                  {labels[part!.category] || part!.category}
                                </span>
                                <h3>{part!.name}</h3>
                                <span>{friendly(part!.model)}</span>
                              </div>
                              <Metrics def={part!.definition} />
                              <details className="raw">
                                <summary>查看原始字段与定义路径</summary>
                                <pre>{part!.raw}</pre>
                                <p>
                                  refund 是存档退款字段；替换时保留原值{" "}
                                  {part!.refund}。
                                </p>
                              </details>
                            </>
                          )}
                          <div className="divider" />
                          <h3 className="subheading">
                            {action === "add" ? "选择供体附件" : "替换为"}
                          </h3>
                          <div className="segmented">
                            <button
                              className={source === "donor" ? "active" : ""}
                              onClick={() => {
                                setSource("donor");
                                setChosen("");
                              }}
                            >
                              从车库选择
                            </button>
                            <button
                              className={source === "catalog" ? "active" : ""}
                              disabled={action === "add"}
                              onClick={() => {
                                setSource("catalog");
                                setChosen("");
                              }}
                            >
                              本机配件库
                            </button>
                          </div>
                          <div className="search">
                            <Search size={15} />
                            <input
                              aria-label="搜索候选配件"
                              placeholder="搜索车型或配件"
                              value={candidateQuery}
                              onChange={(e) =>
                                setCandidateQuery(e.target.value)
                              }
                            />
                          </div>
                          <div className="candidates">
                            {visibleCandidates.slice(0, 150).map((c) => (
                              <button
                                key={c.key}
                                className={
                                  "candidate " +
                                  (chosen === c.key ? "selected" : "")
                                }
                                onClick={() => setChosen(c.key)}
                              >
                                <span>
                                  <strong>{c.name}</strong>
                                  <small>
                                    {c.model} · {c.detail}
                                  </small>
                                </span>
                                {chosen === c.key && <Check size={15} />}
                              </button>
                            ))}
                            {!visibleCandidates.length && (
                              <p className="muted">
                                没有候选配件。可切换配件库，或先建立目录。
                              </p>
                            )}
                          </div>
                          {selected && (
                            <div className="candidate-metrics">
                              <Metrics def={selected.definition} />
                              <small className="break">{selected.path}</small>
                            </div>
                          )}
                          <button
                            className="primary full"
                            disabled={
                              !selected || (!part && action === "replace")
                            }
                            onClick={() => task("校验改装规则", stage)}
                          >
                            <Plus size={16} />
                            加入变更清单
                          </button>
                          <p className="micro">
                            发动机、变速箱、底盘等核心部件禁止重复追加。所有操作由后台再次校验。
                          </p>
                        </>
                      )}
                    </section>
                  </div>
                  {preview && operations.length > 0 && (
                    <section className="changes">
                      <div className="section-head">
                        <h2>
                          待保存的修改{" "}
                          <span className="tag">{operations.length}</span>
                        </h2>
                        <span>尚未写入游戏存档</span>
                      </div>
                      {preview.changes.map((c, n) => (
                        <div className="change" key={n}>
                          <span>
                            {friendly(c.model)}
                            <small>{labels[c.category] || c.category}</small>
                          </span>
                          <code>{c.before ? short(c.before) : "新增"}</code>
                          <ArrowRight size={16} />
                          <code>{short(c.after)}</code>
                          <button
                            className="icon-button"
                            aria-label="移除此修改"
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
                          {w}
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
                  <h1>连接你的游戏。</h1>
                  <p>
                    文件保留在这台电脑上。存档解码器已内置，解包工具由程序自动准备。
                  </p>
                </div>
              </div>
              <div className="settings-grid">
                <section className="card">
                  <h2>文件位置</h2>
                  <button onClick={() => setShowSetup(true)}>
                    <RefreshCw size={16} />
                    重新运行配置向导
                  </button>
                  {(
                    [
                      ["documents", "ETS2 用户数据目录"],
                      ["game", "游戏安装目录"],
                    ] as const
                  ).map(([key, title]) => (
                    <label key={key}>
                      {title}
                      <input
                        value={settings[key]}
                        onChange={(e) =>
                          setSettings({ ...settings, [key]: e.target.value })
                        }
                      />
                    </label>
                  ))}
                  <button
                    className="primary"
                    onClick={() =>
                      task("保存设置", async () => {
                        await rpc("settings", { settings });
                        setCount(
                          (await rpc<{ catalog_count: number }>("init"))
                            .catalog_count,
                        );
                        setDefinitions(await rpc<Definition[]>("catalog"));
                        await refresh();
                        setNotice({ text: "设置已保存", error: false });
                      })
                    }
                  >
                    保存设置
                  </button>
                </section>
                <section className="card">
                  <Database size={30} />
                  <h2>建立本机配件目录</h2>
                  <p>
                    读取 def.scs 及官方车型、轮胎与改装
                    DLC，获得真实定义路径、配件类型和性能参数。首次解包需要一些时间。
                  </p>
                  <div className="stat">
                    {count.toLocaleString()}
                    <span>已索引定义</span>
                  </div>
                  <button
                    className="primary"
                    onClick={() =>
                      task(
                        "正在解包并索引游戏定义，首次运行可能需要数分钟",
                        async () => {
                          await rpc("settings", { settings });
                          const r = await rpc<{
                            count: number;
                            warnings: string[];
                          }>("index");
                          setCount(r.count);
                          setDefinitions(await rpc<Definition[]>("catalog"));
                          if (opened) await open(opened.path);
                          setNotice({
                            text: `已索引 ${r.count} 个配件定义。目录仅涵盖已识别的官方资源。`,
                            error: false,
                          });
                        },
                      )
                    }
                  >
                    <RefreshCw size={16} />
                    建立 / 更新目录
                  </button>
                  <p className="micro">
                    当前不支持 Mod；检测到 Mod 或未知扩展依赖的存档将拒绝打开。
                  </p>
                  <details>
                    <summary>本地缓存与备份目录</summary>
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
                  <h1>从定义认识性能。</h1>
                  <p>
                    参数来自本机游戏文件。显示马力与实际动力参数分开，最高速度不作精确预测。
                  </p>
                </div>
                <button onClick={() => setPage("settings")}>
                  <Database size={16} />
                  更新目录
                </button>
              </div>
              <div className="search catalog-search">
                <Search size={18} />
                <input
                  aria-label="搜索配件目录"
                  placeholder="搜索品牌、型号、engine、tank…"
                  value={catalogQuery}
                  onChange={(e) => setCatalogQuery(e.target.value)}
                />
              </div>
              <div className="catalog-grid">
                {definitions
                  .filter((d) =>
                    `${d.name} ${d.path} ${labels[d.category] || ""}`
                      .toLowerCase()
                      .includes(catalogQuery.toLowerCase()),
                  )
                  .slice(0, 120)
                  .map((d) => (
                    <article className="card definition" key={d.path}>
                      <span className="tag">
                        {labels[d.category] || d.category}
                      </span>
                      <h3>{d.name}</h3>
                      <p>{friendly(d.model)}</p>
                      <Metrics def={d} />
                      <details>
                        <summary>路径与适配条件</summary>
                        <code className="break">{d.path}</code>
                        <p>{d.kind}</p>
                        <p>
                          {d.suitable.join(", ") || "定义未指定 suitable_for"}
                        </p>
                      </details>
                    </article>
                  ))}
              </div>
              {!count && (
                <div className="empty">请先在设置中建立配件目录。</div>
              )}
              <p className="micro">
                为保持列表流畅，每次显示前 120 项。输入关键词可缩小范围。
              </p>
            </>
          )}
          {page === "history" && (
            <>
              <div className="page-title">
                <div>
                  <div className="eyebrow">CHANGE HISTORY</div>
                  <h1>每次改装，都有据可查。</h1>
                  <p>
                    读取游戏另存的结果，检查配件是否保留。车辆身份变化时会提示手动核对。
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
                  刷新
                </button>
              </div>
              <section className="card">
                <label>
                  用于复查的游戏存档
                  <select
                    value={verifyPath}
                    onChange={(e) => setVerifyPath(e.target.value)}
                  >
                    <option value="">选择游戏另存的结果</option>
                    {saves.map((s) => (
                      <option key={s.path} value={s.path}>
                        {s.name} · {s.profile}
                      </option>
                    ))}
                  </select>
                </label>
                <button onClick={() => task("刷新游戏存档", refresh)}>
                  刷新存档列表
                </button>
              </section>
              <div className="history-list">
                {receipts.map((r) => (
                  <article className="card" key={r.id}>
                    <div className="section-head">
                      <h3>{r.changes.length} 项改装</h3>
                      <span>
                        {new Date(
                          Number(r.id.split("-")[0]) * 1000,
                        ).toLocaleString()}
                      </span>
                    </div>
                    <code className="break">{r.output}</code>
                    {r.changes.map((c, i) => (
                      <p key={i}>
                        {friendly(c.model)} · {labels[c.category] || c.category}{" "}
                        → {short(c.after)}
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
                        复查所选存档
                      </button>
                      <button onClick={() => setRestoreId(r.id)}>
                        <Undo2 size={16} />
                        恢复修改前
                      </button>
                    </div>
                    <details>
                      <summary>备份位置</summary>
                      <code className="break">{r.backup}</code>
                    </details>
                  </article>
                ))}
              </div>
              {!receipts.length && (
                <div className="empty">
                  保存第一次改装后，记录将显示在这里。
                </div>
              )}
              {verification.length > 0 && (
                <section className="card">
                  <h2>复查结果</h2>
                  {verification.map((v, i) => (
                    <p key={i}>
                      <strong>
                        {labels[v.category] || v.category}：{v.status}
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
          <span>独立油箱与发动机跨品牌替换已实测 · 外观组合仍需游戏内验证</span>
        </footer>
      </main>
      {showSetup && (
        <Onboarding
          initial={settings}
          onClose={firstSetup ? undefined : () => setShowSetup(false)}
          onComplete={async (s, n) => {
            setOpened(null);
            setOperations([]);
            setPreview(null);
            setSettings(s);
            setCount(n);
            setShowSetup(false);
            setFirstSetup(false);
            // Setup is already committed. Keep post-setup read errors visible in the app.
            await task("正在读取车库…", async () => {
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
          {busy}
        </div>
      )}
      {saveModal && (
        <div className="modal-backdrop">
          <section className="modal">
            <div className="section-head">
              <h2>保存这次改装</h2>
              <button
                aria-label="关闭"
                disabled={!!busy}
                onClick={() => setSaveModal(false)}
              >
                <X size={18} />
              </button>
            </div>
            <p>
              {operations.length}{" "}
              项变更已通过结构和操作规则检查。保存前自动备份。
            </p>
            <label>
              保存方式
              <select
                disabled={!!busy}
                value={saveMode}
                onChange={(e) => setSaveMode(e.target.value)}
              >
                <option value="new">另存为新存档（推荐）</option>
                <option value="overwrite">备份并覆盖当前存档</option>
              </select>
            </label>
            {saveMode === "new" && (
              <label>
                新存档名称
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
              保存后需要在游戏中手动加载。改装升级可能恢复原厂配件。
            </div>
            <button
              className="primary full"
              disabled={!!busy}
              onClick={() => task("备份、校验并写入存档", save)}
            >
              <Save size={17} />
              确认保存
            </button>
          </section>
        </div>
      )}
      {restoreId && (
        <div className="modal-backdrop">
          <section className="modal">
            <h2>恢复修改前的存档？</h2>
            <p>
              仅当目标仍是本工具写入的版本时才会恢复。游戏产生的新进度不会被直接覆盖。
            </p>
            <div className="button-row">
              <button disabled={!!busy} onClick={() => setRestoreId("")}>
                取消
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
                    await refresh();
                    if (opened?.path === path) await open(path);
                    setNotice({ text: "已恢复修改前的游戏数据", error: false });
                  })
                }
              >
                恢复
              </button>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}
function Metrics({ def }: { def: Definition | null | undefined }) {
  if (!def)
    return <p className="micro">该定义尚未索引。原始配件数据会完整保留。</p>;
  const values = Object.entries(def.metrics).filter(([k]) => k !== "price");
  return (
    <dl className="metrics">
      {values.slice(0, 8).map(([k, v]) => (
        <div key={k}>
          <dt>{metrics[k] || k}</dt>
          <dd title={v}>{v}</dd>
        </div>
      ))}
    </dl>
  );
}
