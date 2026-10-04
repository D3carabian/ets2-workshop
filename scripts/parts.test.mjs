import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(
  new URL("../src/parts.ts", import.meta.url),
  "utf8",
);
const js = ts.transpileModule(source, {
  compilerOptions: {
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ESNext,
  },
}).outputText;
const { describePart, power, primaryMetrics, groupOf, truckDefinition } =
  await import(
    `data:text/javascript;base64,${Buffer.from(js).toString("base64")}`
  );
function definition(category = "engine", name = "mx13 315", metrics = {}) {
  return {
    name,
    category,
    model: "daf.2021",
    path: `/def/vehicle/truck/daf.2021/${category}/${name.replaceAll(" ", "_")}.sii`,
    kind: "accessory_engine_data",
    metrics,
    source: "fixture",
    suitable: [],
    conflicts: [],
    requires: [],
  };
}
test("MX variant is not mistaken for horsepower; use game info, not a calculated wiki peak", () => {
  const def = definition("engine", "mx13 315", {
    info: "428 hp (315kW) · 2,150 Nm · 900-1,400 rpm",
    torque: "2150",
  });
  assert.equal(power(def), "428");
  assert.match(describePart(def).title, /DAF.*PACCAR MX-13/);
  assert.equal(describePart(def).summary, "428 hp · 2150 Nm");
  assert.ok(describePart(def).search.includes("315"));
  assert.equal(
    new Map(primaryMetrics(def)).get("标称扭矩转速范围"),
    "900-1,400 rpm",
  );
  assert.equal(power(definition()), null);
  assert.match(describePart(definition()).summary, /功率未知/);
});
test("vehicle data and unknown localized names stay readable and traceable", () => {
  const vehicle = definition("vehicle", "data");
  assert.match(describePart(vehicle).title, /DAF XG.*车辆基础/);
  const part = definition("cabin", "@@missing_localization@@");
  assert.doesNotMatch(describePart(part).title, /@@/);
  assert.match(describePart(part).title, /驾驶室/);
  assert.ok(describePart(part).search.includes(part.path));
});
test("trailer chassis cannot enter truck catalog, shared wheels remain available", () => {
  const def = definition("chassis", "ch 3");
  assert.equal(
    truckDefinition({
      ...def,
      path: "/def/vehicle/trailer_owned/scs.flatbed/chassis/ch_3.sii",
    }),
    false,
  );
  assert.equal(
    truckDefinition({
      ...def,
      path: "/def/vehicle/trailer/scs_lowbed/ch_3.sii",
    }),
    false,
  );
  assert.equal(
    truckDefinition({ ...def, path: "/def/vehicle/f_tire/michelin.sii" }),
    true,
  );
  assert.equal(truckDefinition(def), true);
});
test("UI groups keep engine/interior/exterior/unknown and concrete axle categories distinct", () => {
  assert.equal(groupOf(definition()), "engine");
  assert.equal(groupOf(definition("interior")), "interior");
  assert.equal(groupOf(definition("toyhang")), "interior_accessories");
  assert.equal(groupOf(definition("f_tire")), "exterior");
  assert.equal(groupOf(definition("r_tire")), "exterior");
  assert.equal(groupOf(definition("unknown")), "other");
});
test("transmission summary prioritizes actual ratios and retarder data", () => {
  const def = definition("transmission", "zf 12as3141to", {
    ratios_forward: "12.29 · 9.59 · 1.0 · 0.78",
    differential_ratio: "3.08",
    retarder: "3",
  });
  assert.match(describePart(def).title, /ZF 12AS3141TO/);
  assert.match(describePart(def).summary, /4 挡.*3.08.*3 级/);
});

test("Mercedes internal variant requires exact path and matching installed kW", () => {
  const def = {
    ...definition("engine", "engine 1851", {
      info: "510 hp (375kW) · 1,100 rpm",
    }),
    model: "mercedes.actros2014",
    path: "/def/vehicle/truck/mercedes.actros2014/engine/engine_1851.sii",
  };
  assert.match(describePart(def).title, /Mercedes-Benz.*OM 471 Euro VI/);
  assert.equal(
    new Map(primaryMetrics(def)).get("标称扭矩转速范围"),
    "1,100 rpm",
  );
  assert.doesNotMatch(
    describePart({ ...def, metrics: { info: "510 hp (400kW)" } }).title,
    /OM 471/,
  );
  assert.doesNotMatch(
    describePart({ ...def, path: def.path.replace("actros2014", "actros") })
      .title,
    /OM 471/,
  );
});

test("unmapped accessory subtype stays unclassified instead of guessing exterior", () => {
  const def = {
    ...definition("new_interior_part", "future"),
    path: "/def/vehicle/truck/daf.2021/accessory/new_interior_part/future.sii",
  };
  assert.equal(groupOf(def), "other");
});

test("literal game names that differ from the filename remain visible", () => {
  const def = {
    ...definition("cabin", "Highline"),
    path: "/def/vehicle/truck/scania.s_2016/cabin/highline.sii",
    model: "scania.s_2016",
  };
  assert.match(describePart(def).title, /Highline/);
});
