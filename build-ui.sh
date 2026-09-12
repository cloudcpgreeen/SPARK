#!/usr/bin/env bash
# 构建跨端域组件（一次），并把同一份 button.wasm 分发给各宿主。
# 用法：./build-ui.sh
#
# 关键：这里只 build 一次 Component。Web / RN 侧拿到的是 jco 的**适配产物**
# （js + core.wasm），不是第二个 Component —— 契约与制品始终唯一。
set -euo pipefail
cd "$(dirname "$0")"

echo "==> 构建域组件 components/button（唯一一次 Component build）"
(cd components/button && cargo component build --release)
WASM="$(cd components/button && find "$PWD/target/wasm32-unknown-unknown/release" -maxdepth 1 -name '*.wasm')"

echo "==> 契约自检：确认世界为 domain-world 且零 import"
npx --no-install jco wit "$WASM" | grep -E "world|export|import" || true

echo "==> jco transpile → hosts/web/src/generated（Host 适配产物，非第二个 Component）"
rm -rf hosts/web/src/generated
npx --no-install jco transpile "$WASM" -o hosts/web/src/generated --name button

echo
echo "组件产物: $WASM"
echo "Rust 后端: cargo run -p spark-host -- domain $WASM 3"
echo "Web 宿主 : cd hosts/web && npm install && npm run dev"
