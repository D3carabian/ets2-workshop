// Actual Tauri WebView + native setup; only discovery responses are synthetic.
// Backend discovery has its own host-independent Rust matrix in tests/discovery.rs.
// Run after: cargo build --locked --features custom-protocol --manifest-path src-tauri/Cargo.toml
import { chromium } from "playwright";
import {
  readFile,
  access,
  mkdir,
  writeFile,
  rm,
  mkdtemp,
  readdir,
} from "node:fs/promises";
import { resolve, join, sep } from "node:path";
import { createHash } from "node:crypto";
import { spawn } from "node:child_process";
import { createServer } from "node:net";
import assert from "node:assert/strict";
const repo = resolve(import.meta.dirname, "..");
const exe = resolve(
  process.argv[2] || join(repo, "src-tauri/target/debug/ets2-workshop.exe"),
);
assert(
  exe.startsWith(repo + sep),
  "Use a freshly built executable inside this checkout",
);
const exists = async (p) => {
  try {
    await access(p);
    return true;
  } catch {
    return false;
  }
};
const sha = (b) => createHash("sha256").update(b).digest("hex");
const verification = join(repo, "verification");
await mkdir(verification, { recursive: true });
const root = await mkdtemp(join(verification, "onboarding-synthetic-"));
const app = join(root, "应用 Data");
const game = join(root, "中文 Game");
const docs = join(root, "用户 Documents");
const secondGame = join(root, "Second Game");
const secondDocs = join(
  root,
  "OneDrive",
  "Documents",
  "Euro Truck Simulator 2",
);
await mkdir(join(game, "bin/win_x64"), { recursive: true });
await mkdir(join(docs, "profiles"), { recursive: true });
await mkdir(join(secondDocs, "steam_profiles"), { recursive: true });
await mkdir(secondGame, { recursive: true });
await writeFile(
  join(game, "bin/win_x64/eurotrucks2.exe"),
  "synthetic placeholder; never executed",
);
const archive = Buffer.from(
  await readFile(join(repo, "scripts/fixtures/onboarding.scs.base64"), "utf8"),
  "base64",
);
assert.equal(
  sha(archive),
  "f7160d049b6104616373346bfd9fbf01d78a9f2a43985ef9a78521061e7ed9a8",
);
await writeFile(join(game, "def.scs"), archive);
await mkdir(app);
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
    await Promise.race([exited, new Promise((r) => setTimeout(r, 5000))]);
  }
  if (browser) {
    await Promise.race([
      browser.close(),
      new Promise((r) => setTimeout(r, 5000)),
    ]);
    browser = undefined;
  }
}
async function start() {
  child = spawn(exe, [], {
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
  child.once("error", (e) => {
    launchError = e;
  });
  const deadline = Date.now() + 30000;
  while (!browser && Date.now() < deadline) {
    if (launchError) throw launchError;
    if (child.exitCode !== null)
      throw new Error(`App exited: ${child.exitCode}`);
    try {
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, {
        timeout: 1000,
      });
    } catch {
      await new Promise((r) => setTimeout(r, 200));
    }
  }
  assert(browser, "Desktop WebView CDP endpoint unavailable");
  const context = browser.contexts()[0];
  // The hook persists across reloads. Native setup/download/validation/storage are NOT mocked.
  await context.addInitScript(
    ({ games, documents }) => {
      const original = window.fetch.bind(window);
      window.fetch = (url, options) => {
        if (String(url) === "http://ipc.localhost/rpc" && options?.body) {
          const args = JSON.parse(options.body);
          const action = args?.payload?.action;
          if (action === "detect" || action === "discover") {
            const value =
              action === "detect"
                ? {
                    games,
                    documents,
                    steam_roots: [],
                    notes: ["合成环境：请选择目录。"],
                  }
                : { saves: [], warnings: [] };
            return Promise.resolve(
              new Response(JSON.stringify(value), {
                headers: {
                  "Content-Type": "application/json",
                  "Tauri-Response": "ok",
                },
              }),
            );
          }
        }
        return original(url, options);
      };
    },
    { games: [game, secondGame], documents: [docs, secondDocs] },
  );
  let page;
  while (!page && Date.now() < deadline) {
    page = context
      .pages()
      .find((p) => p.url().startsWith("http://tauri.localhost"));
    if (!page) await new Promise((r) => setTimeout(r, 100));
  }
  assert(page, "Built-in frontend missing; build with custom-protocol");
  page.setDefaultTimeout(30000);
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.reload();
  return page;
}
const rpc = (page, action) =>
  page.evaluate(
    (action) =>
      window.__TAURI_INTERNALS__.invoke("rpc", { payload: { action } }),
    action,
  );
try {
  const page = await start();
  await page.locator(".onboarding fieldset:not([disabled])").waitFor();
  assert.equal(resolve((await rpc(page, "init")).data_dir), app);
  const gameInput = page.locator("input[list=detected-games]");
  const docsInput = page.locator("input[list=detected-docs]");
  assert.equal(await gameInput.inputValue(), "");
  assert.equal(await docsInput.inputValue(), "");
  assert(
    await page.getByRole("button", { name: "确认路径并准备" }).isDisabled(),
  );
  assert.equal(await exists(join(app, "settings.json")), false);
  await page.getByLabel("选择游戏安装位置", { exact: true }).selectOption(game);
  await page.getByLabel("选择用户数据位置", { exact: true }).selectOption(docs);
  await docsInput.fill(join(root, "missing"));
  await page.getByRole("button", { name: "重新检测" }).click();
  await page.locator(".onboarding fieldset:not([disabled])").waitFor();
  assert.equal(await docsInput.inputValue(), join(root, "missing"));
  await page.getByRole("button", { name: "确认路径并准备" }).click();
  await page
    .getByRole("alert")
    .filter({ hasText: "请选择 ETS2 用户数据目录" })
    .waitFor();
  assert.equal(await exists(join(app, "settings.json")), false);
  await docsInput.fill(docs);
  // A deliberately unsupported synthetic header exercises the official fallback.
  // Restore the valid v2 archive before testing successful native setup.
  const unsupported = Buffer.from(archive);
  unsupported.write("BAD!", 0, "ascii");
  await writeFile(join(game, "def.scs"), unsupported);
  const target = join(app, "tools/scs-extractor-1.55/scs_extractor.exe");
  await mkdir(target, { recursive: true });
  await page.getByRole("button", { name: "确认路径并准备" }).click();
  await page
    .getByRole("alert")
    .filter({ hasText: "无法保存解包工具" })
    .waitFor({ timeout: 90000 });
  assert.equal(await exists(join(app, "settings.json")), false);
  assert.deepEqual(
    (await readdir(join(app, "tools/scs-extractor-1.55"))).sort(),
    ["scs_extractor.exe"],
  );
  await rm(target, { recursive: true });
  await page.getByRole("button", { name: "确认路径并准备" }).click();
  await page
    .getByRole("alert")
    .filter({ hasText: "配件目录准备失败" })
    .waitFor({ timeout: 90000 });
  assert.equal(await exists(join(app, "settings.json")), false);
  assert.equal(
    sha(await readFile(target)),
    "55bd670691bee62c218220026a0b055bb597eb2c360e33e635c1c4c5c370ea29",
  );
  await writeFile(join(game, "def.scs"), archive);
  await mkdir(join(app, "settings.json"));
  await page.getByRole("button", { name: "确认路径并准备" }).click();
  await page
    .getByRole("alert")
    .filter({ hasText: "配置未完成" })
    .waitFor({ timeout: 90000 });
  assert.equal((await rpc(page, "init")).needs_setup, true);
  assert.equal(
    (await readdir(app)).filter((n) => n.startsWith("catalog-")).length,
    0,
  );
  await page.screenshot({ path: join(root, "retry.png") });
  await rm(join(app, "settings.json"), { recursive: true });
  await page.getByRole("button", { name: "确认路径并准备" }).click();
  await page
    .locator(".onboarding")
    .waitFor({ state: "detached", timeout: 90000 });
  const saved = await readFile(join(app, "settings.json"));
  const settings = JSON.parse(saved);
  assert.equal(settings.onboarding_version, 1);
  assert.equal(resolve(settings.documents), docs);
  assert.equal(
    sha(await readFile(settings.extractor)),
    "55bd670691bee62c218220026a0b055bb597eb2c360e33e635c1c4c5c370ea29",
  );
  const catalog = JSON.parse(
    await readFile(join(app, settings.catalog_file), "utf8"),
  );
  assert.deepEqual(Object.keys(catalog.definitions), [
    "/def/vehicle/truck/synthetic/engine/test.sii",
  ]);
  assert.equal((await rpc(page, "init")).needs_setup, false);
  assert.equal((await rpc(page, "init")).catalog_rebuild_reason, null);
  await page.screenshot({ path: join(root, "complete.png") });
  await stop();
  const restarted = await start();
  await restarted
    .getByRole("heading", { name: "每辆卡车，都有自己的配置。" })
    .waitFor();
  assert.equal((await rpc(restarted, "init")).needs_setup, false);
  assert.equal(await restarted.locator(".onboarding").count(), 0);
  assert.deepEqual(await readFile(join(app, "settings.json")), saved);
  assert.deepEqual(errors, []);
  const report = {
    status: "passed",
    syntheticAssets: true,
    explicitCandidates: true,
    manualPathPreserved: true,
    confirmationRequired: true,
    fallbackDownloadAndHashVerified: true,
    unsupportedArchiveRejectedWithoutPublication: true,
    failedCacheAndSettingsRetry: true,
    selectiveV2Read: true,
    catalogCount: 1,
    restartPersisted: true,
    pageErrors: errors,
  };
  await writeFile(join(root, "result.json"), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report, null, 2));
} finally {
  await stop();
  // All generated fixtures and browser data stay under the unique verification directory.
  for (const name of [
    "应用 Data",
    "中文 Game",
    "用户 Documents",
    "Second Game",
    "OneDrive",
    "webview",
  ])
    await rm(join(root, name), {
      recursive: true,
      force: true,
      maxRetries: 5,
      retryDelay: 200,
    });
}
