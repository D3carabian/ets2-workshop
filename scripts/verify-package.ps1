param([Parameter(Mandatory)][string]$ZipPath, [string]$ExpectedTag = '')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$release = & (Join-Path $PSScriptRoot 'check-release.ps1') -ExpectedTag $ExpectedTag -PassThru
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead([IO.Path]::GetFullPath($ZipPath))
try {
    $entries = [Collections.Generic.Dictionary[string, object]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($entry in $zip.Entries) {
        $name = $entry.FullName
        if ($name.Contains('\') -or $name.StartsWith('/') -or $name -match '(^|/)\.\.?(/|$)' -or $name.Contains(':')) { throw "Unsafe ZIP path: $name" }
        if ($name.EndsWith('/')) { continue }
        if (-not $entries.TryAdd($name, $entry)) { throw "Duplicate ZIP entry: $name" }
    }
    function Read-Entry([string]$Name) {
        if (-not $entries.ContainsKey($Name)) { throw "Missing ZIP entry: $Name" }
        if ($entries[$Name].Length -gt 256MB) { throw "ZIP entry too large: $Name" }
        $stream = $entries[$Name].Open()
        $memory = [IO.MemoryStream]::new()
        try { $stream.CopyTo($memory); return ,$memory.ToArray() } finally { $stream.Dispose(); $memory.Dispose() }
    }
    $allowed = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($name in @('ETS2 Workshop.exe', 'workshop-app.exe', 'workshop-cli.exe', 'README.md', 'THIRD_PARTY_NOTICES.md', 'licenses/dependencies/inventory.json', 'licenses/dependencies/DEPENDENCIES.md')) {
        [void]$allowed.Add($name)
    }
    foreach ($name in $release.LicenseFiles) {
        $target = "licenses/$name"
        [void]$allowed.Add($target)
        $bytes = Read-Entry $target
        $actualHash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes))
        if ($actualHash -cne (Get-FileHash -LiteralPath (Join-Path $repoRoot $target) -Algorithm SHA256).Hash) { throw "License content changed in ZIP: $target" }
    }
    $inventory = [Text.Encoding]::UTF8.GetString((Read-Entry 'licenses/dependencies/inventory.json')) | ConvertFrom-Json
    if (@($inventory.packages).Count -eq 0) { throw 'Dependency inventory is empty.' }
    foreach ($package in $inventory.packages) {
        if (@($package.texts).Count -eq 0) { throw 'Dependency has no license texts.' }
        foreach ($license in $package.texts) {
            if ($license.file -cnotmatch '^texts/[a-zA-Z0-9._-]+/[a-zA-Z0-9._-]+\.txt$' -or $license.sha256 -cnotmatch '^[a-f0-9]{64}$') { throw 'Invalid dependency license inventory record.' }
            $target = "licenses/dependencies/$($license.file)"
            if (-not $allowed.Add($target)) { throw "Duplicate dependency text: $target" }
            $digest = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData((Read-Entry $target))).ToLowerInvariant()
            if ($digest -cne $license.sha256) { throw "Dependency license hash mismatch: $target" }
        }
    }
    foreach ($name in $allowed) { if (-not $entries.ContainsKey($name)) { throw "Missing ZIP entry: $name" } }
    foreach ($name in $entries.Keys) {
        if (-not $allowed.Contains($name)) { throw "Unexpected ZIP entry: $name" }
        $bytes = Read-Entry $name
        if ($bytes.Length -eq 0) { throw "Empty ZIP entry: $name" }
        if ($name.EndsWith('.exe') -and ($bytes.Length -lt 2 -or $bytes[0] -ne 0x4D -or $bytes[1] -ne 0x5A)) { throw "Invalid executable header: $name" }
        # Search both ordinary byte strings and Windows wide strings, including executables.
        foreach ($text in @([Text.Encoding]::UTF8.GetString($bytes), [Text.Encoding]::Unicode.GetString($bytes))) {
            if ($text -match '(?i)[a-z]:[\\/]+Users[\\/]+[^\\/\s\x00]+|/(?:Users|home)/[^/\s\x00]+') { throw "Personal absolute path found in ZIP entry: $name" }
        }
    }
    $readme = [Text.Encoding]::UTF8.GetString((Read-Entry 'README.md'))
    if ($readme -notmatch '(?m)^## 简体中文\s*$' -or $readme -notmatch '(?m)^## English\s*$' -or ([regex]::Matches($readme, 'ETS2 Workshop\.exe')).Count -lt 2) { throw 'ZIP README must include Chinese and English startup instructions.' }
    Write-Host "Verified ZIP: $ZipPath ($($entries.Count) files)"
} finally { $zip.Dispose() }
