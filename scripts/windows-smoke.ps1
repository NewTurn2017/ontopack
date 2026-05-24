param(
    [string]$PackBin = "pack",
    [string]$Root = "",
    [switch]$KeepPack
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Root)) {
    $Root = Join-Path ([System.IO.Path]::GetTempPath()) ("ontopack-win-smoke-" + [System.Guid]::NewGuid().ToString("N"))
}

try {
    if (Test-Path -LiteralPath $PackBin) {
        $PackBin = (Resolve-Path -LiteralPath $PackBin).Path
    }
    else {
        $PackBin = (Get-Command $PackBin -ErrorAction Stop).Source
    }
}
catch {
    throw "could not resolve -PackBin '$PackBin' before changing directories. Pass an existing path such as .\target\release\pack.exe or ensure 'pack' is on PATH."
}

function Assert-Contains {
    param(
        [string]$Text,
        [string]$Needle,
        [string]$Label
    )
    if (-not $Text.Contains($Needle)) {
        throw "assertion failed: $Label must contain '$Needle'`n$Text"
    }
}

try {
    Write-Host "[1/8] init pack: $Root"
    & $PackBin init $Root | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $Root "notes") | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $Root "assets") | Out-Null

    $note = @"
---
type: note
title: Windows Smoke Note
tags: [windows, smoke]
created: 2026-05-24
---
windows-smoke-keyword portable path validation.
"@
    Set-Content -Path (Join-Path $Root "notes\windows-smoke.md") -Value $note -Encoding UTF8
    [System.IO.File]::WriteAllBytes((Join-Path $Root "assets\evidence.bin"), [byte[]](1,2,3,4))

    Write-Host "[2/8] build index"
    Push-Location $Root
    try {
        & $PackBin build --no-embed | Out-Null

        Write-Host "[3/8] search"
        $search = & $PackBin search "windows-smoke-keyword" --mode keyword -k 1 | Out-String
        Assert-Contains $search "windows-smoke#0000" "keyword search"

        Write-Host "[4/8] doctor"
        $doctor = & $PackBin doctor | Out-String
        Assert-Contains $doctor "doctor: ok=true" "doctor"

        Write-Host "[5/8] export jsonl"
        $jsonl = Join-Path $Root "context.jsonl"
        & $PackBin export --format jsonl --output $jsonl | Out-Null
        if (-not (Test-Path $jsonl)) { throw "jsonl export missing: $jsonl" }
        Assert-Contains (Get-Content -Raw $jsonl) '"note_id":"windows-smoke"' "jsonl export"

        Write-Host "[6/8] bundle archive"
        $bundleDir = Join-Path $Root "bundle-out"
        $archive = Join-Path $Root "bundle.tar.gz"
        & $PackBin bundle $bundleDir --archive $archive | Out-Null
        if (-not (Test-Path (Join-Path $bundleDir "bundle.json"))) { throw "bundle manifest missing" }
        if (-not (Test-Path $archive)) { throw "bundle archive missing" }

        Write-Host "[7/8] import bundle archive"
        $restore = Join-Path $Root "restore"
        & $PackBin init $restore | Out-Null
        Push-Location $restore
        try {
            & $PackBin import $archive | Out-Null
            & $PackBin build --no-embed | Out-Null
            $restored = & $PackBin search "windows-smoke-keyword" --mode keyword -k 1 | Out-String
            Assert-Contains $restored "windows-smoke#0000" "restored search"
        }
        finally {
            Pop-Location
        }
    }
    finally {
        Pop-Location
    }

    Write-Host "[8/8] Windows smoke passed: $Root"
}
finally {
    if (-not $KeepPack -and (Test-Path $Root)) {
        Remove-Item -Recurse -Force $Root
    }
}
