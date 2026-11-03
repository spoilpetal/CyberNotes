$ErrorActionPreference = 'Stop'
$InstallDir = Join-Path $env:LOCALAPPDATA 'CyberNotes\bin'
if (Test-Path $InstallDir) {
    Remove-Item (Join-Path $InstallDir 'cybernotes.exe') -Force -ErrorAction SilentlyContinue
    Remove-Item (Join-Path $InstallDir 'cbn.cmd') -Force -ErrorAction SilentlyContinue
    $Remaining = Get-ChildItem -Force -Path $InstallDir -ErrorAction SilentlyContinue
    if (-not $Remaining) { Remove-Item $InstallDir -Force -ErrorAction SilentlyContinue }
}
$UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($UserPath) {
    $Filtered = ($UserPath -split ';' | Where-Object { $_ -and -not [string]::Equals($_.TrimEnd('\'), $InstallDir.TrimEnd('\'), [System.StringComparison]::OrdinalIgnoreCase) }) -join ';'
    [Environment]::SetEnvironmentVariable('Path', $Filtered, 'User')
}
Write-Host 'CyberNotes commands removed. Your notes and vault data were preserved.'
