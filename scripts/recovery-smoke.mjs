// Synthetic-only desktop test. Build with --features custom-protocol first.
import { chromium } from "playwright";
import { mkdtemp, mkdir, readFile, writeFile, rm } from "node:fs/promises";
import { spawn } from "node:child_process";
import { resolve, join } from "node:path";
import assert from "node:assert/strict";
import { setTimeout as delay } from "node:timers/promises";

const workspace = resolve("verification");
await mkdir(workspace, { recursive: true });
const root = await mkdtemp(join(workspace, "phase3-ui-"));
assert(root.startsWith(workspace + "\\") || root.startsWith(workspace + "/"));
const app = join(root, "app");
const documents = join(root, "game");
const slot = join(documents, "profiles/fixture/save/1");
await mkdir(slot, { recursive: true });
await mkdir(app, { recursive: true });
const count = 5000;
const partPath = (category, name) =>
  `/def/vehicle/truck/scania.s_2016/${category}/${name}.sii`;
const definitions = {};
for (const [category, name] of [
  ["engine", "a"],
  ["engine", "b"],
  ["cabin", "a"],
]) {
  const path = partPath(category, name);
  definitions[path] = {
    path,
    kind: `accessory_${category}_data`,
    unit: `${name}.${category}`,
    name: `synthetic-${category}-${name}`,
    category,
    model: "scania.s_2016",
    source: "synthetic",
    metrics: {},
    suitable: [],
    conflicts: [],
    requires: [],
  };
}
let text = `SiiNunit {\nplayer : p {\n trucks: ${count}\n assigned_truck: t0\n`;
for (let i = 0; i < count; i++) text += ` trucks[${i}]: t${i}\n`;
text += "}\n";
for (let i = 0; i < count; i++)
  text += `vehicle : t${i} {\n license_plate: "SYNTH-${i}|germany"\n accessories: 2\n accessories[0]: e${i}\n accessories[1]: c${i}\n}\nvehicle_accessory : e${i} {\n data_path: "${partPath("engine", "a")}"\n}\nvehicle_accessory : c${i} {\n data_path: "${partPath("cabin", "a")}"\n}\n`;
text += "}\n";
await writeFile(join(slot, "game.sii"), text);
await writeFile(
  join(slot, "info.sii"),
  'SiiNunit {\nsave_container : info {\n name: "Synthetic fleet"\n file_time: 1\n}\n}\n',
);
await writeFile(
  join(app, "settings.json"),
  JSON.stringify({
    documents,
    game: root,
    extractor: "",
    onboarding_version: 1,
  }),
);
await writeFile(
  join(app, "catalog.json"),
  JSON.stringify({
    definitions,
    warnings: [],
    signature: "synthetic",
    archives: [],
  }),
);
const port = 9238;
const proc = spawn(resolve("src-tauri/target/debug/ets2-workshop.exe"), [], {
  windowsHide: true,
  stdio: "ignore",
  env: {
    ...process.env,
    ETS2_WORKSHOP_DATA_DIR: app,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
  },
});
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
  assert(browser, "Native WebView CDP unavailable");
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
  page.setDefaultTimeout(30000);
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  const rpc = (action, extra = {}) =>
    page.evaluate(
      ({ action, extra }) =>
        window.__TAURI_INTERNALS__.invoke("rpc", {
          payload: { action, ...extra },
        }),
      { action, extra },
    );
  assert.equal((await rpc("init")).settings.documents, documents);
  await page
    .locator(".savebar select option")
    .filter({ hasText: "Synthetic fleet" })
    .waitFor({ state: "attached" });
  const source = await page
    .locator(".savebar select option")
    .filter({ hasText: "Synthetic fleet" })
    .getAttribute("value");
  assert(
    source.replaceAll("\\", "/").startsWith(root.replaceAll("\\", "/") + "/"),
  );
  await page.locator(".savebar select").selectOption(source);
  const started = performance.now();
  await page.getByRole("button", { name: "打开存档", exact: true }).click();
  await page.locator(".truck-card").first().waitFor();
  const openMs = performance.now() - started;
  assert.equal(await page.locator(".truck-card").count(), 50);
  await page.getByRole("button", { name: "下一页", exact: true }).click();
  await page
    .locator(".truck-card")
    .first()
    .filter({ hasText: "SYNTH-50" })
    .waitFor();
  await page.getByRole("textbox", { name: "搜索卡车" }).fill("SYNTH-4999");
  await page.locator(".truck-card").filter({ hasText: "SYNTH-4999" }).click();
  assert.equal(await page.locator(".truck-card").count(), 1);
  await page.locator(".part-row").filter({ hasText: "发动机" }).click();
  await page.getByRole("button", { name: "本机配件库", exact: true }).click();
  await page
    .locator(".candidate")
    .filter({ hasText: "synthetic-engine-b" })
    .click();
  await page.getByRole("button", { name: "加入变更清单", exact: true }).click();
  await page.locator(".changes .change").waitFor();
  await page.getByRole("button", { name: /保存修改/ }).click();
  await page.locator(".modal input").fill("Phase3 合成另存");
  await page.getByRole("button", { name: "确认保存", exact: true }).click();
  await page
    .locator(".notice.success")
    .filter({ hasText: "已保存为 Phase3 合成另存" })
    .waitFor();
  assert.match(
    await page.locator(".savebar select option:checked").textContent(),
    /Phase3 合成另存/,
  );
  assert.equal(await readFile(join(slot, "game.sii"), "utf8"), text);
  const [r] = await rpc("history");
  assert(r.output.startsWith(root));
  const savedInfo = await readFile(join(r.output, "../info.sii"), "utf8");
  // Recreate the durable state visible after a final-receipt write failure or crash.
  await writeFile(
    join(app, "history", r.id, "receipt.json"),
    JSON.stringify({ ...r, state: "prepared" }),
  );
  await page.locator(".nav").filter({ hasText: "改装记录" }).click();
  await page.getByRole("button", { name: "刷新", exact: true }).click();
  await page.getByText(/写入未确认或已中断/).waitFor();
  await page.getByRole("button", { name: "清理临时文件", exact: true }).click();
  await page.locator(".notice.success").filter({ hasText: "已清理" }).waitFor();
  await page.getByRole("button", { name: "恢复修改前", exact: true }).click();
  assert.match(await page.locator(".modal").innerText(), /不删除存档槽/);
  await page
    .locator(".modal")
    .getByRole("button", { name: "恢复", exact: true })
    .click();
  await page
    .locator(".notice.success")
    .filter({ hasText: "已恢复修改前" })
    .waitFor();
  assert.equal(await readFile(r.output, "utf8"), text);
  assert.equal(
    await readFile(join(r.output, "../info.sii"), "utf8"),
    savedInfo,
  );
  assert.match(
    await page.locator(".savebar select option:checked").textContent(),
    /Phase3 合成另存/,
  );
  assert.equal(await page.locator(".truck-card").count(), 50);
  assert.deepEqual(errors, []);
  console.log(
    JSON.stringify({
      trucks: count,
      renderedCards: 50,
      openMs: Math.round(openMs),
      saveRestore: "passed",
      pendingHistory: "passed",
      pathSelection: "passed",
      pageErrors: errors,
    }),
  );
} finally {
  if (browser) await browser.close();
  proc.kill();
  await new Promise((resolve) => {
    if (proc.exitCode !== null) resolve();
    else {
      proc.once("exit", resolve);
      setTimeout(resolve, 3000);
    }
  });
  await rm(root, {
    recursive: true,
    force: true,
    maxRetries: 10,
    retryDelay: 300,
  });
}
