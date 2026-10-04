param(
    [string]$NodeDirectory = '',
    [string]$RepositoryRoot = (Split-Path $PSScriptRoot -Parent),
    # A probe seam lets tests exercise discovery without installing fake runtimes.
    [Parameter(DontShow)][scriptblock]$VersionProbe = {
        param([string]$Executable)
        $output = & $Executable --version 2>&1
        if ($LASTEXITCODE -ne 0) { throw "Version command exited with code $LASTEXITCODE" }
        ($output | Out-String).Trim()
    }
)

$nodeCandidates = [System.Collections.Generic.List[string]]::new()
$nodeSeen = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
$nodeDiagnostics = [System.Collections.Generic.List[string]]::new()
$nodeExplicit = -not [string]::IsNullOrWhiteSpace($NodeDirectory)
if ($nodeExplicit) {
    $nodeCandidates.Add($NodeDirectory)
} else {
    foreach ($entry in ($env:Path -split [IO.Path]::PathSeparator)) {
        if (-not [string]::IsNullOrWhiteSpace($entry)) {
            $directory = $entry.Trim().Trim('"')
            if (Test-Path -LiteralPath (Join-Path $directory 'node.exe') -PathType Leaf) {
                $nodeCandidates.Add($directory)
            }
        }
    }
    $localTools = Join-Path $RepositoryRoot '.tools'
    if (Test-Path -LiteralPath $localTools -PathType Container) {
        foreach ($directory in (Get-ChildItem -LiteralPath $localTools -Directory -Filter 'node-v22*-win-x64' | Sort-Object Name -Descending)) {
            $nodeCandidates.Add($directory.FullName)
        }
    }
}

$selectedNodeDirectory = $null
$selectedNodeVersion = $null
foreach ($candidate in $nodeCandidates) {
    try {
        $directory = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($candidate)
        if (-not $nodeSeen.Add($directory)) { continue }
        $executable = Join-Path $directory 'node.exe'
        if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) { throw 'node.exe is missing' }
        $version = (& $VersionProbe $executable | Out-String).Trim()
        if ($version -notmatch '^v22\.\d+\.\d+$') { throw "Found '$version'; Node.js 22 is required" }
        if (-not (Test-Path -LiteralPath (Join-Path $directory 'npm.cmd') -PathType Leaf)) { throw "Found $version, but npm.cmd is missing from the same directory" }
        $selectedNodeDirectory = $directory
        $selectedNodeVersion = $version
        break
    } catch {
        $nodeDiagnostics.Add("  ${candidate}: $($_.Exception.Message)")
    }
}

if (-not $selectedNodeDirectory) {
    if ($nodeDiagnostics.Count -eq 0) { $nodeDiagnostics.Add('  No node.exe candidates found in PATH or .tools/node-v22*-win-x64.') }
    $hint = if ($nodeExplicit) { 'Fix -NodeDirectory: it must contain Node.js 22 node.exe and npm.cmd.' } else { 'Install Node.js 22, extract it under .tools/node-v22*-win-x64, or pass -NodeDirectory.' }
    throw "No usable Node.js 22 runtime found. $hint`n$($nodeDiagnostics -join "`n")"
}
$env:Path = "$selectedNodeDirectory$([IO.Path]::PathSeparator)$env:Path"
Write-Host "Using Node.js $selectedNodeVersion from $selectedNodeDirectory"
