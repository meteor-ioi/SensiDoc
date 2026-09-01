#!/usr/bin/env bash
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd "$DIR"

echo "================================================="
echo "   SensiDoc - 离线信息审计与脱敏工具"
echo "================================================="

# 1. 确保内置目录存在
mkdir -p bin models web lib

# 2. 检查并确保 llama-server 权限
if [ -f "bin/llama-server" ]; then
    chmod +x bin/llama-server
fi

# 3. 启动 Rust 后端服务
echo "正在启动 SensiDoc 后端服务 (Axum + anydoc)..."
echo "访问地址: http://127.0.0.1:3000"
echo "按 Ctrl+C 可优雅退出并安全释放所有进程与显存。"
echo "-------------------------------------------------"

cargo run --release
