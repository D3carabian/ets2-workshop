// Native Tauri acceptance using synthetic saves only. Keeps reports/screenshots for review.
// Usage: node scripts/garage-usability-smoke.mjs [exe] [--baseline|--measure-only] [--keep-open]
import { chromium } from "playwright";
import {
  bindSyntheticCatalog,
  createSyntheticGame,
} from "./synthetic-catalog.mjs";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { spawn } from "node:child_process";
import { createServer } from "node:net";
import { resolve, join, sep } from "node:path";
import { cpus } from "node:os";
import { setTimeout as delay } from "node:timers/promises";
import assert from "node:assert/strict";

const repo = resolve(import.meta.dirname, "..");
const args = process.argv.slice(2);
const baseline = args.includes("--baseline");
const measureOnly = baseline || args.includes("--measure-only");
const keepOpen = args.includes("--keep-open");
const exe = resolve(
  args.find((a) => !a.startsWith("--")) ||
    join(repo, "src-tauri/target/debug/ets2-workshop.exe"),
);
assert(exe.startsWith(repo + sep), "Executable must be inside this checkout");
const output = join(repo, "verification/garage-usability");
await mkdir(output, { recursive: true });
const root = await mkdtemp(join(output, baseline ? "baseline-" : "run-"));
const app = join(root, "app");
const documents = join(root, "documents");
await mkdir(app);
const installation = await createSyntheticGame(join(root, "installation"));
const partPath = (category, name) =>
  `/def/vehicle/truck/scania.s_2016/${category}/${name}.sii`;
const definitions = {};
function definition(category, name, electric = false) {
  const path = partPath(category, name);
  definitions[path] = {
    path,
    kind: `accessory_${category}_data`,
    unit: `${name}.${category}`,
    name: `Synthetic ${name}`,
    category,
    model: "scania.s_2016",
    source: "synthetic",
    metrics:
      category === "engine"
        ? { type: electric ? "electric" : "diesel", torque: "2000" }
        : {},
    suitable: [],
    conflicts: [],
    requires: [],
  };
}
for (let i = 0; i < 200; i++)
  definition("engine", `diesel_${String(i).padStart(3, "0")}`);
