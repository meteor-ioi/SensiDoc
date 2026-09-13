#!/usr/bin/env bash
set -e

# ==============================================================================
# SensiDoc OCR 模型文件下载脚本 (用于构建离线全量版应用包)
# ==============================================================================

PROJECT_ROOT="$( cd "$( dirname "${BASH_SOURCE[0]}" )/.." >/dev/null 2>&1 && pwd )"
OCR_DIR="$PROJECT_ROOT/models/ocr"
mkdir -p "$OCR_DIR"

echo "========================================================"
echo "   准备 SensiDoc OCR 离线模型套件 (PP-OCRv6 + SLANet_plus)"
echo "   目标目录: $OCR_DIR"
echo "========================================================"

download_file() {
    local filename="$1"
    local url="$2"
    local min_size="$3"
    local dest="$OCR_DIR/$filename"

    if [ -f "$dest" ]; then
        local size
        size=$(wc -c < "$dest" | tr -d ' ')
        if [ "$size" -ge "$min_size" ]; then
            echo "   [已就绪] $filename ($size 字节)"
            return 0
        fi
        echo "   [重新下载] $filename 文件不完整 ($size < $min_size 字节)..."
        rm -f "$dest"
    fi

    echo "   [正在下载] $filename 来自 ModelScope..."
    curl -fL --retry 3 --connect-timeout 15 -o "$dest" "$url"
    local final_size
    final_size=$(wc -c < "$dest" | tr -d ' ')
    echo "   [完成] $filename ($final_size 字节)"
}

download_file "PP-OCRv6_det_small.onnx" \
    "https://modelscope.cn/models/RapidAI/RapidOCR/resolve/7d0781614ca1a83d5ad9603f713acb2e74855d72/onnx/PP-OCRv6/det/PP-OCRv6_det_small.onnx" \
    9000000

download_file "PP-OCRv6_rec_small.onnx" \
    "https://modelscope.cn/models/RapidAI/RapidOCR/resolve/7d0781614ca1a83d5ad9603f713acb2e74855d72/onnx/PP-OCRv6/rec/PP-OCRv6_rec_small.onnx" \
    20000000

download_file "slanet-plus.onnx" \
    "https://modelscope.cn/models/RapidAI/RapidTable/resolve/a484f11b64162cc443ccf14582996ca35be33030/slanet-plus.onnx" \
    7000000

download_file "ppocrv6_dict.txt" \
    "https://modelscope.cn/models/RapidAI/RapidOCR/resolve/7d0781614ca1a83d5ad9603f713acb2e74855d72/paddle/PP-OCRv6/rec/PP-OCRv6_rec_small/ppocrv6_dict.txt" \
    70000

echo "========================================================"
echo "✅ OCR 离线模型套件准备就绪！"
echo "========================================================"
