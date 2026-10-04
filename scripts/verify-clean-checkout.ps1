param([string]$NodeDirectory = '', [string]$ExpectedTag = '')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$verificationRoot = Join-Path $repoRoot 'verification'
$runId = [guid]::NewGuid().ToString('N')
$checkout = Join-Path $verificationRoot "clean-$runId"
$temporaryIndex = Join-Path $verificationRoot ".index-$runId"
$originalLocation = Get-Location
$savedEnvironment = @{}
foreach ($name in @('Path', 'GIT_INDEX_FILE', 'CARGO_TARGET_DIR', 'ETS2_WORKSHOP_DATA_DIR')) {
    $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}

try {
    Set-Location -LiteralPath $repoRoot
    $git = (Get-Command git -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    $pwsh = (Get-Command pwsh -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    . (Join-Path $PSScriptRoot 'select-node.ps1') -NodeDirectory $NodeDirectory
    if (-not $ExpectedTag) { $ExpectedTag = 'v' + (Get-Content -LiteralPath (Join-Path $repoRoot 'package.json') -Raw | ConvertFrom-Json).version }
    & (Join-Path $PSScriptRoot 'check-release.ps1') -ExpectedTag $ExpectedTag
    New-Item -ItemType Directory -Path $checkout -Force | Out-Null
    # A separate index snapshots tracked edits, deletions and non-ignored new files.
    # The user's real index, branch and working files remain untouched.
    $env:GIT_INDEX_FILE = $temporaryIndex
    & $git read-tree HEAD
    if ($LASTEXITCODE) { throw 'Cannot initialize the temporary Git index from HEAD.' }
    & $git add --all -- .
    if ($LASTEXITCODE) { throw 'Cannot snapshot current source files into the temporary Git index.' }
    $prefix = $checkout.Replace('\', '/') + '/'
    & $git -c core.autocrlf=true checkout-index --all "--prefix=$prefix"
    if ($LASTEXITCODE) { throw 'Cannot export the clean Windows checkout.' }
    if ($null -eq $savedEnvironment['GIT_INDEX_FILE']) { Remove-Item Env:GIT_INDEX_FILE -ErrorAction SilentlyContinue }
    else { $env:GIT_INDEX_FILE = $savedEnvironment['GIT_INDEX_FILE'] }
    foreach ($forbidden in @('node_modules', 'dist', 'src-tauri/target', 'release', 'AGENTS.md', 'DEVELOPMENT_PLAN.md')) {
        if (Test-Path -LiteralPath (Join-Path $checkout $forbidden)) { throw "Unexpected local file or build output in clean checkout: $forbidden" }
    }
    # Do not inherit a target directory that could contain old release binaries.
    Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    $env:ETS2_WORKSHOP_DATA_DIR = Join-Path $checkout 'verification/app-data'
    Write-Host "Clean verification checkout: $checkout"
    Write-Host 'CARGO_TARGET_DIR is cleared; dependencies and binaries are built in this fresh checkout.'
    Set-Location -LiteralPath $checkout
    $buildScript = Join-Path $checkout 'scripts/build.ps1'
    & $pwsh -NoProfile -File $buildScript -CheckOnly -NodeDirectory $selectedNodeDirectory -ExpectedTag $ExpectedTag
    if ($LASTEXITCODE) { throw 'Clean checkout CheckOnly validation failed.' }
    & $pwsh -NoProfile -File $buildScript -NodeDirectory $selectedNodeDirectory -ExpectedTag $ExpectedTag
    if ($LASTEXITCODE) { throw 'Clean checkout full build or packaging failed.' }
    $zipName = "ets2-workshop-$($ExpectedTag.Substring(1))-windows-x64.zip"
    $zipPath = Join-Path $checkout "release/artifacts/$zipName"
    & (Join-Path $checkout 'scripts/verify-package.ps1') -ZipPath $zipPath -ExpectedTag $ExpectedTag
    $checksumPath = "$zipPath.sha256"
    $hash = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ((Get-Content -LiteralPath $checksumPath -Raw).Trim() -cne "$hash  $zipName") { throw 'The generated ZIP checksum does not match.' }
    $unpacked = Join-Path $checkout 'verification/unpacked'
    Expand-Archive -LiteralPath $zipPath -DestinationPath $unpacked
    $cli = Join-Path $unpacked 'workshop-cli.exe'
    $samples = @(
        @{ File = 'two-ordinal-fields.bsii'; Expected = @('first: alpha', 'second: beta') },
        @{ File = 'unicode-strings.bsii'; Expected = @('name: "\xe4\xb8\xad\xe6\x96\x87"', 'names[0]: "\xe4\xb8\xad\xe6\x96\x87"') }
    )
    foreach ($sample in $samples) {
        $decoded = & $cli decode (Join-Path $checkout "src-tauri/tests/fixtures/$($sample.File)")
        if ($LASTEXITCODE) { throw "Packaged CLI failed to decode $($sample.File)." }
        $text = $decoded -join "`n"
        foreach ($expected in $sample.Expected) {
            if (-not $text.Contains($expected)) { throw "Packaged CLI returned unexpected content for $($sample.File)." }
        }
        Write-Host "Packaged CLI decoded synthetic fixture: $($sample.File)"
    }
    $destination = Join-Path $repoRoot 'release/artifacts'
    New-Item -ItemType Directory -Path $destination -Force | Out-Null
    Copy-Item -LiteralPath $zipPath -Destination (Join-Path $destination $zipName) -Force
    Copy-Item -LiteralPath $checksumPath -Destination (Join-Path $destination "$zipName.sha256") -Force
    Write-Host "Verified clean-build ZIP: $(Join-Path $destination $zipName)"
    Write-Host "SHA256: $hash"
} finally {
    Set-Location -LiteralPath $originalLocation.Path
    foreach ($name in $savedEnvironment.Keys) {
        if ($null -eq $savedEnvironment[$name]) { Remove-Item "Env:$name" -ErrorAction SilentlyContinue }
        else { [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name], 'Process') }
    }
    $allowedRoot = [IO.Path]::GetFullPath($verificationRoot).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    foreach ($temporaryPath in @($checkout, $temporaryIndex, "$temporaryIndex.lock")) {
        $resolved = [IO.Path]::GetFullPath($temporaryPath)
        if (-not $resolved.StartsWith($allowedRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Refusing to remove files outside the verification directory.' }
        if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
    }
}
