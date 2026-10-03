param([string]$ExpectedTag = '')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$package = Get-Content -LiteralPath (Join-Path $repoRoot 'package.json') -Raw | ConvertFrom-Json
$lock = Get-Content -LiteralPath (Join-Path $repoRoot 'package-lock.json') -Raw | ConvertFrom-Json -AsHashtable
$tauri = Get-Content -LiteralPath (Join-Path $repoRoot 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
$cargoText = Get-Content -LiteralPath (Join-Path $repoRoot 'src-tauri/Cargo.toml') -Raw
$cargoVersion = [regex]::Match($cargoText, '(?ms)^\[package\]\s*(.*?)(?=^\[|\z)').Groups[1].Value
$cargoVersion = [regex]::Match($cargoVersion, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
$version = $package.version
if ($version -notmatch '^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$') { throw 'Invalid release version.' }
if ($version -ne $lock.version -or $version -ne $lock.packages[''].version -or $version -ne $tauri.version -or $version -ne $cargoVersion) { throw 'package.json, package-lock.json, Cargo.toml and tauri.conf.json versions must match.' }
if ($ExpectedTag -and $ExpectedTag -cne "v$version") { throw "Tag $ExpectedTag does not match v$version." }
# Never package the existing release directory: use explicit inputs and fresh staging.
$inputs = @{
    'ETS2 Workshop.exe' = 'src-tauri/target/release/workshop-launcher.exe'
    'workshop-app.exe' = 'src-tauri/target/release/ets2-workshop.exe'
    'workshop-cli.exe' = 'src-tauri/target/release/workshop-cli.exe'
    'README.md' = 'README.md'
    'THIRD_PARTY_NOTICES.md' = 'THIRD_PARTY_NOTICES.md'
}
foreach ($source in $inputs.Values) {
    if (-not (Test-Path -LiteralPath (Join-Path $repoRoot $source) -PathType Leaf)) { throw "Missing package input: $source" }
}
$licenseRoot = Join-Path $repoRoot 'licenses'
$licenseFiles = @(Get-ChildItem -LiteralPath $licenseRoot -File -Recurse)
if ($licenseFiles.Count -eq 0) { throw 'Third-party license texts are required.' }
foreach ($file in $licenseFiles) {
    if ($file.Extension -notin @('.md', '.txt') -and $file.Name -notmatch '^(LICENSE|COPYING|NOTICE)([.-].*)?$') { throw "Unexpected license file: $($file.Name)" }
}
$artifactRoot = Join-Path $repoRoot 'release/artifacts'
New-Item -ItemType Directory -Path $artifactRoot -Force | Out-Null
$stage = Join-Path $artifactRoot ('.stage-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $stage | Out-Null
try {
    foreach ($entry in $inputs.GetEnumerator()) { Copy-Item -LiteralPath (Join-Path $repoRoot $entry.Value) -Destination (Join-Path $stage $entry.Key) }
    foreach ($file in $licenseFiles) {
        $relative = $file.FullName.Substring($licenseRoot.Length).TrimStart('\', '/')
        $destination = Join-Path (Join-Path $stage 'licenses') $relative
        New-Item -ItemType Directory -Path (Split-Path $destination) -Force | Out-Null
        Copy-Item -LiteralPath $file.FullName -Destination $destination
    }
    & node (Join-Path $PSScriptRoot 'licenses.mjs') (Join-Path $stage 'licenses/dependencies')
    if ($LASTEXITCODE) { throw 'Dependency license collection failed' }
    $zipName = "ets2-workshop-$version-windows-x64.zip"
    $zipPath = Join-Path $artifactRoot $zipName
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zipPath -Force
    $hash = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText("$zipPath.sha256", "$hash  $zipName`n", [Text.UTF8Encoding]::new($false))
    Write-Output "Package: $zipPath"
    Write-Output "SHA256: $hash"
} finally {
    $resolvedStage = [IO.Path]::GetFullPath($stage)
    $allowedRoot = [IO.Path]::GetFullPath($artifactRoot) + [IO.Path]::DirectorySeparatorChar
    if (-not $resolvedStage.StartsWith($allowedRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Refusing to remove staging outside artifact directory.' }
    Remove-Item -LiteralPath $resolvedStage -Recurse -Force
}
