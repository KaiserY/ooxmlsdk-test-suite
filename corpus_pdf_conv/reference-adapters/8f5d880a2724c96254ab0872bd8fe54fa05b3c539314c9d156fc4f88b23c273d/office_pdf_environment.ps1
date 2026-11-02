# Shared Office-only environment controls. Never change Windows culture or clock.
function Get-OfficeLanguageKey {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey(
        'Software\Microsoft\Office\16.0\Common\LanguageResources', $true)
    if ($null -eq $key) { throw 'Office language preferences are unavailable.' }
    return $key
}

function Get-OfficeLanguageSnapshot {
    $key = Get-OfficeLanguageKey
    try {
        $values = @(foreach ($name in $key.GetValueNames()) {
            [ordered]@{name=$name;kind=[string]$key.GetValueKind($name)
                value=$key.GetValue($name,$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)}
        })
        return ,$values
    } finally { $key.Dispose() }
}

function Restore-OfficeLanguageSnapshot {
    param($Snapshot)
    $key = Get-OfficeLanguageKey
    try {
        $names = @($Snapshot | ForEach-Object { $_.name })
        foreach ($name in @($key.GetValueNames())) {
            if ($name -notin $names) { $key.DeleteValue($name) }
        }
        foreach ($entry in $Snapshot) {
            $key.SetValue($entry.name,$entry.value,[Microsoft.Win32.RegistryValueKind]::$($entry.kind))
        }
    } finally { $key.Dispose() }
    $restoredSnapshot = Get-OfficeLanguageSnapshot
    # Registry snapshots use dictionaries in memory and PSCustomObject after
    # JSON reload. Explicit lookup sorts both by the value, not CLR properties.
    $actual = @($restoredSnapshot | Sort-Object { $_.name })
    $expected = @($Snapshot | Sort-Object { $_.name })
    if (($actual | ConvertTo-Json -Depth 8 -Compress) -cne
        ($expected | ConvertTo-Json -Depth 8 -Compress)) {
        throw 'Office language preference restoration failed.'
    }
}

function Set-OfficeUiLanguage {
    param([string]$Language)
    $culture = [Globalization.CultureInfo]::GetCultureInfo($Language)
    $key = Get-OfficeLanguageKey
    try {
        $key.SetValue('UILanguageTag',$culture.Name,[Microsoft.Win32.RegistryValueKind]::String)
        $key.SetValue('UILanguage',$culture.LCID,[Microsoft.Win32.RegistryValueKind]::DWord)
        # x-none first means following Windows; that would override this request.
        $key.SetValue('UIFallbackLanguages',$culture.Name,[Microsoft.Win32.RegistryValueKind]::String)
    } finally { $key.Dispose() }
}

function Get-OfficeAuthoringLanguage {
    $key = Get-OfficeLanguageKey
    try { $value = [string]$key.GetValue('PreferredEditingLanguage') }
    finally { $key.Dispose() }
    if ([string]::IsNullOrWhiteSpace($value)) { throw 'Office authoring language is not explicitly configured.' }
    return [Globalization.CultureInfo]::GetCultureInfo($value).Name
}

function Get-OfficeUiLanguages {
    $key = Get-OfficeLanguageKey
    try { $value = [string]$key.GetValue('UISnapshotLanguages') }
    finally { $key.Dispose() }
    if ([string]::IsNullOrWhiteSpace($value)) { throw 'Office installed UI language inventory is unavailable.' }
    return @($value.Split(';') | Where-Object { $_ -and $_ -ne 'x-none' } |
        ForEach-Object { [Globalization.CultureInfo]::GetCultureInfo($_).Name } | Sort-Object -Unique)
}

function Get-OfficeRenderEnvironment {
    param($Application)
    $settings = $Application.LanguageSettings
    try { $ui = [Globalization.CultureInfo]::GetCultureInfo([int]$settings.LanguageID(2)).Name }
    finally { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($settings) }
    $zone = [TimeZoneInfo]::Local
    $iana = $null
    if (-not [TimeZoneInfo]::TryConvertWindowsIdToIanaId($zone.Id,[ref]$iana)) {
        throw "Cannot map Windows time zone to IANA: $($zone.Id)"
    }
    return [ordered]@{ui_language=$ui;format_locale=(Get-Culture).Name
        document_language=(Get-OfficeAuthoringLanguage);time_zone=$iana
        windows_time_zone=$zone.Id;clock=$null}
}

function Assert-OfficeRenderEnvironment {
    param($Actual,$Expected)
    foreach ($name in @('ui_language','format_locale','document_language','time_zone')) {
        if ($Actual[$name] -cne $Expected.$name) {
            throw "Office environment mismatch for ${name}: requested=$($Expected.$name), actual=$($Actual[$name])"
        }
    }
}

function Start-OfficeExportClock {
    return [ordered]@{opened_at_utc=[DateTime]::UtcNow.ToString('o')
        refresh_started_at_utc=$null;refresh_completed_at_utc=$null
        export_started_at_utc=$null;export_completed_at_utc=$null}
}

function Update-OfficeWordClockFields {
    param($Document)
    # Keep package language/formatting and locked fields intact. Refresh only
    # time-bearing fields, including linked header/footer and text-frame stories.
    foreach ($first in $Document.StoryRanges) {
        $story = $first
        while ($null -ne $story) {
            foreach ($field in $story.Fields) {
                if (-not $field.Locked -and $field.Code.Text -match '^\s*(DATE|TIME|CREATEDATE|SAVEDATE|PRINTDATE)(\s|$)') {
                    [void]$field.Update()
                }
            }
            $story = $story.NextStoryRange
        }
    }
}
