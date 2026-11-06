# Full Windows install: binary + engines + Start Menu / Desktop shortcuts.
# Uninstall:  .\tools\install-windows.ps1 -Uninstall
# One-liner:  irm https://raw.githubusercontent.com/lasaths/upscale/master/tools/install-windows.ps1 | iex
param(
    [switch]$Uninstall,
    [switch]$NoDesktop
)

$ErrorActionPreference = "Stop"

$DataRoot = if ($env:LOKU_DATA) { $env:LOKU_DATA } else { Join-Path $env:LOCALAPPDATA "loku" }
$ReceiptDir = Join-Path $env:LOCALAPPDATA "upscale-ui"
$LokuExe = Join-Path $env:USERPROFILE ".cargo\bin\loku.exe"
$StartMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Loku.lnk"
$Desktop = Join-Path ([Environment]::GetFolderPath("Desktop")) "Loku.lnk"
$SetupRawUrl = "https://raw.githubusercontent.com/lasaths/upscale/master/tools/setup.ps1"
$InstallerUrl = "https://github.com/lasaths/upscale/releases/latest/download/upscale-ui-installer.ps1"

function Remove-Shortcut([string]$Path) {
    if (Test-Path $Path) { Remove-Item $Path -Force }
}

function New-LokuShortcut([string]$Path, [string]$Target) {
    $w = New-Object -ComObject WScript.Shell
    $s = $w.CreateShortcut($Path)
    $s.TargetPath = $Target
    $s.WorkingDirectory = Split-Path $Target
    $s.IconLocation = "$Target,0"
    $s.Description = "Loku image upscaler"
    $s.Save()
}

function Uninstall-Loku {
    Write-Host "==> Uninstall Loku"
    Remove-Shortcut $StartMenu
    Remove-Shortcut $Desktop
    if (Test-Path $LokuExe) {
        Remove-Item $LokuExe -Force
        Write-Host "  removed $LokuExe"
    }
    if (Test-Path $DataRoot) {
        Remove-Item $DataRoot -Recurse -Force
        Write-Host "  removed $DataRoot"
    }
    if (Test-Path $ReceiptDir) {
        Remove-Item $ReceiptDir -Recurse -Force
        Write-Host "  removed $ReceiptDir"
    }
    Write-Host "Done."
}

if ($Uninstall) {
    Uninstall-Loku
    return
}

Write-Host "==> Install Loku binary"
irm $InstallerUrl | iex

if (-not (Test-Path $LokuExe)) {
    throw "loku.exe not found at $LokuExe after installer"
}

# Prefer local setup.ps1 when run from a clone; otherwise download into a temp tree.
$LocalSetup = if ($PSScriptRoot) { Join-Path $PSScriptRoot "setup.ps1" } else { $null }
if ($LocalSetup -and (Test-Path $LocalSetup)) {
    $Setup = $LocalSetup
} else {
    $tmp = Join-Path $env:TEMP "loku-setup-$(Get-Random)"
    $toolsDir = Join-Path $tmp "tools"
    New-Item -ItemType Directory -Force -Path $toolsDir | Out-Null
    $Setup = Join-Path $toolsDir "setup.ps1"
    Write-Host "==> Fetch setup.ps1"
    Invoke-WebRequest -Uri $SetupRawUrl -OutFile $Setup -UseBasicParsing
}

Write-Host "==> Download engines into $DataRoot"
& $Setup -InstallRoot $DataRoot

Write-Host "==> Shortcuts"
New-LokuShortcut $StartMenu $LokuExe
Write-Host "  Start Menu: $StartMenu"
if (-not $NoDesktop) {
    New-LokuShortcut $Desktop $LokuExe
    Write-Host "  Desktop: $Desktop"
}

# Self-contained uninstall next to the engines (no network needed).
$UninstallHelper = Join-Path $DataRoot "uninstall.ps1"
@(
    '$ErrorActionPreference = "Stop"'
    '$DataRoot = if ($env:LOKU_DATA) { $env:LOKU_DATA } else { Join-Path $env:LOCALAPPDATA "loku" }'
    '$ReceiptDir = Join-Path $env:LOCALAPPDATA "upscale-ui"'
    '$LokuExe = Join-Path $env:USERPROFILE ".cargo\bin\loku.exe"'
    '$StartMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Loku.lnk"'
    '$Desktop = Join-Path ([Environment]::GetFolderPath("Desktop")) "Loku.lnk"'
    'foreach ($p in @($StartMenu, $Desktop, $LokuExe)) { if (Test-Path $p) { Remove-Item $p -Force } }'
    'foreach ($d in @($DataRoot, $ReceiptDir)) { if (Test-Path $d) { Remove-Item $d -Recurse -Force } }'
    'Write-Host "Loku uninstalled."'
) | Set-Content -Path $UninstallHelper -Encoding UTF8

Write-Host ""
Write-Host "Done. Launch Loku from the Start Menu, Desktop, or: loku"
Write-Host "Uninstall:  & `"$UninstallHelper`""
Write-Host "         or .\tools\install-windows.ps1 -Uninstall"
