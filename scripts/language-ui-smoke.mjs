// Isolated browser UI regression: real React app, synthetic IPC; never reads game saves.
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";
import { mkdir } from "node:fs/promises";

const server = await createServer({
  server: { host: "127.0.0.1", port: 1431, strictPort: true },
});
await server.listen();
const browser = await chromium.launch({
  channel: process.env.ETS2_TEST_BROWSER || "msedge",
  headless: true,
});
try {
  const page = await browser.newPage({
    viewport: { width: 1440, height: 940 },
  });
  const errors = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  await page.addInitScript(() => {
    const engine = (name, hp) => ({
      path: `/def/vehicle/truck/daf.2021/engine/${name}.sii`,
      category: "engine",
      model: "daf.2021",
      name,
      kind: "accessory_engine_data",
      unit: name,
      source: "synthetic",
      metrics: { info: `${hp} hp`, torque: "2150" },
      suitable: [],
      conflicts: [],
      requires: [],
    });
    const current = engine("mx13_315", 428),
      replacement = engine("mx13_390", 530);
    const truck = {
      id: "truck.synthetic",
      plate: "TEST LANG",
      model: "daf.2021",
      current: true,
      location: "synthetic",
      accessories: [
        {
          id: "part.synthetic",
          index: 0,
          kind: "vehicle_accessory",
          path: current.path,
          category: "engine",
          name: current.name,
          model: current.model,
          refund: "100",
          raw: "",
          definition: current,
        },
      ],
    };
    const settings = {
      documents: "synthetic-documents",
      game: "synthetic-game",
      extractor: "",
      onboarding_version: 1,
    };
    const save = {
      path: "synthetic/game.sii",
      name: "Synthetic save",
      profile: "Test",
      modified: 1000,
      error: null,
    };
    const changed = {
      truck_id: truck.id,
      model: truck.model,
      plate: truck.plate,
      category: "engine",
      action: "replace",
      before: current.path,
      after: replacement.path,
    };
    window.__languageTestCalls = [];
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    window.__TAURI_INTERNALS__ = {
      transformCallback: () => 1,
      unregisterCallback: () => {},
      invoke: async (command, args) => {
        if (command.startsWith("plugin:event|")) return 1;
        const payload = args.payload;
        window.__languageTestCalls.push(payload);
        switch (payload.action) {
          case "init":
            return {
              settings,
              catalog_count: 2,
              data_dir: "synthetic-data",
              needs_setup: false,
              catalog_rebuild_reason: null,
            };
          case "catalog":
            return [current, replacement];
          case "discover":
            return { saves: [save], warnings: [] };
          case "settings":
            return { settings: payload.settings, catalog_changed: false };
          case "history":
            return [];
          case "open":
            if (payload.path === "invalid") throw "无法读取指定的存档";
            return {
              path: save.path,
              hash: "synthetic",
              trucks: [truck],
              warnings: [],
            };
          case "preview":
            return {
              trucks: [truck],
              changes: payload.operations.map(() => changed),
              warnings: [],
            };
          case "detect":
            return {
              games: [settings.game],
              documents: [settings.documents],
              steam_roots: [],
              notes: [],
            };
          case "commit":
            return {
              id: "1000-1",
              output: save.path,
              backup: "synthetic-backup",
              state: "completed",
              warning: null,
              changes: [changed],
            };
          default:
            throw new Error(`Unexpected synthetic action: ${payload.action}`);
        }
      },
    };
  });
  await page.goto("http://127.0.0.1:1431");
  await page.getByRole("button", { name: "打开存档", exact: true }).click();
  await page.locator(".part-row").click();
  await page.getByRole("button", { name: "本机配件库", exact: true }).click();
  await page.getByLabel("搜索候选配件").fill("530");
  await page.locator(".candidate").click();
  await page.getByRole("button", { name: "加入变更清单", exact: true }).click();
  await page.locator(".status-pending.has-pending").waitFor();
  assert.equal(await page.locator(".changes").count(), 0);
  const callsBefore = await page.evaluate(
    () => window.__languageTestCalls.length,
  );
  await page.getByLabel("语言 / Language", { exact: true }).selectOption("en");
  await page.getByRole("button", { name: "Save changes" }).waitFor();
  assert.equal(
    await page.locator(".titlebar-save b").innerText(),
    "1",
    "Language switch preserves staged operation",
  );
  assert.equal(
    await page.getByLabel("Search replacement parts").inputValue(),
    "530",
    "Search state survives language switch",
  );
  assert.match(
    await page.locator(".notice.success").innerText(),
    /Change staged/,
  );
  assert.equal(
    await page.evaluate(() => window.__languageTestCalls.length),
    callsBefore,
    "Switching does not reload backend or App",
  );
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  assert(
    await page
      .getByRole("button", { name: "Run setup again", exact: true })
      .isDisabled(),
  );
  await page.getByRole("button", { name: /^Garage/ }).click();
  await page.locator(".titlebar-save").click();
  const review = page.locator(".modal.change-review");
  await review.locator(".changes .change").waitFor();
  assert.equal(await review.locator(".change").count(), 1);
  await review.locator("input").fill("语言保留");
  await page
    .getByLabel("语言 / Language", { exact: true })
    .selectOption("zh-CN");
  assert.equal(await review.locator("input").inputValue(), "语言保留");
  assert.match(await review.innerText(), /保存这次改装/);
  assert.equal(await review.locator(".change").count(), 1);
  await page.getByLabel("语言 / Language", { exact: true }).selectOption("en");
  await review.getByRole("button", { name: /^Confirm save/ }).click();
  await page
    .locator(".notice.success")
    .filter({ hasText: "Saved as 语言保留" })
    .waitFor();
  const commit = await page.evaluate(() =>
    window.__languageTestCalls.find((c) => c.action === "commit"),
  );
  assert.equal(commit.operations.length, 1);
  assert.match(commit.operations[0].candidate_path, /mx13_390/);
  assert.equal(commit.name, "语言保留");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page
    .getByRole("button", { name: "Run setup again", exact: true })
    .click();
  await page.getByRole("dialog", { name: "First-run setup" }).waitFor();
  await page
    .locator(".onboarding .language-picker select")
    .selectOption("zh-CN");
  assert.equal(
    await page.locator("input#game-directory").inputValue(),
    "synthetic-game",
  );
  await page.getByRole("button", { name: "取消", exact: true }).click();
  await page.getByLabel("语言 / Language", { exact: true }).selectOption("en");
  await page.reload();
  await page.getByRole("button", { name: "Open save", exact: true }).waitFor();
  assert.equal(
    await page.getByLabel("语言 / Language", { exact: true }).inputValue(),
    "en",
    "Language preference persists on reload",
  );
  assert.equal(await page.locator("html").getAttribute("lang"), "en");
  await page.getByText("Manual path", { exact: true }).click();
  await page.getByLabel("game.sii path", { exact: true }).fill("invalid");
  await page.getByRole("button", { name: "Open", exact: true }).click();
  await page
    .locator(".notice.error")
    .filter({ hasText: "The operation could not finish" })
    .waitFor();
  await page.getByText("Original details", { exact: true }).click();
  assert.match(
    await page.locator(".notice.error").innerText(),
    /无法读取指定的存档/,
  );
  assert.deepEqual(errors, []);
  await mkdir("verification/language", { recursive: true });
  await page.screenshot({
    path: "verification/language/english.png",
    fullPage: true,
  });
  console.log(
    "Language UI passed: staged operation, searches, setup draft and save modal survive switches; save payload, stored preference and original error details verified.",
  );
} finally {
  await browser.close();
  await server.close();
}
