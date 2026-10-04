// Native WebView + real backend, entirely synthetic data and localized names.
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { mkdtemp, mkdir, writeFile, readFile } from "node:fs/promises";
import { resolve, join } from "node:path";
import { setTimeout as delay } from "node:timers/promises";

await mkdir(resolve("verification"), { recursive: true });
const root = await mkdtemp(resolve("verification/p4-native-"));
const app = join(root, "app"),
  slot = join(root, "game/profiles/fixture/save/1");
await mkdir(app, { recursive: true });
await mkdir(slot, { recursive: true });
const definitions = {};
function definition(model, category, id, names, metrics = {}) {
  const path = `/def/vehicle/truck/${model}/${category}/${id}.sii`;
  const def = {
    path,
    model,
    category,
    name: names.en,
    raw_name: `@@fixture_${id}@@`,
    names,
    category_names: {
      en: category,
      zh_cn: category === "engine" ? "发动机" : "底盘",
    },
    kind: `accessory_${category}_data`,
    unit: id,
    source: "synthetic",
    metrics,
    suitable: [],
    requires: [],
    conflicts: [],
  };
  definitions[path] = def;
  return def;
}
const a = definition(
  "daf.2021",
  "engine",
  "a",
  { en: "Game Engine Alpha", zh_cn: "游戏引擎甲" },
  { info: "428 hp", torque: "2150" },
);
const b = definition(
  "daf.2021",
  "engine",
  "b",
  { en: "Game Engine Beta", zh_cn: "游戏引擎乙" },
  { info: "530 hp", torque: "2600" },
);
const chassis = definition(
  "daf.2021",
  "chassis",
  "4x2",
  { en: "Game Chassis", zh_cn: "游戏底盘" },
  { tank_size: "600" },
);
const cabin = definition("daf.2021", "cabin", "cab", {
  en: "Game Cabin",
  zh_cn: "游戏驾驶室",
});
for (let i = 0; i < 160; i++)
  definition(
    "daf.2021",
    "engine",
    `engine${i}`,
    { en: `Engine ${i}`, zh_cn: `引擎 ${i}` },
    { info: `${200 + i} hp`, torque: "2000" },
  );
definition(
  "future.make",
  "engine",
  "unknown-model",
  { en: "Unidentified Engine", zh_cn: "未识别车型引擎" },
  { info: "600 hp", torque: "2000" },
);
const tire = {
  ...a,
  path: "/def/vehicle/f_tire/shared.sii",
  model: "通用",
  category: "f_tire",
  name: "Shared Tire",
  names: { en: "Shared Tire", zh_cn: "共享轮胎" },
};
definitions[tire.path] = tire;
const parts = [a, chassis, cabin];
let text =
  'SiiNunit {\nplayer : p {\n trucks: 1\n trucks[0]: t\n assigned_truck: t\n}\nvehicle : t {\n license_plate: "P4 TEST|germany"\n accessories: 3\n accessories[0]: a0\n accessories[1]: a1\n accessories[2]: a2\n}\n';
