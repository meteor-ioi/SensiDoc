# ==============================================================================
# SensiDoc Windows 安装包一键打包脚本 (PowerShell)
# ==============================================================================

param(
    [string]$Version = "1.3.0"
)

$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
Set-Location $ProjectRoot

Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "   SensiDoc Windows 安装包打包流水线 (v$Version)" -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan

# 1. 编译 Rust 生产版本二进制
Write-Host "==> 正在编译 Rust Release 二进制..." -ForegroundColor Green
cargo build --release

if (-not (Test-Path "target/release/sensidoc.exe")) {
    Write-Error "未找到编译产物 target/release/sensidoc.exe，构建失败！"
    exit 1
}

# 2. 检查 Inno Setup 编译器 (ISCC.exe)
$isccPath = $null
$candidatePaths = @(
    "ISCC.exe",
    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
    "C:\Program Files\Inno Setup 6\ISCC.exe",
    "${env:LOCALAPPDATA}\Programs\Inno Setup 6\ISCC.exe"
)

foreach ($p in $candidatePaths) {
    if (Get-Command $p -ErrorAction SilentlyContinue) {
        $isccPath = $p
        break
    }
    if (Test-Path $p) {
        $isccPath = $p
        break
    }
}

if (-not $isccPath) {
    Write-Host "==> 未检测到 Inno Setup，尝试通过 choco 自动安装..." -ForegroundColor Yellow
    if (Get-Command "choco" -ErrorAction SilentlyContinue) {
        choco install innosetup -y --no-progress
        $isccPath = "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
    } else {
        Write-Error "未找到 Inno Setup 编译器 ISCC.exe，请先安装 Inno Setup 6 (https://jrsoftware.org/isdl.php)"
        exit 1
    }
}

# 3. 创建 dist 目录
$distDir = Join-Path $ProjectRoot "dist"
if (-not (Test-Path $distDir)) {
    New-Item -ItemType Directory -Path $distDir | Out-Null
}

# 4. 执行 Inno Setup 编译
Write-Host "==> 正在使用 Inno Setup 构建安装包..." -ForegroundColor Green
& $isccPath "/DMyAppVersion=$Version" "scripts/installer.iss"

$installerName = "sensidoc-v${Version}-windows-x86_64-setup.exe"
$installerPath = Join-Path $distDir $installerName

if (Test-Path $installerPath) {
    Write-Host "========================================================" -ForegroundColor Cyan
    Write-Host "✅ Windows 安装包制作成功！" -ForegroundColor Green
    Write-Host "📦 安装包路径: $installerPath" -ForegroundColor Green
    Write-Host "========================================================" -ForegroundColor Cyan
} else {
    Write-Error "安装包生成异常，未在 dist 目录找到期望文件！"
    exit 1
}
