# ==============================================================================
# SensiDoc Windows 双架构 llama.cpp 运行时依赖下载脚本 (PowerShell)
# 支持 x86_64 (AVX2 通用) 与 arm64 (高通骁龙 Copilot+ PC)
# ==============================================================================

param(
    [ValidateSet("x86_64", "arm64")]
    [string]$Arch = "x86_64",
    [string]$Version = "b11026",
    [string]$Mirror = ""
)

$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$TargetDir = Join-Path $ProjectRoot "bin/windows-$Arch"

Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "   SensiDoc Windows llama.cpp 运行时下载器" -ForegroundColor Cyan
Write-Host "   目标架构: $Arch | 构建版本: $Version" -ForegroundColor Cyan
Write-Host "   目标目录: $TargetDir" -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan

# 1. 确定目标文件名与候选 URL (适配 llama.cpp 最新命名规则与历史规则)
$candidateFiles = @()
if ($Arch -eq "x86_64") {
    $candidateFiles = @(
        "llama-${Version}-bin-win-cpu-x64.zip",
        "llama-${Version}-bin-win-avx2-x64.zip",
        "llama-${Version}-bin-win-x64.zip"
    )
} else {
    $candidateFiles = @(
        "llama-${Version}-bin-win-cpu-arm64.zip",
        "llama-${Version}-bin-win-opencl-adreno-arm64.zip",
        "llama-${Version}-bin-win-arm64.zip",
        "llama-${Version}-bin-win-llvm-arm64.zip"
    )
}

if (-not (Test-Path $TargetDir)) {
    New-Item -ItemType Directory -Force -Path $TargetDir | Out-Null
}

$tempZip = Join-Path $ProjectRoot "bin/llama_win_${Arch}_temp.zip"
$downloadSuccess = $false

foreach ($fileName in $candidateFiles) {
    $urls = @()
    if ($Mirror) {
        $urls += "$Mirror/$fileName"
    }
    # CI 环境下优先直连 GitHub 官方 Releases 源；本地国内环境优先走加速镜像
    if ($env:GITHUB_ACTIONS -eq "true") {
        $urls += "https://github.com/ggerganov/llama.cpp/releases/download/${Version}/${fileName}"
    } else {
        $urls += "https://ghfast.top/https://github.com/ggerganov/llama.cpp/releases/download/${Version}/${fileName}"
        $urls += "https://github.com/ggerganov/llama.cpp/releases/download/${Version}/${fileName}"
    }

    foreach ($url in $urls) {
        Write-Host "==> 正在尝试从源下载: $url" -ForegroundColor Green
        try {
            # 采用 Invoke-WebRequest 或 curl.exe
            if (Get-Command "curl.exe" -ErrorAction SilentlyContinue) {
                & curl.exe -fL --retry 2 --connect-timeout 10 -o "$tempZip" "$url"
            } else {
                [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
                Invoke-WebRequest -Uri $url -OutFile $tempZip -TimeoutSec 60
            }

            if ((Test-Path $tempZip) -and ((Get-Item $tempZip).Length -gt 1048576)) {
                Write-Host "✅ 下载成功: $fileName ($([math]::Round((Get-Item $tempZip).Length / 1MB, 2)) MB)" -ForegroundColor Green
                $downloadSuccess = $true
                break
            } else {
                if (Test-Path $tempZip) { Remove-Item -Force $tempZip }
            }
        } catch {
            Write-Host "   下载未完成或网络超时，尝试备选源..." -ForegroundColor Gray
            if (Test-Path $tempZip) { Remove-Item -Force $tempZip }
        }
    }

    if ($downloadSuccess) {
        break
    }
}

if (-not $downloadSuccess) {
    Write-Error "❌ 未能成功下载 Windows $Arch 架构的 llama.cpp 运行时包。请检查网络，或手动下载上述 zip 包解压至 $TargetDir"
    exit 1
}

# 2. 解压并将关键二进制提取至目标目录
Write-Host "==> 正在解压运行时并收录核心动态库至 $TargetDir..." -ForegroundColor Green
$tempExtractDir = Join-Path $ProjectRoot "bin/temp_extract_${Arch}"
if (Test-Path $tempExtractDir) { Remove-Item -Recurse -Force $tempExtractDir }
New-Item -ItemType Directory -Force -Path $tempExtractDir | Out-Null

Expand-Archive -Path $tempZip -DestinationPath $tempExtractDir -Force

# 检索解压出来的可执行程序和 DLL
$extractedBin = Get-ChildItem -Path $tempExtractDir -Recurse -Filter "llama-server.exe" | Select-Object -First 1
if (-not $extractedBin) {
    Write-Error "❌ 解压文件中未找到 llama-server.exe！"
    Remove-Item -Recurse -Force $tempExtractDir
    Remove-Item -Force $tempZip
    exit 1
}

$binSourceDir = $extractedBin.DirectoryName

# 拷贝 llama-server.exe 及所有 dll
Copy-Item -Path "$binSourceDir/llama-server.exe" -Destination "$TargetDir/llama-server.exe" -Force
Get-ChildItem -Path $binSourceDir -Filter "*.dll" | ForEach-Object {
    Copy-Item -Path $_.FullName -Destination $TargetDir -Force
}

# 清理临时文件
Remove-Item -Recurse -Force $tempExtractDir
Remove-Item -Force $tempZip

Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "✅ Windows $Arch 架构 llama.cpp 运行时准备就绪！" -ForegroundColor Green
Write-Host "   可执行文件: $(Join-Path $TargetDir 'llama-server.exe')" -ForegroundColor Green
$dllCount = (Get-ChildItem -Path $TargetDir -Filter "*.dll").Count
Write-Host "   已收录配套 DLL 数量: $dllCount" -ForegroundColor Green
Write-Host "========================================================" -ForegroundColor Cyan
