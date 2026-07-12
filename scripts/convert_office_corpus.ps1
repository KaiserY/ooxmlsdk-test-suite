[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$CorpusRoot,

    [Parameter(Mandatory = $true)]
    [string]$OutputRoot,

    [Parameter(Mandatory = $true)]
    [string]$ListFile,

    [string]$AnnotationsFile,

    [ValidateRange(1, 1000)]
    [int]$RecycleEvery = 25,

    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not $AnnotationsFile) {
    $AnnotationsFile = Join-Path $PSScriptRoot "office_corpus_annotations.jsonl"
}

$wordExtensions = @(".doc", ".dot", ".docx", ".dotx", ".docm", ".dotm")
$excelExtensions = @(".xls", ".xlt", ".xlsx", ".xltx", ".xlsm", ".xltm", ".xlsb")
$powerPointExtensions = @(
    ".ppt", ".pps", ".pot", ".pptx", ".ppsx", ".potx", ".pptm", ".ppsm", ".potm"
)
$utf8NoBom = New-Object Text.UTF8Encoding($false)
$manifestSchemaVersion = 2
$generatorVersion = 3
$referenceEngine = "Microsoft Office"
$exportProfile = "office-fixed-format-print-no-macros-no-links-v2"

function Get-TextSha256 {
    param([string]$Text)

    $algorithm = [Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
        return ([BitConverter]::ToString($algorithm.ComputeHash($bytes))).Replace("-", "").ToLowerInvariant()
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
    $count = 0
    foreach ($root in $roots) {
        if (-not (Test-Path -LiteralPath $root)) {
            continue
        }
        foreach ($font in Get-ChildItem -LiteralPath $root -File | Sort-Object Name) {
            $entries.Add(("{0}|{1}|{2}|{3}" -f $root, $font.Name, $font.Length, $font.LastWriteTimeUtc.Ticks))
            $count += 1
        }
    }
    return [ordered]@{
        file_count = $count
        fingerprint_kind = "name-size-mtime"
        sha256 = Get-TextSha256 ([string]::Join("`n", $entries))
    }
}

function Get-ReferenceEnvironment {
    $windows = Get-ItemProperty -LiteralPath "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion"
    $office = Get-ItemProperty -LiteralPath "HKLM:\SOFTWARE\Microsoft\Office\ClickToRun\Configuration" -ErrorAction SilentlyContinue
    $printerSettings = $null
    try {
        Add-Type -AssemblyName System.Drawing
        $printerSettings = New-Object Drawing.Printing.PrinterSettings
    }
    catch {
        $printerSettings = $null
    }

    $fullBuild = "{0}.{1}" -f $windows.CurrentBuildNumber, $windows.UBR
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

    return [ordered]@{
        schema_version = 1
        reference_engine = $referenceEngine
        generator_version = $generatorVersion
        export_profile = $exportProfile
        windows = [ordered]@{
            full_build = $fullBuild
        }
        office = [ordered]@{
            platform = if ($null -ne $office) { [string]$office.Platform } else { "" }
            version_to_report = if ($null -ne $office) { [string]$office.VersionToReport } else { "" }
        }
        locale = [ordered]@{
            culture = [Globalization.CultureInfo]::CurrentCulture.Name
            ui_culture = [Globalization.CultureInfo]::CurrentUICulture.Name
            time_zone = [TimeZoneInfo]::Local.Id
        }
        default_paper = $defaultPaper
        fonts = Get-FontFingerprint
        dependencies = @(
            "office-build",
            "windows-build",
            "installed-font-files",
            "locale-and-time-zone",
            "default-paper"
        )
    }
}

$corpus = (Get-Item -LiteralPath $CorpusRoot).FullName.TrimEnd("\", "/")
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
$output = (Get-Item -LiteralPath $OutputRoot).FullName.TrimEnd("\", "/")

$environment = Get-ReferenceEnvironment
$environmentJson = $environment | ConvertTo-Json -Compress -Depth 10
$environmentId = Get-TextSha256 $environmentJson
$environmentPath = Join-Path $output "environment.json"
$environmentDocument = [ordered]@{
    schema_version = 1
    environment_id = $environmentId
    observed_at_utc = [DateTime]::UtcNow.ToString("o")
    environment = $environment
}
$environmentTemporaryPath = $environmentPath + ".tmp"
$environmentDocumentJson = ($environmentDocument | ConvertTo-Json -Depth 10) -replace "`r`n", "`n"
[IO.File]::WriteAllText(
    $environmentTemporaryPath,
    $environmentDocumentJson + "`n",
    $utf8NoBom
)
Move-Item -LiteralPath $environmentTemporaryPath -Destination $environmentPath -Force

$manifestRecords = @{}
$manifestPaths = @{}
$configuredAnnotations = @{}
if ($AnnotationsFile -and (Test-Path -LiteralPath $AnnotationsFile)) {
    foreach ($line in Get-Content -LiteralPath $AnnotationsFile -Encoding UTF8) {
        if ([string]::IsNullOrWhiteSpace($line)) {
            continue
        }
        $annotationRecord = $line | ConvertFrom-Json
        $key = [string]$annotationRecord.file
        if ($configuredAnnotations.ContainsKey($key)) {
            throw "Duplicate annotation entry: $key"
        }
        $configuredAnnotations[$key] = @($annotationRecord.annotations)
    }
}

function Release-ComObject {
    param($Value)

    if ($null -ne $Value -and [Runtime.InteropServices.Marshal]::IsComObject($Value)) {
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($Value)
    }
}

function Get-ApplicationFamily {
    param([string]$Extension)

    if ($wordExtensions -contains $Extension) {
        return "Word"
    }
    if ($excelExtensions -contains $Extension) {
        return "Excel"
    }
    if ($powerPointExtensions -contains $Extension) {
        return "PowerPoint"
    }
    return $null
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
    throw "Unsupported Office application family: $Family"
}

function Stop-OfficeApplication {
    param($Application, [string]$Family)

    if ($null -eq $Application) {
        return
    }
    try {
        $Application.Quit()
    }
    catch {
        Write-Warning ("Failed to quit {0}: {1}" -f $Family, $_.Exception.Message)
    }
    Release-ComObject $Application
}

function Export-WithWord {
    param($Application, [string]$InputPath, [string]$OutputPath)

    $document = $null
    try {
        $document = $Application.Documents.Open($InputPath, $false, $true, $false)
        $document.ExportAsFixedFormat($OutputPath, 17)
    }
    finally {
        if ($null -ne $document) {
            $document.Close(0)
            Release-ComObject $document
        }
    }
}

function Export-WithExcel {
    param($Application, [string]$InputPath, [string]$OutputPath)

    $workbook = $null
    try {
        $workbook = $Application.Workbooks.Open($InputPath, 0, $true)
        $workbook.ExportAsFixedFormat(0, $OutputPath)
    }
    finally {
        if ($null -ne $workbook) {
            $workbook.Close($false)
            Release-ComObject $workbook
        }
    }
}

function Export-WithPowerPoint {
    param($Application, [string]$InputPath, [string]$OutputPath)

    $presentation = $null
    try {
        $presentation = $Application.Presentations.Open($InputPath, -1, 0, 0)
        # ppSaveAsPDF (32) uses PowerPoint's native fixed-format exporter and has
        # a simpler COM signature than ExportAsFixedFormat on PowerShell 5.1.
        $presentation.SaveAs($OutputPath, 32)
    }
    finally {
        if ($null -ne $presentation) {
            $presentation.Close()
            Release-ComObject $presentation
        }
    }
}

function Assert-Pdf {
    param([string]$Path)

    $stream = [IO.File]::OpenRead($Path)
    try {
        if ($stream.Length -lt 5) {
            throw "PDF output is shorter than its header"
        }
        $header = New-Object byte[] 5
        [void]$stream.Read($header, 0, $header.Length)
        if ([Text.Encoding]::ASCII.GetString($header) -ne "%PDF-") {
            throw "Output does not start with a PDF header"
        }

        # Native Office PDFs keep the root page-tree object near the beginning
        # of the file. Reject Excel's valid-but-empty zero-page export, which is
        # not useful as a visible-output reference and is rejected by pdfinfo.
        $stream.Position = 0
        $scanLength = [int][Math]::Min($stream.Length, 8MB)
        $buffer = New-Object byte[] $scanLength
        $bytesRead = $stream.Read($buffer, 0, $buffer.Length)
        $prefix = [Text.Encoding]::ASCII.GetString($buffer, 0, $bytesRead)
        $pageTree = [Text.RegularExpressions.Regex]::Match(
            $prefix,
            "/Type\s*/Pages\b(?:(?!endobj).)*?/Count\s+(\d+)",
            [Text.RegularExpressions.RegexOptions]::Singleline
        )
        if (-not $pageTree.Success) {
            throw "Could not verify the Office PDF page count"
        }
        if ([int64]$pageTree.Groups[1].Value -lt 1) {
            throw "Office exported a zero-page PDF"
        }
    }
    finally {
        $stream.Dispose()
    }
}

function Protect-ErrorMessage {
    param([string]$Message)

    if ([string]::IsNullOrEmpty($Message)) {
        return ""
    }
    return ($Message -replace '(?i)[A-Z]:\\Users\\[^\\\r\n]+\\', 'C:\Users\...\')
}

function Get-FailureClass {
    param([string]$Message)

    if ($Message -match "zero-page PDF|verify the Office PDF page count") {
        return "invalid-pdf-output"
    }
    if ($Message -match "\u6587\u4ef6\u963b\u6b62|File Block") {
        return "office-file-block"
    }
    if ($Message -match "\u5e2e\u52a9\u4fdd\u62a4|Protected View|detected.*problem") {
        return "office-security-rejected"
    }
    if ($Message -match "\u5bc6\u7801|password|encrypted|encryption") {
        return "encrypted-or-password-protected"
    }
    if ($Message -match "\u635f\u574f|\u65e0\u6548|corrupt|invalid file format") {
        return "invalid-or-corrupt-source"
    }
    if ($Message -match "\u5bfc\u51fa\u5931\u8d25|\u50a8\u5b58\u6b64\u6587\u4ef6|ExportAsFixedFormat|SaveAs") {
        return "pdf-export-failed"
    }
    return "office-conversion-error"
}

function Initialize-Manifest {
    param([string]$CorpusName)

    if ($manifestRecords.ContainsKey($CorpusName)) {
        return
    }

    $corpusOutput = Join-Path $output $CorpusName
    New-Item -ItemType Directory -Force -Path $corpusOutput | Out-Null
    $manifestPath = Join-Path $corpusOutput "manifest.jsonl"
    $records = @{}
    if (Test-Path -LiteralPath $manifestPath) {
        foreach ($line in Get-Content -LiteralPath $manifestPath -Encoding UTF8) {
            if (-not [string]::IsNullOrWhiteSpace($line)) {
                $record = $line | ConvertFrom-Json
                if ($record.schema_version -eq 1) {
                    $record.schema_version = $manifestSchemaVersion
                    $record | Add-Member -NotePropertyName reference_engine -NotePropertyValue $referenceEngine
                    $record | Add-Member -NotePropertyName source_extension -NotePropertyValue ([IO.Path]::GetExtension([string]$record.file).TrimStart(".").ToLowerInvariant())
                    $record | Add-Member -NotePropertyName source_bytes -NotePropertyValue 0
                    $record | Add-Member -NotePropertyName environment_id -NotePropertyValue ""
                    $record | Add-Member -NotePropertyName annotations -NotePropertyValue @()
                    $record.PSObject.Properties.Remove("windows_build")
                    $record.PSObject.Properties.Remove("locale")
                }
                elseif ($record.schema_version -ne $manifestSchemaVersion) {
                    throw "Unsupported schema_version in $manifestPath"
                }
                if ($record.PSObject.Properties.Name -notcontains "annotations") {
                    $record | Add-Member -NotePropertyName annotations -NotePropertyValue @()
                }
                if ($record.PSObject.Properties.Name -notcontains "failure_class") {
                    $failureClass = if ($record.status -eq "failed") {
                        Get-FailureClass ([string]$record.error)
                    }
                    else {
                        ""
                    }
                    $record | Add-Member -NotePropertyName failure_class -NotePropertyValue $failureClass
                }
                elseif ($record.status -eq "failed" -and $record.failure_class -eq "office-conversion-error") {
                    $record.failure_class = Get-FailureClass ([string]$record.error)
                }
                if ($record.PSObject.Properties.Name -notcontains "error_code") {
                    $record | Add-Member -NotePropertyName error_code -NotePropertyValue ""
                }
                if ($record.PSObject.Properties.Name -notcontains "attempts") {
                    $record | Add-Member -NotePropertyName attempts -NotePropertyValue 1
                }
                if ($record.PSObject.Properties.Name -contains "error") {
                    $record.error = Protect-ErrorMessage ([string]$record.error)
                }
                $records[[string]$record.file] = $record
            }
        }
    }
    $manifestRecords[$CorpusName] = $records
    $manifestPaths[$CorpusName] = $manifestPath
}

function Write-Manifest {
    param([string]$CorpusName)

    $manifestPath = $manifestPaths[$CorpusName]
    $temporaryPath = $manifestPath + ".tmp"
    $builder = New-Object Text.StringBuilder
    foreach ($file in @($manifestRecords[$CorpusName].Keys | Sort-Object)) {
        [void]$builder.Append(($manifestRecords[$CorpusName][$file] | ConvertTo-Json -Compress -Depth 10))
        [void]$builder.Append("`n")
    }
    [IO.File]::WriteAllText($temporaryPath, $builder.ToString(), $utf8NoBom)
    Move-Item -LiteralPath $temporaryPath -Destination $manifestPath -Force
}

function Save-ManifestRecord {
    param(
        [string]$CorpusName,
        [string]$File,
        [System.Collections.IDictionary]$Record
    )

    Initialize-Manifest $CorpusName
    $manifestRecords[$CorpusName][$File] = [pscustomobject]$Record
    Write-Manifest $CorpusName
}

function Write-ProgressRecord {
    param([string]$Status, [string]$Family, [string]$Relative)

    Write-Output ("{0}|{1}|{2}" -f $Status, $Family, $Relative.Replace("\", "/"))
}

$files = foreach ($line in Get-Content -LiteralPath $ListFile) {
    $relative = $line.Trim()
    if ($relative -and -not $relative.StartsWith("#")) {
        $relative = $relative.Replace("/", "\")
        $item = Get-Item -LiteralPath (Join-Path $corpus $relative)
        if (-not $item.FullName.StartsWith($corpus + "\", [StringComparison]::OrdinalIgnoreCase)) {
            throw "List entry escapes the corpus root: $relative"
        }
        $item
    }
}
$duplicateFiles = @($files | Group-Object FullName | Where-Object Count -gt 1)
if ($duplicateFiles.Count -ne 0) {
    throw "List file contains duplicate entries: $($duplicateFiles.Name -join ', ')"
}
$files = @($files | Sort-Object FullName)

$applications = @{}
$applicationCounts = @{}
$stageRoot = Join-Path $env:TEMP ("ooxmlsdk-office-conv-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $stageRoot | Out-Null

try {
    foreach ($source in $files) {
        $started = Get-Date
        $relative = $source.FullName.Substring($corpus.Length).TrimStart([char[]]@("\", "/"))
        $relativeParts = $relative.Split([char[]]@("\", "/"), 2)
        if ($relativeParts.Count -ne 2) {
            throw "Office corpus file is not below a corpus source directory: $relative"
        }
        $corpusName = $relativeParts[0]
        $sourceWithinCorpus = $relativeParts[1].Replace("\", "/")
        Initialize-Manifest $corpusName

        $extension = $source.Extension.ToLowerInvariant()
        $family = Get-ApplicationFamily $extension
        $relativeDirectory = Split-Path -Parent $sourceWithinCorpus
        $destinationDirectory = if ($relativeDirectory) {
            Join-Path (Join-Path $output $corpusName) $relativeDirectory
        }
        else {
            Join-Path $output $corpusName
        }
        # Retaining the source extension keeps the mapping one-to-one when both
        # sample.doc and sample.docx exist in one corpus directory.
        $outputWithinCorpus = if ($relativeDirectory) {
            (Join-Path $relativeDirectory ($source.Name + ".pdf")).Replace("\", "/")
        }
        else {
            $source.Name + ".pdf"
        }
        $destination = Join-Path (Join-Path $output $corpusName) $outputWithinCorpus
        $sourceHash = (Get-FileHash -LiteralPath $source.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        $existing = $manifestRecords[$corpusName][$sourceWithinCorpus]
        $annotations = @()
        if ($null -ne $existing -and $existing.PSObject.Properties.Name -contains "annotations") {
            $annotations = @($existing.annotations)
        }
        $annotationKey = $relative.Replace("\", "/")
        if ($configuredAnnotations.ContainsKey($annotationKey)) {
            $annotations = @($configuredAnnotations[$annotationKey])
        }

        if (-not $family) {
            Save-ManifestRecord $corpusName $sourceWithinCorpus ([ordered]@{
                schema_version = $manifestSchemaVersion
                file = $sourceWithinCorpus
                source_extension = $extension.TrimStart(".")
                source_bytes = $source.Length
                source_sha256 = $sourceHash
                status = "unsupported"
                reference_engine = $referenceEngine
                environment_id = $environmentId
                application = ""
                application_version = ""
                application_build = ""
                export_profile = $exportProfile
                output = ""
                output_bytes = 0
                output_sha256 = ""
                converted_at_utc = [DateTime]::UtcNow.ToString("o")
                elapsed_ms = 0
                attempts = 0
                annotations = $annotations
                failure_class = "unsupported-extension"
                error_code = ""
                error = "Unsupported extension: $extension"
            })
            Write-ProgressRecord "unsupported" "" $relative
            continue
        }

        if (-not $Force -and $null -ne $existing -and
            $existing.status -eq "converted" -and
            $existing.source_sha256 -eq $sourceHash -and
            $existing.environment_id -eq $environmentId -and
            (Test-Path -LiteralPath $destination)) {
            try {
                Assert-Pdf $destination
                $outputHash = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()
                if ($outputHash -eq $existing.output_sha256) {
                    if ($configuredAnnotations.ContainsKey($annotationKey)) {
                        $existing.annotations = $annotations
                        Write-Manifest $corpusName
                    }
                    Write-ProgressRecord "skipped" $family $relative
                    continue
                }
            }
            catch {
                # A missing, truncated, or changed PDF is regenerated below.
            }
        }

        if ($applications.ContainsKey($family) -and $applicationCounts[$family] -ge $RecycleEvery) {
            Stop-OfficeApplication $applications[$family] $family
            $applications.Remove($family)
            $applicationCounts.Remove($family)
            [GC]::Collect()
            [GC]::WaitForPendingFinalizers()
        }
        if (-not $applications.ContainsKey($family)) {
            $applications[$family] = New-OfficeApplication $family
            $applicationCounts[$family] = 0
        }
        $application = $applications[$family]
        $applicationCounts[$family] = [int]$applicationCounts[$family] + 1
        $applicationVersion = [string]$application.Version
        $applicationBuild = try { [string]$application.Build } catch { "" }

        $stageDirectory = Join-Path $stageRoot ([Guid]::NewGuid().ToString("N"))
        New-Item -ItemType Directory -Path $stageDirectory | Out-Null
        $stageInput = Join-Path $stageDirectory $source.Name
        $stageOutput = Join-Path $stageDirectory ($source.BaseName + ".pdf")
        $attempts = 0

        try {
            Copy-Item -LiteralPath $source.FullName -Destination $stageInput
            while ($attempts -lt 2) {
                $attempts += 1
                try {
                    switch ($family) {
                        "Word" { Export-WithWord $application $stageInput $stageOutput }
                        "Excel" { Export-WithExcel $application $stageInput $stageOutput }
                        "PowerPoint" { Export-WithPowerPoint $application $stageInput $stageOutput }
                    }
                    Assert-Pdf $stageOutput
                    break
                }
                catch {
                    if ($attempts -ge 2) {
                        throw
                    }
                    Stop-OfficeApplication $application $family
                    $applications.Remove($family)
                    $applicationCounts.Remove($family)
                    $application = New-OfficeApplication $family
                    $applications[$family] = $application
                    $applicationCounts[$family] = 1
                    $applicationVersion = [string]$application.Version
                    $applicationBuild = try { [string]$application.Build } catch { "" }
                    Remove-Item -LiteralPath $stageOutput -Force -ErrorAction SilentlyContinue
                }
            }
            New-Item -ItemType Directory -Force -Path $destinationDirectory | Out-Null
            Copy-Item -LiteralPath $stageOutput -Destination $destination -Force
            $pdf = Get-Item -LiteralPath $destination
            $outputHash = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()
            Save-ManifestRecord $corpusName $sourceWithinCorpus ([ordered]@{
                schema_version = $manifestSchemaVersion
                file = $sourceWithinCorpus
                source_extension = $extension.TrimStart(".")
                source_bytes = $source.Length
                source_sha256 = $sourceHash
                status = "converted"
                reference_engine = $referenceEngine
                environment_id = $environmentId
                application = $family
                application_version = $applicationVersion
                application_build = $applicationBuild
                export_profile = $exportProfile
                output = $outputWithinCorpus
                output_bytes = $pdf.Length
                output_sha256 = $outputHash
                converted_at_utc = [DateTime]::UtcNow.ToString("o")
                elapsed_ms = [int]((Get-Date) - $started).TotalMilliseconds
                attempts = $attempts
                annotations = $annotations
                failure_class = ""
                error_code = ""
                error = ""
            })
            Write-ProgressRecord "converted" $family $relative
        }
        catch {
            $errorMessage = Protect-ErrorMessage $_.Exception.Message
            # Do not leave a stale or structurally invalid artifact behind a
            # failed record. The manifest remains the evidence for the failure.
            Remove-Item -LiteralPath $destination -Force -ErrorAction SilentlyContinue
            Save-ManifestRecord $corpusName $sourceWithinCorpus ([ordered]@{
                schema_version = $manifestSchemaVersion
                file = $sourceWithinCorpus
                source_extension = $extension.TrimStart(".")
                source_bytes = $source.Length
                source_sha256 = $sourceHash
                status = "failed"
                reference_engine = $referenceEngine
                environment_id = $environmentId
                application = $family
                application_version = $applicationVersion
                application_build = $applicationBuild
                export_profile = $exportProfile
                output = ""
                output_bytes = 0
                output_sha256 = ""
                converted_at_utc = [DateTime]::UtcNow.ToString("o")
                elapsed_ms = [int]((Get-Date) - $started).TotalMilliseconds
                attempts = $attempts
                annotations = $annotations
                failure_class = (Get-FailureClass $errorMessage)
                error_code = ("0x{0:X8}" -f ($_.Exception.HResult -band 0xFFFFFFFFL))
                error = $errorMessage
            })
            Write-ProgressRecord "failed" $family $relative
        }
        finally {
            Remove-Item -LiteralPath $stageDirectory -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}
finally {
    foreach ($family in @($applications.Keys)) {
        Stop-OfficeApplication $applications[$family] $family
    }
    Remove-Item -LiteralPath $stageRoot -Recurse -Force -ErrorAction SilentlyContinue
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
}

foreach ($corpusName in @($manifestRecords.Keys | Sort-Object)) {
    Write-Manifest $corpusName
    Write-Output ("manifest|{0}" -f $manifestPaths[$corpusName])
}
