import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(
  new URL("../src/garage.ts", import.meta.url),
  "utf8",
);
const js = ts.transpileModule(source, {
  compilerOptions: {
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ESNext,
  },
}).outputText;
const { orderFleet, newSaveName } = await import(
  `data:text/javascript;base64,${Buffer.from(js).toString("base64")}`
);

test("fleet groups the active truck first and employees last without mutating save order", () => {
  const truck = (id, kind, current = false) => ({
    id,
    current,
    driver: { kind },
  });
  const input = [
    truck("employee-a", "employee"),
    truck("unknown", "unknown"),
    truck("unassigned", "unassigned"),
    truck("employee-b", "employee"),
    truck("active", "player", true),
    { id: "legacy-fixture", current: false },
  ];
  const original = [...input];
  assert.deepEqual(
    orderFleet(input).map((t) => t.id),
    [
      "active",
      "unknown",
      "unassigned",
      "legacy-fixture",
      "employee-a",
      "employee-b",
    ],
  );
  assert.deepEqual(input, original);
  assert.equal(orderFleet(input)[0], input[4]);
});

test("stable fleet grouping includes all 5000 drivers once", () => {
  const input = Array.from({ length: 5000 }, (_, id) => ({
    id: String(id),
    current: id === 4999,
    driver: { kind: id % 2 ? "employee" : "unassigned" },
  }));
  const output = orderFleet(input);
  assert.equal(output[0].id, "4999");
  assert.deepEqual(
    output.slice(1, 2501),
    input.filter((t) => t.driver.kind === "unassigned"),
  );
  assert.deepEqual(
    output.slice(2501),
    input.filter((t) => t.driver.kind === "employee" && !t.current),
  );
  assert.equal(new Set(output.map((t) => t.id)).size, 5000);
});

test("save names use local calendar time including milliseconds", () => {
  assert.equal(
    newSaveName(new Date(2026, 9, 4, 14, 32, 8, 123)),
    "Workshop_2026-10-04_14-32-08-123",
  );
  assert.equal(
    newSaveName(new Date(2026, 0, 2, 3, 4, 5, 6)),
    "Workshop_2026-01-02_03-04-05-006",
  );
  assert.notEqual(
    newSaveName(new Date(2026, 0, 2, 3, 4, 5, 6)),
    newSaveName(new Date(2026, 0, 2, 3, 4, 5, 7)),
  );
});
