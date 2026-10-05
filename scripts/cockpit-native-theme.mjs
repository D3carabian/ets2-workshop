// Connect to an isolated, already launched native garage-ui fixture. No OS input.
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const browser = await chromium.connectOverCDP("http://127.0.0.1:9231");
try {
  let page;
  for (let attempt = 0; attempt < 100 && !page; attempt++) {
    page = browser
      .contexts()
      .flatMap((context) => context.pages())
      .find((p) => p.url().includes("tauri.localhost"));
    if (!page) await new Promise((resolve) => setTimeout(resolve, 100));
  }
  assert(page);
  const root = resolve("verification/garage-ui").replaceAll("\\", "/");
  const status = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("rpc", { payload: { action: "init" } }),
  );
  assert.equal(status.data_dir.replaceAll("\\", "/"), `${root}/app`);
  assert.equal(status.settings.documents.replaceAll("\\", "/"), `${root}/game`);
  const source = `${root}/game/profiles/fixture/save/1/game.sii`;
  const original = await readFile(source);
  const errors = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  await page.evaluate(() =>
    localStorage.setItem("ets2-workshop.language", "zh-CN"),
  );
  await page.reload();
  await page.getByRole("button", { name: "打开存档", exact: true }).click();
  await page.getByLabel("配件类别", { exact: true }).selectOption("engine");
  await page.locator(".part-row").click();
  await page.getByRole("button", { name: "本机配件库", exact: true }).click();
  await page.getByLabel("搜索候选配件").fill("483");
  await page.locator(".candidate").click();
  await page.getByRole("button", { name: "加入变更清单", exact: true }).click();
  await page.locator(".notice.success").waitFor();
  assert.equal(await page.locator(".changes").count(), 0);
  await page.locator(".titlebar-save").click();
  await page.locator(".changes .change").waitFor();
  const pending = await page.locator(".changes .change").innerText();
  await page.getByRole("button", { name: "继续改装", exact: true }).click();
  await mkdir("verification/cockpit-native", { recursive: true });
  for (const [mode, name] of [
    ["light", "日间模式"],
    ["dark", "夜间模式"],
  ]) {
    await page.getByRole("button", { name: "设置", exact: true }).click();
    await page.getByRole("button", { name, exact: true }).click();
    assert.equal(await page.locator("html").getAttribute("data-theme"), mode);
    await page.screenshot({
      path: `verification/cockpit-native/${mode}-settings.png`,
    });
    await page.getByRole("button", { name: "车库", exact: true }).click();
    await page.locator(".titlebar-save").click();
    assert.equal(await page.locator(".changes .change").innerText(), pending);
    for (const viewport of [
      { width: 1080, height: 720 },
      { width: 1440, height: 940 },
    ]) {
      await page.setViewportSize(viewport);
      await page.evaluate(() => document.fonts.ready);
      assert.equal(
        await page.evaluate(
          () => document.documentElement.scrollWidth > innerWidth,
        ),
        false,
      );
      assert.equal((await page.locator(".titlebar").boundingBox()).height, 40);
      assert(
        await page.evaluate(() =>
          document.fonts.check('600 30px "Barlow Condensed"'),
        ),
      );
      await page.screenshot({
        path: `verification/cockpit-native/${mode}-${viewport.width}.png`,
      });
    }
    await page.getByRole("button", { name: "继续改装", exact: true }).click();
  }
  // Invoke the native close command: the OS close-request event must be intercepted.
  await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("plugin:window|close", { label: "main" }),
  );
  await page.locator(".exit-dialog").waitFor();
  await page.keyboard.press("Escape");
  await page.locator(".change-review").waitFor();
  assert.equal(await page.locator(".changes .change").innerText(), pending);
  await page.getByRole("button", { name: "继续改装", exact: true }).click();
  await page.reload();
  await page.locator(".titlebar").waitFor();
  assert.equal(await page.locator("html").getAttribute("data-theme"), "dark");
  assert.deepEqual(await readFile(source), original);
  assert.deepEqual(errors, []);
  const report = {
    passed: true,
    themes: ["light", "dark"],
    dimensions: ["1080x720", "1440x940"],
    stagedStatePreserved: true,
    sourceUnchanged: true,
    fontsLoadedUnderCsp: true,
    nativeCloseRequestIntercepted: true,
    nativeWindowGestures: "deferred: user is using the desktop",
    errors,
  };
  await writeFile(
    "verification/cockpit-native/report.json",
    JSON.stringify(report, null, 2),
  );
  console.log(JSON.stringify(report));
} finally {
  await browser.close();
}
