param([switch]$CheckOnly, [string]$ExpectedTag = '', [string]$NodeDirectory = '')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Set-Location (Join-Path $PSScriptRoot '..')
. (Join-Path $PSScriptRoot 'select-node.ps1') -NodeDirectory $NodeDirectory
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path $cargoBin) { $env:Path = "$cargoBin;$env:Path" }
& (Join-Path $PSScriptRoot 'check-release.ps1') -ExpectedTag $ExpectedTag
node (Join-Path $PSScriptRoot 'licenses.mjs') --check-sources
if ($LASTEXITCODE) { throw 'Source license verification failed' }
if (-not (Test-Path 'package-lock.json') -or -not (Test-Path 'src-tauri/Cargo.lock')) { throw 'Both dependency lockfiles are required.' }
npm.cmd ci
if ($LASTEXITCODE) { throw 'npm ci failed' }
& (Join-Path $PSScriptRoot 'test-node.ps1')
& (Join-Path $PSScriptRoot 'test-package.ps1')
# Exercise the real collector in ordinary CI without leaving an old inventory to reuse.
$licenseCheck = Join-Path (Get-Location) ('release/artifacts/.license-check-' + [guid]::NewGuid().ToString('N'))
try {
    node (Join-Path $PSScriptRoot 'licenses.mjs') $licenseCheck
    if ($LASTEXITCODE) { throw 'Dependency license verification failed' }
} finally {
    $allowedRoot = [IO.Path]::GetFullPath((Join-Path (Get-Location) 'release/artifacts')) + [IO.Path]::DirectorySeparatorChar
    if (-not [IO.Path]::GetFullPath($licenseCheck).StartsWith($allowedRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe license check directory.' }
    if (Test-Path -LiteralPath $licenseCheck) { Remove-Item -LiteralPath $licenseCheck -Recurse -Force }
}
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
if ($LASTEXITCODE) { throw 'Rust formatting check failed' }
npm.cmd run build
if ($LASTEXITCODE) { throw 'Frontend build failed' }
node --test scripts/parts.test.mjs scripts/i18n.test.mjs scripts/garage.test.mjs
if ($LASTEXITCODE) { throw 'Frontend rule tests failed' }
cargo test --locked --manifest-path src-tauri/Cargo.toml
if ($LASTEXITCODE) { throw 'Tests failed' }
if ($CheckOnly) { return }
# Strip build-machine source and dependency paths from release executables.
$originalEncodedFlags = [Environment]::GetEnvironmentVariable('CARGO_ENCODED_RUSTFLAGS', 'Process')
try {
    $existingRustFlags = if ($env:CARGO_ENCODED_RUSTFLAGS) { @($env:CARGO_ENCODED_RUSTFLAGS -split [char]31) } else { @($env:RUSTFLAGS -split ' ' | Where-Object { $_ }) }
    $env:CARGO_ENCODED_RUSTFLAGS = (@($existingRustFlags) + @(
    "--remap-path-prefix=$((Get-Location).Path)=/workspace",
    "--remap-path-prefix=$env:USERPROFILE=/build-user"
    )) -join [char]31
    cargo build --locked --release --features custom-protocol --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target --bin ets2-workshop --bin workshop-cli --bin workshop-launcher
    if ($LASTEXITCODE) { throw 'Desktop build failed' }
    & (Join-Path $PSScriptRoot 'package.ps1') -ExpectedTag $ExpectedTag
} finally {
    if ($null -eq $originalEncodedFlags) { Remove-Item Env:CARGO_ENCODED_RUSTFLAGS -ErrorAction SilentlyContinue }
    else { $env:CARGO_ENCODED_RUSTFLAGS = $originalEncodedFlags }
}
