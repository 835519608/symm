# 本地构建 Windows 便携安装包（需已安装 Inno Setup 6）。
# 用法（仓库根目录）：
#   .\packaging\windows\build-installer.ps1
#   .\packaging\windows\build-installer.ps1 -SkipBuild

param(
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $Root

$versionLine = Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"(.+)"' | Select-Object -First 1
if (-not $versionLine) { throw "Cannot read version from Cargo.toml" }
$semver = $versionLine.Matches.Groups[1].Value

if (-not $SkipBuild) {
    Write-Host "Building symm (GUI) + symm-cli (release)..."
    cargo build --release --features gui --bin symm --bin symm-cli
}

$iscc = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
if (-not (Test-Path $iscc)) {
    throw "Inno Setup not found: $iscc`nInstall: https://jrsoftware.org/isinfo.php"
}

Write-Host "Compiling installer (AppVersion=$semver)..."
& $iscc "/DAppVersion=$semver" "packaging\windows\symm-setup.iss"

$out = Join-Path $PSScriptRoot "dist\symm-setup-windows-x64.exe"
if (-not (Test-Path $out)) { throw "Installer not produced: $out" }
Write-Host "OK: $out"
