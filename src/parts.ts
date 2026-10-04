import type { Accessory, Definition } from "./types";

// Display metadata only. These groups never change the backend's part categories.
export const groups = [
  ["cabin", "驾驶室"],
  ["chassis", "底盘"],
  ["engine", "发动机"],
  ["transmission", "变速箱"],
  ["interior", "内饰"],
  ["paint_job", "涂装"],
  ["exterior", "外观配件"],
  ["interior_accessories", "内饰配件"],
  ["other", "其他 / 未识别"],
] as const;

export const labels: Record<string, string> = {
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
  f_disc: "前轮轮盘",
  r_disc: "后轮轮盘",
  f_hub: "前轮轮毂",
  r_hub: "后轮轮毂",
  f_cover: "前轮轮毂盖",
  r_cover: "后轮轮毂盖",
  f_nuts: "前轮螺母",
  r_nuts: "后轮螺母",
  beacon: "警示灯",
  r_grill: "车顶灯架",
  f_grill: "前部防护杆",
  b_grill: "保险杠灯架",
  sunshld: "遮阳板",
  sunshield: "遮阳板",
  head_light: "前大灯",
  badge: "车型铭牌",
  f_bumper: "前保险杠",
  r_bumper: "后保险杠",
  sideskirt: "侧裙",
  sideskrt_bar: "侧裙护杆",
  mirror: "后视镜",
  f_mirror: "前部补盲镜",
  s_mirror: "侧面补盲镜",
  f_mudflap: "前挡泥板",
  r_mudflap: "后挡泥板",
  r_fender: "后轮翼子板",
  exhaust_l: "左侧排气管",
  exhaust_r: "右侧排气管",
  exhaust_m: "排气管",
  toyhang: "悬挂饰品",
  toystand: "仪表台饰品",
  toyseat: "座椅饰品",
  toybed: "卧铺饰品",
  toybig: "车内饰品",
  toypanel: "车内面板饰品",
  toyac: "车内饰品",
  steering_w: "方向盘",
  curtain_f: "前窗帘",
  l_pillow: "靠枕",
  carpet: "地毯",
  drv_plate: "驾驶员名牌",
  codrv_plate: "副驾驶名牌",
  codrv_seat: "副驾驶座椅",
  set_dashbrd: "仪表台",
  set_glass: "车内玻璃饰件",
  set_lglass: "车内侧窗饰件",
  set_cuphold: "杯架",
  cup_holder: "杯架",
  int_display: "车内显示屏",
  intlight_bck: "车内照明",
  intlight_bgr: "车内照明",
  unknown: "未识别配件",
};
const interiorCategories = new Set([
  "toyhang",
  "toystand",
  "toyseat",
  "toybed",
  "toybig",
  "toypanel",
  "toyac",
  "steering_w",
  "curtain_f",
  "l_pillow",
  "carpet",
  "drv_plate",
  "codrv_plate",
  "codrv_seat",
  "set_dashbrd",
  "set_glass",
  "set_lglass",
  "set_cuphold",
  "cup_holder",
  "int_display",
  "intlight_bck",
  "intlight_bgr",
]);
export function categoryLabel(category: string) {
  return labels[category] || "未识别配件";
}
export function groupOf(
  part: Pick<Accessory | Definition, "category" | "path">,
) {
  if (
    [
      "cabin",
      "chassis",
      "engine",
      "transmission",
      "interior",
      "paint_job",
    ].includes(part.category)
  )
    return part.category;
  if (interiorCategories.has(part.category)) return "interior_accessories";
  if (part.category === "vehicle" || part.category === "unknown")
    return "other";
  if (
    [
      "tank",
      "beacon",
      "r_grill",
      "f_grill",
      "b_grill",
      "sunshld",
      "sunshield",
      "head_light",
      "badge",
      "f_bumper",
      "r_bumper",
      "sideskirt",
      "sideskrt_bar",
      "mirror",
      "f_mirror",
      "s_mirror",
      "f_mudflap",
      "r_mudflap",
      "r_fender",
      "exhaust_l",
      "exhaust_r",
      "exhaust_m",
    ].includes(part.category) ||
    /^(f|r)_(tire|disc|hub|nuts|cover|rim|wheel)$/.test(part.category)
  )
    return "exterior";
  return "other";
}

