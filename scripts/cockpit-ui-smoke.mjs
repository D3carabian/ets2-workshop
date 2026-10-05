import assert from "node:assert/strict";
import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";
const browser = await chromium.launch({ channel: "msedge", headless: true });
try {
  const page = await browser.newPage({
    viewport: { width: 1080, height: 720 },
    colorScheme: "light",
  });
  const errors = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  await page.goto(
    "http://127.0.0.1:1420/scripts/fixtures/cockpit-preview.html",
  );
  await page.getByRole("button", { name: "打开存档", exact: true }).click();
  await page.locator(".part-row").filter({ hasText: "发动机" }).click();
  await page.getByRole("button", { name: "本机配件库", exact: true }).click();
  const before = await page.locator(".garage-grid").boundingBox();
  const candidates = page.locator(".candidate");
  await candidates.first().click();
  const nextName = await candidates.nth(1).locator("strong").innerText();
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await page.locator(".notice.success").waitFor();
  assert.equal(
    await page.locator(".changes").count(),
    0,
    "No docked change tray",
  );
  const after = await page.locator(".garage-grid").boundingBox();
  assert(
    after.height >= before.height - 45,
    "Only the success notice may use vertical space",
  );
  const save = page.locator(".titlebar-save");
  await save.hover();
  await page.locator(".save-peek").waitFor({ state: "visible" });
  assert((await page.locator(".save-peek").innerText()).includes(nextName));
  await save.click();
  await page.locator(".change-review").waitFor();
  assert.equal(await page.locator(".changes .change").count(), 1);
  assert((await page.locator(".changes").innerText()).includes(nextName));
  assert(await save.isDisabled());
  await page.locator(".modal input").fill("Preserved name");
  await page.keyboard.press("Escape");
  await page.locator(".change-review").waitFor({ state: "hidden" });
  await page.getByRole("button", { name: "关闭窗口", exact: true }).click();
  await page.locator(".exit-dialog").waitFor();
  await page.keyboard.press("Escape");
  await page.locator(".change-review").waitFor();
  assert.equal(
    await page.locator(".modal input").inputValue(),
    "Preserved name",
  );
  await page.getByRole("button", { name: "继续改装", exact: true }).click();
  await page.getByRole("button", { name: "关闭窗口", exact: true }).click();
  await page
    .locator(".exit-dialog")
    .getByRole("button", { name: "关闭", exact: true })
    .click();
  await page.locator(".change-review").waitFor();
  await page.getByRole("button", { name: "继续改装", exact: true }).click();
  console.log(
    "PASS: keyboard stage, no docked tray, animated hover preview, review, exit X/Escape return to review",
  );
  await page.getByRole("button", { name: "设置", exact: true }).click();
  const cards = await page
    .locator(".settings-grid > .card")
    .evaluateAll((els) =>
      els.map((el) => {
        const r = el.getBoundingClientRect();
        return { top: r.top, bottom: r.bottom };
      }),
    );
  assert(
    Math.abs(cards[0].top - cards[1].top) < 1 &&
      Math.abs(cards[0].bottom - cards[1].bottom) < 1,
  );
  await page.getByRole("button", { name: "跟随系统", exact: true }).click();
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForFunction(
    () => document.documentElement.dataset.theme === "dark",
  );
  await page.emulateMedia({ colorScheme: "light" });
  await page.waitForFunction(
    () => document.documentElement.dataset.theme === "light",
  );
  await page.getByRole("button", { name: "夜间模式", exact: true }).click();
  await page.emulateMedia({ colorScheme: "dark" });
  await page.emulateMedia({ colorScheme: "light" });
  assert.equal(await page.locator("html").getAttribute("data-theme"), "dark");
  await page.getByRole("button", { name: "跟随系统", exact: true }).click();
  assert.equal(await page.locator("html").getAttribute("data-theme"), "light");
  assert.equal(
    await page.locator("h1").evaluate((el) => getComputedStyle(el).userSelect),
    "none",
  );
  assert.equal(
    await page
      .locator(".settings-grid input")
      .first()
      .evaluate((el) => getComputedStyle(el).userSelect),
    "text",
  );
  console.log(
    "PASS: settings alignment, system theme changes, manual override, static text not selectable, inputs editable",
  );
  await mkdir("verification/cockpit-review", { recursive: true });
  for (const [theme, themeName] of [
    ["light", "日间模式"],
    ["dark", "夜间模式"],
  ]) {
    await page.getByRole("button", { name: themeName, exact: true }).click();
    await page.screenshot({
      path: `verification/cockpit-review/${theme}-settings.png`,
    });
    await page.getByRole("button", { name: "车库", exact: true }).click();
    await page.locator(".inspector").evaluate((el) => {
      el.scrollTop = 0;
    });
    await page.screenshot({
      path: `verification/cockpit-review/${theme}-garage.png`,
    });
    await page.locator(".titlebar-save").click();
    for (const size of [
      { width: 1080, height: 720 },
      { width: 1440, height: 940 },
    ]) {
      await page.setViewportSize(size);
      assert.equal(
        await page.evaluate(
          () => document.documentElement.scrollWidth > innerWidth,
        ),
        false,
      );
      assert(
        (await page.locator(".change-review").boundingBox()).width >= 900,
        "Review must use the broad separate workspace",
      );
      const box = await page
        .getByRole("button", { name: /确认保存/ })
        .boundingBox();
      assert(box.y + box.height <= size.height);
      await page.screenshot({
        path: `verification/cockpit-review/${theme}-${size.width}.png`,
      });
    }
    await page.getByRole("button", { name: "继续改装", exact: true }).click();
    await page.getByRole("button", { name: "设置", exact: true }).click();
  }
  await page.getByRole("button", { name: "跟随系统", exact: true }).click();
  await page.reload();
  assert.equal(
    await page.evaluate(() => localStorage.getItem("ets2-workshop.theme")),
    "system",
  );
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForFunction(
    () => document.documentElement.dataset.theme === "dark",
  );
  await page.getByLabel("语言 / Language", { exact: true }).selectOption("en");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  assert.equal(
    await page
      .getByRole("button", { name: "Follow system", exact: true })
      .getAttribute("aria-pressed"),
    "true",
  );
  await page.addInitScript(() => {
    Storage.prototype.getItem = () => {
      throw Error("blocked");
    };
    Storage.prototype.setItem = () => {
      throw Error("blocked");
    };
  });
  await page.reload();
  await page.getByRole("button", { name: "设置", exact: true }).click();
  await page.getByRole("button", { name: "日间模式", exact: true }).click();
  assert.equal(await page.locator("html").getAttribute("data-theme"), "light");
  assert.deepEqual(errors, []);
  console.log(
    "PASS: themes/sizes, persisted system setting, English, unavailable storage; no page errors",
  );
} finally {
  await browser.close();
}
