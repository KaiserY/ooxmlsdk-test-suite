[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ReferenceFile,

    [Parameter(Mandatory = $true)]
    [string]$OutputFile,

    [string[]]$CandidateRoots = @(),

    [string]$CandidateMapFile,

    [string[]]$OfficeCloudRoots = @(),

    [switch]$InstallAvailable
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$utf8NoBom = New-Object Text.UTF8Encoding($false)
Add-Type -AssemblyName System.Drawing

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class OoxmlsdkFontBroadcast {
    [DllImport("user32.dll", SetLastError = true)]
    public static extern IntPtr SendMessageTimeout(
        IntPtr hWnd,
        uint Msg,
        UIntPtr wParam,
        IntPtr lParam,
        uint flags,
        uint timeout,
        out UIntPtr result
    );
}
"@

function Normalize-FontName {
    param([string]$Name)

    return (($Name.Trim().TrimStart("@") -replace '\s+', ' ').ToLowerInvariant())
}

function Add-FontAlias {
    param([Collections.Generic.HashSet[string]]$Aliases, [string]$Name)

    if (-not [string]::IsNullOrWhiteSpace($Name)) {
        [void]$Aliases.Add((Normalize-FontName $Name))
    }
}

function Get-InstalledFontAliases {
    $aliases = New-Object 'Collections.Generic.HashSet[string]' (
        [StringComparer]::OrdinalIgnoreCase
    )
    $collection = New-Object Drawing.Text.InstalledFontCollection
    try {
        foreach ($family in $collection.Families) {
            Add-FontAlias $aliases $family.Name
        }
    }
    finally {
        $collection.Dispose()
    }

    foreach ($key in @(
        "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts",
        "HKCU:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts"
    )) {
        $properties = Get-ItemProperty -LiteralPath $key -ErrorAction SilentlyContinue
        if ($null -eq $properties) {
            continue
        }
        foreach ($property in $properties.PSObject.Properties) {
            if ($property.Name -like "PS*") {
                continue
            }
            $name = $property.Name -replace '\s*\([^)]*\)\s*$', ''
            $name = $name -replace '\s*\[ooxmlsdk [^]]+\]\s*$', ''
            foreach ($alias in $name.Split("&")) {
                Add-FontAlias $aliases $alias
            }
        }
    }
    return $aliases
}

function Get-FontFiles {
    param([string[]]$Roots)

    $files = New-Object Collections.Generic.List[IO.FileInfo]
    foreach ($root in $Roots) {
        if (-not (Test-Path -LiteralPath $root)) {
            continue
        }
        foreach ($file in Get-ChildItem -LiteralPath $root -File -Recurse -ErrorAction SilentlyContinue) {
            if ($file.Extension.ToLowerInvariant() -in @(".ttf", ".ttc", ".otf")) {
                $files.Add($file)
            }
        }
    }
    return @($files | Sort-Object FullName -Unique)
}

function Get-CandidateFontMap {
    param([IO.FileInfo[]]$Files)

    $map = @{}
    foreach ($file in $Files) {
        $collection = New-Object Drawing.Text.PrivateFontCollection
        try {
            $collection.AddFontFile($file.FullName)
            foreach ($family in $collection.Families) {
                $name = Normalize-FontName $family.Name
                if (-not $map.ContainsKey($name)) {
                    $map[$name] = New-Object Collections.Generic.List[string]
                }
                $map[$name].Add($file.FullName)
            }
        }
        catch {
            # Some variable/color/CFF fonts are usable by Office but not by
            # GDI+'s PrivateFontCollection. They remain visible in the report
            # as unresolved instead of being installed under a guessed name.
        }
        finally {
            $collection.Dispose()
        }
    }
    return $map
}

function Install-UserFontFile {
    param([string]$Source, [string]$Family)

    $targetRoot = Join-Path $env:LOCALAPPDATA "Microsoft\Windows\Fonts"
    New-Item -ItemType Directory -Force -Path $targetRoot | Out-Null
    $sourceFile = Get-Item -LiteralPath $Source
    $target = Join-Path $targetRoot $sourceFile.Name
    $copyRequired = $true
    if (Test-Path -LiteralPath $target) {
        $sourceHash = (Get-FileHash -LiteralPath $Source -Algorithm SHA256).Hash
        $targetHash = (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash
        if ($sourceHash -eq $targetHash) {
            $copyRequired = $false
        }
        else {
            $target = Join-Path $targetRoot (
                "{0}-{1}{2}" -f
                $sourceFile.BaseName,
                $sourceHash.Substring(0, 12).ToLowerInvariant(),
                $sourceFile.Extension
            )
            if (Test-Path -LiteralPath $target) {
                $copyRequired = $false
            }
        }
    }
    if ($copyRequired) {
        Copy-Item -LiteralPath $Source -Destination $target
    }
    $registry = "HKCU:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts"
    if (-not (Test-Path -LiteralPath $registry)) {
        New-Item -Path $registry | Out-Null
    }
    $valueName = "{0} [ooxmlsdk {1}] (TrueType)" -f $Family, $sourceFile.Name
    New-ItemProperty `
        -LiteralPath $registry `
        -Name $valueName `
        -Value $target `
        -PropertyType String `
        -Force | Out-Null
    return $target
}

$references = Get-Content -Raw -LiteralPath $ReferenceFile | ConvertFrom-Json
$installedBefore = Get-InstalledFontAliases
$candidateMap = Get-CandidateFontMap (Get-FontFiles $CandidateRoots)
if (-not [string]::IsNullOrWhiteSpace($CandidateMapFile)) {
    $configuredCandidates = Get-Content -Raw -LiteralPath $CandidateMapFile | ConvertFrom-Json
    foreach ($font in $configuredCandidates.fonts) {
        $name = Normalize-FontName ([string]$font.name)
        if (-not $candidateMap.ContainsKey($name)) {
            $candidateMap[$name] = New-Object Collections.Generic.List[string]
        }
        foreach ($file in @($font.files)) {
            if (-not $candidateMap[$name].Contains([string]$file)) {
                $candidateMap[$name].Add([string]$file)
            }
        }
    }
}
$cloudMap = Get-CandidateFontMap (Get-FontFiles $OfficeCloudRoots)
$missingBefore = @($references.fonts | Where-Object {
    -not $installedBefore.Contains((Normalize-FontName $_.name))
})
$installedNow = New-Object Collections.Generic.List[object]

if ($InstallAvailable) {
    foreach ($reference in $missingBefore) {
        $normalized = Normalize-FontName $reference.name
        if (-not $candidateMap.ContainsKey($normalized)) {
            continue
        }
        foreach ($source in @($candidateMap[$normalized])) {
            $target = Install-UserFontFile $source $reference.name
            $installedNow.Add([ordered]@{
                name = [string]$reference.name
                source = [string]$source
                target = [string]$target
            })
        }
    }
    if ($installedNow.Count -gt 0) {
        [UIntPtr]$result = [UIntPtr]::Zero
        [void][OoxmlsdkFontBroadcast]::SendMessageTimeout(
            [IntPtr]0xffff,
            0x001d,
            [UIntPtr]::Zero,
            [IntPtr]::Zero,
            2,
            5000,
            [ref]$result
        )
    }
}

$installedAfter = Get-InstalledFontAliases
$fontResults = foreach ($reference in $references.fonts) {
    $normalized = Normalize-FontName $reference.name
    $status = if ($installedAfter.Contains($normalized)) {
        "installed"
    }
    elseif ($cloudMap.ContainsKey($normalized)) {
        "office-cloud-available"
    }
    elseif ($candidateMap.ContainsKey($normalized)) {
        "candidate-requires-restart"
    }
    else {
        "unavailable"
    }
    [pscustomobject][ordered]@{
        name = [string]$reference.name
        document_count = [int]$reference.document_count
        occurrence_count = [int]$reference.occurrence_count
        status = $status
        examples = @($reference.examples)
    }
}
$statusCounts = [ordered]@{}
foreach ($group in @($fontResults | Group-Object status | Sort-Object Name)) {
    $statusCounts[$group.Name] = $group.Count
}
$document = [ordered]@{
    schema_version = 1
    campaign_id = [string]$references.campaign_id
    audited_at_utc = [DateTime]::UtcNow.ToString("o")
    referenced_font_count = $references.fonts.Count
    installed_family_and_registry_alias_count = $installedAfter.Count
    candidate_roots = @($CandidateRoots)
    office_cloud_roots = @($OfficeCloudRoots)
    installed_now = $installedNow.ToArray()
    status_counts = $statusCounts
    fonts = @($fontResults)
}
$json = ($document | ConvertTo-Json -Depth 10) -replace "`r`n", "`n"
[IO.File]::WriteAllText($OutputFile, $json + "`n", $utf8NoBom)
Write-Output ("referenced|{0}" -f $document.referenced_font_count)
Write-Output ("installed_now|{0}" -f $installedNow.Count)
foreach ($name in $statusCounts.Keys) {
    Write-Output ("status|{0}|{1}" -f $name, $statusCounts[$name])
}
