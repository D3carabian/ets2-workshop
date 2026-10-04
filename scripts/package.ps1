param([string]$ExpectedTag = '')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$release = & (Join-Path $PSScriptRoot 'check-release.ps1') -ExpectedTag $ExpectedTag -PassThru
$version = $release.Version
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
    if ((Get-Item -LiteralPath (Join-Path $repoRoot $source)).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Links are not package inputs: $source" }
}
$licenseRoot = Join-Path $repoRoot 'licenses'
$licenseFiles = @($release.LicenseFiles | ForEach-Object { Get-Item -LiteralPath (Join-Path $licenseRoot $_) })
$artifactRoot = Join-Path $repoRoot 'release/artifacts'
New-Item -ItemType Directory -Path $artifactRoot -Force | Out-Null
$stage = Join-Path $artifactRoot ('.stage-' + [guid]::NewGuid().ToString('N'))
$candidateZip = "$stage.zip"
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
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $candidateZip
    & (Join-Path $PSScriptRoot 'verify-package.ps1') -ZipPath $candidateZip -ExpectedTag $ExpectedTag
    $hash = (Get-FileHash -LiteralPath $candidateZip -Algorithm SHA256).Hash.ToLowerInvariant()
    Move-Item -LiteralPath $candidateZip -Destination $zipPath -Force
    [IO.File]::WriteAllText("$zipPath.sha256", "$hash  $zipName`n", [Text.UTF8Encoding]::new($false))
    Write-Output "Package: $zipPath"
    Write-Output "SHA256: $hash"
} finally {
    $resolvedStage = [IO.Path]::GetFullPath($stage)
    $allowedRoot = [IO.Path]::GetFullPath($artifactRoot) + [IO.Path]::DirectorySeparatorChar
    if (-not $resolvedStage.StartsWith($allowedRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Refusing to remove staging outside artifact directory.' }
    Remove-Item -LiteralPath $resolvedStage -Recurse -Force
    if (Test-Path -LiteralPath $candidateZip) { Remove-Item -LiteralPath $candidateZip -Force }
}
