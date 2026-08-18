[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$CorpusRoot,

    [Parameter(Mandatory = $true)]
    [string]$ListFile,

    [Parameter(Mandatory = $true)]
    [string]$OutputRoot
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

$corpus = Get-Item -LiteralPath $CorpusRoot
$list = Get-Item -LiteralPath $ListFile
$output = Get-Item -LiteralPath $OutputRoot
if (-not $corpus.PSIsContainer -or -not $output.PSIsContainer -or $list.PSIsContainer) {
    throw "CorpusRoot/OutputRoot must be directories and ListFile must be a file."
}
if (-not (Test-WslPath $corpus.FullName) -or -not $corpus.FullName.EndsWith(
    "\ooxmlsdk-test-suite\corpus",
    [StringComparison]::OrdinalIgnoreCase
)) {
    throw "CorpusRoot must be this test-suite's WSL corpus directory."
}
if (-not (Test-WslTempPath $list.FullName)) {
    throw "ListFile must be under WSL /tmp."
}
if (-not (Test-WslTempPath $output.FullName)) {
    throw "OutputRoot must be an existing WSL /tmp directory."
}

$relativePaths = @(
    Get-Content -LiteralPath $list.FullName -Encoding UTF8 |
        Where-Object { -not [String]::IsNullOrWhiteSpace($_) }
)
if ($relativePaths.Count -lt 1 -or $relativePaths.Count -gt 25) {
    throw "ListFile must contain 1 to 25 non-empty paths."
}

$corpusPrefix = $corpus.FullName.TrimEnd("\") + "\"
$word = $null
$document = $null
$started = [Diagnostics.Stopwatch]::StartNew()
$converted = 0

try {
    $word = New-Object -ComObject "Word.Application"
    $word.Visible = $false
    $word.DisplayAlerts = 0
    $word.AutomationSecurity = 3

    for ($index = 0; $index -lt $relativePaths.Count; $index += 1) {
        $relative = ([string]$relativePaths[$index]).Replace("/", "\")
        $segments = $relative.Split("\")
        if (
            [IO.Path]::IsPathRooted($relative) -or
            $segments -contains "." -or
            $segments -contains ".."
        ) {
            throw "Only normalized corpus-relative paths are allowed: $relative"
        }
        $input = Get-Item -LiteralPath (Join-Path $corpus.FullName $relative)
        if (
            $input.PSIsContainer -or
            -not $input.FullName.StartsWith($corpusPrefix, [StringComparison]::OrdinalIgnoreCase)
        ) {
            throw "Input is outside the allowed corpus: $relative"
        }
        if ($input.Extension -notin @(".doc", ".dot", ".docx", ".dotx", ".docm", ".dotm")) {
            throw "Input is not a supported Word document: $relative"
        }
        $pdf = Join-Path $output.FullName ("case-{0:D3}.pdf" -f $index)
        if (Test-Path -LiteralPath $pdf) {
            throw "Probe output already exists; use a new mktemp directory: $pdf"
        }

        $caseStarted = [Diagnostics.Stopwatch]::StartNew()
        try {
            $document = $word.Documents.Open($input.FullName, $false, $true, $false)
            $document.ExportAsFixedFormat($pdf, 17)
        }
        finally {
            if ($null -ne $document) {
                $document.Close(0)
                [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($document)
                $document = $null
            }
        }
        $converted += 1
        "converted|{0}|{1}|{2}" -f $index, $caseStarted.ElapsedMilliseconds, $relative
    }
}
finally {
    if ($null -ne $document) {
        $document.Close(0)
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($document)
    }
    if ($null -ne $word) {
        $word.Quit()
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($word)
    }
}

"summary|{0}|{1}" -f $converted, $started.ElapsedMilliseconds