const models: Record<string, string> = {
  "daf.2021": "DAF XG / XG+ / XF",
  "daf.xd": "DAF XD",
  "daf.xf": "DAF XF105",
  "daf.xf_euro6": "DAF XF Euro 6",
  "daf.xf_electric": "DAF XF Electric",
  "iveco.hiway": "Iveco Stralis Hi-Way",
  "iveco.stralis": "Iveco Stralis",
  "iveco.sway": "Iveco S-Way",
  "man.tgx": "MAN TGX",
  "man.tgx_euro6": "MAN TGX Euro 6",
  "man.tgx_2020": "MAN TGX 2020",
  "mercedes.actros": "Mercedes-Benz Actros",
  "mercedes.actros2014": "Mercedes-Benz New Actros",
  "mercedes.actros_2014": "Mercedes-Benz New Actros",
  "renault.magnum": "Renault Magnum",
  "renault.premium": "Renault Premium",
  "renault.t": "Renault T",
  "renault.etech_t": "Renault E-Tech T",
  "scania.r": "Scania R",
  "scania.streamline": "Scania Streamline",
  "scania.r_2016": "Scania R 2016",
  "scania.s_2016": "Scania S",
  "scania.s_2024e": "Scania S BEV",
  "volvo.fh16": "Volvo FH Classic",
  "volvo.fh16_2012": "Volvo FH 2012",
  "volvo.fh_2021": "Volvo FH 2021",
  "volvo.fh_2024": "Volvo FH6",
};
const brands: Record<string, string> = {
  daf: "DAF",
  iveco: "Iveco",
  man: "MAN",
  mercedes: "Mercedes-Benz",
  renault: "Renault",
  scania: "Scania",
  volvo: "Volvo",
};
export function friendly(model: string) {
  return models[model] || model.replaceAll("_", " ").replaceAll(".", " ");
}
export function truckDefinition(def: Definition) {
  // Shared wheels/accessories remain available; trailer and AI definitions do not.
  return /^\/def\/vehicle\/(truck|[fr]_(?:tire|disc|hub|nuts|cover|wheel|rim)|addon_hookup)\//.test(
    def.path,
  );
}

export type PartDisplay = {
  title: string;
  origin: string;
  summary: string;
  search: string;
};
const clean = (s: string) => s.replace(/@@[^@]+@@/g, "").trim();
const stemOf = (path: string) =>
  path
    .split("/")
    .pop()
    ?.replace(/\.sii$/i, "") || "";
