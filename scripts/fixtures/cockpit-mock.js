// Browser-only fake Tauri backend for previewing the UI with `npm run dev`.
// Loaded ONLY by design/cockpit/tools/mock-preview.html. Never import it from src/.
// Data is fictional and mirrors the reference screens (Scania S 730 etc.).
(() => {
  const D = (path, category, zh, en, metrics, kind) => ({
    path,
    kind: kind || "accessory_engine_data",
    unit: "",
    name: en,
    raw_name: en,
    names: { zh_cn: zh, en },
    category_names: {},
    name_alias: null,
    category,
    model: path.split("/")[4] || "",
    source: "def.scs",
    metrics,
    suitable: [],
    conflicts: [],
    requires: [],
  });
  const eng = (m, stem, name, hp, nm, vol, rpm) =>
    D(`/def/vehicle/truck/${m}/engine/${stem}.sii`, "engine", name, name, {
      info: `${hp} hp · ${nm.toLocaleString("en-US")} Nm · 1,000-1,300 rpm`,
      torque: String(nm),
      volume: String(vol),
      rpm_limit: String(rpm),
      type: "diesel",
    });
  const defs = [
    eng("scania.s_2016", "dc16_730", "Scania DC16 730", 730, 3500, 16.4, 2300),
    eng("scania.s_2016", "dc16_770", "Scania DC16 770", 770, 3700, 16.4, 2300),
    eng("scania.s_2016", "dc13_500", "Scania DC13 500", 500, 2550, 12.7, 2300),
    eng("volvo.fh16_2012", "d17k780", "Volvo D17K780", 780, 3800, 16.1, 2200),
    eng("volvo.fh_2021", "d13k540", "Volvo D13K540", 540, 2600, 12.8, 2200),
    eng("daf.2021", "mx13_390", "PACCAR MX-13 530", 530, 2600, 12.9, 2200),
    eng("man.tgx_2020", "d3876_640", "MAN D3876 640", 640, 3000, 15.2, 2200),
    eng("renault.t", "dti13_520", "Renault DTI 13 520", 520, 2550, 12.8, 2200),
    eng(
      "iveco.sway",
      "cursor13_570",
      "Iveco Cursor 13 570",
      570,
      2500,
      12.9,
      2200,
    ),
    D(
      "/def/vehicle/truck/scania.s_2016/transmission/grso926r.sii",
      "transmission",
      "Scania GRSO926R",
      "Scania GRSO926R",
      {
        ratios_forward:
          "11.32, 9.16, 7.19, 5.82, 4.57, 3.70, 2.48, 2.01, 1.58, 1.28, 1.0, 0.81",
        differential_ratio: "2.31",
        retarder: "0",
      },
      "accessory_transmission_data",
    ),
    D(
      "/def/vehicle/truck/scania.s_2016/accessory/fueltank/tank_900.sii",
      "tank",
      "Scania S 独立油箱",
      "Scania S standalone tank",
      { fuel_tank_size: "900" },
      "accessory_addon_data",
    ),
    D(
      "/def/vehicle/truck/daf.2021/accessory/fueltank/tank_1465.sii",
      "tank",
      "DAF XG 独立油箱",
      "DAF XG standalone tank",
      { fuel_tank_size: "1465" },
      "accessory_addon_data",
    ),
    D(
      "/def/vehicle/f_tire/kmax_s.sii",
      "f_tire",
      "Goodyear KMAX S 315/70 R22.5",
      "Goodyear KMAX S 315/70 R22.5",
      { wet_grip: "0.92", roll_resistance: "0.0062" },
      "accessory_wheel_data",
    ),
    D(
      "/def/vehicle/r_tire/kmax_d.sii",
      "r_tire",
      "Goodyear KMAX D 315/70 R22.5",
      "Goodyear KMAX D 315/70 R22.5",
      { wet_grip: "0.94", roll_resistance: "0.0065" },
      "accessory_wheel_data",
    ),
    D(
      "/def/vehicle/truck/scania.s_2016/cabin/highline.sii",
      "cabin",
      "Highline 驾驶室",
      "Highline cabin",
      {},
      "accessory_cabin_data",
    ),
    D(
      "/def/vehicle/truck/scania.s_2016/chassis/4x2.sii",
      "chassis",
      "Scania S 4x2 底盘",
      "Scania S 4x2 chassis",
      { tank_size: "400" },
      "accessory_chassis_data",
    ),
    D(
      "/def/vehicle/truck/scania.s_2016/accessory/beacon/led_bar.sii",
      "beacon",
      "橙色 LED 灯条",
      "Orange LED light bar",
      {},
      "accessory_addon_data",
    ),
    D(
      "/def/vehicle/truck/scania.s_2016/accessory/sunshld/led.sii",
      "sunshld",
      "LED 灯遮阳板",
      "LED sun visor",
      {},
      "accessory_addon_data",
    ),
  ];
  const byPath = new Map(defs.map((d) => [d.path, d]));
  const acc = (truck, i, path, category) => ({
    id: `${truck}.a${i}`,
    index: i,
    kind: byPath.get(path)?.kind || "vehicle_accessory",
    path,
    category,
    name: path.split("/").pop().replace(".sii", ""),
    model: path.split("/")[4] || "",
    refund: "0",
    raw: `vehicle_accessory : ${truck}.a${i} {\n data_path: "${path}"\n refund: 0\n}`,
    definition: byPath.get(path) || null,
  });
  const parts = (id) => [
    acc(id, 0, "/def/vehicle/truck/scania.s_2016/chassis/4x2.sii", "chassis"),
    acc(id, 1, "/def/vehicle/truck/scania.s_2016/cabin/highline.sii", "cabin"),
    acc(
      id,
      2,
      "/def/vehicle/truck/scania.s_2016/engine/dc16_730.sii",
      "engine",
    ),
    acc(
      id,
      3,
      "/def/vehicle/truck/scania.s_2016/transmission/grso926r.sii",
      "transmission",
    ),
    acc(
      id,
      4,
      "/def/vehicle/truck/scania.s_2016/accessory/fueltank/tank_900.sii",
      "tank",
    ),
    acc(id, 5, "/def/vehicle/f_tire/kmax_s.sii", "f_tire"),
    acc(id, 6, "/def/vehicle/r_tire/kmax_d.sii", "r_tire"),
    acc(
      id,
      7,
      "/def/vehicle/truck/scania.s_2016/accessory/beacon/led_bar.sii",
      "beacon",
    ),
    acc(
      id,
      8,
      "/def/vehicle/truck/scania.s_2016/accessory/sunshld/led.sii",
      "sunshld",
    ),
    acc(
      id,
      9,
      "/def/vehicle/truck/scania.s_2016/accessory/lightbar/c_lightbar_x.sii",
      "unknown",
    ),
  ];
  const T = (id, model, plate, driver, current) => ({
    id,
    plate,
    model,
    current: !!current,
    location: "berlin",
    driver,
    accessories: parts(id),
  });
  const trucks = [
    T(
      "t1",
      "scania.s_2016",
      "B SK 730",
      { kind: "player", id: null, name: null },
      true,
    ),
    T("t2", "volvo.fh_2021", "M VF 1612", {
      kind: "employee",
      id: "e1",
      name: "Lukas Weber",
    }),
    T("t3", "daf.2021", "WA 5821K", {
      kind: "employee",
      id: "e2",
      name: "Marta Nowak",
    }),
    T("t4", "renault.t", "GH-417-TR", {
      kind: "unassigned",
      id: null,
      name: null,
    }),
    T("t5", "man.tgx_2020", "W 7712 TX", {
      kind: "employee",
      id: "e3",
      name: "Jan Novák",
    }),
    T("t6", "mercedes.actros2014", "HH AC 2014", {
      kind: "employee",
      id: "e4",
      name: "Sofie Lind",
    }),
    T("t7", "iveco.sway", "MI 5W4Y", {
      kind: "employee",
      id: "e5",
      name: "Paolo Greco",
    }),
    T("t8", "scania.r_2016", "SVR 2016", {
      kind: "unassigned",
      id: null,
      name: null,
    }),
  ];
  const DOCS = "C:/Synthetic/ETS2";
  const SAVE = `${DOCS}/profiles/fixture/save/7/game.sii`;
  const preview = (ops) => {
    const next = JSON.parse(JSON.stringify(trucks));
    const changes = [];
    for (const op of ops) {
      const t = next.find((x) => x.id === op.truck_id);
      const a = t.accessories.find((x) => x.id === op.accessory_id);
      changes.push({
        truck_id: t.id,
        model: t.model,
        plate: t.plate,
        category: a.category,
        action: op.action,
        before: a.path,
        after: op.candidate_path,
      });
      a.path = op.candidate_path;
      a.definition = byPath.get(op.candidate_path) || null;
    }
    return { changes, warnings: [], trucks: next };
  };
  const receipts = [
    {
      id: "1791101560-a",
      source: SAVE,
      output: `${DOCS}/profiles/fixture/save/Workshop_2026-10-04_00-12-40-118/game.sii`,
      backup: "C:/Synthetic/Workshop/backups/1791101560-a",
      state: "completed",
      warning: null,
      info_hash: null,
      before_hash: "",
      after_hash: "",
      changes: [
        {
          truck_id: "t1",
          model: "scania.s_2016",
          plate: "B SK 730",
          category: "tank",
          action: "replace",
          before: "",
          after: "/def/vehicle/truck/daf.2021/accessory/fueltank/tank_1465.sii",
        },
        {
          truck_id: "t1",
          model: "scania.s_2016",
          plate: "B SK 730",
          category: "engine",
          action: "replace",
          before: "",
          after: "/def/vehicle/truck/volvo.fh16_2012/engine/d17k780.sii",
        },
      ],
    },
  ];
  async function rpc(p) {
    switch (p.action) {
      case "init":
        return {
          settings: {
            documents: DOCS.replaceAll("/", "\\"),
            game: "D:\\SteamLibrary\\steamapps\\common\\Euro Truck Simulator 2",
            extractor: "",
            onboarding_version: 1,
          },
          catalog_count: 4216,
          catalog_name_schema: 1,
          catalog_warnings: [],
          catalog_rebuild_reason: null,
          data_dir: "C:\\Synthetic\\Workshop",
          needs_setup: false,
        };
      case "catalog":
        return defs;
      case "history":
        return receipts;
      case "discover":
        return {
          saves: [
            {
              path: SAVE,
              name: "1.61 手动存档",
              profile: "Synthetic Driver",
              modified: 1791089294,
              error: null,
            },
          ],
          warnings: [],
        };
      case "detect":
        return {
          games: [
            "D:\\SteamLibrary\\steamapps\\common\\Euro Truck Simulator 2",
            "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Euro Truck Simulator 2",
          ],
          documents: [DOCS.replaceAll("/", "\\")],
          steam_roots: [],
          notes: [],
        };
      case "open":
        return { path: SAVE, hash: "mock", trucks, warnings: [] };
      case "preview":
        return preview(p.operations);
      case "settings":
        return { settings: p.settings, catalog_changed: false };
      default:
        return null;
    }
  }
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
  window.__TAURI_INTERNALS__ = {
    metadata: {
      currentWindow: { label: "main" },
      currentWebview: { windowLabel: "main", label: "main" },
    },
    transformCallback: (cb) => {
      const id = Math.floor(Math.random() * 1e9);
      window["_" + id] = cb;
      return id;
    },
    unregisterCallback: () => {},
    invoke: async (cmd, args) => (cmd === "rpc" ? rpc(args.payload) : null),
  };
})();