parts.forEach(
  (d, i) =>
    (text += `vehicle_accessory : a${i} {\n data_path: "${d.path}"\n refund: 0\n}\n`),
);
text += "}\n";
await writeFile(join(slot, "game.sii"), text);
await writeFile(
  join(slot, "info.sii"),
  'SiiNunit {\nsave_container : info {\n name: "P4 synthetic"\n file_time: 1\n dependencies: 0\n}\n}\n',
);
await writeFile(
  join(app, "settings.json"),
  JSON.stringify({
    documents: join(root, "game"),
    game: root,
    extractor: "",
    onboarding_version: 1,
  }),
);
await writeFile(
  join(app, "catalog.json"),
  JSON.stringify({
    definitions,
    archives: [],
    warnings: [],
    signature: "synthetic",
    name_schema: 1,
  }),
);
const port = 9232;
const proc = spawn(resolve("src-tauri/target/debug/ets2-workshop.exe"), [], {
  windowsHide: true,
  stdio: "ignore",
  env: {
    ...process.env,
    ETS2_WORKSHOP_DATA_DIR: app,
    WEBVIEW2_USER_DATA_FOLDER: join(root, "webview"),
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
  },
});
let browser;
try {
  for (let i = 0; i < 80; i++) {
    try {
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
      break;
    } catch {
      await delay(200);
    }
  }
  assert(browser);
  let page;
  for (let i = 0; i < 50; i++) {
    page = browser
      .contexts()
      .flatMap((c) => c.pages())
      .find((p) => p.url().includes("tauri.localhost"));
    if (page) break;
    await delay(100);
  }
  assert(page);
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.getByRole("button", { name: "打开存档", exact: true }).click();
  await page.getByLabel("配件类别", { exact: true }).selectOption("chassis");
  await page.locator(".part-row").click();
  await page.getByRole("button", { name: "本机配件库", exact: true }).click();
  await page.locator(".candidate").click();
  assert.equal(
    await page
      .getByRole("button", { name: "加入变更清单", exact: true })
      .isDisabled(),
    true,
  );
  assert.match(
    await page.locator(".inspector").innerText(),
    /此核心部件暂不支持替换/,
  );
  await page.getByLabel("配件类别", { exact: true }).selectOption("engine");
  await page.locator(".part-row").click();
  assert.equal(await page.locator(".candidate").count(), 50);
  assert.doesNotMatch(
    await page.locator(".candidates").innerText(),
    /未识别车型引擎/,
  );
  for (let i = 0; i < 3; i++)
    await page
      .locator(".candidate-pagination")
      .getByRole("button", { name: "下一页", exact: true })
      .click();
  assert.match(
    await page.locator(".candidate").last().innerText(),
    /未识别车型引擎/,
  );
  await page.getByLabel("搜索候选配件").fill("530");
  assert.equal(await page.locator(".candidate").count(), 1);
  assert.match(await page.locator(".candidate").innerText(), /游戏引擎乙/);
  await page.locator(".candidate").click();
  await page.getByRole("button", { name: "加入变更清单", exact: true }).click();
  await page.waitForSelector(".changes .change");
  await page.getByLabel("语言 / Language", { exact: true }).selectOption("en");
  assert.match(await page.locator(".changes").innerText(), /Game Engine Beta/);
  assert.equal(await page.locator(".changes .change").count(), 1);
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  assert.equal(
    await page
      .getByRole("button", { name: "Build / update library", exact: true })
      .isDisabled(),
    true,
  );
  await page.locator(".nav").filter({ hasText: "Garage" }).click();
  await page.getByRole("button", { name: "Save changes" }).click();
  await page.locator(".modal input").fill("P4 bilingual save");
  await page
    .getByLabel("语言 / Language", { exact: true })
    .selectOption("zh-CN");
  assert.equal(
    await page.locator(".modal input").inputValue(),
    "P4 bilingual save",
  );
  await page.getByRole("button", { name: "确认保存", exact: true }).click();
  await page
    .locator(".notice.success")
    .filter({ hasText: "已保存为 P4 bilingual save" })
    .waitFor();
  assert.equal(await readFile(join(slot, "game.sii"), "utf8"), text);
  const records = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("rpc", {
      payload: { action: "history" },
    }),
  );
  assert.equal(records.length, 1);
  assert(records[0].output.startsWith(root));
  assert.match(
    await readFile(records[0].output, "utf8"),
    new RegExp(b.path.replaceAll(".", "\\.")),
  );
  await page.getByRole("button", { name: "配件目录", exact: true }).click();
  assert.equal(await page.locator(".catalog-grid article").count(), 60);
  for (let i = 0; i < 2; i++)
    await page
      .locator(".catalog-pagination")
      .getByRole("button", { name: "下一页", exact: true })
      .click();
  assert.match(
    await page.locator(".catalog-grid article").last().innerText(),
    /未识别车型/,
  );
  await page.getByLabel("目录分类").selectOption("engine");
  await page.getByLabel("搜索配件目录").fill("530");
  assert.equal(await page.locator(".catalog-grid article").count(), 1);
  await page.screenshot({ path: join(root, "p4-zh.png") });
  await page.getByLabel("语言 / Language", { exact: true }).selectOption("en");
  assert.match(
    await page.locator(".catalog-grid").innerText(),
    /Game Engine Beta/,
  );
  await page.screenshot({ path: join(root, "p4-en.png") });
  assert.deepEqual(errors, []);
  await writeFile(
    join(root, "result.json"),
    JSON.stringify(
      {
        passed: true,
        names: true,
        brandOrdering: true,
        allPagesReachable: true,
        readOnlyReasons: true,
        languageStateAndSave: true,
        pageErrors: errors,
      },
      null,
      2,
    ),
  );
  console.log(`P4 native passed: ${root}`);
} finally {
  if (browser) await browser.close();
  proc.kill();
}
