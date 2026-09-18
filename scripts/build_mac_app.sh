#!/usr/bin/env bash
set -e

# ==============================================================================
# SensiDoc macOS Application (.app & .dmg) 独立拆包流水线
# 支持 arm64 (Apple Silicon M系列) 与 x86_64 (Intel) 独立打包与极致瘦身 (告别臃肿 lipo)
# ==============================================================================

PROJECT_ROOT="$( cd "$( dirname "${BASH_SOURCE[0]}" )/.." >/dev/null 2>&1 && pwd )"
cd "$PROJECT_ROOT"

APP_NAME="SensiDoc"
RAW_VER="${1:-${APP_VERSION:-1.4.1}}"
VERSION="${RAW_VER#v}"
TARGET_ARG="${2:-auto}" # 可选: arm64, x86_64, all, auto (默认当前主机架构)
BUNDLE_ID="com.sensidoc.desktop"
DIST_DIR="$PROJECT_ROOT/dist"
APP_BUNDLE="$DIST_DIR/$APP_NAME.app"
CONTENTS_DIR="$APP_BUNDLE/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

mkdir -p "$DIST_DIR"

# 1. 准备/检查 macOS 图标 (.icns)
if [ ! -f "assets/AppIcon.icns" ]; then
    echo "==> 正在从 assets/sensidoc_mac.png 生成 AppIcon.icns..."
    mkdir -p assets/AppIcon.iconset
    sips -z 16 16     assets/sensidoc_mac.png --out assets/AppIcon.iconset/icon_16x16.png >/dev/null 2>&1
    sips -z 32 32     assets/sensidoc_mac.png --out assets/AppIcon.iconset/icon_16x16@2x.png >/dev/null 2>&1
    sips -z 32 32     assets/sensidoc_mac.png --out assets/AppIcon.iconset/icon_32x32.png >/dev/null 2>&1
    sips -z 64 64     assets/sensidoc_mac.png --out assets/AppIcon.iconset/icon_32x32@2x.png >/dev/null 2>&1
    sips -z 128 128   assets/sensidoc_mac.png --out assets/AppIcon.iconset/icon_128x128.png >/dev/null 2>&1
    sips -z 256 256   assets/sensidoc_mac.png --out assets/AppIcon.iconset/icon_128x128@2x.png >/dev/null 2>&1
    sips -z 256 256   assets/sensidoc_mac.png --out assets/AppIcon.iconset/icon_256x256.png >/dev/null 2>&1
    sips -z 512 512   assets/sensidoc_mac.png --out assets/AppIcon.iconset/icon_256x256@2x.png >/dev/null 2>&1
    sips -z 512 512   assets/sensidoc_mac.png --out assets/AppIcon.iconset/icon_512x512.png >/dev/null 2>&1
    sips -z 1024 1024 assets/sensidoc_mac.png --out assets/AppIcon.iconset/icon_512x512@2x.png >/dev/null 2>&1
    iconutil -c icns assets/AppIcon.iconset -o assets/AppIcon.icns
    rm -rf assets/AppIcon.iconset
fi