// Exact-path fallback verified against the 1.61 name tokens. Only apply when the
// installed definition still declares the matching kW value; no numeric ID guessing.
const mercedesNames: Record<string, [string, number]> = {
  engine_1842: ["OM 471 Euro VI", 310],
  engine_1845: ["OM 471 Euro VI", 330],
  engine_1848: ["OM 471 Euro VI", 350],
  engine_1851: ["OM 471 Euro VI", 375],
  engine_1852: ["OM 473 Euro VI", 380],
  engine_1858: ["OM 473 Euro VI", 425],
  engine_1863: ["OM 473 Euro VI", 460],
};
function engineName(stem: string, def?: Definition | null) {
  const verified = mercedesNames[stem];
  if (
    verified &&
    def?.path === `/def/vehicle/truck/mercedes.actros2014/engine/${stem}.sii` &&
    Number(def.metrics.info?.match(/(\d+)\s*kW/i)?.[1]) === verified[1]
  )
    return `${verified[0]} (${verified[1]} kW)`;
  if (/^engine_|^\d+$/.test(stem)) return `发动机 · 变体 ${stem}`;
  // The trailing number in MX filenames is kW, not hp. Power comes only from info.
  const mx = stem.match(/^mx_?(11|13)(?:_|\b)/i);
  if (mx)
    return `PACCAR MX-${mx[1]}${stem.slice(mx[0].length) ? " " + stem.slice(mx[0].length).replaceAll("_", " ") : ""}`;
  if (/^mx\d/i.test(stem)) return `PACCAR ${stem.toUpperCase()}`;
  return stem
    .replace(/_/g, " ")
    .replace(/^(cursor)(\d+)/i, "Cursor $2")
    .replace(/^(dti|dc|d|om)([a-z0-9]+)/i, (code: string) =>
      code.toUpperCase(),
    );
}
export function power(def: Definition) {
  return (
    def.metrics.info
      ?.replaceAll("@@hp@@", "hp")
      .match(/\b(\d+(?:[.,]\d+)?)\s*hp\b/i)?.[1] || null
  );
}
export function primaryMetrics(def: Definition): [string, string][] {
  const m = def.metrics;
  const value = (key: string, unit = "") =>
    m[key] && !m[key].includes("@@") ? `${clean(m[key])}${unit}` : "未知";
  switch (def.category) {
    case "engine":
      return [
        ["游戏标称功率", power(def) ? `${power(def)} hp` : "未知"],
        ["峰值扭矩", value("torque", " Nm")],
        ...(m.secondary_torque
          ? [["附加扭矩", value("secondary_torque", " Nm")] as [string, string]]
          : []),
        [
          "标称扭矩转速范围",
          m.info?.match(/([\d,]+(?:\s*[-–]\s*[\d,]+)?)\s*(?:rpm|@@rpm@@)/i)?.[1]
            ? `${m.info.match(/([\d,]+(?:\s*[-–]\s*[\d,]+)?)\s*(?:rpm|@@rpm@@)/i)![1]} rpm`
            : "未知",
        ],
        ["排量", value("volume", " L")],
        ["转速上限", value("rpm_limit", " rpm")],
      ];
    case "transmission":
      return [
        [
          "前进挡",
          m.ratios_forward
            ? `${(m.ratios_forward.match(/-?\d+(?:\.\d+)?/g) || []).length} 挡`
            : "未知",
        ],
        ["主减速比", value("differential_ratio")],
        [
          "缓速器",
          m.retarder == null
            ? "未标明"
            : Number(m.retarder) > 0
              ? `${m.retarder} 级`
              : "无",
        ],
        ["前进挡齿比", value("ratios_forward")],
        ["倒挡齿比", value("ratios_reverse")],
      ];
    case "tank":
      return [["独立油箱容量", value("fuel_tank_size", " L")]];
    case "chassis":
      return [["底盘油箱容量", value("tank_size", " L")]];
    default:
      return Object.entries(m)
        .filter(([key, v]) => key !== "price" && !v.includes("@@"))
        .map(([key, v]) => [metricLabels[key] || key, clean(v)]);
  }
}
export const metricLabels: Record<string, string> = {
  type: "动力类型",
  info: "游戏显示信息",
  torque_curve: "扭矩曲线",
  consumption_coef: "油耗系数",
  roll_resistance: "滚动阻力",
  wet_grip: "湿地抓地力",
  noise_volume: "滚动噪声",
  radius: "半径",
};
export function describePart(
  def: Definition | null | undefined,
  fallback?: Pick<Accessory, "path" | "category" | "name" | "model">,
): PartDisplay {
  const p = def || fallback;
  if (!p)
    return {
      title: "未识别配件",
      origin: "来源未知",
      summary: "参数未知",
      search: "",
    };
  const stem = stemOf(p.path);
  const brand = brands[p.model.split(".")[0]] || "";
  const origin = brand
    ? `车型：${friendly(p.model)}`
    : p.model && p.model !== "通用"
      ? `车型：${friendly(p.model)}`
      : "车型未标明";
  const rawName = clean(p.name);
  const isFilename = rawName.replaceAll(" ", "_") === stem;
  let name = rawName;
  if (p.category === "vehicle") name = `${friendly(p.model)} · 车辆基础`;
  else if (p.category === "engine" && (isFilename || !name))
    name = engineName(stem, def);
  else if (p.category === "transmission" && (isFilename || !name))
    name = stem.replaceAll("_", " ").toUpperCase();
  else if (p.category === "tank") name = "独立油箱";
  else if (
    p.category === "chassis" &&
    isFilename &&
    /^\d+x\d+(?:_|$)/i.test(stem)
  )
    name = `${stem.replaceAll("_", " ")} 底盘`;
  else if (!name || isFilename || name === "data")
    name = categoryLabel(p.category);
  const title = `${brand && !name.toLowerCase().startsWith(brand.toLowerCase()) ? brand + " · " : ""}${name}`;
  let summary = "参数未知";
  if (def) {
    const rows = primaryMetrics(def);
    if (p.category === "engine")
      summary = `${power(def) ? power(def) + " hp" : "功率未知"} · ${def.metrics.torque ? def.metrics.torque + " Nm" : "扭矩未知"}`;
    else if (p.category === "transmission")
      summary = rows
        .slice(0, 3)
        .map(([k, v]) => `${k} ${v}`)
        .join(" · ");
    else if (p.category === "tank" || p.category === "chassis")
      summary = rows.map(([k, v]) => `${k} ${v}`).join(" · ");
    else
      summary =
        rawName && !isFilename ? categoryLabel(p.category) : "游戏名称未解析";
  }
  // Keep the exact variant searchable and inspectable, never invent a product name.
  if (name === categoryLabel(p.category) && p.category !== "tank")
    summary += ` · 变体 ${stem}`;
  return {
    title,
    origin,
    summary,
    search:
      `${title} ${origin} ${summary} ${p.name} ${p.path} ${categoryLabel(p.category)} ${def?.metrics.info || ""}`.toLowerCase(),
  };
}
