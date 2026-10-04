# Isolated synthetic package tests; never touches player data or reuses compiled binaries.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$testParent = Join-Path $repoRoot 'release/artifacts'
New-Item -ItemType Directory -Path $testParent -Force | Out-Null
$fixture = Join-Path $testParent ('.package-test-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $fixture | Out-Null
$script:passed = 0
function Assert-Rejected([string]$Name, [scriptblock]$Action, [string]$Expected) {
    $failure = $null
    try { & $Action | Out-Null } catch { $failure = $_.Exception.Message }
    if ($null -eq $failure -or $failure -notmatch $Expected) { throw "Test '$Name' did not fail as expected ($Expected): $failure" }
    $script:passed++
    Write-Host "PASS: $Name"
}
function Invoke-SourceCheck {
    $output = & node (Join-Path $fixture 'scripts/licenses.mjs') --check-sources 2>&1
    if ($LASTEXITCODE -ne 0) { throw "License source check rejected: $output" }
}
try {
    foreach ($relative in @('scripts/package.ps1', 'scripts/check-release.ps1', 'scripts/verify-package.ps1', 'scripts/licenses.mjs', 'package.json', 'package-lock.json', 'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock', 'src-tauri/tauri.conf.json', 'src-tauri/vendor/decrypt-truck/LICENSE', 'README.md', 'THIRD_PARTY_NOTICES.md')) {
        $destination = Join-Path $fixture $relative
        New-Item -ItemType Directory -Path (Split-Path $destination) -Force | Out-Null
        Copy-Item -LiteralPath (Join-Path $repoRoot $relative) -Destination $destination
    }
    Copy-Item -LiteralPath (Join-Path $repoRoot 'licenses') -Destination (Join-Path $fixture 'licenses') -Recurse
    $release = & (Join-Path $fixture 'scripts/check-release.ps1') -PassThru
    $tag = "v$($release.Version)"
    Assert-Rejected 'wrong tag rejected before binaries are checked' { & (Join-Path $fixture 'scripts/package.ps1') -ExpectedTag 'v999.0.0' } 'Tag .* does not match'
    foreach ($relative in @('package.json', 'package-lock.json', 'src-tauri/tauri.conf.json', 'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock')) {
        $file = Join-Path $fixture $relative
        $original = [IO.File]::ReadAllBytes($file)
        try {
            $text = [IO.File]::ReadAllText($file)
            if ($relative -eq 'src-tauri/Cargo.lock') {
                $text = [regex]::Replace($text, '(?ms)(\[\[package\]\]\s*name = "ets2-workshop"\s*version = ")[^"]+', '${1}999.0.0')
            } else { $text = $text.Replace('"' + $release.Version + '"', '"999.0.0"') }
            [IO.File]::WriteAllText($file, $text)
            Assert-Rejected "version mismatch in $relative" { & (Join-Path $fixture 'scripts/check-release.ps1') } 'versions must match'
        } finally { [IO.File]::WriteAllBytes($file, $original) }
    }
    Invoke-SourceCheck
    $license = Join-Path $fixture 'licenses/upstream/alloc-stdlib/LICENSE.txt'
    $original = [IO.File]::ReadAllBytes($license)
    try {
        Remove-Item -LiteralPath $license
        Assert-Rejected 'missing license source' { Invoke-SourceCheck } 'License source check rejected'
        [IO.File]::WriteAllBytes($license, $original)
        [IO.File]::AppendAllText($license, "`nmodified")
        Assert-Rejected 'modified license source' { Invoke-SourceCheck } 'License source check rejected'
    } finally { [IO.File]::WriteAllBytes($license, $original) }
    $pollution = Join-Path $fixture 'licenses/player-notes.md'
    try {
        [IO.File]::WriteAllText($pollution, 'synthetic pollution')
        Assert-Rejected 'license whitelist rejects extra text' { & (Join-Path $fixture 'scripts/check-release.ps1') } 'whitelist'
    } finally { Remove-Item -LiteralPath $pollution }
    $binRoot = Join-Path $fixture 'src-tauri/target/release'
    New-Item -ItemType Directory -Path $binRoot -Force | Out-Null
    foreach ($name in @('workshop-launcher.exe', 'ets2-workshop.exe', 'workshop-cli.exe')) { [IO.File]::WriteAllText((Join-Path $binRoot $name), 'MZ synthetic binary for packaging tests only') }
    # Exercise the real package/ZIP validators with a deterministic synthetic license collector.
    @'
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
const root = process.argv[2];
const file = 'texts/npm-synthetic-1.0.0/01-LICENSE.txt';
const bytes = Buffer.from('Synthetic license text\n');
fs.mkdirSync(path.dirname(path.join(root, file)), { recursive: true });
fs.writeFileSync(path.join(root, file), bytes);
fs.writeFileSync(path.join(root, 'DEPENDENCIES.md'), '# Synthetic dependency licenses\n');
fs.writeFileSync(path.join(root, 'inventory.json'), JSON.stringify({ packages: [{ ecosystem: 'npm', name: 'synthetic', version: '1.0.0', texts: [{ file, sha256: createHash('sha256').update(bytes).digest('hex') }] }] }));
'@ | Set-Content -LiteralPath (Join-Path $fixture 'scripts/licenses.mjs')
    foreach ($name in @('AGENTS.md', 'DEVELOPMENT_PLAN.md', 'game.sii', 'settings.json')) { [IO.File]::WriteAllText((Join-Path $fixture $name), 'synthetic private content') }
    & (Join-Path $fixture 'scripts/package.ps1') -ExpectedTag $tag | Out-Null
    $zipPath = Join-Path $fixture "release/artifacts/ets2-workshop-$($release.Version)-windows-x64.zip"
    $originalZip = [IO.File]::ReadAllBytes($zipPath)
    $hashLine = (Get-Content -LiteralPath "$zipPath.sha256" -Raw).Trim()
    if ($hashLine -cne ((Get-FileHash $zipPath -Algorithm SHA256).Hash.ToLowerInvariant() + '  ' + [IO.Path]::GetFileName($zipPath))) { throw 'Package SHA256 sidecar mismatch.' }
    $script:passed++
    Write-Host 'PASS: package, ZIP whitelist, bilingual README and SHA256 sidecar'
    function Mutate-Zip([string]$Name, [byte[]]$Bytes, [switch]$Delete) {
        [IO.File]::WriteAllBytes($zipPath, $originalZip)
        $archive = [IO.Compression.ZipFile]::Open($zipPath, [IO.Compression.ZipArchiveMode]::Update)
        try {
            $entry = $archive.GetEntry($Name)
            if ($null -ne $entry) { $entry.Delete() }
            if (-not $Delete) {
                $entry = $archive.CreateEntry($Name)
                $stream = $entry.Open()
                try { $stream.Write($Bytes) } finally { $stream.Dispose() }
            }
        } finally { $archive.Dispose() }
    }
    foreach ($name in @('AGENTS.md', 'DEVELOPMENT_PLAN.md', 'game.sii', 'cache/player.json', 'licenses/player-notes.md')) {
        Mutate-Zip $name ([Text.Encoding]::UTF8.GetBytes('synthetic pollution'))
        Assert-Rejected "ZIP rejects $name" { & (Join-Path $fixture 'scripts/verify-package.ps1') -ZipPath $zipPath } 'Unexpected ZIP entry'
    }
    Mutate-Zip 'ETS2 Workshop.exe' @() -Delete
    Assert-Rejected 'missing launcher' { & (Join-Path $fixture 'scripts/verify-package.ps1') -ZipPath $zipPath } 'Missing ZIP entry'
    Mutate-Zip 'README.md' ([Text.Encoding]::UTF8.GetBytes('# English only'))
    Assert-Rejected 'README without both languages' { & (Join-Path $fixture 'scripts/verify-package.ps1') -ZipPath $zipPath } 'Chinese and English'
    Mutate-Zip 'licenses/dependencies/texts/npm-synthetic-1.0.0/01-LICENSE.txt' ([Text.Encoding]::UTF8.GetBytes('modified'))
    Assert-Rejected 'dependency license hash mismatch' { & (Join-Path $fixture 'scripts/verify-package.ps1') -ZipPath $zipPath } 'hash mismatch'
    Mutate-Zip 'licenses/DecryptTruck-MIT.txt' ([Text.Encoding]::UTF8.GetBytes('modified'))
    Assert-Rejected 'fixed license changed inside ZIP' { & (Join-Path $fixture 'scripts/verify-package.ps1') -ZipPath $zipPath } 'License content changed'
    foreach ($encoding in @([Text.Encoding]::UTF8, [Text.Encoding]::Unicode)) {
        Mutate-Zip 'workshop-app.exe' ([byte[]](@(0x4D, 0x5A) + $encoding.GetBytes('C:\Users\SyntheticPlayer\Documents\save')))
        Assert-Rejected "personal path ($($encoding.WebName))" { & (Join-Path $fixture 'scripts/verify-package.ps1') -ZipPath $zipPath } 'Personal absolute path'
    }
    Write-Host "Package checks passed: $script:passed"
} finally {
    $resolved = [IO.Path]::GetFullPath($fixture)
    $allowed = [IO.Path]::GetFullPath($testParent) + [IO.Path]::DirectorySeparatorChar
    if (-not $resolved.StartsWith($allowed, [StringComparison]::OrdinalIgnoreCase)) { throw 'Refusing to remove fixture outside artifact directory.' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
