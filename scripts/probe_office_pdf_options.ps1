[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$CorpusRoot,

    [Parameter(Mandatory = $true)]
    [string]$PlanFile,

    [Parameter(Mandatory = $true)]
    [string]$OutputRoot,

    [string]$ProcessIdFile
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Test-WslPath {
    param([string]$Path)

    return $Path -match '^\\\\wsl(?:\.localhost|\$)\\[^\\]+\\'
}

function Test-WslTempPath {
    param([string]$Path)

    return $Path -match '^\\\\wsl(?:\.localhost|\$)\\[^\\]+\\tmp(?:\\|$)'
}

function Release-ComObject {
    param($Value)

    if ($null -ne $Value -and [Runtime.InteropServices.Marshal]::IsComObject($Value)) {
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($Value)
    }
}

function Assert-ExactProperties {
    param($Value, [string[]]$Names, [string]$Label)

    $actual = @($Value.PSObject.Properties.Name | Sort-Object)
    $expected = @($Names | Sort-Object)
    if ([string]::Join("`n", $actual) -ne [string]::Join("`n", $expected)) {
        throw "$Label properties must be exactly [$($expected -join ', ')]; got [$($actual -join ', ')]."
    }
}

function Assert-Boolean {
    param($Value, [string]$Label)

    if ($Value -isnot [bool]) {
        throw "$Label must be a JSON boolean."
    }
}

function Assert-Integer {
    param($Value, [string]$Label)

    if ($Value -isnot [int] -and $Value -isnot [long]) {
        throw "$Label must be a JSON integer."
    }
}

function Assert-String {
    param($Value, [string]$Label)

    if ($Value -isnot [string] -or [string]::IsNullOrWhiteSpace($Value)) {
        throw "$Label must be a non-empty JSON string."
    }
}

function Get-ApplicationFamily {
    param([string]$Extension)

    switch ($Extension.ToLowerInvariant()) {
        { $_ -in @(".docx", ".docm", ".dotx", ".dotm") } { return "Word" }
        { $_ -in @(".xlsx", ".xlsm", ".xltx", ".xltm") } { return "Excel" }
        { $_ -in @(".pptx", ".pptm", ".ppsx", ".ppsm", ".potx", ".potm") } {
            return "PowerPoint"
        }
    }
    throw "The focused PDF-options probe accepts OOXML Office formats only: $Extension"
}

function Assert-ProbeOptions {
    param($Options, [string]$Family, [int]$PageCount)

    Assert-ExactProperties $Options @(
        "bitmap_missing_fonts",
        "bookmarks",
        "include_document_properties",
        "page_from",
        "page_to",
        "pdf_a_1",
        "print_hidden_slides",
        "quality",
        "tagged_pdf"
    ) "options"
    foreach ($name in @(
        "bitmap_missing_fonts",
        "include_document_properties",
        "pdf_a_1",
        "print_hidden_slides",
        "tagged_pdf"
    )) {
        Assert-Boolean $Options.$name "options.$name"
    }
    Assert-String $Options.quality "options.quality"
    Assert-String $Options.bookmarks "options.bookmarks"
    if ($Options.quality -notin @("print", "screen")) {
        throw "options.quality must be print or screen."
    }
    if ($Options.bookmarks -notin @("none", "headings", "word-bookmarks")) {
        throw "options.bookmarks must be none, headings, or word-bookmarks."
    }
    if ($Family -ne "Word" -and $Options.bookmarks -ne "none") {
        throw "$Family does not expose Word's bookmark export selector."
    }
    if ($Family -ne "PowerPoint" -and $Options.print_hidden_slides) {
        throw "print_hidden_slides applies only to PowerPoint."
    }
    if ($Family -eq "Excel") {
        if ($Options.pdf_a_1) {
            throw "Excel Workbook.ExportAsFixedFormat has no per-call PDF/A selector."
        }
        if (-not $Options.tagged_pdf) {
            throw "Excel Workbook.ExportAsFixedFormat has no per-call structure-tag selector; the recorded Office profile emits tags."
        }
        if (-not $Options.bitmap_missing_fonts) {
            throw "Excel Workbook.ExportAsFixedFormat has no per-call bitmap-missing-fonts selector."
        }
    }
    $hasFrom = $null -ne $Options.page_from
    $hasTo = $null -ne $Options.page_to
    if ($hasFrom -ne $hasTo) {
        throw "options.page_from and options.page_to must both be null or both be integers."
    }
    if ($hasFrom) {
        Assert-Integer $Options.page_from "options.page_from"
        Assert-Integer $Options.page_to "options.page_to"
        $from = [long]$Options.page_from
        $to = [long]$Options.page_to
        if ($from -lt 1 -or $to -lt $from -or $to -gt $PageCount) {
            throw "Invalid 1-based page range $from-$to for $PageCount source pages."
        }
    }
}

function Get-FixedFormatQualityValue {
    param([string]$Family, [string]$Quality)

    switch ($Family) {
        "Word" { return $(if ($Quality -eq "print") { 0 } else { 1 }) }
        "Excel" { return $(if ($Quality -eq "print") { 0 } else { 1 }) }
        "PowerPoint" { return $(if ($Quality -eq "print") { 2 } else { 1 }) }
    }
    throw "Unsupported Office family for fixed-format quality: $Family"
}

function Export-WithWord {
    param($Application, [string]$InputPath, [string]$OutputPath, $Options)

    $document = $null
    try {
        $document = $Application.Documents.Open($InputPath, $false, $true, $false)
        Assert-ProbeOptions $Options "Word" ([int]$document.ComputeStatistics(2))
        $range = if ($null -eq $Options.page_from) { 0 } else { 3 }
        $from = if ($null -eq $Options.page_from) { 1 } else { [int]$Options.page_from }
        $to = if ($null -eq $Options.page_to) { 1 } else { [int]$Options.page_to }
        $bookmarks = switch ([string]$Options.bookmarks) {
            "none" { 0 }
            "headings" { 1 }
            "word-bookmarks" { 2 }
        }
        $quality = Get-FixedFormatQualityValue "Word" ([string]$Options.quality)
        $document.ExportAsFixedFormat(
            $OutputPath,
            17,
            $false,
            $quality,
            $range,
            $from,
            $to,
            0,
            [bool]$Options.include_document_properties,
            $false,
            $bookmarks,
            [bool]$Options.tagged_pdf,
            [bool]$Options.bitmap_missing_fonts,
            [bool]$Options.pdf_a_1
        )
    }
    finally {
        if ($null -ne $document) {
            $document.Close(0)
            Release-ComObject $document
        }
    }
}

function Export-WithExcel {
    param($Application, [string]$InputPath, [string]$OutputPath, $Options)

    $workbook = $null
    try {
        $workbook = $Application.Workbooks.Open($InputPath, 0, $true)
        $missing = [Type]::Missing
        Assert-ProbeOptions $Options "Excel" ([int]::MaxValue)
        $from = if ($null -eq $Options.page_from) { $missing } else { [int]$Options.page_from }
        $to = if ($null -eq $Options.page_to) { $missing } else { [int]$Options.page_to }
        $quality = Get-FixedFormatQualityValue "Excel" ([string]$Options.quality)
        $workbook.ExportAsFixedFormat(
            0,
            $OutputPath,
            $quality,
            [bool]$Options.include_document_properties,
            $false,
            $from,
            $to,
            $false
        )
    }
    finally {
        if ($null -ne $workbook) {
            $workbook.Close($false)
            Release-ComObject $workbook
        }
    }
}

function Export-WithPowerPoint {
    param($Application, [string]$InputPath, [string]$OutputPath, $Options)

    $presentation = $null
    $printRange = $null
    try {
        $presentation = $Application.Presentations.Open($InputPath, -1, 0, 0)
        $slideCount = [int]$presentation.Slides.Count
        Assert-ProbeOptions $Options "PowerPoint" $slideCount
        $from = if ($null -eq $Options.page_from) { 1 } else { [int]$Options.page_from }
        $to = if ($null -eq $Options.page_to) { $slideCount } else { [int]$Options.page_to }
        $printRange = $presentation.PrintOptions.Ranges.Add($from, $to)
        $quality = Get-FixedFormatQualityValue "PowerPoint" ([string]$Options.quality)
        $presentation.ExportAsFixedFormat(
            $OutputPath,
            2,
            $quality,
            0,
            1,
            1,
            $(if ($Options.print_hidden_slides) { -1 } else { 0 }),
            $printRange,
            4,
            "",
            [bool]$Options.include_document_properties,
            $false,
            [bool]$Options.tagged_pdf,
            [bool]$Options.bitmap_missing_fonts,
            [bool]$Options.pdf_a_1
        )
    }
    finally {
        if ($null -ne $printRange) {
            Release-ComObject $printRange
        }
        if ($null -ne $presentation) {
            $presentation.Close()
            Release-ComObject $presentation
        }
    }
}

function New-OfficeApplication {
    param([string]$Family)

    switch ($Family) {
        "Word" {
            $application = New-Object -ComObject "Word.Application"
            $application.Visible = $false
            $application.DisplayAlerts = 0
            $application.AutomationSecurity = 3
            return $application
        }
        "Excel" {
            $application = New-Object -ComObject "Excel.Application"
            $application.Visible = $false
            $application.DisplayAlerts = $false
            $application.AskToUpdateLinks = $false
            $application.AutomationSecurity = 3
            return $application
        }
        "PowerPoint" {
            $application = New-Object -ComObject "PowerPoint.Application"
            $application.DisplayAlerts = 1
            $application.AutomationSecurity = 3
            return $application
        }
    }
}

function Stop-OfficeApplication {
    param($Application)

    if ($null -ne $Application) {
        try { $Application.Quit() } catch {}
        Release-ComObject $Application
    }
}

function Get-OfficeProcessIds {
    param([string]$Family)

    $name = switch ($Family) {
        "Word" { "WINWORD" }
        "Excel" { "EXCEL" }
        "PowerPoint" { "POWERPNT" }
    }
    return @(Get-Process -Name $name -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
}

function Write-OfficeProcessId {
    param([string]$Family, [int[]]$BeforeIds)

    if ([string]::IsNullOrWhiteSpace($ProcessIdFile)) {
        return
    }
    $name = switch ($Family) {
        "Word" { "WINWORD" }
        "Excel" { "EXCEL" }
        "PowerPoint" { "POWERPNT" }
    }
    $process = Get-Process -Name $name -ErrorAction SilentlyContinue |
        Where-Object { $_.Id -notin $BeforeIds } |
        Sort-Object StartTime -Descending |
        Select-Object -First 1
    if ($null -ne $process) {
        [IO.File]::WriteAllText($ProcessIdFile, [string]$process.Id)
    }
}

$corpus = Get-Item -LiteralPath $CorpusRoot
$plan = Get-Item -LiteralPath $PlanFile
$output = Get-Item -LiteralPath $OutputRoot
if (-not $corpus.PSIsContainer -or -not $output.PSIsContainer -or $plan.PSIsContainer) {
    throw "CorpusRoot/OutputRoot must be directories and PlanFile must be a file."
}
$isSuiteCorpus = (Test-WslPath $corpus.FullName) -and $corpus.FullName.EndsWith(
    "\ooxmlsdk-test-suite\corpus",
    [StringComparison]::OrdinalIgnoreCase
)
$isTemporaryInputRoot = Test-WslTempPath $corpus.FullName
if (-not $isSuiteCorpus -and -not $isTemporaryInputRoot) {
    throw "CorpusRoot must be this test-suite's WSL corpus directory or a WSL /tmp minimal-case directory."
}
if (-not (Test-WslTempPath $plan.FullName) -or -not (Test-WslTempPath $output.FullName)) {
    throw "PlanFile and OutputRoot must be under WSL /tmp."
}
if (@(Get-ChildItem -LiteralPath $output.FullName -Force).Count -ne 0) {
    throw "OutputRoot must be empty."
}

$records = @()
$seen = @{}
foreach ($line in Get-Content -LiteralPath $plan.FullName -Encoding UTF8) {
    if ([string]::IsNullOrWhiteSpace($line)) {
        continue
    }
    $record = $line | ConvertFrom-Json
    Assert-ExactProperties $record @("file", "options", "schema_version") "plan record"
    Assert-Integer $record.schema_version "plan record schema_version"
    Assert-String $record.file "plan record file"
    if ([long]$record.schema_version -ne 1) {
        throw "Unsupported plan schema_version: $($record.schema_version)"
    }
    $rawRelative = [string]$record.file
    if ($rawRelative.Contains("\") -or $rawRelative.StartsWith("/") -or
        $rawRelative.EndsWith("/") -or $rawRelative.Contains("//")) {
        throw "Only normalized forward-slash corpus-relative paths are allowed: $rawRelative"
    }
    $relative = $rawRelative.Replace("/", "\")
    $segments = $relative.Split("\")
    if ([IO.Path]::IsPathRooted($relative) -or $segments -contains "" -or
        $segments -contains "." -or $segments -contains "..") {
        throw "Only normalized corpus-relative paths are allowed: $relative"
    }
    if ($seen.ContainsKey($relative)) {
        throw "Each source may have exactly one PDF option assignment: $relative"
    }
    $seen[$relative] = $true
    $records += $record
}
if ($records.Count -lt 1 -or $records.Count -gt 12) {
    throw "PlanFile must contain 1 to 12 records."
}

$corpusPrefix = $corpus.FullName.TrimEnd("\") + "\"
$applications = @{}
$stageRoot = Join-Path $env:TEMP ("ooxmlsdk-pdf-options-probe-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $stageRoot | Out-Null
$utf8NoBom = New-Object Text.UTF8Encoding($false)

try {
    for ($index = 0; $index -lt $records.Count; $index += 1) {
        $record = $records[$index]
        $relative = ([string]$record.file).Replace("/", "\")
        $source = Get-Item -LiteralPath (Join-Path $corpus.FullName $relative)
        if ($source.PSIsContainer -or -not $source.FullName.StartsWith(
            $corpusPrefix,
            [StringComparison]::OrdinalIgnoreCase
        )) {
            throw "Input is outside the allowed input root: $relative"
        }
        $family = Get-ApplicationFamily $source.Extension
        if (-not $applications.ContainsKey($family)) {
            $processIdsBefore = Get-OfficeProcessIds $family
            $applications[$family] = New-OfficeApplication $family
            Write-OfficeProcessId $family $processIdsBefore
        }
        $application = $applications[$family]

        $stageDirectory = Join-Path $stageRoot ("case-{0:D3}" -f $index)
        New-Item -ItemType Directory -Path $stageDirectory | Out-Null
        $stageInput = Join-Path $stageDirectory $source.Name
        $stageOutput = Join-Path $stageDirectory ($source.BaseName + ".pdf")
        Copy-Item -LiteralPath $source.FullName -Destination $stageInput

        $started = [Diagnostics.Stopwatch]::StartNew()
        switch ($family) {
            "Word" { Export-WithWord $application $stageInput $stageOutput $record.options }
            "Excel" { Export-WithExcel $application $stageInput $stageOutput $record.options }
            "PowerPoint" {
                Export-WithPowerPoint $application $stageInput $stageOutput $record.options
            }
        }
        $destination = Join-Path $output.FullName ("case-{0:D3}.pdf" -f $index)
        Copy-Item -LiteralPath $stageOutput -Destination $destination
        $applicationBuild = ""
        try {
            $applicationBuild = [string]$application.Build
        }
        catch {}
        $result = [ordered]@{
            schema_version = 1
            file = ([string]$record.file).Replace("\", "/")
            application = $family
            application_version = [string]$application.Version
            application_build = $applicationBuild
            options = $record.options
            source_sha256 = (Get-FileHash -LiteralPath $source.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            output = [IO.Path]::GetFileName($destination)
            output_bytes = (Get-Item -LiteralPath $destination).Length
            output_sha256 = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()
            elapsed_ms = $started.ElapsedMilliseconds
        }
        $resultPath = Join-Path $output.FullName ("case-{0:D3}.json" -f $index)
        [IO.File]::WriteAllText(
            $resultPath,
            (($result | ConvertTo-Json -Depth 10) -replace "`r`n", "`n") + "`n",
            $utf8NoBom
        )
        "converted|{0}|{1}|{2}|{3}" -f $index, $family, $started.ElapsedMilliseconds, $record.file
    }
}
finally {
    foreach ($family in @($applications.Keys)) {
        Stop-OfficeApplication $applications[$family]
    }
    if (Test-Path -LiteralPath $stageRoot) {
        $stageArchive = Join-Path $output.FullName "_worker-stage"
        Move-Item -LiteralPath $stageRoot -Destination $stageArchive
    }
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
}

"summary|{0}" -f $records.Count
