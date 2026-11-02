[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$OutputPath,

    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[0-9a-f]{64}$')]
    [string]$ProbeScriptSha256
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$campaignId = "office-ooxml-pdf-options-v2"
$converterVersion = 2
. (Join-Path $PSScriptRoot "office_pdf_environment.ps1")
$utf8NoBom = New-Object Text.UTF8Encoding($false)

function Get-TextSha256 {
    param([string]$Text)

    $algorithm = [Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
        return ([BitConverter]::ToString($algorithm.ComputeHash($bytes))).Replace(
            "-",
            ""
        ).ToLowerInvariant()
    }
    finally {
        $algorithm.Dispose()
    }
}

function Get-FontFingerprint {
    $roots = @(
        (Join-Path $env:WINDIR "Fonts"),
        (Join-Path $env:LOCALAPPDATA "Microsoft\Windows\Fonts")
    )
    $entries = New-Object Collections.Generic.List[string]
    foreach ($root in $roots) {
        if (-not (Test-Path -LiteralPath $root)) {
            continue
        }
        foreach ($font in Get-ChildItem -LiteralPath $root -File | Sort-Object Name) {
            $entries.Add((
                "{0}|{1}|{2}|{3}" -f
                $root,
                $font.Name,
                $font.Length,
                $font.LastWriteTimeUtc.Ticks
            ))
        }
    }
    return [ordered]@{
        file_count = $entries.Count
        fingerprint_kind = "name-size-mtime"
        sha256 = Get-TextSha256 ([string]::Join("`n", $entries))
    }
}

$windows = Get-ItemProperty -LiteralPath "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion"
$office = Get-ItemProperty `
    -LiteralPath "HKLM:\SOFTWARE\Microsoft\Office\ClickToRun\Configuration" `
    -ErrorAction SilentlyContinue
$printerSettings = $null
try {
    Add-Type -AssemblyName System.Drawing
    $printerSettings = New-Object Drawing.Printing.PrinterSettings
}
catch {
    $printerSettings = $null
}

$defaultPaper = [ordered]@{
    name = ""
    width_hundredths_inch = 0
    height_hundredths_inch = 0
    landscape = $false
}
if ($null -ne $printerSettings -and $printerSettings.IsValid) {
    $paper = $printerSettings.DefaultPageSettings.PaperSize
    $defaultPaper = [ordered]@{
        name = [string]$paper.PaperName
        width_hundredths_inch = [int]$paper.Width
        height_hundredths_inch = [int]$paper.Height
        landscape = [bool]$printerSettings.DefaultPageSettings.Landscape
    }
}

if (@(Get-Process WINWORD,EXCEL,POWERPNT -ErrorAction SilentlyContinue).Count) {
    throw 'Close Office applications before capturing the campaign environment.'
}
$applications = @()
foreach ($family in @('Word','Excel','PowerPoint')) {
    $app = $null
    try {
        $app = New-Object -ComObject "$family.Application"
        $observed = Get-OfficeRenderEnvironment $app
        $applications += [ordered]@{name=$family;ui_language=$observed.ui_language;version=[string]$app.Version}
    } finally {
        if ($null -ne $app) { $app.Quit(); [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($app) }
    }
}
$uiLanguages = @(Get-OfficeUiLanguages)
$environment = [ordered]@{
    schema_version = 1
    campaign_id = $campaignId
    converter_version = $converterVersion
    probe_script_sha256 = $ProbeScriptSha256
    controller_script_sha256 = (Get-FileHash (Join-Path $PSScriptRoot 'invoke_office_pdf_worker.ps1') -Algorithm SHA256).Hash.ToLowerInvariant()
    environment_helper_sha256 = (Get-FileHash (Join-Path $PSScriptRoot 'office_pdf_environment.ps1') -Algorithm SHA256).Hash.ToLowerInvariant()
    reference_engine = "Microsoft Office"
    windows = [ordered]@{
        product_name = [string]$windows.ProductName
        display_version = [string]$windows.DisplayVersion
        full_build = "{0}.{1}" -f $windows.CurrentBuildNumber, $windows.UBR
    }
    office = [ordered]@{
        ui_languages = $uiLanguages
        applications = $applications
        platform = if ($null -ne $office) { [string]$office.Platform } else { "" }
        product_release_ids = if ($null -ne $office) {
            [string]$office.ProductReleaseIds
        }
        else { "" }
        version_to_report = if ($null -ne $office) {
            [string]$office.VersionToReport
        }
        else { "" }
    }
    locale = [ordered]@{
        culture = [Globalization.CultureInfo]::CurrentCulture.Name
        ui_culture = $applications[0].ui_language
        document_language = Get-OfficeAuthoringLanguage
        iana_time_zone = $observed.time_zone
        time_zone = [TimeZoneInfo]::Local.Id
    }
    default_paper = $defaultPaper
    fonts = Get-FontFingerprint
    dependencies = @(
        "probe-script",
        "environment-controller",
        "office-build",
        "windows-build",
        "installed-font-files",
        "locale-and-time-zone",
        "default-paper",
        "per-source-office-export-configuration"
    )
}
$environmentJson = $environment | ConvertTo-Json -Compress -Depth 10
$document = [ordered]@{
    schema_version = 1
    environment_id = Get-TextSha256 $environmentJson
    observed_at_utc = [DateTime]::UtcNow.ToString("o")
    environment = $environment
}
$json = ($document | ConvertTo-Json -Depth 10) -replace "`r`n", "`n"
[IO.File]::WriteAllText($OutputPath, $json + "`n", $utf8NoBom)
Write-Output ("environment|{0}" -f $document.environment_id)
