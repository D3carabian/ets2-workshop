param(
    [Parameter(Mandatory = $true)][string]$PackageDirectory,
    [ValidateSet('Present', 'Missing')][string]$ExpectedRuntime
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Read-only: no downloads, installation, registry edits or application data writes.
$package = (Resolve-Path -LiteralPath $PackageDirectory).Path
$cli = Join-Path $package 'workshop-cli.exe'
if (-not (Test-Path -LiteralPath $cli -PathType Leaf)) { throw 'The extracted package must contain workshop-cli.exe.' }
$status = (& $cli runtime-status | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $status -notin @('true', 'false')) { throw 'Runtime detection command failed.' }
$present = $status -eq 'true'
if ($ExpectedRuntime -and $present -ne ($ExpectedRuntime -eq 'Present')) {
    throw "Runtime baseline mismatch: expected $ExpectedRuntime, detected $status."
}
[ordered]@{
    RuntimePresent = $present
    CliSha256 = (Get-FileHash -LiteralPath $cli -Algorithm SHA256).Hash.ToLowerInvariant()
    OSVersion = [Environment]::OSVersion.Version.ToString()
    Is64BitOS = [Environment]::Is64BitOperatingSystem
} | ConvertTo-Json
