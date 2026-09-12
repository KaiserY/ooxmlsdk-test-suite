[CmdletBinding()]
param(
    [string]$CorpusRoot,
    [string]$PlanFile,
    [string]$OutputRoot,
    [string]$ProcessIdFile,
    [int]$TimeoutSeconds = 120,
    [string]$RestoreSnapshot
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'office_pdf_environment.ps1')
if ($RestoreSnapshot) {
    if (Test-Path -LiteralPath $RestoreSnapshot) {
        Restore-OfficeLanguageSnapshot (Get-Content -LiteralPath $RestoreSnapshot -Raw | ConvertFrom-Json)
    }
    exit 0
}
if ($TimeoutSeconds -le 0) { throw 'TimeoutSeconds must be positive.' }
if (@(Get-Process WINWORD,EXCEL,POWERPNT -ErrorAction SilentlyContinue).Count) {
    throw 'Close existing Office applications before running the serial campaign.'
}
$records = @(Get-Content -LiteralPath $PlanFile | Where-Object { $_.Trim() } | ForEach-Object { $_ | ConvertFrom-Json })
if ($records.Count -ne 1 -or $records[0].schema_version -ne 2) {
    throw 'The environment-controlled worker requires exactly one schema-2 record.'
}
$record = $records[0]
$workerRoot = Split-Path -Parent $PlanFile
$snapshotPath = Join-Path $workerRoot 'office-language-snapshot.json'
$snapshot = Get-OfficeLanguageSnapshot
$snapshot | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $snapshotPath -Encoding utf8
$worker = $null
$workerExit = 1
try {
    Set-OfficeUiLanguage $record.environment.ui_language
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = Join-Path $PSHOME 'pwsh.exe'
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.StandardOutputEncoding = [Text.UTF8Encoding]::new($false)
    $start.StandardErrorEncoding = [Text.UTF8Encoding]::new($false)
    foreach ($argument in @('-NoProfile','-NonInteractive','-STA','-ExecutionPolicy','Bypass',
        '-File',(Join-Path $PSScriptRoot 'probe_office_pdf_options.ps1'),
        '-CorpusRoot',$CorpusRoot,'-PlanFile',$PlanFile,'-OutputRoot',$OutputRoot,
        '-ProcessIdFile',$ProcessIdFile)) { $start.ArgumentList.Add($argument) }
    $worker = [Diagnostics.Process]::Start($start)
    [IO.File]::WriteAllText((Join-Path $workerRoot 'controller-child-process-id.txt'),[string]$worker.Id)
    $stdout = $worker.StandardOutput.ReadToEndAsync()
    $stderr = $worker.StandardError.ReadToEndAsync()
    if (-not $worker.WaitForExit($TimeoutSeconds * 1000)) {
        $worker.Kill($true)
        $worker.WaitForExit()
        $workerExit = 124
    } else { $workerExit = $worker.ExitCode }
    [IO.File]::WriteAllText((Join-Path $workerRoot 'office-stdout.log'),$stdout.GetAwaiter().GetResult())
    [IO.File]::WriteAllText((Join-Path $workerRoot 'office-stderr.log'),$stderr.GetAwaiter().GetResult())
    if ($workerExit -ne 0) {
        Write-Output (Get-Content (Join-Path $workerRoot 'office-stderr.log') -Raw)
        Write-Output "Office worker exit code: $workerExit"
    }
} finally {
    if ($null -ne $worker) {
        if (-not $worker.HasExited) { $worker.Kill($true); $worker.WaitForExit() }
        $worker.Dispose()
    }
    if ($ProcessIdFile -and (Test-Path -LiteralPath $ProcessIdFile)) {
        $officePid = [int](Get-Content -LiteralPath $ProcessIdFile -Raw)
        $officeProcess = Get-Process -Id $officePid -ErrorAction SilentlyContinue
        if ($null -ne $officeProcess -and $officeProcess.ProcessName -in @('WINWORD','EXCEL','POWERPNT')) {
            Stop-Process -Id $officePid -Force
        }
    }
    Restore-OfficeLanguageSnapshot $snapshot
    [IO.File]::WriteAllText((Join-Path $workerRoot 'environment-restored.json'),'{"restored":true}')
}
exit $workerExit
