Add-Type -Path (Join-Path $PSScriptRoot 'NativeChartSnapshot.cs')
Add-Type -Path (Join-Path $PSScriptRoot 'BibliographySnapshot.cs')
function Get-ChartSnapshot { param($Document) return @{} }

Add-Type -Path (Join-Path $PSScriptRoot 'TextGdiObserver.cs')
