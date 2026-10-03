param([switch]$CheckOnly, [string]$ExpectedTag = '')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Set-Location (Join-Path $PSScriptRoot '..')
$localNode = Join-Path (Get-Location) '.tools\node-v22.16.0-win-x64'
if (-not (Get-Command node -ErrorAction SilentlyContinue) -and (Test-Path $localNode)) { $env:Path = "$localNode;$env:Path" }
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path $cargoBin) { $env:Path = "$cargoBin;$env:Path" }
if ((node --version) -notmatch '^v22\.') { throw 'Node.js 22 is required.' }
if (-not (Test-Path 'package-lock.json') -or -not (Test-Path 'src-tauri/Cargo.lock')) { throw 'Both dependency lockfiles are required.' }
npm.cmd ci
if ($LASTEXITCODE) { throw 'npm ci failed' }
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
if ($LASTEXITCODE) { throw 'Rust formatting check failed' }
npm.cmd run build
if ($LASTEXITCODE) { throw 'Frontend build failed' }
cargo test --locked --manifest-path src-tauri/Cargo.toml
if ($LASTEXITCODE) { throw 'Tests failed' }
if ($CheckOnly) { return }
cargo build --locked --release --features custom-protocol --manifest-path src-tauri/Cargo.toml --bin ets2-workshop --bin workshop-cli --bin workshop-launcher
if ($LASTEXITCODE) { throw 'Desktop build failed' }
& (Join-Path $PSScriptRoot 'package.ps1') -ExpectedTag $ExpectedTag