definition("engine", "electric_forbidden", true);
definition("cabin", "highline");
const names = Object.fromEntries(
  Array.from({ length: 5000 }, (_, i) => [
    `driver.${i}`,
    `Synthetic Driver ${String(i).padStart(4, "0")}`,
  ]),
);
const sources = new Map();
for (const count of [1000, 5000]) {
  const slot = join(documents, "profiles/fixture/save", String(count));
  await mkdir(slot, { recursive: true });
  let text = `SiiNunit {\nplayer : p {\n trucks: ${count}\n assigned_truck: t${count - 1}\n`;
  for (let i = 0; i < count; i++) text += ` trucks[${i}]: t${i}\n`;
  text += `}\ngarage : g {\n vehicles: ${count}\n drivers: ${count}\n`;
  for (let i = 0; i < count; i++)
    text += ` vehicles[${i}]: t${i}\n drivers[${i}]: ${i === 1 ? "null" : `driver.${String(i).padStart(4, "0")}`}\n`;
  text += "}\n";
  for (let i = 0; i < count; i++)
    text += `driver_ai : driver.${String(i).padStart(4, "0")} {\n assigned_truck: null\n}\nvehicle : t${i} {\n license_plate: "SYNTH-${i}|germany"\n accessories: 2\n accessories[0]: e${i}\n accessories[1]: c${i}\n}\nvehicle_accessory : e${i} {\n data_path: "${partPath("engine", "diesel_000")}"\n}\nvehicle_accessory : c${i} {\n data_path: "${partPath("cabin", "highline")}"\n}\n`;
  text += "}\n";
  const path = join(slot, "game.sii");
  await writeFile(path, text);
  await writeFile(
    join(slot, "info.sii"),
    `SiiNunit {\nsave_container : info {\n name: "Synthetic fleet ${count}"\n file_time: ${count}\n}\n}\n`,
  );
  sources.set(count, { path, text });
}
await writeFile(
  join(app, "settings.json"),
  JSON.stringify({
    documents,
    game: installation,
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
    driver_names: names,
    driver_names_schema: 1,
  }),
);
await bindSyntheticCatalog(join(app, "catalog.json"), installation);
const server = createServer();
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const port = server.address().port;
await new Promise((r) => server.close(r));
const child = spawn(exe, [], {
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
const report = {
  baseline,
  measureOnly,
  recordedAt: new Date().toISOString(),
  exe,
  root,
  app,
  documents,
  pid: child.pid,
  port,
  cpu: cpus()[0]?.model,
  opened: {},
  checks: [],
  screenshots: [],
  errors: [],
};
const reportPath = join(root, "report.json");
await writeFile(
  join(
    output,
    baseline
      ? "baseline-latest.json"
      : measureOnly
        ? "measure-latest.json"
        : "latest.json",
  ),
  JSON.stringify({ reportPath, root, pid: child.pid, port }, null, 2),
);
let browser;
let succeeded = false;
try {
  for (let i = 0; i < 120 && !browser; i++) {
    try {
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, {
        timeout: 1000,
      });
    } catch {
      await delay(250);
    }
  }
  assert(browser, "Native WebView CDP unavailable");
  let page;
  for (let i = 0; i < 100 && !page; i++) {
    page = browser
      .contexts()
      .flatMap((c) => c.pages())
      .find((p) => p.url().includes("tauri.localhost"));
    if (!page) await delay(100);
  }
  assert(page);
  page.setDefaultTimeout(30000);
  page.on("pageerror", (e) => report.errors.push(String(e)));
  const rpc = (action, extra = {}) =>
    page.evaluate(
      ({ action, extra }) =>
        window.__TAURI_INTERNALS__.invoke("rpc", {
          payload: { action, ...extra },
        }),
      { action, extra },
    );
  assert.equal((await rpc("init")).settings.documents, documents);
  await page.locator(".language-picker select").selectOption("zh-CN");
  await page
    .locator(".savebar select option")
    .filter({ hasText: "Synthetic fleet 5000" })
    .waitFor({ state: "attached" });
  const idle = () => page.locator(".busy").waitFor({ state: "hidden" });
  async function open(count) {
    const option = page
      .locator(".savebar select option")
      .filter({ hasText: `Synthetic fleet ${count}` })
      .first();
    await page
      .locator(".savebar select")
      .selectOption(await option.getAttribute("value"));
    const start = performance.now();
    await page.locator(".savebar > button").first().click();
    await idle();
    await page.locator(".truck-card").first().waitFor();
    return performance.now() - start;
  }
  for (const count of [1000, 5000]) {
    const timings = [];
    for (let i = 0; i < 3; i++) timings.push(await open(count));
    report.opened[count] = {
      samplesMs: timings,
      medianMs: [...timings].sort((a, b) => a - b)[1],
    };
    await writeFile(reportPath, JSON.stringify(report, null, 2));
  }
  if (!measureOnly) {
    await page.evaluate(() => {
      const original = window.fetch.bind(window);
      window.__garageDiscoverCount = 0;
      window.fetch = function (url, options) {
        if (
          String(url) === "http://ipc.localhost/rpc" &&
          options?.body &&
          JSON.parse(options.body)?.payload?.action === "discover"
        )
          window.__garageDiscoverCount++;
        return original(url, options);
      };
    });
    const discoverCount = () =>
      page.evaluate(() => window.__garageDiscoverCount);
    async function checkFilter() {
      const filter = page.locator(".save-filter:visible").first();
      const checkbox = filter.locator("input");
      const before = await checkbox.isChecked();
      const calls = await discoverCount();
      await filter.locator(":scope > span").click();
      const box = await filter.boundingBox();
      await page.mouse.click(box.x + box.width - 3, box.y + box.height / 2);
      assert.equal(await checkbox.isChecked(), before);
      assert.equal(await discoverCount(), calls);
      await filter.locator("label").click();
      await idle();
      assert.equal(await checkbox.isChecked(), !before);
      await checkbox.focus();
      await checkbox.press("Space");
      await idle();
      assert.equal(await checkbox.isChecked(), before);
      assert.equal(await discoverCount(), calls + 2);
    }
    await checkFilter();
    report.checks.push(
      "checkbox helper/blank are inert; label and Space work on garage",
    );
    assert.match(
      await page.locator(".truck-card").nth(0).innerText(),
      /SYNTH-4999/,
    );
    assert.match(
      await page.locator(".truck-card").nth(1).innerText(),
      /SYNTH-1\b/,
    );
    assert.match(
      await page.locator(".truck-card").nth(2).innerText(),
      /Synthetic Driver 0000/,
    );
    const search = page.locator(".fleet .search input");
    await search.fill("Synthetic Driver 0420");
    assert.equal(await page.locator(".truck-card").count(), 1);
    await page.locator(".truck-card").click();
    await page.locator(".part-row").filter({ hasText: "发动机" }).click();
    await page.getByRole("button", { name: "本机配件库", exact: true }).click();
    const candidate = page
      .locator(".candidate")
      .filter({ hasText: "Synthetic diesel_001" });
    await candidate.click();
    await page.locator(".candidate-metrics").waitFor();
    await candidate.click();
    assert.equal(await page.locator(".candidate-metrics").count(), 0);
    assert(await page.locator(".stage-button").isDisabled());
    const candidateSearch = page.locator(".inspector .search input");
    await candidateSearch.fill("electric_forbidden");
    await page.locator(".candidate").click();
    await page.locator(".stage-button").click();
    await page.locator(".notice.error").waitFor();
    assert.equal(await page.locator(".changes").count(), 0);
    await page.locator(".candidate").click();
    assert.equal(await page.locator(".candidate-metrics").count(), 0);
    assert.equal(await page.locator(".notice.error").count(), 0);
    await candidateSearch.fill("");
    report.checks.push(
      "candidate deselection hides comparison; illegal engine staging rejected",
    );
    const session = await page.context().newCDPSession(page);
    for (const locale of ["zh-CN", "en"])
      for (const [width, height] of [
        [1080, 720],
        [1440, 900],
        [1920, 1080],
      ])
        for (const scale of [1, 1.25]) {
          await session.send("Emulation.setDeviceMetricsOverride", {
            width,
            height,
            deviceScaleFactor: scale,
            mobile: false,
          });
          await page.locator(".language-picker select").selectOption(locale);
          await page.locator(".inspector").evaluate((node) => {
            node.scrollTop = node.scrollHeight;
          });
          await delay(80);
          const measurements = await page.evaluate(() => {
            const rect = (s) =>
              document.querySelector(s).getBoundingClientRect();
            const center = (s) => {
              const r = rect(s);
              return r.y + r.height / 2;
            };
            return {
              width: innerWidth,
              height: innerHeight,
              dpr: devicePixelRatio,
              paginationGap:
                rect(".stage-button").top -
                rect(".candidate-pagination").bottom,
              fleetHeadingDelta: Math.abs(
                center(".fleet .section-head h2") -
                  center(".fleet .section-head > span"),
              ),
              searchCenterDelta: Math.abs(
                center(".fleet .search input") - center(".fleet .search svg"),
              ),
              savebarCenterDelta: Math.abs(
                center(".savebar > button") - center(".savebar > details"),
              ),
              headerText: document.querySelector("header").innerText,
              horizontalOverflow:
                document.documentElement.scrollWidth > innerWidth,
            };
          });
          assert.equal(measurements.dpr, scale);
          assert.equal(measurements.width, width);
          assert(
            measurements.paginationGap >= 11.9,
            JSON.stringify(measurements),
          );
          assert(
            measurements.fleetHeadingDelta < 1 &&
              measurements.searchCenterDelta < 1 &&
              measurements.savebarCenterDelta < 1,
            JSON.stringify(measurements),
          );
          assert(
            !measurements.horizontalOverflow,
            JSON.stringify(measurements),
          );
          assert.equal(
            await page
              .locator(
                "header .breadcrumb, header .chip, header .header-backup",
              )
              .count(),
            0,
          );
          assert.equal(await page.locator("header select").count(), 1);
          const screenshot = join(
            root,
            `${locale}-${width}x${height}-${scale}.png`,
          );
          // WebView2's default screenshot can retain CSS dimensions under emulation.
          // Direct CDP capture keeps the emulated DPR instead of normalizing to CSS pixels.
          const capture = await session.send("Page.captureScreenshot", {
            format: "png",
            clip: { x: 0, y: 0, width, height, scale: 1 },
            captureBeyondViewport: false,
          });
          const png = Buffer.from(capture.data, "base64");
          await writeFile(screenshot, png);
          const physicalSize = {
            width: png.readUInt32BE(16),
            height: png.readUInt32BE(20),
          };
          assert.equal(physicalSize.width, Math.round(width * scale));
          assert.equal(physicalSize.height, Math.round(height * scale));
          report.screenshots.push({
            locale,
            width,
            height,
            scale,
            screenshot,
            physicalSize,
            measurements,
          });
        }
    report.checks.push(
      "12 native WebView viewport/DPR screenshots and alignment measurements",
    );
    await session.send("Emulation.setDeviceMetricsOverride", {
      width: 1440,
      height: 900,
      deviceScaleFactor: 1,
      mobile: false,
    });
    await page.locator(".language-picker select").selectOption("zh-CN");
    async function stagePart(index) {
      await page.locator(".part-row").filter({ hasText: "发动机" }).click();
      await page
        .getByRole("button", { name: "本机配件库", exact: true })
        .click();
      await page
        .locator(".candidate")
        .filter({
          hasText: `Synthetic diesel_${String(index).padStart(3, "0")}`,
        })
        .click();
      await page.locator(".stage-button").click();
      await page.locator(".changes .change").waitFor();
    }
    await stagePart(1);
    assert.match(
      await page.locator(".truck-card.selected").innerText(),
      /Synthetic Driver 0420/,
    );
    await page.locator(".change button").click();
    await page.locator(".changes").waitFor({ state: "hidden" });
    assert.match(
      await page.locator(".truck-card.selected").innerText(),
      /Synthetic Driver 0420/,
    );
    await stagePart(1);
    report.checks.push("driver remains after preview and undo");
    await page.getByRole("button", { name: /保存修改/ }).click();
    const firstName = await page.locator(".modal input").inputValue();
    assert.match(
      firstName,
      /^Workshop_\d{4}-\d{2}-\d{2}_\d{2}-\d{2}-\d{2}-\d{3}$/,
    );
    await page.getByRole("button", { name: "确认保存", exact: true }).click();
    await idle();
    await page.locator(".backup-status").waitFor();
    assert.equal(
      await readFile(sources.get(5000).path, "utf8"),
      sources.get(5000).text,
    );
    await stagePart(2);
    await page.getByRole("button", { name: /保存修改/ }).click();
    const secondName = await page.locator(".modal input").inputValue();
    assert.notEqual(firstName, secondName);
    const manualName = "Synthetic manual retry";
    await page.locator(".modal input").fill(manualName);
    await page.locator(".modal select").selectOption("overwrite");
    await page.locator(".modal select").selectOption("new");
    assert.equal(await page.locator(".modal input").inputValue(), manualName);
    const [receipt] = await rpc("history");
    assert(receipt.output.startsWith(root + sep));
    const savedText = await readFile(receipt.output, "utf8");
    await writeFile(receipt.output, savedText + "\n");
    try {
      await page.getByRole("button", { name: "确认保存", exact: true }).click();
      await idle();
      await page.locator(".notice.error").waitFor();
    } finally {
      await writeFile(receipt.output, savedText);
    }
    await page.getByRole("button", { name: /保存修改/ }).click();
    assert.equal(await page.locator(".modal input").inputValue(), manualName);
    await page.getByRole("button", { name: "确认保存", exact: true }).click();
    await idle();
    await page
      .locator(".notice.success")
      .filter({ hasText: manualName })
      .waitFor();
    const history = await rpc("history");
    assert(
      history.length >= 2 &&
        history.every((r) => r.output.startsWith(root + sep)),
    );
    await page.locator(".nav").nth(2).click();
    await page.locator(".history-verification summary").first().click();
    await checkFilter();
    report.checks.push(
      "history checkbox helper/blank are inert; label and Space work",
    );
    await page.locator(".nav").first().click();
    report.checks.push(
      "timestamp names differ; manual name survives mode changes and real source-change failure/retry; synthetic saves only",
    );
    report.saveNames = { firstName, secondName, manualName };
    await open(5000);
  }
  assert.deepEqual(report.errors, []);
  succeeded = true;
  report.status = "passed";
} catch (error) {
  report.status = "failed";
  report.failure = error.stack || String(error);
  throw error;
} finally {
  report.keptOpen = succeeded && keepOpen;
  await writeFile(reportPath, JSON.stringify(report, null, 2));
  console.log(
    JSON.stringify(
      {
        reportPath,
        status: report.status,
        opened: report.opened,
        keptOpen: report.keptOpen,
        pid: child.pid,
        port,
      },
      null,
      2,
    ),
  );
  if (!(succeeded && keepOpen)) {
    child.kill();
    if (browser) await browser.close().catch(() => {});
  } else {
    // Keep this tool session alive so its native child is not reaped on exit.
    await new Promise((done) => child.once("exit", done));
    report.keptOpen = false;
    await writeFile(reportPath, JSON.stringify(report, null, 2));
    await browser.close().catch(() => {});
  }
}
