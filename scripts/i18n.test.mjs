import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(
  new URL("../src/i18n-core.ts", import.meta.url),
  "utf8",
);
const js = ts.transpileModule(source, {
  compilerOptions: {
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ESNext,
  },
}).outputText;
const { translate, storedLocale, LANGUAGE_STORAGE_KEY } = await import(
  `data:text/javascript;base64,${Buffer.from(js).toString("base64")}`
);

test("language storage tolerates first run, invalid preference and blocked storage", () => {
  assert.equal(storedLocale({ getItem: () => null }), "zh-CN");
  assert.equal(storedLocale({ getItem: () => "de" }), "zh-CN");
  assert.equal(
    storedLocale({
      getItem: () => {
        throw new Error("blocked");
      },
    }),
    "zh-CN",
  );
  assert.equal(
    storedLocale({
      getItem: (key) => {
        assert.equal(key, LANGUAGE_STORAGE_KEY);
        return "en";
      },
    }),
    "en",
  );
});

test("translation preserves unknown source text and user values without interpreting them", () => {
  assert.equal(
    translate("en", "未知游戏名称 @@example@@"),
    "未知游戏名称 @@example@@",
  );
  assert.equal(
    translate("en", "已保存为 {name}。请在游戏中手动加载；备份已保留。", {
      name: "测试{name}",
    }),
    "Saved as 测试{name}. Load it manually in the game; the backup has been kept.",
  );
  assert.equal(
    translate(
      "zh-CN",
      "已索引 {count} 个配件定义。目录仅涵盖已识别的官方资源。",
      { count: 0 },
    ),
    "已索引 0 个配件定义。目录仅涵盖已识别的官方资源。",
  );
});

test("explicit UI translation keys have English text", async () => {
  for (const filename of ["App.tsx", "Onboarding.tsx", "TitleBar.tsx", "ChangeReview.tsx"]) {
    const contents = await readFile(
      new URL(`../src/${filename}`, import.meta.url),
      "utf8",
    );
    const tree = ts.createSourceFile(
      filename,
      contents,
      ts.ScriptTarget.Latest,
      true,
      ts.ScriptKind.TSX,
    );
    function visit(node) {
      if (
        ts.isCallExpression(node) &&
        ["tr", "t"].includes(node.expression.getText(tree)) &&
        node.arguments[0] &&
        ts.isStringLiteral(node.arguments[0])
      ) {
        const text = node.arguments[0].text;
        if (/[\u4e00-\u9fff]/.test(text))
          assert.notEqual(
            translate("en", text),
            text,
            `${filename}: missing English text for ${text}`,
          );
      }
      ts.forEachChild(node, visit);
    }
    visit(tree);
  }
});
