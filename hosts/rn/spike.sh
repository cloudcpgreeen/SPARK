#!/usr/bin/env bash
# RN spike：jco 产物能否过 Hermes 的编译门。
#
# 这一步**只证明语法/编译**，不证明运行时 —— Hermes 的 WebAssembly 是否打开、
# 能否实例化 button.core.wasm，需要真机/模拟器上的 RN app（见 README.md 的「未验证」）。
# 结论不依赖本脚本的成败：契约与组件本身一个字都没改。
set -euo pipefail
cd "$(dirname "$0")/../.."

WASM="components/button/target/wasm32-unknown-unknown/release/button.wasm"
OUT="$(mktemp -d)"
SPIKE="$(mktemp -d)"

[ -f "$WASM" ] || { echo "先跑 ./build-ui.sh"; exit 1; }

echo "==> 1. jco transpile（RN 需要 --tla-compat：Hermes 无 top-level await）"
jco transpile "$WASM" -o "$OUT" --name button \
  --tla-compat --no-namespaced-exports --no-nodejs-compat

echo "==> 2. 按 Metro 的方式打成一个 CJS bundle"
(cd "$SPIKE" && npm init -y >/dev/null && npm install --no-audit --no-fund esbuild >/dev/null 2>&1)
"$SPIKE/node_modules/.bin/esbuild" "$OUT/button.js" --bundle --format=cjs \
  --platform=neutral --outfile="$OUT/button.cjs"

echo "==> 3. Hermes 编译成字节码"
(cd "$SPIKE" && npm install --no-audit --no-fund hermes-compiler >/dev/null 2>&1)
HERMESC="$SPIKE/node_modules/hermes-compiler/hermesc/osx-bin/hermesc"
"$HERMESC" -emit-binary -out "$OUT/button.hbc" "$OUT/button.cjs"

echo
echo "PASS: Hermes 接受 jco 产物（$(wc -c < "$OUT/button.hbc") bytes 字节码）"
echo "未验证：Hermes 运行时是否提供 WebAssembly（见 README.md）"
