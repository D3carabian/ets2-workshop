// Run garage-ui-fixture.mjs, Vite, then the baseline debug Tauri executable with
// ETS2_WORKSHOP_DATA_DIR=verification/garage-ui/app and WebView2 CDP port 9231.
import { chromium } from "playwright";
import assert from "node:assert/strict";
import { resolve } from "node:path";
import { mkdir, writeFile, readFile } from "node:fs/promises";
const root = resolve("verification/garage-ui").replaceAll("\\", "/");
const browser = await chromium.connectOverCDP("http://127.0.0.1:9231");
try {
  const page = browser
    .contexts()
    .flatMap((c) => c.pages())
    .find((p) => p.url().includes("localhost:1420"));
  assert(
    page,
    "The baseline Tauri debug app must be using this worktree's Vite server",
  );
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  const init = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("rpc", { payload: { action: "init" } }),
  );
  assert.equal(init.settings.documents.replaceAll("\\", "/"), `${root}/game`);
  assert.equal(init.data_dir.replaceAll("\\", "/"), `${root}/app`);
  const source = `${root}/game/profiles/fixture/save/1/game.sii`;
  const original = await readFile(source, "utf8");
  const option = await page
    .locator(".savebar select option")
    .evaluateAll(
      (options, source) =>
        options.find((o) => o.value.replaceAll("\\", "/") === source)?.value,
      source,
    );
  assert(option);
  await page.locator(".savebar select").selectOption(option);
  await page.getByRole("button", { name: "打开存档", exact: true }).click();
  await page.waitForSelector(".truck-card");
  assert.equal(await page.locator(".truck-card").count(), 80);
  const measurements = [];
  for (const viewport of [
    { width: 1440, height: 940 },
    { width: 1080, height: 720 },
  ]) {
    await page.setViewportSize(viewport);
    await page
      .locator(".fleet")
      .evaluate((el) => (el.scrollTop = el.scrollHeight));
    const before = await page.locator(".inspector").boundingBox();
    const windowBefore = await page.evaluate(() => window.scrollY);
    await page.locator(".truck-card").filter({ hasText: "TEST 079" }).click();
    const after = await page.locator(".inspector").boundingBox();
    assert.equal(
      after.y,
      before.y,
      "Selecting a low vehicle must not move the right panel",
    );
    assert.equal(await page.evaluate(() => window.scrollY), windowBefore);
    assert.ok(
      await page.locator(".fleet").evaluate((el) => el.scrollTop > 1000),
    );
    assert.equal(
      await page.locator(".inspector").evaluate((el) => el.scrollTop),
      0,
    );
    await page.getByLabel("配件类别", { exact: true }).selectOption("engine");
    assert.equal(await page.locator(".part-row").count(), 1);
    await page.locator(".part-row").click();
    await page.getByRole("button", { name: "本机配件库", exact: true }).click();
    await page.getByLabel("搜索候选配件").fill("428");
    assert.equal(await page.locator(".candidate").count(), 1);
    const text = await page.locator(".candidate").innerText();
    assert.match(text, /DAF.*PACCAR MX-13/);
    assert.match(text, /428 hp.*2150 Nm/);
    assert.doesNotMatch(text, /dlc_.*\.scs|@@/);
    await page.locator(".candidate").click();
    assert.match(
      await page.locator(".comparison").innerText(),
      /游戏标称功率.*\n428 hp\n428 hp/,
    );
    await page.locator(".inspector").evaluate((el) => (el.scrollTop = 0));
    await mkdir(`${root}/screenshots`, { recursive: true });
    await page.screenshot({
      path: `${root}/screenshots/garage-${viewport.width}.png`,
      fullPage: false,
    });
    measurements.push({
      viewport,
      inspectorTop: after.y,
      fleetScroll: await page.locator(".fleet").evaluate((el) => el.scrollTop),
    });
    await page.getByLabel("配件类别", { exact: true }).selectOption("chassis");
    await page.locator(".part-row").click();
    await page.getByLabel("搜索候选配件").fill("");
    assert.equal(
      await page.locator(".candidate").count(),
      1,
      "Trailer chassis should not be offered",
    );
    assert.doesNotMatch(await page.locator(".candidate").innerText(), /ch 3/);
    await page
      .getByLabel("配件类别", { exact: true })
      .selectOption("interior_accessories");
    assert.equal(await page.locator(".part-row").count(), 1);
    assert.match(await page.locator(".part-row").innerText(), /悬挂饰品/);
    await page.getByLabel("配件类别", { exact: true }).selectOption("other");
    assert.equal(await page.locator(".part-row").count(), 1);
    assert.match(await page.locator(".part-row").innerText(), /车辆基础/);
    assert.doesNotMatch(
      await page.locator(".part-row").innerText(),
      /\bdata\b/,
    );
    await page.getByLabel("配件类别", { exact: true }).selectOption("all");
    // Switch to a different truck so the next iteration also exercises a real selection change.
    await page.locator(".truck-card").filter({ hasText: "TEST 078" }).click();
  }
  await page.getByLabel("配件类别", { exact: true }).selectOption("engine");
  await page.locator(".part-row").click();
  await page.getByLabel("搜索候选配件").fill("483");
  await page.locator(".candidate").click();
  await page.getByRole("button", { name: "加入变更清单", exact: true }).click();
  await page.waitForSelector(".changes .change");
  assert.equal(await page.locator(".changes .change").count(), 1);
  await page.locator(".truck-card").filter({ hasText: "TEST 079" }).click();
  await page.getByLabel("配件类别", { exact: true }).selectOption("interior");
  assert.equal(
    await page.locator(".changes .change").count(),
    1,
    "Changing truck/category must preserve staged edits",
  );
  assert.equal(
    await readFile(source, "utf8"),
    original,
    "Staging must not write the fixture save",
  );
  assert.deepEqual(errors, []);
  await writeFile(
    `${root}/result.json`,
    JSON.stringify({ passed: true, measurements, errors }, null, 2),
  );
  console.log(JSON.stringify({ passed: true, measurements, errors }));
} finally {
  await browser.close();
}
