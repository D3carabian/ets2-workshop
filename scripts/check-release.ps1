param([string]$ExpectedTag = '', [switch]$PassThru)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$package = Get-Content -LiteralPath (Join-Path $repoRoot 'package.json') -Raw | ConvertFrom-Json
$lock = Get-Content -LiteralPath (Join-Path $repoRoot 'package-lock.json') -Raw | ConvertFrom-Json -AsHashtable
$tauri = Get-Content -LiteralPath (Join-Path $repoRoot 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
$cargoText = Get-Content -LiteralPath (Join-Path $repoRoot 'src-tauri/Cargo.toml') -Raw
$cargoPackage = [regex]::Match($cargoText, '(?ms)^\[package\]\s*(.*?)(?=^\[|\z)').Groups[1].Value
$cargoVersion = [regex]::Match($cargoPackage, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
$cargoName = [regex]::Match($cargoPackage, '(?m)^name\s*=\s*"([^"]+)"').Groups[1].Value
$cargoLock = Get-Content -LiteralPath (Join-Path $repoRoot 'src-tauri/Cargo.lock') -Raw
$rootPackages = @([regex]::Matches($cargoLock, '(?ms)^\[\[package\]\]\s*(.*?)(?=^\[\[|\z)') | Where-Object {
    [regex]::Match($_.Groups[1].Value, '(?m)^name\s*=\s*"([^"]+)"').Groups[1].Value -ceq $cargoName
})
if ($rootPackages.Count -ne 1) { throw 'Cargo.lock must contain exactly one application package.' }
$cargoLockVersion = [regex]::Match($rootPackages[0].Groups[1].Value, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
$version = $package.version
if ($version -notmatch '^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$') { throw 'Invalid release version.' }
if ($version -cne $lock.version -or $version -cne $lock.packages[''].version -or $version -cne $tauri.version -or $version -cne $cargoVersion -or $version -cne $cargoLockVersion) {
    throw 'package.json, package-lock.json, Cargo.toml, Cargo.lock and tauri.conf.json versions must match.'
}
if ($ExpectedTag -and $ExpectedTag -cne "v$version") { throw "Tag $ExpectedTag does not match v$version." }
$licenseFiles = @(
    'DecryptTruck-MIT.txt'
    'locale-cityhash-MIT.txt'
    'upstream/SOURCES.md'
    'upstream/alloc-stdlib/LICENSE.txt'
    'upstream/defmt-parser/LICENSE-APACHE.txt'
    'upstream/defmt-parser/LICENSE-MIT.txt'
    'upstream/selectors/LICENSE-MPL-2.0.txt'
    'upstream/webview2-com/LICENSE.txt'
    'upstream/webview2-com-macros/LICENSE.txt'
)
$licenseRoot = Join-Path $repoRoot 'licenses'
$actualFiles = @(Get-ChildItem -LiteralPath $licenseRoot -Recurse -Force | ForEach-Object {
    if ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Links are not package inputs: $($_.Name)" }
    if (-not $_.PSIsContainer) { [IO.Path]::GetRelativePath($licenseRoot, $_.FullName).Replace('\', '/') }
})
if (@(Compare-Object $licenseFiles $actualFiles -CaseSensitive).Count) { throw 'License source files do not match the release whitelist.' }
$result = [pscustomobject]@{ Version = $version; LicenseFiles = $licenseFiles }
if ($PassThru) { $result } else { Write-Host "Release inputs checked: v$version" }
