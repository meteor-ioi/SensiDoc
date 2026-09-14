# ==============================================================================
# SensiDoc Windows 安装包一键打包脚本 (PowerShell)
# ==============================================================================

param(
    [string]$Version = "1.3.1"
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

# 4. 执行 Inno Setup 编译 (标准版)
Write-Host "==> 正在使用 Inno Setup 构建标准版安装包..." -ForegroundColor Green
& $isccPath "/DMyAppVersion=$Version" "scripts/installer.iss"

$installerName = "sensidoc-v${Version}-windows-x86_64-setup.exe"
$installerPath = Join-Path $distDir $installerName

if (Test-Path $installerPath) {
    Write-Host "✅ Windows 标准版安装包制作成功: $installerPath" -ForegroundColor Green
} else {
    Write-Error "标准版安装包生成异常，未在 dist 目录找到期望文件！"
    exit 1
}

# 5. 打包标准版绿色免安装便携版 (ZIP)
Write-Host "==> 正在生成 Windows 标准版绿色免安装压缩包..." -ForegroundColor Green
$pkgDir = Join-Path $distDir "SensiDoc-v$Version-windows-x86_64"
$zipPath = Join-Path $distDir "sensidoc-v$Version-windows-x86_64.zip"

if (Test-Path $pkgDir) { Remove-Item -Recurse -Force $pkgDir }
New-Item -ItemType Directory -Force -Path $pkgDir | Out-Null
Copy-Item -Path "target/release/sensidoc.exe" -Destination "$pkgDir/sensidoc.exe"
Copy-Item -Recurse -Path "web" -Destination "$pkgDir/web"
if (Test-Path "assets/sensidoc_win.ico") {
    Copy-Item -Path "assets/sensidoc_win.ico" -Destination "$pkgDir/sensidoc.ico"
}
if (Test-Path "README.md") {
    Copy-Item -Path "README.md" -Destination "$pkgDir/README.md"
}
New-Item -ItemType Directory -Force -Path "$pkgDir/models" | Out-Null
Compress-Archive -Path "$pkgDir/*" -DestinationPath $zipPath -Force
Remove-Item -Recurse -Force $pkgDir
Write-Host "📦 标准版绿色压缩包: $zipPath" -ForegroundColor Green

# 6. 若检测到 models/ocr 模型套件，构建离线增强版 (Full)
$ocrDetPath = "models/ocr/PP-OCRv6_det_small.onnx"
if (Test-Path $ocrDetPath) {
    Write-Host "==> 检测到 OCR 模型套件，正在构建 Windows 离线增强版 (Full)..." -ForegroundColor Cyan

    # 6.1 Inno Setup 构建离线增强版安装包
    & $isccPath "/DMyAppVersion=$Version" "/DIncludeOcrModels=1" "/DOutputSuffix=-full" "scripts/installer.iss"
    $fullInstallerName = "sensidoc-v${Version}-windows-x86_64-full-setup.exe"
    $fullInstallerPath = Join-Path $distDir $fullInstallerName
    if (Test-Path $fullInstallerPath) {
        Write-Host "✅ Windows 离线增强版安装包制作成功: $fullInstallerPath" -ForegroundColor Green
    }

    # 6.2 打包离线增强版绿色免安装便携版 (ZIP)
    $fullPkgDir = Join-Path $distDir "SensiDoc-v$Version-windows-x86_64-full"
    $fullZipPath = Join-Path $distDir "sensidoc-v$Version-windows-x86_64-full.zip"
    if (Test-Path $fullPkgDir) { Remove-Item -Recurse -Force $fullPkgDir }
    New-Item -ItemType Directory -Force -Path $fullPkgDir | Out-Null
    Copy-Item -Path "target/release/sensidoc.exe" -Destination "$fullPkgDir/sensidoc.exe"
    Copy-Item -Recurse -Path "web" -Destination "$fullPkgDir/web"
    if (Test-Path "assets/sensidoc_win.ico") {
        Copy-Item -Path "assets/sensidoc_win.ico" -Destination "$fullPkgDir/sensidoc.ico"
    }
    if (Test-Path "README.md") {
        Copy-Item -Path "README.md" -Destination "$fullPkgDir/README.md"
    }
    New-Item -ItemType Directory -Force -Path "$fullPkgDir/models/ocr" | Out-Null
    Copy-Item -Recurse -Path "models/ocr/*" -Destination "$fullPkgDir/models/ocr/"
    Compress-Archive -Path "$fullPkgDir/*" -DestinationPath $fullZipPath -Force
    Remove-Item -Recurse -Force $fullPkgDir
    Write-Host "📦 离线增强版绿色压缩包: $fullZipPath" -ForegroundColor Green
} else {
    Write-Host "ℹ️ 未检测到 models/ocr 模型文件，跳过 Windows 离线增强版构建。" -ForegroundColor Yellow
}

Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "✅ Windows 应用构建流水线全部完成！" -ForegroundColor Green
Write-Host "========================================================" -ForegroundColor Cyan
