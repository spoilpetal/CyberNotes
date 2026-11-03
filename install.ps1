$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$InstallDir = Join-Path $env:LOCALAPPDATA 'CyberNotes\bin'
$Binary = Join-Path $Root 'target\release\cybernotes.exe'
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Cargo is required to build from source. For a binary install, download the Windows release and follow the README.'
}
Push-Location $Root
try { cargo build --release; if ($LASTEXITCODE -ne 0) { throw 'Cargo build failed.' } }
finally { Pop-Location }
if (-not (Test-Path $Binary)) { throw "Release executable was not produced: $Binary" }
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item $Binary (Join-Path $InstallDir 'cybernotes.exe') -Force
$Alias = Join-Path $InstallDir 'cbn.cmd'
Set-Content -Path $Alias -Value '@echo off' -Encoding ascii
Add-Content -Path $Alias -Value '"%~dp0cybernotes.exe" %*' -Encoding ascii
$UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (($UserPath -split ';') -notcontains $InstallDir) {
    $NewPath = if ($UserPath) { (($UserPath.TrimEnd(';') + ';' + $InstallDir).Trim(';')) } else { $InstallDir }
    [Environment]::SetEnvironmentVariable('Path', $NewPath, 'User')
    Write-Host "Added $InstallDir to your user PATH. Open a new terminal."
}
$DataRoot = if ($env:XDG_DATA_HOME) { $env:XDG_DATA_HOME } else { $env:APPDATA }
$Vault = Join-Path (Join-Path $DataRoot 'cybernotes') 'vault.cbn'
if (-not (Test-Path $Vault)) {
    & (Join-Path $InstallDir 'cybernotes.exe') setup
    if ($LASTEXITCODE -ne 0) { throw 'CyberNotes first-time setup failed.' }
} else {
    Write-Host 'Existing notes detected; vault data and password settings were preserved.'
}
Write-Host 'Installed cybernotes and cbn. Your notes are stored separately and will not be removed by uninstall.ps1.' 
