// Actual desktop + backend state transitions; every writable file is synthetic.
// Build the custom-protocol desktop and examples/synthetic_catalog before running.
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { createServer } from "node:net";
import {
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  writeFile,
} from "node:fs/promises";
import { join, resolve } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import {
  bindSyntheticCatalog,
  createSyntheticGame,
} from "./synthetic-catalog.mjs";

const repo = resolve(import.meta.dirname, "..");
const verification = join(repo, "verification");
await mkdir(verification, { recursive: true });
const root = await mkdtemp(join(verification, "review-state-"));
const app = join(root, "app"),
  docs = join(root, "documents");
const slot = join(docs, "profiles/fixture/save/1"),
  save = join(slot, "game.sii");
await mkdir(app);
await mkdir(slot, { recursive: true });
const gameA = await createSyntheticGame(join(root, "Game A"));
const gameB = await createSyntheticGame(join(root, "Game B"));
const definitions = {};
function definition(category, id) {
  const path = `/def/vehicle/truck/daf.2021/${category}/${id}.sii`;
  const part = {
    path,
    category,
    model: "daf.2021",
    name: id,
    raw_name: id,
    names: { en: id, zh_cn: id },
    kind: `accessory_${category}_data`,
    unit: `${id}.${category}`,
    source: "synthetic-review",
    metrics: {},
    suitable: [],
    requires: [],
    conflicts: [],
  };
  definitions[path] = part;
  return part;
}
const engine = definition("engine", "engine-original"),
  nextEngine = definition("engine", "engine-replacement"),
  transmission = definition("transmission", "transmission-original"),
  nextTransmission = definition("transmission", "transmission-replacement");
