// Use an empty ETS2_WORKSHOP_DATA_DIR under verification/onboarding/app.
import { chromium } from "playwright";
import { readFile, access, mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";
const root = resolve("verification/onboarding");
const exists = async (p) => {
  try {
    await access(p);
    return true;
  } catch {
    return false;
  }
};
const browser = await chromium.connectOverCDP("http://127.0.0.1:9229");
try {
  const page = browser
    .contexts()
    .flatMap((c) => c.pages())
    .find((p) => p.url().startsWith("http://tauri.localhost"));
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.locator(".onboarding").waitFor();
  await page.locator(".onboarding fieldset:not([disabled])").waitFor();
  const init = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("rpc", { payload: { action: "init" } }),
  );
  assert.equal(resolve(init.data_dir), resolve(root, "app"));
  assert.equal(await exists(resolve(root, "app/settings.json")), false);
  const game = page.locator("input[list=detected-games]");
  const docs = page.locator("input[list=detected-docs]");
  assert(
    (await game.inputValue()).length > 0,
    "Auto-detection must find installed game",
  );
  await docs.fill(resolve(root, "missing"));
  await page.getByRole("button", { name: "确认路径并准备" }).click();
  await page
    .getByRole("alert")
    .filter({ hasText: "请选择 ETS2 用户数据目录" })
    .waitFor();
  assert.equal(await exists(resolve(root, "app/settings.json")), false);
  await docs.fill(resolve(root, "game"));
  await mkdir("verification/screenshots", { recursive: true });
  await page.screenshot({
    path: "verification/screenshots/onboarding-confirm.png",
  });
  await page.getByRole("button", { name: "确认路径并准备" }).click();
  await page
    .locator(".onboarding")
    .waitFor({ state: "detached", timeout: 300000 });
  const settings = JSON.parse(
    await readFile(resolve(root, "app/settings.json"), "utf8"),
  );
  assert.equal(settings.onboarding_version, 1);
  assert.equal(resolve(settings.documents), resolve(root, "game"));
  assert(!("dll" in settings));
  const exe = await readFile(settings.extractor);
  assert.equal(
    createHash("sha256").update(exe).digest("hex"),
    "55bd670691bee62c218220026a0b055bb597eb2c360e33e635c1c4c5c370ea29",
  );
  const catalog = JSON.parse(
    await readFile(resolve(root, "app/catalog.json"), "utf8"),
  );
  assert(Object.keys(catalog.definitions).length > 26000);
  const state = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("rpc", { payload: { action: "init" } }),
  );
  assert.equal(state.needs_setup, false);
  assert.equal(errors.length, 0);
  const report = {
    status: "passed",
    autoDetection: true,
    invalidPathBlocked: true,
    confirmationRequired: true,
    officialDownloadHashVerified: true,
    nativeDecoder: true,
    catalogCount: Object.keys(catalog.definitions).length,
    settingsPersisted: true,
    pageErrors: errors,
  };
  await writeFile(
    "verification/onboarding-smoke-result.json",
    JSON.stringify(report, null, 2),
  );
  console.log(JSON.stringify(report, null, 2));
} finally {
  await browser.close();
}
