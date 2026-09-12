#!/usr/bin/env bash
# 构建跨端域组件（每个一次），并把同一份 wasm 分发给各宿主。
# 用法：./build-ui.sh
#
# 关键：每个 Component 只 build 一次。Web / RN 侧拿到的是 jco 的**适配产物**
# （js + core.wasm），不是第二个 Component —— 契约与制品始终唯一。
#
# P2：`counter-store.wasm` 的 sha256 会打印出来。换 Host 的 capability 实现时，
# 这个值必须不变 —— 那就是「组件不需要重新编译」的证据。
set -euo pipefail
cd "$(dirname "$0")"

build_component() {
  local dir="$1" out="$2"
  echo "==> 构建 components/${dir}（唯一一次 Component build）" >&2
  (cd "components/$dir" && cargo component build --release)
  echo "$PWD/components/$dir/target/wasm32-unknown-unknown/release/${out}.wasm"
}

BUTTON="$(build_component button button)"
STORE="$(build_component counter-store counter_store)"

echo
echo "==> 契约自检（世界 + import/export）"
npx --no-install jco wit "$BUTTON" | grep -E "^world|  (world|export|import)" || true
npx --no-install jco wit "$STORE" | grep -E "^world|  (world|export|import)" || true

echo
echo "==> 制品 sha256（P2 判据：换 Host 实现时 STORE 的值不变）"
echo "button.wasm        $(shasum -a 256 "$BUTTON" | cut -d' ' -f1)"
echo "counter-store.wasm $(shasum -a 256 "$STORE" | cut -d' ' -f1)"

echo
echo "==> jco transpile → hosts/web/src/generated/<name>（Host 适配产物，非第二个 Component）"
rm -rf hosts/web/src/generated
mkdir -p hosts/web/src/generated
npx --no-install jco transpile "$BUTTON" -o hosts/web/src/generated/button --name button
# --map 生成的是**字面相对 import**，所以 capability 实现必须落在产物目录旁边。
npx --no-install jco transpile "$STORE" -o hosts/web/src/generated/store --name counter-store \
  --map 'spark:capability/storage=./storage.js'
cp hosts/web/src/capability/storage.js hosts/web/src/generated/store/storage.js

echo
echo "组件产物: $BUTTON"
echo "          $STORE"
echo "Rust 后端: cargo run -p spark-host -- domain $BUTTON 3"
echo "           cargo run -p spark-host -- store $STORE 3        # 健康存储 → reloaded: 3"
echo "           cargo run -p spark-host -- store $STORE 3 --deny # 拒绝写入 → reloaded: 0"
echo "Web 宿主 : cd hosts/web && npm install && npm run dev"