# 2. 生成 DMG 的通用打包函数
build_app_and_dmg() {
    local edition_label="$1"       # 如 "标准版 (arm64)"
    local dmg_filename="$2"        # DMG 输出文件名
    local include_ocr="$3"         # true / false
    local main_bin="$4"            # 对应的单架构二进制路径
    local llama_bin="$5"           # llama-server 路径 (可选)
    local lib_dir="$6"             # 动态链接库目录 (可选)

    echo "--------------------------------------------------------"
    echo "==> 正在打包 macOS [$edition_label]..."
    echo "--------------------------------------------------------"

    rm -rf "$APP_BUNDLE"
    mkdir -p "$MACOS_DIR"
    mkdir -p "$RESOURCES_DIR"

    # 拷贝主可执行文件
    cp "$main_bin" "$MACOS_DIR/sensidoc"
    chmod +x "$MACOS_DIR/sensidoc"

    # 拷贝前端静态资源 (web/)
    mkdir -p "$RESOURCES_DIR/web"
    cp -R web/* "$RESOURCES_DIR/web/"

    # 拷贝对应架构的内置推理引擎与动态库
    if [ -n "$llama_bin" ] && [ -f "$llama_bin" ]; then
        mkdir -p "$RESOURCES_DIR/bin"
        cp "$llama_bin" "$RESOURCES_DIR/bin/llama-server"
        chmod +x "$RESOURCES_DIR/bin/llama-server"
        echo "   [内置模型引擎] 已装载对应架构 llama-server: $llama_bin"
    fi

    if [ -n "$lib_dir" ] && [ -d "$lib_dir" ]; then
        mkdir -p "$RESOURCES_DIR/lib"
        cp -R "$lib_dir"/* "$RESOURCES_DIR/lib/"
        echo "   [动态链接库] 已装载对应架构动态库: $lib_dir"
    fi

    # 拷贝应用图标
    cp "assets/AppIcon.icns" "$RESOURCES_DIR/AppIcon.icns"

    # 如果是离线增强版，拷贝 OCR 原生模型套件
    if [ "$include_ocr" = "true" ]; then
        echo "   [离线全量] 正在将 OCR 模型套件内置打包至 Resources/models/ocr..."
        mkdir -p "$RESOURCES_DIR/models/ocr"
        cp -R models/ocr/* "$RESOURCES_DIR/models/ocr/"
    fi

    # 生成 Info.plist
    cat > "$CONTENTS_DIR/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>zh_CN</string>
    <key>CFBundleDisplayName</key>
    <string>SensiDoc</string>
    <key>CFBundleExecutable</key>
    <string>sensidoc</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIdentifier</key>
    <string>$BUNDLE_ID</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSRequiresAquaSystemAppearance</key>
    <false/>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2026 SensiDoc Team. All rights reserved.</string>
</dict>
</plist>
EOF

    # 生成 PkgInfo
    echo "APPL????" > "$CONTENTS_DIR/PkgInfo"

    # 代码签名 (Ad-Hoc 本地签名，防止 Gatekeeper 拦截)
    codesign --force --deep --sign - "$APP_BUNDLE" 2>/dev/null || true

    # 打包分发文件 (DMG)
    if command -v hdiutil >/dev/null 2>&1; then
        echo "==> 正在构建 DMG 安装映像: $dmg_filename..."
        local dmg_tmp="$DIST_DIR/dmg_tmp_${edition_label// /_}"
        rm -rf "$dmg_tmp"
        mkdir -p "$dmg_tmp"
        cp -R "$APP_BUNDLE" "$dmg_tmp/"
        ln -s /Applications "$dmg_tmp/Applications"

        rm -f "$DIST_DIR/$dmg_filename"
        hdiutil create -volname "$APP_NAME" -srcfolder "$dmg_tmp" -ov -format UDZO "$DIST_DIR/$dmg_filename" -quiet
        rm -rf "$dmg_tmp"
        echo "💿 [$edition_label] DMG 安装映像已生成: $DIST_DIR/$dmg_filename"
    fi
}

# 3. 单架构独立编译与构建流水线 (彻底消除 lipo 带来的 50MB+ 体积翻倍)
build_arch_pipeline() {
    local target_arch="$1"
    local rust_target=""
    local arch_label=""

    if [ "$target_arch" = "arm64" ] || [ "$target_arch" = "aarch64" ]; then
        target_arch="arm64"
        rust_target="aarch64-apple-darwin"
        arch_label="Apple Silicon (arm64)"
    elif [ "$target_arch" = "x86_64" ] || [ "$target_arch" = "intel" ]; then
        target_arch="x86_64"
        rust_target="x86_64-apple-darwin"
        arch_label="Intel (x86_64)"
    else
        echo "❌ 不支持的架构参数: $target_arch (仅支持 arm64 / x86_64 / all)"
        exit 1
    fi

    echo "========================================================"
    echo "   SensiDoc macOS [$arch_label] 独立打包 (v$VERSION)"
    echo "========================================================"

    rustup target add "$rust_target" 2>/dev/null || true

    local host_arch
    host_arch=$(uname -m)
    local main_bin=""

    # 优先精确架构编译
    echo "==> 正在编译纯净单架构二进制 ($rust_target)..."
    if cargo build --release --target "$rust_target"; then
        main_bin="target/$rust_target/release/sensidoc"
    elif [ "$host_arch" = "$target_arch" ]; then
        echo "==> 降级使用宿主原生编译器构建..."
        cargo build --release
        main_bin="target/release/sensidoc"
    else
        echo "❌ 目标架构 $rust_target 跨平台编译失败！"
        exit 1
    fi

    if [ ! -f "$main_bin" ]; then
        echo "❌ 未找到编译产物: $main_bin"
        exit 1
    fi

    local bin_size
    bin_size=$(ls -lh "$main_bin" | awk '{print $5}')
    echo "✅ 单架构主程序构建成功: $main_bin (精简体积: $bin_size)"

    # 匹配对应架构的 llama.cpp 运行时 (避免跨架构错配 Bad CPU type)
    local arch_bin_dir="bin/macos-$target_arch"
    local arch_lib_dir="lib/macos-$target_arch"
    local resolved_llama_bin=""
    local resolved_lib_dir=""

    if [ -f "$arch_bin_dir/llama-server" ]; then
        resolved_llama_bin="$arch_bin_dir/llama-server"
    elif [ "$target_arch" = "arm64" ] && [ -f "bin/llama-server" ]; then
        # 兼容当前根目录下现有的 Apple Silicon arm64 llama-server
        resolved_llama_bin="bin/llama-server"
    fi

    if [ -d "$arch_lib_dir" ]; then
        resolved_lib_dir="$arch_lib_dir"
    elif [ "$target_arch" = "arm64" ] && [ -d "lib" ]; then
        resolved_lib_dir="lib"
    fi

    # 3.1 始终构建标准版 DMG (独立单架构命名)
    local standard_dmg="sensidoc-v${VERSION}-macos-${target_arch}.dmg"
    build_app_and_dmg "标准版 ($target_arch)" "$standard_dmg" "false" "$main_bin" "$resolved_llama_bin" "$resolved_lib_dir"

    # 3.2 若检测到 models/ocr 模型套件，构建离线增强版 DMG
    local full_dmg="sensidoc-v${VERSION}-macos-${target_arch}-full.dmg"
    if [ -f "models/ocr/PP-OCRv6_det_small.onnx" ]; then
        build_app_and_dmg "离线增强版 ($target_arch)" "$full_dmg" "true" "$main_bin" "$resolved_llama_bin" "$resolved_lib_dir"
    else
        echo "ℹ️ 未检测到 models/ocr 模型文件，跳过 $target_arch 离线增强版 DMG 构建。"
    fi
}

# 4. 执行调度
if [ "$TARGET_ARG" = "all" ]; then
    echo "==> 正在顺次构建双架构独立 DMG (arm64 与 x86_64)..."
    build_arch_pipeline "arm64"
    build_arch_pipeline "x86_64"
elif [ "$TARGET_ARG" = "auto" ]; then
    DETECTED_ARCH=$(uname -m)
    echo "==> 自动检测到本机架构: $DETECTED_ARCH"
    build_arch_pipeline "$DETECTED_ARCH"
else
    build_arch_pipeline "$TARGET_ARG"
fi

echo "========================================================"
echo "✅ macOS 应用构建流水线完成！"
echo "📦 独立产物目录: $DIST_DIR"
ls -lh "$DIST_DIR"/*.dmg 2>/dev/null || true
echo "========================================================"
