import assert from "node:assert/strict";
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { mkdir, mkdtemp, writeFile, readFile } from "node:fs/promises";
import { resolve, join } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { createCipheriv } from "node:crypto";
import { deflateSync } from "node:zlib";
import {
  createSyntheticGame,
  bindSyntheticCatalog,
} from "./synthetic-catalog.mjs";

// All saves are synthetic; no game or player files are modified.
const baseline = process.argv.includes("--baseline");
await mkdir(resolve("verification/save-performance"), { recursive: true });
const root = await mkdtemp(resolve("verification/save-perf-"));
const app = join(root, "app"),
  docs = join(root, "game");
await mkdir(app, { recursive: true });
const installation = await createSyntheticGame(join(root, "installation"));
function encryptedInfo(name) {
  const plain = Buffer.from(
    `SiiNunit {\nsave_container : info {\n name: "${name}"\n dependencies: 0\n}\n}\n`,
  );
  const iv = Buffer.alloc(16, 7);
  const cipher = createCipheriv(
    "aes-256-cbc",
    Buffer.from(
      "2a5fcb1791d22fb60245b3d8369ed0b2c27371563fbf1f3c9edf6b11825a5d0a",
      "hex",
    ),
    iv,
  );
  const len = Buffer.alloc(4);
  len.writeUInt32LE(plain.length);
  return Buffer.concat([
    Buffer.from("ScsC"),
    Buffer.alloc(32),
    iv,
    len,
    cipher.update(deflateSync(plain)),
    cipher.final(),
  ]);
}
const defPath = "/def/vehicle/truck/daf.2021/engine/test.sii";
const game = `SiiNunit {\nplayer : p {\n trucks: 1\n trucks[0]: t\n assigned_truck: t\n}\nvehicle : t {\n accessories: 1\n accessories[0]: a\n}\nvehicle_accessory : a {\n data_path: "${defPath}"\n refund: 0\n}\n}\n`;
const slots = [
  ...Array.from({ length: 10 }, (_, i) => String(i + 1)),
  "quicksave",
  ...Array.from({ length: 100 }, (_, i) =>
    i ? `autosave_job_${i}` : "autosave",
  ),
];
for (const slot of slots) {
  const dir = join(docs, "profiles/fixture/save", slot);
  await mkdir(dir, { recursive: true });
  await writeFile(join(dir, "game.sii"), game);
  await writeFile(
    join(dir, "info.sii"),
    encryptedInfo(slot === "1" ? "autosave named manual" : slot),
  );
}
await writeFile(
  join(app, "settings.json"),
  JSON.stringify({
    documents: docs,
    game: installation,
    extractor: "",
    onboarding_version: 1,
  }),
);
await writeFile(
  join(app, "catalog.json"),
  JSON.stringify({
    definitions: {
      [defPath]: {
        path: defPath,
        model: "daf.2021",
        category: "engine",
        name: "Test Engine",
        kind: "accessory_engine_data",
        unit: "test",
        source: "synthetic",
        metrics: {},
        suitable: [],
        requires: [],
        conflicts: [],
      },
    },
    archives: [],
    warnings: [],
    signature: "synthetic",
    name_schema: 1,
  }),
);
await bindSyntheticCatalog(join(app, "catalog.json"), installation);
const port = 9236;
const proc = spawn(
  resolve(
    baseline
      ? "verification/save-performance/baseline.exe"
      : "src-tauri/target/debug/ets2-workshop.exe",
  ),
  [],
  {
    windowsHide: true,
    stdio: "ignore",
    env: {
      ...process.env,
      ETS2_WORKSHOP_DATA_DIR: app,
      WEBVIEW2_USER_DATA_FOLDER: join(root, "webview"),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
    },
  },
);
let browser;
try {
  for (let i = 0; i < 100; i++) {
    try {
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
      break;
    } catch {
      await delay(200);
    }
  }
  assert(browser);
  let page;
  for (let i = 0; i < 100; i++) {
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
  await page.waitForFunction(
    (n) =>
      [...document.querySelectorAll(".savebar select option")].filter((o) =>
        o.value.replaceAll("\\", "/").includes("/profiles/fixture/save/"),
      ).length === n && !document.querySelector("fieldset")?.disabled,
    baseline ? 111 : 11,
    { timeout: 60000 },
  );
  async function scan(include) {
    return page.evaluate(async (include) => {
      const start = performance.now();
      const discovered = await window.__TAURI_INTERNALS__.invoke("rpc", {
        payload: { action: "discover", include_autosaves: include },
      });
      const list = Array.isArray(discovered) ? discovered : discovered.saves;
      return {
        ms: Math.round(performance.now() - start),
        total: list.length,
        count: list.filter((s) =>
          s.path.replaceAll("\\", "/").includes("/profiles/fixture/save/"),
        ).length,
        errors: list.filter(
          (s) =>
            s.path.replaceAll("\\", "/").includes("/profiles/fixture/save/") &&
            s.error,
        ).length,
      };
    }, include);
  }
  const manual = await scan(false),
    all = await scan(true),
    warm = await scan(true);
  assert.equal(manual.count, baseline ? 111 : 11);
  assert.equal(all.count, 111);
  assert.equal(warm.count, 111);
  assert.equal(all.errors, 0);
  assert.equal(warm.errors, 0);
  let toggleMs = null;
  if (!baseline) {
    const filter = page.getByRole("checkbox", { name: "包含自动存档" });
    assert.equal(await filter.isChecked(), false);
    await filter.check();
    await page.waitForFunction(
      () =>
        [...document.querySelectorAll(".savebar select option")].filter((o) =>
          o.value.replaceAll("\\", "/").includes("/profiles/fixture/save/"),
        ).length === 111,
    );
    const auto = await page
      .locator(".savebar option")
      .evaluateAll(
        (options) =>
          options.find((o) =>
            o.value
              .replaceAll("\\", "/")
              .endsWith("/profiles/fixture/save/autosave/game.sii"),
          )?.value,
      );
    assert(auto);
    await page.getByLabel("选择存档", { exact: true }).selectOption(auto);
    await page.getByRole("button", { name: "打开存档", exact: true }).click();
    await page.locator(".opened-save").waitFor();
    await filter.uncheck();
    await page.waitForFunction(
      () =>
        [...document.querySelectorAll(".savebar select option")].filter((o) =>
          o.value.replaceAll("\\", "/").includes("/profiles/fixture/save/"),
        ).length === 11,
    );
    assert.match(await page.locator(".opened-save").innerText(), /autosave/);
    assert.equal(await page.locator(".truck-card").count(), 1);
    const toggleStart = performance.now();
    await filter.check();
    await page.waitForFunction(
      () =>
        [...document.querySelectorAll(".savebar select option")].filter((o) =>
          o.value.replaceAll("\\", "/").includes("/profiles/fixture/save/"),
        ).length === 111,
    );
    toggleMs = Math.round(performance.now() - toggleStart);
    assert.match(await page.locator(".opened-save").innerText(), /autosave/);
    await filter.uncheck();
    await page.waitForFunction(
      () =>
        [...document.querySelectorAll(".savebar select option")].filter((o) =>
          o.value.replaceAll("\\", "/").includes("/profiles/fixture/save/"),
        ).length === 11,
    );
    await page.reload();
    await page.waitForFunction(
      () =>
        [...document.querySelectorAll(".savebar select option")].filter((o) =>
          o.value.replaceAll("\\", "/").includes("/profiles/fixture/save/"),
        ).length === 11,
    );
    assert.equal(await filter.isChecked(), false);
    await filter.check();
    await page.waitForFunction(
      () =>
        [...document.querySelectorAll(".savebar select option")].filter((o) =>
          o.value.replaceAll("\\", "/").includes("/profiles/fixture/save/"),
        ).length === 111,
    );
    await page.reload();
    await page.waitForFunction(
      () =>
        [...document.querySelectorAll(".savebar select option")].filter((o) =>
          o.value.replaceAll("\\", "/").includes("/profiles/fixture/save/"),
        ).length === 111,
    );
    assert.equal(await filter.isChecked(), true);
    await page.screenshot({
      path: join(root, "autosave-filter.png"),
      fullPage: true,
    });
  }
  assert.deepEqual(errors, []);
  for (const slot of slots)
    assert.equal(
      await readFile(
        join(docs, "profiles/fixture/save", slot, "game.sii"),
        "utf8",
      ),
      game,
    );
  const result = { baseline, manual, all, warm, toggleMs };
  await writeFile(join(root, "result.json"), JSON.stringify(result, null, 2));
  await writeFile(
    resolve(
      `verification/save-performance/${baseline ? "baseline" : "current"}-result.json`,
    ),
    JSON.stringify({ ...result, root }, null, 2),
  );
  console.log(JSON.stringify(result));
} finally {
  if (browser) await browser.close();
  proc.kill();
}
