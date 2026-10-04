$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$selector = Join-Path $PSScriptRoot 'select-node.ps1'
$originalPath = $env:Path
$fixture = Join-Path ([IO.Path]::GetTempPath()) ("ets2-node-tests-" + [guid]::NewGuid().ToString('N'))
$versions = @{}
$probe = { param($Executable) $versions[$Executable] }.GetNewClosure()

function New-FakeNode([string]$Directory, [string]$Version, [switch]$WithoutNpm) {
    New-Item -ItemType Directory -Path $Directory -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $Directory 'node.exe') -Value ''
    if (-not $WithoutNpm) { Set-Content -LiteralPath (Join-Path $Directory 'npm.cmd') -Value '' }
    $versions[(Join-Path $Directory 'node.exe')] = $Version
    $Directory
}

function Assert-Selected([string]$PathValue, [string]$Expected, [string]$Explicit = '') {
    $env:Path = $PathValue
    & $selector -RepositoryRoot $fixture -NodeDirectory $Explicit -VersionProbe $probe
    if (($env:Path -split [IO.Path]::PathSeparator)[0] -ne $Expected) { throw "Expected selection: $Expected; got $env:Path" }
}

function Assert-Rejected([string]$PathValue, [string[]]$ExpectedMessages, [string]$Explicit = '') {
    $env:Path = $PathValue
    $failure = $null
    try { & $selector -RepositoryRoot $fixture -NodeDirectory $Explicit -VersionProbe $probe } catch { $failure = $_.Exception.Message }
    if (-not $failure) { throw 'Expected Node.js selection to fail.' }
    foreach ($message in $ExpectedMessages) {
        if (-not $failure.Contains($message)) { throw "Missing diagnostic '$message': $failure" }
    }
    if ($env:Path -ne $PathValue) { throw 'A failed selection changed PATH.' }
}

try {
    $wrong = New-FakeNode (Join-Path $fixture 'wrong') 'v24.1.0'
    $path22 = New-FakeNode (Join-Path $fixture 'path22') 'v22.16.0'
    $missingNpm = New-FakeNode (Join-Path $fixture 'missing-npm') 'v22.16.0' -WithoutNpm
    Assert-Selected "$wrong;$path22" $path22
    Assert-Selected "$missingNpm;$path22" $path22
    Assert-Rejected $wrong @('v24.1.0', $wrong, '-NodeDirectory')
    Assert-Rejected '' @('No node.exe candidates found')
    Assert-Rejected $path22 @('node.exe is missing', 'Fix -NodeDirectory') (Join-Path $fixture 'absent')
    Assert-Rejected $path22 @('v24.1.0', 'Fix -NodeDirectory') $wrong
    Assert-Rejected $path22 @('npm.cmd is missing') $missingNpm
    $local22 = New-FakeNode (Join-Path $fixture '.tools/node-v22.17.0-win-x64') 'v22.17.0'
    Assert-Selected $wrong $local22
    Assert-Selected "$wrong;$path22" $path22
    Assert-Selected $path22 $local22 $local22
    Write-Host 'Node.js discovery checks passed (10 cases).'
} finally {
    $env:Path = $originalPath
    $resolvedFixture = [IO.Path]::GetFullPath($fixture)
    $temporaryRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if (-not $resolvedFixture.StartsWith($temporaryRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Refusing to remove a fixture outside the temporary directory.' }
    if (Test-Path -LiteralPath $resolvedFixture) { Remove-Item -LiteralPath $resolvedFixture -Recurse -Force }
}
