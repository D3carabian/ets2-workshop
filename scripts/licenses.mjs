import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const supplements = {
  'alloc-stdlib@0.3.0': {
    source: 'https://github.com/dropbox/rust-alloc-no-stdlib/blob/0a81fd6928ea3b33c8cd484aa4575d50ffb98012/LICENSE',
    files: [['alloc-stdlib/LICENSE.txt', 'c0c56f26d9c051cac4d200c34c84e7ae9aaa853e01a982a1df08b09931e518ae']],
  },
  'defmt-parser@1.0.0': {
    source: 'https://github.com/knurling-rs/defmt/tree/4a8cdb44891ed57b8ff5a023b6bec7137c48708f (repository LICENSE-MIT and LICENSE-APACHE)',
    files: [
      ['defmt-parser/LICENSE-APACHE.txt', '8173d5c29b4f956d532781d2b86e4e30f83e6b7878dce18c919451d6ba707c90'],
      ['defmt-parser/LICENSE-MIT.txt', '2710a622a896bba67356913d4d0492cab5465f61b2ecce6d880aeb483834fb50'],
    ],
  },
  'selectors@0.38.0': {
    source: 'https://www.mozilla.org/media/MPL/2.0/index.815ca599c9df.txt; license URL explicitly specified by selectors/lib.rs at servo/stylo commit 572ecba2d1600e7c3d490586692a209faf703baa',
    files: [['selectors/LICENSE-MPL-2.0.txt', 'fab3dd6bdab226f1c08630b1dd917e11fcb4ec5e1e020e2c16f83a0a13863e85']],
  },
  'webview2-com@0.39.1': {
    source: 'https://github.com/wravery/webview2-rs/blob/edc2caf886175ccaebe86078c9cfe1ae2a187328/LICENSE',
    files: [['webview2-com/LICENSE.txt', '0dcf41516e608bbcb6cdc5229feb7b86fe4a643b85e7df251133c93408fdac73']],
  },
  'webview2-com-macros@0.8.1': {
    source: 'https://github.com/wravery/webview2-rs/blob/dffa41a8a46d3f5565eefbff2de57d38d399f158/LICENSE',
    files: [['webview2-com-macros/LICENSE.txt', '0dcf41516e608bbcb6cdc5229feb7b86fe4a643b85e7df251133c93408fdac73']],
  },
};
supplements['webview2-com-sys@0.39.1'] = supplements['webview2-com@0.39.1'];
// Validate every checked-in source even when a dependency now supplies its own text.
const sourceFiles = new Map(Object.values(supplements).flatMap(s => s.files)
  .map(([file, hash]) => [`licenses/upstream/${file}`, hash]));
for (const file of ['licenses/DecryptTruck-MIT.txt', 'src-tauri/vendor/decrypt-truck/LICENSE']) {
  sourceFiles.set(file, '3c64dbae48fba3efd8591f8e7e41ca6b9df76b9b2b3425f1e5c41c82a2f28b9c');
}
for (const [relative, expectedHash] of sourceFiles) {
  const actualHash = createHash('sha256').update(fs.readFileSync(path.join(root, relative))).digest('hex');
  if (actualHash !== expectedHash) throw new Error(`Source license hash mismatch: ${relative}`);
}
if (process.argv[2] === '--check-sources') {
  console.log(`Verified ${sourceFiles.size} source license files.`);
  process.exit(0);
}
const destination = path.resolve(process.argv[2] ?? '');
const artifactRoot = path.join(root, 'release', 'artifacts') + path.sep;
if (!destination.startsWith(artifactRoot) || fs.existsSync(destination)) {
  throw new Error('License output must be a new directory under release/artifacts.');
}
const metadata = JSON.parse(execFileSync('cargo', [
  'metadata', '--locked', '--format-version', '1', '--features', 'custom-protocol',
  '--filter-platform', 'x86_64-pc-windows-msvc',
  '--manifest-path', path.join(root, 'src-tauri', 'Cargo.toml'),
], { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }));
const lock = JSON.parse(fs.readFileSync(path.join(root, 'package-lock.json'), 'utf8'));
const entries = [];
const decoder = new TextDecoder('utf-8', { fatal: true });
fs.mkdirSync(destination, { recursive: true });
const npmParents = {
  '@esbuild/win32-x64': 'esbuild',
  '@rollup/rollup-win32-x64-gnu': 'rollup',
  '@rollup/rollup-win32-x64-msvc': 'rollup',
  '@tauri-apps/cli-win32-x64-msvc': '@tauri-apps/cli',
};

