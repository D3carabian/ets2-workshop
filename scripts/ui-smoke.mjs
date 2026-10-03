// Exercises the actual packaged Tauri WebView through its test-only CDP port.
// Launch with ETS2_WORKSHOP_DATA_DIR pointing to verification/ui-test/app.
import { chromium } from "playwright";
import { mkdir, writeFile } from "node:fs/promises";
import assert from "node:assert/strict";
import { resolve } from "node:path";
const fixture = resolve(
  process.env.ETS2_UI_FIXTURE || "verification/ui-test",
).replaceAll("\\", "/");
assert(fixture.startsWith(resolve("verification").replaceAll("\\", "/") + "/"));
const browser = await chromium.connectOverCDP("http://127.0.0.1:9229");
try {
  const page = browser
    .contexts()
    .flatMap((c) => c.pages())
    .find((p) => p.url().startsWith("http://tauri.localhost"));
  assert(page, "Tauri page missing");
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.waitForSelector('.savebar select option[value]:not([value=""])', {
    state: "attached",
  });
  const init = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("rpc", { payload: { action: "init" } }),
  );
  assert(
    init.settings.documents.replaceAll("\\", "/") === fixture + "/game",
    "Refuse to test against live saves",
  );
  const sourcePath = await page
    .locator(".savebar select option")
    .evaluateAll(
      (options, prefix) =>
        options.find((o) =>
          o.value
            .replaceAll("\\", "/")
            .startsWith(prefix),
        )?.value,
      fixture + "/game/profiles/fixture/save/1/",
    );
  assert(sourcePath);
  await page.locator(".savebar select").selectOption(sourcePath);
  await page.getByRole("button", { name: "打开存档", exact: true }).click();
  await page.waitForSelector(".truck-card");
  assert.equal(await page.locator(".truck-card").count(), 5);
  await page.locator(".truck-card").filter({ hasText: "Scania S" }).click();
  await page.getByRole("button", { name: "添加附件", exact: true }).click();
  await page.locator(".inspector select").selectOption("engine");
  await page.locator(".candidate").filter({ hasText: "d17a780" }).click();
  await page.getByRole("button", { name: "加入变更清单", exact: true }).click();
  await page.locator(".notice.error").filter({ hasText: "禁止追加" }).waitFor();
  assert.equal(await page.locator(".changes .change").count(), 0);
  await page.locator(".part-row").filter({ hasText: "发动机" }).click();
  await page.locator(".candidate").filter({ hasText: "d17a780" }).click();
  await page.getByRole("button", { name: "加入变更清单", exact: true }).click();
  await page.waitForSelector(".changes .change");
  assert.match(await page.locator(".changes").innerText(), /d17a780/);
  await mkdir("verification/screenshots", { recursive: true });
  await page.screenshot({
    path: "verification/screenshots/garage-preview.png",
    fullPage: true,
  });
  await page.getByRole("button", { name: /保存修改/ }).click();
  await page.locator(".modal input").fill("UI_smoke_780");
  await page.getByRole("button", { name: "确认保存", exact: true }).click();
  await page
    .locator(".notice.success")
    .filter({ hasText: "已保存为 UI_smoke_780" })
    .waitFor({ timeout: 30000 });
  await page.locator(".nav").filter({ hasText: "改装记录" }).click();
  const resultPath = await page
    .locator("select option")
    .evaluateAll(
      (options) =>
        options.find((o) => o.textContent.includes("UI_smoke_780"))?.value,
    );
  assert(resultPath);
  await page.locator("select").selectOption(resultPath);
  await page
    .getByRole("button", { name: "复查所选存档", exact: true })
    .first()
    .click();
  await page
    .getByText("发动机：已保留", { exact: true })
    .waitFor({ timeout: 30000 });
  await page.screenshot({
    path: "verification/screenshots/history-verified.png",
    fullPage: true,
  });
  await page
    .getByRole("button", { name: "恢复修改前", exact: true })
    .first()
    .click();
  await page
    .locator(".modal")
    .getByRole("button", { name: "恢复", exact: true })
    .click();
  await page
    .locator(".notice.success")
    .filter({ hasText: "已恢复修改前" })
    .waitFor({ timeout: 30000 });
  assert.equal(errors.length, 0, errors.join("\n"));
  const report = {
    status: "passed",
    tested: [
      "actual desktop launch",
      "isolated encrypted save decrypt",
      "five owned trucks",
      "full accessory list",
      "duplicate engine rejected in UI",
      "Volvo engine staging",
      "new save commit",
      "saved-result verification",
      "backup restore",
    ],
    resultPath,
    pageErrors: errors,
  };
  await writeFile(
    "verification/ui-smoke-result.json",
    JSON.stringify(report, null, 2),
  );
  console.log(JSON.stringify(report, null, 2));
} finally {
  await browser.close();
}
