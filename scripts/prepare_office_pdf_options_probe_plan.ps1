[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$GoldenTaskFile,

    [Parameter(Mandatory = $true)]
    [string]$InputListFile,

    [Parameter(Mandatory = $true)]
    [string]$OutputPlanFile
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

function Assert-ExactProperties {
    param($Value, [string[]]$Names, [string]$Label)

    $actual = @($Value.PSObject.Properties.Name | Sort-Object)
    $expected = @($Names | Sort-Object)
    if ([string]::Join("`n", $actual) -ne [string]::Join("`n", $expected)) {
        throw "$Label properties must be exactly [$($expected -join ', ')]; got [$($actual -join ', ')]."
    }
}

function ConvertTo-CanonicalJson {
    param($Value)

    return ($Value | ConvertTo-Json -Depth 20 -Compress)
}

$taskPath = Get-Item -LiteralPath $GoldenTaskFile
$listPath = Get-Item -LiteralPath $InputListFile
if ($taskPath.PSIsContainer -or $listPath.PSIsContainer) {
    throw "GoldenTaskFile and InputListFile must be files."
}
if (-not (Test-WslPath $taskPath.FullName)) {
    throw "GoldenTaskFile must be a WSL path."
}
if (-not (Test-WslTempPath $listPath.FullName) -or -not (Test-WslTempPath $OutputPlanFile)) {
    throw "InputListFile and OutputPlanFile must be under WSL /tmp."
}
$provenancePath = $OutputPlanFile + ".provenance.json"
if ((Test-Path -LiteralPath $OutputPlanFile) -or (Test-Path -LiteralPath $provenancePath)) {
    throw "Output plan and provenance paths must not already exist."
}

$task = Get-Content -LiteralPath $taskPath.FullName -Raw -Encoding UTF8 | ConvertFrom-Json
Assert-ExactProperties $task @("assignment", "conversion") "golden task"
$assignment = $task.assignment
$conversion = $task.conversion
$officeNames = @(
    "bitmap_missing_fonts",
    "bookmarks",
    "include_document_properties",
    "page_from",
    "page_to",
    "pdf_a_1",
    "print_hidden_slides",
    "quality",
    "tagged_pdf"
)
Assert-ExactProperties $assignment.office $officeNames "assignment.office"
Assert-ExactProperties $conversion.office_options $officeNames "conversion.office_options"
$assignmentOptions = ConvertTo-CanonicalJson $assignment.office
$conversionOptions = ConvertTo-CanonicalJson $conversion.office_options
if ($assignmentOptions -cne $conversionOptions) {
    throw "The accepted assignment and conversion record do not contain identical Office options."
}
if ([string]$assignment.configuration_id -cne [string]$conversion.configuration_id) {
    throw "The accepted assignment and conversion record do not contain the same configuration ID."
}

$files = @()
$seen = @{}
foreach ($line in Get-Content -LiteralPath $listPath.FullName -Encoding UTF8) {
    if ([string]::IsNullOrWhiteSpace($line)) {
        continue
    }
    $file = $line.Trim()
    if ($file.Contains("\") -or $file.StartsWith("/") -or $file.EndsWith("/") -or
        $file.Contains("//") -or $file.Split("/") -contains "." -or
        $file.Split("/") -contains "..") {
        throw "Only normalized forward-slash input-relative paths are allowed: $file"
    }
    if ($seen.ContainsKey($file)) {
        throw "Each minimum input may appear only once: $file"
    }
    $seen[$file] = $true
    $files += $file
}
if ($files.Count -lt 1 -or $files.Count -gt 12) {
    throw "InputListFile must contain 1 to 12 paths."
}

$utf8NoBom = New-Object Text.UTF8Encoding($false)
$lines = foreach ($file in $files) {
    ConvertTo-CanonicalJson ([ordered]@{
        schema_version = 1
        file = $file
        options = $assignment.office
    })
}
[IO.File]::WriteAllText($OutputPlanFile, ([string]::Join("`n", $lines) + "`n"), $utf8NoBom)
$provenance = [ordered]@{
    schema_version = 1
    golden_task = $taskPath.FullName
    golden_task_sha256 = (Get-FileHash -LiteralPath $taskPath.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    configuration_id = [string]$assignment.configuration_id
    corpus = [string]$assignment.corpus
    file = [string]$assignment.file
    family = [string]$assignment.family
    options = $assignment.office
    input_files = $files
}
[IO.File]::WriteAllText(
    $provenancePath,
    (($provenance | ConvertTo-Json -Depth 20) -replace "`r`n", "`n") + "`n",
    $utf8NoBom
)

"prepared|{0}|{1}" -f $assignment.configuration_id, $files.Count