function collect(ecosystem, name, version, license, directory, declaredFile, source) {
  const files = new Set();
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (entry.isFile() && /^(licen[cs]e|copying|notice)(?:[._-].*)?$/i.test(entry.name)) {
      files.add(path.join(directory, entry.name));
    }
    // Some packages keep multiple licenses in a dedicated directory.
    if (entry.isDirectory() && /^(licenses?|licences?)$/i.test(entry.name)) {
      for (const child of fs.readdirSync(path.join(directory, entry.name), { withFileTypes: true })) {
        if (child.isFile()) files.add(path.join(directory, entry.name, child.name));
      }
    }
  }
  if (declaredFile) files.add(path.resolve(directory, declaredFile));
  let licenseTextSource = 'License files distributed in the dependency package';
  if (!files.size && ecosystem === 'cargo' && supplements[`${name}@${version}`]) {
    const supplement = supplements[`${name}@${version}`];
    for (const [relative, expectedHash] of supplement.files) {
      const file = path.join(root, 'licenses', 'upstream', relative);
      const actualHash = createHash('sha256').update(fs.readFileSync(file)).digest('hex');
      if (actualHash !== expectedHash) throw new Error(`Supplementary license hash mismatch: ${relative}`);
      files.add(file);
    }
    licenseTextSource = `Supplement from exact upstream source: ${supplement.source}`;
  }
  if (!files.size && ecosystem === 'npm' && npmParents[name]) {
    const parentName = npmParents[name];
    const parentDirectory = path.join(root, 'node_modules', parentName);
    const parent = JSON.parse(fs.readFileSync(path.join(parentDirectory, 'package.json'), 'utf8'));
    if (parent.name !== parentName || parent.version !== version) throw new Error(`Platform license parent version mismatch: ${name}`);
    for (const entry of fs.readdirSync(parentDirectory, { withFileTypes: true })) {
      if (entry.isFile() && /^(licen[cs]e|copying|notice)(?:[._-].*)?$/i.test(entry.name)) files.add(path.join(parentDirectory, entry.name));
    }
    licenseTextSource = `Platform binary inherits license texts from ${parentName}@${version}; ${parent.repository?.url || parent.repository}`;
  }
  const identity = `${ecosystem}-${name}-${version}`;
  const slug = identity.replace(/[^a-zA-Z0-9._-]/g, '_');
  const copied = [];
  for (const input of [...files].sort()) {
    const real = fs.realpathSync(input);
    if (!fs.statSync(real).isFile()) throw new Error(`Not a regular license file: ${identity}`);
    const bytes = fs.readFileSync(real);
    if (bytes.length > 2 * 1024 * 1024 || bytes.includes(0)) throw new Error(`Non-text license file: ${identity}`);
    decoder.decode(bytes); // Refuse binary or invalid UTF-8 rather than shipping arbitrary files.
    const digest = createHash('sha256').update(bytes).digest('hex');
    const filename = `${String(copied.length + 1).padStart(2, '0')}-${path.basename(input).replace(/[^a-zA-Z0-9._-]/g, '_')}.txt`;
    const relative = `texts/${slug}/${filename}`;
    fs.mkdirSync(path.dirname(path.join(destination, relative)), { recursive: true });
    fs.writeFileSync(path.join(destination, relative), bytes);
    copied.push({ file: relative, sha256: digest });
  }
  entries.push({ ecosystem, name, version, license: license || 'Not declared', source, licenseTextSource, texts: copied });
}

const resolvedPackages = new Set(metadata.resolve.nodes.map(node => node.id));
for (const pkg of metadata.packages.sort((a, b) => a.id.localeCompare(b.id))) {
  if (!resolvedPackages.has(pkg.id)) continue;
  if (metadata.workspace_members.includes(pkg.id)) continue;
  collect('cargo', pkg.name, pkg.version, pkg.license, path.dirname(pkg.manifest_path), pkg.license_file,
    pkg.source || 'vendored source in repository');
}
const omitted = [];
for (const [relative, locked] of Object.entries(lock.packages).sort(([a], [b]) => a.localeCompare(b))) {
  if (!relative) continue;
  const directory = path.resolve(root, relative);
  if (!directory.startsWith(path.join(root, 'node_modules') + path.sep)) throw new Error('Unexpected npm package path.');
  const manifest = path.join(directory, 'package.json');
  if (!fs.existsSync(manifest)) {
    if (!locked.optional) throw new Error(`Installed dependency is missing: ${relative}`);
    omitted.push({ name: relative, version: locked.version, reason: 'Optional package not installed on this build platform' });
    continue;
  }
  const pkg = JSON.parse(fs.readFileSync(manifest, 'utf8'));
  if (pkg.version !== locked.version) throw new Error(`npm version does not match lock: ${relative}`);
  const license = typeof pkg.license === 'string' ? pkg.license : pkg.license?.type;
  collect('npm', pkg.name, pkg.version, license || locked.license, directory, null, locked.resolved || 'npm lockfile');
}
const report = {
  scope: 'Cargo metadata dependency graph for x86_64-pc-windows-msvc, including build/dev dependencies; all installed npm lockfile dependencies. Inclusion does not imply a package is linked into the executable.',
  packages: entries,
  omittedOptionalNpmPackages: omitted,
};
fs.writeFileSync(path.join(destination, 'inventory.json'), JSON.stringify(report, null, 2) + '\n');
const escape = value => String(value).replaceAll('|', '\\|').replaceAll('\n', ' ');
const lines = ['# Dependency licenses', '', report.scope, '',
  'License files are copied unchanged from dependency packages or the verified upstream supplements described below. This report does not replace license terms.', '',
  '| Ecosystem | Package | Version | Declared license | Included texts | License text source |',
  '| --- | --- | --- | --- | --- | --- |'];
for (const entry of entries) {
  const links = entry.texts.map(text => `[text](${text.file})`).join(', ') || '**No license text found**';
  lines.push(`| ${entry.ecosystem} | ${escape(entry.name)} | ${escape(entry.version)} | ${escape(entry.license)} | ${links} | ${escape(entry.licenseTextSource)} |`);
}
fs.writeFileSync(path.join(destination, 'DEPENDENCIES.md'), lines.join('\n') + '\n');
const missing = entries.filter(entry => !entry.texts.length);
console.log(`License inventory: ${entries.length} packages, ${entries.reduce((count, entry) => count + entry.texts.length, 0)} texts, ${omitted.length} optional npm packages omitted.`);
if (missing.length) {
  throw new Error('Dependencies without license text: ' + missing.map(entry => `${entry.ecosystem}:${entry.name}@${entry.version}`).join(', '));
}
