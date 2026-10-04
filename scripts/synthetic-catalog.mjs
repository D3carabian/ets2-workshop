// Development fixtures only: the Rust example uses the production freshness API.
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, realpath, writeFile } from "node:fs/promises";
import { isAbsolute, join, relative, resolve } from "node:path";
import { promisify } from "node:util";

const repo = resolve(import.meta.dirname, "..");
const verification = join(repo, "verification");
const exec = promisify(execFile);

export async function createSyntheticGame(game) {
  await mkdir(verification, { recursive: true });
  const destination = resolve(game);
  const lexical = relative(verification, destination);
  assert(lexical && !lexical.startsWith("..") && !isAbsolute(lexical));
  await mkdir(join(destination, "bin/win_x64"), { recursive: true });
  const base = await realpath(verification);
  for (const directory of [destination, join(destination, "bin/win_x64")]) {
    const inside = relative(base, await realpath(directory));
    assert(inside && !inside.startsWith("..") && !isAbsolute(inside));
  }
  const archive = Buffer.from(
    await readFile(
      join(repo, "scripts/fixtures/onboarding.scs.base64"),
      "utf8",
    ),
    "base64",
  );
  assert.equal(
    createHash("sha256").update(archive).digest("hex"),
    "f7160d049b6104616373346bfd9fbf01d78a9f2a43985ef9a78521061e7ed9a8",
  );
  await writeFile(join(destination, "def.scs"), archive);
  await writeFile(
    join(destination, "bin/win_x64/eurotrucks2.exe"),
    "synthetic placeholder; never executed",
  );
  return destination;
}

export async function bindSyntheticCatalog(catalog, game) {
  const example = join(
    repo,
    "src-tauri/target/debug/examples/synthetic_catalog.exe",
  );
  try {
    await exec(example, [resolve(catalog), resolve(game)], {
      windowsHide: true,
    });
  } catch (error) {
    throw new Error(
      `Synthetic catalog binding failed. Build first: cargo build --locked --manifest-path src-tauri/Cargo.toml --example synthetic_catalog\n${error.stderr || error.message}`,
    );
  }
}
