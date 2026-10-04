// Native WebView + real backend, entirely synthetic data and localized names.
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { mkdtemp, mkdir, writeFile, readFile } from "node:fs/promises";
import { resolve, join } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import {
  createSyntheticGame,
  bindSyntheticCatalog,
} from "./synthetic-catalog.mjs";

await mkdir(resolve("verification"), { recursive: true });
const root = await mkdtemp(resolve("verification/p4-native-"));
const app = join(root, "app"),
  slot = join(root, "game/profiles/fixture/save/1");
await mkdir(app, { recursive: true });
await mkdir(slot, { recursive: true });
const installation = await createSyntheticGame(join(root, "installation"));
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
    game: installation,
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
await bindSyntheticCatalog(join(app, "catalog.json"), installation);
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
  const packageVersion = JSON.parse(
    await readFile(resolve("package.json"), "utf8"),
  ).version;
  assert.equal(
    (await page.locator(".rail-bottom").innerText()).trim(),
    `${packageVersion} / PUBLIC PREVIEW`,
  );
  assert.equal(await page.locator("footer").count(), 0);
  assert(
    await page
      .locator(".brand-icon img")
      .evaluate((img) => img.complete && img.naturalWidth === 256),
    "Selected C logo loads from the packaged app",
  );
  assert.equal(await page.locator(".backup-status").count(), 0);
  const layoutChecks = [];
  for (const viewport of [
    { width: 1080, height: 720 },
    { width: 1440, height: 940 },
    { width: 2560, height: 1392 },
  ]) {
    await page.setViewportSize(viewport);
    await page.locator(".inspector").evaluate((el) => (el.scrollTop = 0));
    await delay(100);
    const measure = await page.evaluate(() => {
      const box = (selector) => {
        const r = document.querySelector(selector).getBoundingClientRect();
        return {
          top: r.top,
          bottom: r.bottom,
          height: r.height,
          center: r.top + r.height / 2,
        };
      };
      return {
        inspector: box(".inspector"),
        last: box(".inspector > :last-child"),
        candidates: box(".candidates"),
        language: box(".header-right .language-picker select"),
        chip: box(".header-right .chip"),
        backup: box(".header-backup"),
        fleetHeading: box(".fleet .section-head h2"),
        fleetCount: box(".fleet .section-head > span"),
      };
    });
    assert(
      Math.abs(measure.language.center - measure.chip.center) < 2,
      "Header controls vertically align",
    );
    assert(
      Math.abs(measure.language.center - measure.backup.center) < 2,
      "Backup label aligns with language",
    );
    assert(
      Math.abs(measure.fleetHeading.bottom - measure.fleetCount.bottom) < 5,
      "Fleet heading and count share a baseline",
    );
    if (viewport.height > 1000) {
      assert(
        measure.candidates.height > 350,
        "Long candidate list uses the available height",
      );
      assert(
        measure.inspector.bottom - measure.last.bottom < 32,
        "No empty white tail in the inspector",
      );
    }
    await page
      .locator(".inspector")
      .evaluate((el) => (el.scrollTop = el.scrollHeight));
    const button = await page
      .getByRole("button", { name: "加入变更清单", exact: true })
      .boundingBox();
    const panel = await page.locator(".inspector").boundingBox();
    assert(
      button.y >= panel.y &&
        button.y + button.height <= panel.y + panel.height + 1,
      "Stage button remains accessible at every size",
    );
    await page.locator(".inspector").evaluate((el) => (el.scrollTop = 0));
    await page.screenshot({ path: join(root, `layout-${viewport.width}.png`) });
    layoutChecks.push({ viewport, ...measure });
  }
  await page.setViewportSize({ width: 1440, height: 940 });

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
  await page
    .locator(".inspector .backup-reminder")
    .filter({ hasText: "保存时自动备份原存档" })
    .waitFor();
  assert.equal(
    await page.locator(".backup-status").count(),
    0,
    "Selecting a part must not claim a backup exists",
  );
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
  assert.equal(
    await readFile(join(records[0].backup, "game.sii"), "utf8"),
    text,
  );
  await page
    .locator(".backup-status")
    .filter({ hasText: "原存档已备份" })
    .waitFor();
  await page
    .locator(".backup-status")
    .getByRole("button", { name: "恢复此备份", exact: true })
    .click();
  await page.getByRole("button", { name: "取消", exact: true }).click();
  await page.getByRole("button", { name: "查看全部备份", exact: true }).click();
  assert.equal(await page.locator(".history-list .history-restore").count(), 1);
  assert.equal(
    await page.locator(".history-verification").getAttribute("open"),
    null,
  );
  await page.locator(".history-verification summary").click();
  assert.match(
    await page.locator(".history-verification").innerText(),
    /不会恢复备份或修改存档/,
  );
  await page.screenshot({ path: join(root, "backups-and-checking.png") });

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
        layoutChecks,
        backupTimingAndEntrance: true,
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
