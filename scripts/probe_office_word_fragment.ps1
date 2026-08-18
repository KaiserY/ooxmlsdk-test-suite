[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$OutputRoot,

    [Parameter(Mandatory = $true)]
    [string]$Text,

    [ValidateSet("Rtl", "Ltr")]
    [string]$ParagraphDirection = "Ltr",

    [ValidateSet("Rtl", "Ltr")]
    [string]$RunDirection = "Ltr",

    [string]$FontName = "Arial",

    [string]$ComplexFontName = $FontName,

    [ValidateRange(1, 100)]
    [int]$FontSizePt = 11,

    [ValidateRange(1, 100)]
    [int]$ComplexFontSizePt = $FontSizePt,

    [ValidateRange(0, 65535)]
    [int]$LanguageId = 1033
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Test-WslTempPath {
    param([string]$Path)

    return $Path -match '^\\\\wsl(?:\.localhost|\$)\\[^\\]+\\tmp(?:\\|$)'
}

$output = Get-Item -LiteralPath $OutputRoot
if (-not $output.PSIsContainer -or -not (Test-WslTempPath $output.FullName)) {
    throw "OutputRoot must be an existing WSL /tmp directory."
}
$docxPath = Join-Path $output.FullName "fragment.docx"
$pdfPath = Join-Path $output.FullName "fragment.pdf"
if ((Test-Path -LiteralPath $docxPath) -or (Test-Path -LiteralPath $pdfPath)) {
    throw "Probe output already exists; use a new mktemp directory."
}

$word = $null
$document = $null
$range = $null
$font = $null
$paragraphFormat = $null
$selection = $null
$started = [Diagnostics.Stopwatch]::StartNew()
$createdMs = 0
$savedMs = 0
$exportedMs = 0

try {
    $word = New-Object -ComObject "Word.Application"
    $word.Visible = $false
    $word.DisplayAlerts = 0
    $word.AutomationSecurity = 3
    $document = $word.Documents.Add()
    $createdMs = $started.ElapsedMilliseconds

    $range = $document.Range(0, 0)
    $range.Text = $Text
    $font = $range.Font
    $font.Name = $FontName
    $font.NameBi = $ComplexFontName
    $font.Size = $FontSizePt
    $font.SizeBi = $ComplexFontSizePt
    $range.LanguageID = $LanguageId
    $paragraphFormat = $range.ParagraphFormat
    $selection = $word.Selection
    $selection.SetRange($range.Start, $range.End)
    if ($ParagraphDirection -eq "Rtl") {
        $paragraphFormat.Alignment = 2
        $paragraphFormat.ReadingOrder = 1
        $selection.RtlPara()
    }
    else {
        $paragraphFormat.Alignment = 0
        $paragraphFormat.ReadingOrder = 0
        $selection.LtrPara()
    }
    if ($RunDirection -eq "Rtl") {
        $selection.RtlRun()
    }
    else {
        $selection.LtrRun()
    }

    $document.SaveAs2($docxPath, 16)
    $savedMs = $started.ElapsedMilliseconds
    $document.ExportAsFixedFormat($pdfPath, 17)
    $exportedMs = $started.ElapsedMilliseconds
}
finally {
    foreach ($value in @($selection, $paragraphFormat, $font, $range)) {
        if ($null -ne $value -and [Runtime.InteropServices.Marshal]::IsComObject($value)) {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($value)
        }
    }
    if ($null -ne $document) {
        $document.Close(0)
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($document)
    }
    if ($null -ne $word) {
        $word.Quit()
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($word)
    }
}

[ordered]@{
    docx = $docxPath
    pdf = $pdfPath
    paragraph_direction = $ParagraphDirection
    run_direction = $RunDirection
    word_start_and_document_ms = $createdMs
    save_docx_ms = $savedMs - $createdMs
    export_pdf_ms = $exportedMs - $savedMs
    total_ms = $started.ElapsedMilliseconds
} | ConvertTo-Json -Compress
