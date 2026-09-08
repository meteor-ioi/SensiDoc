#!/usr/bin/env bash
set -e

# ==============================================================================
# SensiDoc macOS Application (.app & .dmg) 一键构建打包脚本
# ==============================================================================

PROJECT_ROOT="$( cd "$( dirname "${BASH_SOURCE[0]}" )/.." >/dev/null 2>&1 && pwd )"
cd "$PROJECT_ROOT"

APP_NAME="SensiDoc"
RAW_VER="${1:-${APP_VERSION:-1.0.2}}"
VERSION="${RAW_VER#v}"
BUNDLE_ID="com.sensidoc.desktop"
DIST_DIR="$PROJECT_ROOT/dist"
APP_BUNDLE="$DIST_DIR/$APP_NAME.app"
CONTENTS_DIR="$APP_BUNDLE/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

echo "========================================================"
echo "   SensiDoc macOS 应用打包流水线 (v$VERSION)"
echo "========================================================"

# 1. 编译 Rust 生产版本二进制 (Release Profile, 优先构建 Universal 架构以兼容 M 芯片与 Intel 芯片)
echo "==> 检查并安装跨架构编译目标 (aarch64 & x86_64)..."
rustup target add aarch64-apple-darwin x86_64-apple-darwin 2>/dev/null || true

IS_UNIVERSAL=false
MAIN_BIN="target/release/sensidoc"

if rustup target list --installed | grep -q "x86_64-apple-darwin" && rustup target list --installed | grep -q "aarch64-apple-darwin"; then
    echo "==> 正在编译 aarch64 (Apple Silicon M系列) 架构..."
    if cargo build --release --target aarch64-apple-darwin && cargo build --release --target x86_64-apple-darwin; then
        echo "==> 正在使用 lipo 合并生成 Universal 2 通用二进制 (兼容 Apple Silicon 与 Intel 芯片)..."
        mkdir -p target/universal/release
        lipo -create -output target/universal/release/sensidoc \
            target/aarch64-apple-darwin/release/sensidoc \
            target/x86_64-apple-darwin/release/sensidoc
        MAIN_BIN="target/universal/release/sensidoc"
        IS_UNIVERSAL=true
    else
        echo "==> 双架构跨编译失败，降级为宿主架构构建..."
        cargo build --release
    fi
else
    echo "==> 采用宿主架构编译 Release 二进制..."
    cargo build --release
fi

# 2. 检查并生成 macOS 图标 (.icns)
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

# 3. 初始化并清理 dist 目录
echo "==> 准备 App Bundle 目录结构..."
rm -rf "$APP_BUNDLE"
mkdir -p "$MACOS_DIR"
mkdir -p "$RESOURCES_DIR"

# 4. 拷贝主可执行文件
cp "$MAIN_BIN" "$MACOS_DIR/sensidoc"
chmod +x "$MACOS_DIR/sensidoc"

# 5. 拷贝前端静态资源 (web/)
mkdir -p "$RESOURCES_DIR/web"
cp -R web/* "$RESOURCES_DIR/web/"

# 6. 拷贝内置推理引擎与动态库 (bin/ 和 lib/)
mkdir -p "$RESOURCES_DIR/bin"
if [ -f "bin/llama-server" ]; then
    cp "bin/llama-server" "$RESOURCES_DIR/bin/"
    chmod +x "$RESOURCES_DIR/bin/llama-server"
fi

if [ -d "lib" ]; then
    mkdir -p "$RESOURCES_DIR/lib"
    cp -R lib/* "$RESOURCES_DIR/lib/"
fi

# 7. 拷贝应用图标
cp "assets/AppIcon.icns" "$RESOURCES_DIR/AppIcon.icns"

# 8. 生成 Info.plist
echo "==> 写入 Info.plist 元数据..."
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

# 9. 生成 PkgInfo
echo "APPL????" > "$CONTENTS_DIR/PkgInfo"

# 10. 代码签名 (Ad-Hoc 本地签名，防止 macOS Gatekeeper 拦截)
echo "==> 正在执行本地 Ad-Hoc 代码签名..."
codesign --force --deep --sign - "$APP_BUNDLE" 2>/dev/null || true

# 11. 打包分发文件 (DMG)
echo "==> 正在生成分发包..."
if [ "$IS_UNIVERSAL" = true ]; then
    DMG_NAME="sensidoc-v${VERSION}-macOS-universal.dmg"
else
    ARCH="$(uname -m)"
    DMG_NAME="sensidoc-v${VERSION}-macOS-${ARCH}.dmg"
fi
STD_DMG_NAME="sensidoc-v${VERSION}-macOS.dmg"

cd "$DIST_DIR"
rm -f "$DMG_NAME" "$STD_DMG_NAME"

# 制作 DMG 磁盘映像 (如果 hdiutil 可用)
if command -v hdiutil >/dev/null 2>&1; then
    echo "==> 正在构建 DMG 安装映像 $DMG_NAME..."
    DMG_TMP="$DIST_DIR/dmg_tmp"
    rm -rf "$DMG_TMP"
    mkdir -p "$DMG_TMP"
    cp -R "$APP_NAME.app" "$DMG_TMP/"
    ln -s /Applications "$DMG_TMP/Applications"
    
    hdiutil create -volname "$APP_NAME" -srcfolder "$DMG_TMP" -ov -format UDZO "$DMG_NAME" -quiet
    rm -rf "$DMG_TMP"

    # 若为 Universal 或单架构，同步生成标准化 sensidoc-v${VERSION}-macOS.dmg
    cp "$DMG_NAME" "$STD_DMG_NAME" 2>/dev/null || true
fi

echo "========================================================"
echo "✅ macOS 应用构建完成！"
echo "📦 应用程序 Bundle: $APP_BUNDLE"
if [ -f "$DIST_DIR/$STD_DMG_NAME" ]; then
    echo "💿 DMG 安装映像: $DIST_DIR/$STD_DMG_NAME"
fi
if [ -f "$DIST_DIR/$DMG_NAME" ] && [ "$DMG_NAME" != "$STD_DMG_NAME" ]; then
    echo "💿 架构专属 DMG: $DIST_DIR/$DMG_NAME"
fi
echo "========================================================"
