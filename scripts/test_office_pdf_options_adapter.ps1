[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Load declarations only: this test must never launch Office or execute the
# adapter's corpus/staging entry point.
$adapter = Join-Path $PSScriptRoot "probe_office_pdf_options.ps1"
$tokens = $null
$parseErrors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($adapter, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count -ne 0) { throw ($parseErrors | Out-String) }
foreach ($definition in $ast.FindAll({
    param($node)
    $node -is [Management.Automation.Language.FunctionDefinitionAst]
}, $false)) {
    . ([ScriptBlock]::Create($definition.Extent.Text))
}

function Assert-Equal($Actual, $Expected, [string]$Label) {
    if (($Actual | ConvertTo-Json -Depth 10 -Compress) -cne
        ($Expected | ConvertTo-Json -Depth 10 -Compress)) {
        throw "$Label mismatch: actual=$Actual expected=$Expected"
    }
}

foreach ($quality in @("print", "screen")) {
    foreach ($includeXps in @($false, $true)) {
        $script:calls = [Collections.Generic.List[object]]::new()
        $script:closed = $false
        $script:document = [pscustomobject]@{}
        $script:document | Add-Member ScriptMethod ComputeStatistics { return 1 }
        $script:document | Add-Member ScriptMethod ExportAsFixedFormat {
            $script:calls.Add($args.Clone())
        }
        $script:document | Add-Member ScriptMethod Close {
            Assert-Equal $args[0] 0 "close without saving"
            $script:closed = $true
        }
        $documents = [pscustomobject]@{}
        $documents | Add-Member ScriptMethod Open {
            Assert-Equal @($args) @("input.docx", $false, $true, $false) "read-only open"
            return $script:document
        }
        $application = [pscustomobject]@{ Documents = $documents }
        $options = [pscustomobject]@{
            quality = $quality
            include_document_properties = $true
            page_from = 1
            page_to = 1
            tagged_pdf = $false
            bookmarks = "word-bookmarks"
            pdf_a_1 = $false
            bitmap_missing_fonts = $false
            print_hidden_slides = $false
        }
        Export-WithWord $application "input.docx" "output.pdf" $options `
            -IncludeDiagnosticXps:$includeXps
        Assert-Equal $script:closed $true "document closed"
        Assert-Equal $script:calls.Count $(if ($includeXps) { 2 } else { 1 }) "call count"
        $qualityValue = if ($quality -eq "print") { 0 } else { 1 }
        $expected = @("output.pdf", 17, $false, $qualityValue, 3, 1, 1, 0,
            $true, $false, 2, $false, $false, $false)
        Assert-Equal $script:calls[0] $expected "unchanged PDF arguments"
        if ($includeXps) {
            $expected[0] = "output.xps"
            $expected[1] = 18
            Assert-Equal $script:calls[1] $expected "XPS differs only in path/format"
        }
    }
}

# Failure of the companion export must still close the read-only document.
$script:closed = $false
$script:document | Add-Member -Force ScriptMethod ExportAsFixedFormat {
    if ($args[1] -eq 18) { throw "expected companion failure" }
}
$caught = $false
try {
    Export-WithWord $application "input.docx" "output.pdf" $options -IncludeDiagnosticXps
} catch {
    if ($_.ToString() -notlike "*expected companion failure*") { throw }
    $caught = $true
}
Assert-Equal $caught $true "failure propagated"
Assert-Equal $script:closed $true "failure closes document"
# EMF is opt-in, after the unchanged fixed-format exports, and never changes
# their quality/range arguments. Mock only the byte transport, not PDF calls.
function Export-WordContentEmf {
    param($Document, [string]$OutputPath)
    Assert-Equal ([object]::ReferenceEquals($Document, $script:document)) $true "same open document"
    Assert-Equal $OutputPath "output.emf" "EMF companion path"
    Assert-Equal $script:calls.Count $script:expectedFixedCalls "EMF follows PDF/XPS"
    $script:emfCalls += 1
    if ($script:failEmf) { throw "expected EMF failure" }
}
$script:document | Add-Member -Force ScriptMethod ExportAsFixedFormat {
    $script:calls.Add($args.Clone())
}
foreach ($quality in @("print", "screen")) {
    foreach ($includeXps in @($false, $true)) {
        foreach ($includeEmf in @($false, $true)) {
            $script:calls = [Collections.Generic.List[object]]::new()
            $script:closed = $false
            $script:emfCalls = 0
            $script:failEmf = $false
            $script:expectedFixedCalls = if ($includeXps) { 2 } else { 1 }
            $options.quality = $quality
            Export-WithWord $application "input.docx" "output.pdf" $options `
                -IncludeDiagnosticXps:$includeXps -IncludeDiagnosticEmf:$includeEmf
            Assert-Equal $script:closed $true "EMF combination closes document"
            Assert-Equal $script:emfCalls ([int]$includeEmf) "EMF opt-in"
            Assert-Equal $script:calls.Count $script:expectedFixedCalls "fixed export count unchanged"
            $qualityValue = if ($quality -eq "print") { 0 } else { 1 }
            Assert-Equal $script:calls[0] @("output.pdf", 17, $false, $qualityValue, 3, 1, 1, 0,
                $true, $false, 2, $false, $false, $false) "EMF preserves PDF arguments"
        }
    }
}
$script:calls = [Collections.Generic.List[object]]::new()
$script:expectedFixedCalls = 1
$script:failEmf = $true
$script:closed = $false
$caught = $false
try {
    Export-WithWord $application "input.docx" "output.pdf" $options -IncludeDiagnosticEmf
} catch {
    if ($_.ToString() -notlike "*expected EMF failure*") { throw }
    $caught = $true
}
Assert-Equal $caught $true "EMF failure propagated"
Assert-Equal $script:closed $true "EMF failure closes document"
"PASS: Word PDF/XPS/EMF adapter arguments and cleanup"
