# Installs nodeloom-agent and nodeloom-master on Windows (x64 or ARM64).
#
#   powershell -ExecutionPolicy Bypass -File install.ps1
#
# Run from an unpacked release archive, or anywhere else to download the
# newest release with the GitHub CLI (the repository is private, so run
# `gh auth login` first).
#
#   -Version v0.1.0-beta.1   release tag to download (default: newest, betas included)
#   -InstallDir <path>       default: %LOCALAPPDATA%\Nodeloom\bin
param(
    [string]$Version = $env:NODELOOM_VERSION,
    [string]$InstallDir = $(if ($env:NODELOOM_INSTALL_DIR) { $env:NODELOOM_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Nodeloom\bin' }),
    [string]$Repo = $(if ($env:NODELOOM_REPO) { $env:NODELOOM_REPO } else { 'jagari/Nodeloom' })
)
$ErrorActionPreference = 'Stop'

function Fail([string]$message) {
    Write-Error "install.ps1: $message"
    exit 1
}

$here = $PSScriptRoot
$work = $null
if ((Test-Path (Join-Path $here 'nodeloom-agent.exe')) -and (Test-Path (Join-Path $here 'nodeloom-master.exe'))) {
    $src = $here
} else {
    if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
        Fail 'no binaries next to this script and no GitHub CLI (gh) to download them'
    }
    $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    $target = switch ($arch) {
        'X64' { 'windows-x64' }
        'Arm64' { 'windows-arm64' }
        default { Fail "unsupported CPU $arch" }
    }
    if (-not $Version) {
        # `gh release download` alone skips pre-releases; betas are what exist.
        $Version = gh release list --repo $Repo --limit 1 --json tagName --jq '.[0].tagName'
        if (-not $Version) { Fail "could not list releases of $Repo (run: gh auth login)" }
    }
    $work = Join-Path ([System.IO.Path]::GetTempPath()) ("nodeloom-" + [guid]::NewGuid())
    New-Item -ItemType Directory -Path $work | Out-Null
    Write-Host "Downloading $Version for $target from $Repo..."
    gh release download $Version --repo $Repo --dir $work --pattern "nodeloom-*-$target.zip" --pattern 'SHA256SUMS'
    if ($LASTEXITCODE -ne 0) { Fail 'download failed' }
    $archive = Get-ChildItem $work -Filter "nodeloom-*-$target.zip" | Select-Object -First 1
    $sums = Join-Path $work 'SHA256SUMS'
    if (Test-Path $sums) {
        $line = Get-Content $sums | Where-Object { $_ -match " $([regex]::Escape($archive.Name))$" }
        $expected = ($line -split ' ')[0]
        $actual = (Get-FileHash $archive.FullName -Algorithm SHA256).Hash.ToLower()
        if ($expected -ne $actual) { Fail "checksum mismatch for $($archive.Name)" }
    }
    Expand-Archive $archive.FullName -DestinationPath $work
    $src = (Get-ChildItem $work -Recurse -Filter 'nodeloom-agent.exe' | Select-Object -First 1).DirectoryName
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
foreach ($bin in 'nodeloom-agent.exe', 'nodeloom-master.exe') {
    $to = Join-Path $InstallDir $bin
    Copy-Item (Join-Path $src $bin) $to -Force
    # Files from the internet carry a mark that makes SmartScreen block
    # unsigned programs; the beta is not signed.
    Unblock-File $to
}
if ($work) { Remove-Item $work -Recurse -Force }

Write-Host "Installed to ${InstallDir}:"
& (Join-Path $InstallDir 'nodeloom-agent.exe') --version
& (Join-Path $InstallDir 'nodeloom-master.exe') --version

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (-not (($userPath -split ';') -contains $InstallDir)) {
    $newPath = if ($userPath) { "$userPath;$InstallDir" } else { $InstallDir }
    [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
    Write-Host "Added $InstallDir to your PATH; open a new terminal to use it."
}

Write-Host ''
Write-Host 'Next steps (docs/BETA.md has the full guide):'
Write-Host '  1. On one machine:   nodeloom-master'
Write-Host '     It prints a dashboard link; open it in a browser.'
Write-Host '     Allow it through Windows Firewall on private networks when asked.'
Write-Host '  2. On each machine:  nodeloom-agent'
Write-Host '     Approve it in the dashboard when the codes match.'