const original = `SiiNunit {\nplayer : p {\n trucks: 1\n trucks[0]: t\n assigned_truck: t\n}\nvehicle : t {\n license_plate: "SYNTHETIC REVIEW"\n accessories: 2\n accessories[0]: e\n accessories[1]: g\n}\nvehicle_accessory : e {\n data_path: "${engine.path}" // untouched\n refund: 0\n}\nvehicle_accessory : g {\n data_path: "${transmission.path}"\n refund: 0\n}\n}\n`;
await writeFile(save, original);
await writeFile(
  join(slot, "info.sii"),
  'SiiNunit {\nsave_container : info {\n name: "Review synthetic"\n dependencies: 0\n}\n}\n',
);
await writeFile(
  join(app, "settings.json"),
  JSON.stringify({
    game: gameA,
    documents: docs,
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
const operations = [
  {
    truck_id: "t",
    action: "replace",
    accessory_id: "e",
    candidate_path: nextEngine.path,
  },
  {
    truck_id: "t",
    action: "replace",
    accessory_id: "g",
    candidate_path: nextTransmission.path,
  },
];
const server = createServer();
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const port = server.address().port;
await new Promise((r) => server.close(r));
let child, browser;
const errors = [];
async function stop() {
  if (child && child.exitCode === null && child.signalCode === null) {
    const exited = new Promise((r) => child.once("exit", r));
    child.kill();
    await Promise.race([exited, delay(5000)]);
  }
  if (browser) {
    await Promise.race([browser.close(), delay(5000)]);
    browser = undefined;
  }
}
async function start() {
  child = spawn(join(repo, "src-tauri/target/debug/ets2-workshop.exe"), [], {
    cwd: repo,
    windowsHide: true,
    stdio: "ignore",
    env: {
      ...process.env,
      ETS2_WORKSHOP_DATA_DIR: app,
      WEBVIEW2_USER_DATA_FOLDER: join(root, "webview"),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1`,
    },
  });
  let launchError;
  child.once("error", (error) => {
    launchError = error;
  });
  const deadline = Date.now() + 30000;
  while (!browser && Date.now() < deadline) {
    if (launchError) throw launchError;
    assert.equal(child.exitCode, null, "Desktop exited before CDP connected");
    try {
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, {
        timeout: 1000,
      });
    } catch {
      await delay(200);
    }
  }
  assert(browser, "Desktop CDP unavailable");
  let page;
  while (!page && Date.now() < deadline) {
    page = browser
      .contexts()
      .flatMap((c) => c.pages())
      .find((p) => p.url().includes("tauri.localhost"));
    if (!page) await delay(100);
  }
  assert(page, "Build the desktop with custom-protocol");
  page.setDefaultTimeout(30000);
  page.on("pageerror", (error) => errors.push(String(error)));
  await page.getByRole("button", { name: "打开存档", exact: true }).waitFor();
  return page;
}
const rpc = (page, action, args = {}) =>
  page.evaluate(
    (payload) => window.__TAURI_INTERNALS__.invoke("rpc", { payload }),
    { action, ...args },
  );
async function rejected(page, action, args, pattern) {
  await assert.rejects(rpc(page, action, args), (error) =>
    pattern.test(String(error)),
  );
}
async function openSave(page) {
  const normalized = save.replaceAll("\\", "/");
  await page.waitForFunction(
    (wanted) =>
      [...document.querySelectorAll(".savebar option")].some(
        (o) => o.value.replaceAll("\\", "/") === wanted,
      ),
    normalized,
  );
  const value = await page
    .locator(".savebar option")
    .evaluateAll(
      (options, wanted) =>
        options.find((o) => o.value.replaceAll("\\", "/") === wanted)?.value,
      normalized,
    );
  assert(value);
  await page.getByLabel("选择存档", { exact: true }).selectOption(value);
  await page.getByRole("button", { name: "打开存档", exact: true }).click();
  await page.locator(".opened-save").waitFor();
  assert.equal(await page.locator(".truck-card").count(), 1);
}
async function choose(page, category, name) {
  await page.getByLabel("配件类别", { exact: true }).selectOption(category);
  await page.locator(".part-row").click();
  await page.getByRole("button", { name: "本机配件库", exact: true }).click();
  await page.getByLabel("搜索候选配件").fill(name);
  await page.locator(".candidate").filter({ hasText: name }).click();
}
async function generations() {
  return (await readdir(app))
    .filter((name) => /^catalog-.*\.json$/.test(name))
    .sort();
}
// FileShare.Read lets us verify the old bytes while rejecting atomic replacement.
async function lockSettings() {
  const locked = spawn(
    "powershell.exe",
    [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      "$lock = [System.IO.File]::Open($env:ETS2_SYNTHETIC_LOCK_PATH, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::Read); try { [Console]::Out.WriteLine('LOCKED'); [Console]::Out.Flush(); [Console]::ReadLine() | Out-Null } finally { $lock.Dispose() }",
    ],
    {
      windowsHide: true,
      stdio: ["pipe", "pipe", "pipe"],
      env: {
        ...process.env,
        ETS2_SYNTHETIC_LOCK_PATH: join(app, "settings.json"),
      },
    },
  );
  let output = "",
    stderr = "";
  locked.stdout.on("data", (data) => {
    output += data;
  });
  locked.stderr.on("data", (data) => {
    stderr += data;
  });
  const deadline = Date.now() + 10000;
  while (
    !output.includes("LOCKED") &&
    Date.now() < deadline &&
    locked.exitCode === null
  )
    await delay(50);
  if (!output.includes("LOCKED")) {
    locked.kill();
    throw new Error(`Settings lock failed: ${stderr}`);
  }
  return async () => {
    const exited = new Promise((r) => locked.once("exit", r));
    locked.stdin.end("\n");
    await Promise.race([exited, delay(5000)]);
    if (locked.exitCode === null) locked.kill();
  };
}
try {
  let page = await start();
  const stale = await rpc(page, "init");
  assert.match(stale.catalog_rebuild_reason, /重新建立/);
  assert.equal((await rpc(page, "catalog")).length, 4);
  await page
    .getByText(
      "配件目录需要更新，当前仅供查看。请在设置中重新建立目录后再修改。",
      { exact: true },
    )
    .waitFor();
  await openSave(page);
  await choose(page, "engine", "engine-replacement");
  assert(
    await page
      .getByRole("button", { name: "加入变更清单", exact: true })
      .isDisabled(),
  );
  await rejected(page, "preview", { operations: [operations[0]] }, /重新建立/);
  await page.screenshot({ path: join(root, "legacy-catalog-read-only.png") });
  await stop();
  await bindSyntheticCatalog(join(app, "catalog.json"), gameA);

  page = await start();
  assert.equal((await rpc(page, "init")).catalog_rebuild_reason, null);
  await openSave(page);
  for (const [index, category] of ["engine", "transmission"].entries()) {
    await choose(page, category, `${category}-replacement`);
    await page
      .getByRole("button", { name: "加入变更清单", exact: true })
      .click();
    await page.waitForFunction(
      (count) =>
        Number(document.querySelector(".titlebar-save b")?.textContent) ===
        count,
      index + 1,
    );
  }
  assert.equal((await rpc(page, "preview", { operations })).changes.length, 2);
  const before = await rpc(page, "init"),
    oldCatalog = await rpc(page, "catalog");
  const oldSettingsBytes = await readFile(join(app, "settings.json"));
  await page.getByRole("button", { name: "设置", exact: true }).click();
  await page.getByLabel("游戏安装目录", { exact: true }).fill(gameB);
  for (const name of ["保存设置", "重新运行配置向导", "建立 / 更新目录"])
    assert(
      await page.getByRole("button", { name, exact: true }).isDisabled(),
      name,
    );
  const proposed = { ...before.settings, game: gameB };
  for (const action of ["settings", "setup"])
    await rejected(page, action, { settings: proposed }, /先保存或移除/);
  await rejected(page, "index", {}, /先保存或移除/);
  assert.deepEqual((await rpc(page, "init")).settings, before.settings);
  assert.deepEqual(
    await readFile(join(app, "settings.json")),
    oldSettingsBytes,
  );
  assert.equal((await rpc(page, "preview", { operations })).changes.length, 2);
  await page.screenshot({ path: join(root, "pending-switch-blocked.png") });

  await page.getByRole("button", { name: /^车库/ }).click();
  await page.locator(".titlebar-save").click();
  for (const count of [1, 0]) {
    await page
      .getByRole("button", { name: "移除此修改", exact: true })
      .first()
      .click();
    await page.waitForFunction(
      (count) => document.querySelectorAll(".changes .change").length === count,
      count,
    );
  }
  // Settings publication failure must preserve backend settings, catalog and open session.
  await page.getByRole("button", { name: "继续改装", exact: true }).click();
  const priorGenerations = await generations();
  const unlock = await lockSettings();
  try {
    await rejected(
      page,
      "settings",
      { settings: proposed },
      /配置未完成|无法保存设置/,
    );
    assert.deepEqual((await rpc(page, "init")).settings, before.settings);
    assert.deepEqual(await rpc(page, "catalog"), oldCatalog);
    assert.equal(
      (await rpc(page, "preview", { operations: [] })).trucks.length,
      1,
    );
    assert.deepEqual(await generations(), priorGenerations);
    assert.deepEqual(
      await readFile(join(app, "settings.json")),
      oldSettingsBytes,
    );
  } finally {
    await unlock();
  }
  assert.equal(await page.locator(".opened-save").count(), 1);

  // Retry through the real UI, with a complete build/publication of Game B's catalog.
  await page.getByRole("button", { name: "设置", exact: true }).click();
  await page.getByRole("button", { name: "保存设置", exact: true }).click();
  await page
    .locator(".notice.success")
    .filter({ hasText: "设置已保存" })
    .waitFor({ timeout: 90000 });
  const after = await rpc(page, "init");
  assert.equal(resolve(after.settings.game), gameB);
  assert.equal(after.catalog_rebuild_reason, null);
  assert.match(after.settings.catalog_file, /^catalog-.*\.json$/);
  assert.equal(after.catalog_count, 1);
  assert.deepEqual(
    (await rpc(page, "catalog")).map((part) => part.path),
    ["/def/vehicle/truck/synthetic/engine/test.sii"],
  );
  await rejected(page, "preview", { operations: [] }, /先打开存档/);
  await page.getByRole("button", { name: /^车库/ }).click();
  assert.equal(await page.locator(".opened-save").count(), 0);
  assert.equal(await page.locator(".truck-card").count(), 0);
  assert.equal(await readFile(save, "utf8"), original);
  assert.deepEqual(await rpc(page, "history"), []);
  assert.deepEqual(errors, []);
  await page.screenshot({ path: join(root, "switch-clears-session.png") });
  await writeFile(
    join(root, "result.json"),
    JSON.stringify(
      {
        passed: true,
        legacyCatalogReadOnly: true,
        twoOperationsBlockSettingsSetupAndIndex: true,
        publicationFailurePreservesSettingsCatalogAndSession: true,
        actualGameSwitchBuildsCatalogAndClearsSession: true,
        originalSaveUnchanged: true,
        pageErrors: errors,
      },
      null,
      2,
    ),
  );
  console.log(`Review state native passed: ${root}`);
} finally {
  await stop();
  // Screenshots/result remain as evidence; generated game, settings and WebView files are disposable.
  for (const name of ["app", "documents", "Game A", "Game B", "webview"])
    await rm(join(root, name), {
      recursive: true,
      force: true,
      maxRetries: 5,
      retryDelay: 200,
    });
}
