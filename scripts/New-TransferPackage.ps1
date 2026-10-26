[CmdletBinding()]
param(
    [string]$Destination,
    [switch]$WithoutPaperCorpus,
    [string]$GitExecutable = "git"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$scriptDirectory = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = [System.IO.Path]::GetFullPath((Join-Path $scriptDirectory ".."))

if ([string]::IsNullOrWhiteSpace($Destination)) {
    $Destination = Join-Path $projectRoot "handoff"
}

$destinationPath = [System.IO.Path]::GetFullPath($Destination)
if ($destinationPath -eq $projectRoot) {
    throw "Destination cannot be the project root. Use the default handoff directory or another dedicated directory."
}

[System.IO.Directory]::CreateDirectory($destinationPath) | Out-Null

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$archiveName = "paper-master-transfer-$timestamp.zip"
$archivePath = Join-Path $destinationPath $archiveName
$temporaryArchivePath = "$archivePath.tmp"
$checksumPath = "$archivePath.sha256"

if (Test-Path -LiteralPath $archivePath -PathType Leaf) {
    throw "Archive already exists: $archivePath"
}

if (Test-Path -LiteralPath $temporaryArchivePath -PathType Leaf) {
    throw "Temporary archive already exists: $temporaryArchivePath"
}

function Get-ProjectRelativePath {
    param([Parameter(Mandatory = $true)][string]$FullName)

    return $FullName.Substring($projectRoot.Length).TrimStart([char[]]@('\', '/'))
}

function Test-ExcludedPath {
    param([Parameter(Mandatory = $true)][string]$RelativePath)

    $path = $RelativePath.Replace('\', '/')
    if ($path -match '(?i)(^|/)(paper-master-private|private|secrets|\.private)(/|$)' -or $path -match '(?i)(webvpn|credentials|凭据).*\.txt$') {
        return $true
    }

    if ($path -match '(^|/)(node_modules|\.pnpm-store|\.cargo-home|dist|target|handoff|library-cache|program file|data|logs)(/|$)') {
        return $true
    }

    if ($path -match '^src-tauri/gen(/|$)') {
        return $true
    }

    if ($path -match '(^|/)\.handoff-verify(/|$)') {
        return $true
    }

    if ($path -match '(^|/)\.env($|\.)') {
        return $true
    }

    if ($path -match '\.(log|db|db-shm|db-wal|sqlite|sqlite3)$') {
        return $true
    }

    if ($path -match '^application/.*\.(exe|lnk)$') {
        return $true
    }

    if ($path -match '^\.git/(index\.lock|shallow\.lock)$') {
        return $true
    }

    if ($WithoutPaperCorpus -and $path -match '^paper(/|$)') {
        return $true
    }

    if ($path -match '\.pdf$' -and $path -notmatch '^paper/') {
        return $true
    }

    return $false
}

$gitBranch = "unavailable"
$gitCommit = "unavailable"
$gitStatus = "unavailable"

try {
    $gitBranch = (& $GitExecutable -C $projectRoot branch --show-current 2>$null | Select-Object -First 1)
    $gitCommit = (& $GitExecutable -C $projectRoot rev-parse HEAD 2>$null | Select-Object -First 1)
    $statusLines = @(& $GitExecutable -C $projectRoot status --short 2>$null)
    $gitStatus = if ($statusLines.Count -eq 0) { "clean" } else { $statusLines -join "`n" }

    $remoteUrls = @(& $GitExecutable -C $projectRoot remote get-url --all origin 2>$null)
    $credentialedRemote = $remoteUrls | Where-Object { $_ -match '^[a-z][a-z0-9+.-]*://[^/@]+:[^/@]+@' } | Select-Object -First 1
    if ($credentialedRemote) {
        throw "A Git remote URL appears to contain embedded credentials. Remove the credentials before packaging."
    }
}
catch {
    if ($_.Exception.Message -like "A Git remote URL appears*") {
        throw
    }
}

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

$sourceFiles = Get-ChildItem -LiteralPath $projectRoot -Recurse -Force -File -ErrorAction Stop |
    Where-Object { -not ($_.Attributes -band [System.IO.FileAttributes]::ReparsePoint) } |
    ForEach-Object {
        $relativePath = Get-ProjectRelativePath -FullName $_.FullName
        if (-not (Test-ExcludedPath -RelativePath $relativePath)) {
            [pscustomobject]@{
                File = $_
                RelativePath = $relativePath.Replace('\', '/')
            }
        }
    } |
    Sort-Object RelativePath

$archiveStream = $null
$zipArchive = $null
$includedBytes = [int64]0
$includedPaths = [System.Collections.Generic.List[string]]::new()

try {
    $archiveStream = [System.IO.File]::Open(
        $temporaryArchivePath,
        [System.IO.FileMode]::CreateNew,
        [System.IO.FileAccess]::ReadWrite,
        [System.IO.FileShare]::None
    )
    $zipArchive = [System.IO.Compression.ZipArchive]::new(
        $archiveStream,
        [System.IO.Compression.ZipArchiveMode]::Create,
        $false
    )

    foreach ($item in $sourceFiles) {
        $entryName = "paper-master/$($item.RelativePath)"
        $entry = $zipArchive.CreateEntry($entryName, [System.IO.Compression.CompressionLevel]::Optimal)
        $entry.LastWriteTime = [System.DateTimeOffset]::new($item.File.LastWriteTime)

        $inputStream = $null
        $outputStream = $null
        try {
            $inputStream = [System.IO.File]::OpenRead($item.File.FullName)
            $outputStream = $entry.Open()
            $inputStream.CopyTo($outputStream)
        }
        finally {
            if ($outputStream) { $outputStream.Dispose() }
            if ($inputStream) { $inputStream.Dispose() }
        }

        $includedBytes += $item.File.Length
        $includedPaths.Add($item.RelativePath)
    }

    $manifestLines = [System.Collections.Generic.List[string]]::new()
    $manifestLines.Add("paper-master transfer package")
    $manifestLines.Add("Generated: $((Get-Date).ToString('yyyy-MM-dd HH:mm:ss zzz'))")
    $manifestLines.Add("Branch: $gitBranch")
    $manifestLines.Add("HEAD: $gitCommit")
    $manifestLines.Add("Git status at packaging:")
    $manifestLines.Add($gitStatus)
    $manifestLines.Add("")
    $manifestLines.Add("Included files: $($includedPaths.Count)")
    $manifestLines.Add("Uncompressed bytes: $includedBytes")
    $manifestLines.Add("Paper corpus included: $(-not $WithoutPaperCorpus)")
    $manifestLines.Add("")
    $manifestLines.Add("Excluded: node_modules, pnpm/Cargo caches, target/dist/gen build outputs, handoff archives, env files, databases, logs, application executables/shortcuts, and external LM Studio programs/models/caches/logs/settings.")
    $manifestLines.Add("The repository's LM Studio adapter source is included because it is required to compile the project; no LM Studio runtime or model is bundled.")
    $manifestLines.Add("")
    $manifestLines.Add("FILE LIST")
    foreach ($includedPath in $includedPaths) {
        $manifestLines.Add($includedPath)
    }

    $manifestEntry = $zipArchive.CreateEntry("paper-master/_TRANSFER_MANIFEST.txt", [System.IO.Compression.CompressionLevel]::Optimal)
    $manifestStream = $null
    $manifestWriter = $null
    try {
        $manifestStream = $manifestEntry.Open()
        $utf8WithoutBom = [System.Text.UTF8Encoding]::new($false)
        $manifestWriter = [System.IO.StreamWriter]::new($manifestStream, $utf8WithoutBom)
        $manifestWriter.Write(($manifestLines -join "`n"))
    }
    finally {
        if ($manifestWriter) { $manifestWriter.Dispose() }
        elseif ($manifestStream) { $manifestStream.Dispose() }
    }
}
finally {
    if ($zipArchive) { $zipArchive.Dispose() }
    if ($archiveStream) { $archiveStream.Dispose() }
}

[System.IO.File]::Move($temporaryArchivePath, $archivePath)

$hash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
$checksumText = "$hash *$archiveName`n"
[System.IO.File]::WriteAllText($checksumPath, $checksumText, [System.Text.Encoding]::ASCII)

[pscustomobject]@{
    Archive = $archivePath
    Checksum = $checksumPath
    SHA256 = $hash
    IncludedFiles = $includedPaths.Count
    UncompressedMB = [math]::Round($includedBytes / 1MB, 2)
    ArchiveMB = [math]::Round((Get-Item -LiteralPath $archivePath).Length / 1MB, 2)
    PaperCorpusIncluded = -not $WithoutPaperCorpus
}
