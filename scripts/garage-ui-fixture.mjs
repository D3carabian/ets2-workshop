// Fully synthetic saves/catalog for UI checks. No installed game or player data needed.
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import {
  createSyntheticGame,
  bindSyntheticCatalog,
} from "./synthetic-catalog.mjs";
const root = resolve("verification/garage-ui");
const game = resolve(root, "game");
const app = resolve(root, "app");
const slot = resolve(game, "profiles/fixture/save/1");
await mkdir(slot, { recursive: true });
await mkdir(app, { recursive: true });
const installation = await createSyntheticGame(resolve(root, "installation"));
const entries = [
  [
    "engine",
    "mx13_315",
    "mx13 315",
    {
      info: "428 hp (315kW) · 2,150 Nm · 900-1,400 rpm",
      torque: "2150",
      volume: "12.9",
      rpm_limit: "2200",
    },
  ],
  [
    "engine",
    "mx13_355",
    "mx13 355",
    { info: "483 hp (355kW)", torque: "2350" },
  ],
  [
    "engine",
    "mx13_390",
    "mx13 390",
    { info: "530 hp (390kW)", torque: "2600" },
  ],
  [
    "transmission",
    "zf_12as3141to",
    "zf 12as3141to",
    {
      ratios_forward:
        "12.29 · 9.59 · 7.41 · 5.78 · 4.57 · 3.59 · 2.69 · 2.10 · 1.62 · 1.27 · 1.0 · 0.78",
      differential_ratio: "3.08",
      retarder: "3",
    },
  ],
  ["cabin", "test_cabin", "Fixture Cabin", {}],
  ["chassis", "4x2", "4x2", { tank_size: "600" }],
  ["interior", "standard", "Standard", {}],
  ["paint_job", "test_paint", "Fixture Paint", {}],
  ["vehicle", "data", "data", {}],
  ["toyhang", "fixture", "Fixture Accessory", {}],
  ["beacon", "fixture", "Fixture Beacon", {}],
];
const definitions = {};
for (const [category, file, name, metrics] of entries) {
  const path =
    category === "vehicle"
      ? "/def/vehicle/truck/daf.2021/data.sii"
      : `/def/vehicle/truck/daf.2021/${["toyhang", "beacon"].includes(category) ? "accessory/" : ""}${category}/${file}.sii`;
  definitions[path] = {
    path,
    kind: `accessory_${category}_data`,
    unit: `${file}.${category}`,
    name,
    category,
    model: "daf.2021",
    source: "synthetic-ui-fixture",
    metrics,
    suitable: [],
    conflicts: [],
    requires: [],
  };
}
const trailer = "/def/vehicle/trailer_owned/scs.flatbed/chassis/ch_3.sii";
definitions[trailer] = {
  ...Object.values(definitions).find((d) => d.category === "chassis"),
  path: trailer,
  model: "通用",
  name: "ch 3",
};
await writeFile(
  resolve(app, "catalog.json"),
  JSON.stringify({
    definitions,
    signature: "synthetic",
    warnings: [],
    archives: [],
  }),
);
await bindSyntheticCatalog(resolve(app, "catalog.json"), installation);
await writeFile(
  resolve(app, "settings.json"),
  JSON.stringify({
    game: installation,
    documents: game,
    extractor: "",
    onboarding_version: 1,
  }),
);
const parts = Object.values(definitions).filter(
  (d) => d.path !== trailer && !/mx13_(355|390)/.test(d.path),
);
const count = 80;
let text = `SiiNunit {\nplayer : player {\n trucks: ${count}\n assigned_truck: truck0\n`;
for (let i = 0; i < count; i++) text += ` trucks[${i}]: truck${i}\n`;
text += "}\n";
for (let i = 0; i < count; i++) {
  text += `vehicle : truck${i} {\n license_plate: "TEST ${String(i).padStart(3, "0")}|germany"\n accessories: ${parts.length}\n`;
  parts.forEach((_, j) => (text += ` accessories[${j}]: part${i}_${j}\n`));
  text += "}\n";
  parts.forEach(
    (d, j) =>
      (text += `vehicle_accessory : part${i}_${j} {\n data_path: "${d.path}"\n refund: 0\n}\n`),
  );
}
text += "}\n";
await writeFile(resolve(slot, "game.sii"), text);
await writeFile(
  resolve(slot, "info.sii"),
  'SiiNunit {\nsave_container : info {\n name: "Garage UI Fixture"\n dependencies: 0\n}\n}\n',
);
console.log(root);
